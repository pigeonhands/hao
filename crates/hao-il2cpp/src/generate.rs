//! Generates declaration assemblies ("dummy DLLs") from Il2Cpp metadata: every
//! type, field, method signature, property and event, with stub method bodies.

use std::collections::HashMap;

use hao::{
    dotnet::{
        entries::MemberRef,
        entries::{
            AssemblyRef, CANamedArgument, CAType, CAValue, Constant, CustomAttribute,
            CustomAttributeCtor, CustomAttributeValue, Entry, Event, Field, GenericParam,
            GenericParamConstraint, InterfaceImpl, MemberRefParent, Method, Param, Property,
            ResolutionScope, TypeDef, TypeDefOrRef, TypeRef, TypeSpec, Version,
        },
        il::{Instruction, MethodBody, OpCode},
        md::streams::tables_stream::{
            EventFlags, FieldFlags, GenericParamFlags, MethodFlags, MethodImplFlags, ParamFlags,
            PropertyFlags, TypeAttributes,
        },
        module::{ModuleKind, TargetRuntime},
        signature::{
            ArrayShape, CallingConvention, ElementType, FieldSig, MemberRefSig, MethodSig,
            PrimitiveType, PropertySig, SignatureFlags, TypeSig,
        },
    },
    Module,
};

use crate::{
    binary::{element, Binary, Il2CppType, TypeKind},
    error::Result,
    metadata::{Metadata, TypeDefinitionIndex},
};

/// One generated assembly per Il2Cpp image.
#[cfg_attr(feature = "reflect", derive(facet::Facet))]
pub struct GeneratedAssembly {
    /// The image name, e.g. `Assembly-CSharp.dll`.
    pub file_name: String,
    pub module: Module,
}

struct Generator<'a> {
    metadata: &'a Metadata,
    binary: &'a Binary,
    modules: Vec<Module>,
    types: Vec<Entry<TypeDef>>,
    type_image: Vec<usize>,
    type_refs: Vec<HashMap<TypeDefinitionIndex, Entry<TypeRef>>>,
    primitive_types: HashMap<u8, TypeDefinitionIndex>,
    default_member: Option<TypeDefinitionIndex>,
    /// `attribute_data_range` entries `(token, start_offset)`, in global order.
    attribute_ranges: Vec<(u32, u32)>,
    /// Per image, a map from a metadata token to its index into `attribute_ranges`.
    /// Tokens are only unique within an image, so the lookup is scoped per image.
    attribute_tokens: Vec<HashMap<u32, usize>>,
}

/// A fixed constructor argument recovered from the attribute blob, or `Skip` when
/// the value was parsed (so the stream stays aligned) but can't be represented
/// faithfully — in which case the whole attribute is dropped rather than guessed.
enum Arg {
    Value(CAValue),
    Skip,
}

/// The il2cpp attribute blob tags that aren't plain ECMA element types.
const TAG_ENUM: u8 = 0x55;
const TAG_TYPE_INDEX: u8 = 0xFF;

fn read_compressed_u32(data: &[u8], position: &mut usize) -> Option<u32> {
    let mut next = || {
        let b = *data.get(*position)?;
        *position += 1;
        Some(b as u32)
    };
    let b = next()?;
    Some(if b & 0x80 == 0 {
        b
    } else if b & 0xC0 == 0x80 {
        ((b & 0x3F) << 8) | next()?
    } else if b & 0xE0 == 0xC0 {
        ((b & 0x1F) << 24) | (next()? << 16) | (next()? << 8) | next()?
    } else if b == 0xF0 {
        u32::from_le_bytes([next()? as u8, next()? as u8, next()? as u8, next()? as u8])
    } else if b == 0xFE {
        u32::MAX - 1
    } else if b == 0xFF {
        u32::MAX
    } else {
        return None;
    })
}

fn read_compressed_i32(data: &[u8], position: &mut usize) -> Option<i32> {
    let encoded = read_compressed_u32(data, position)?;
    if encoded == u32::MAX {
        return Some(i32::MIN);
    }
    let value = (encoded >> 1) as i32;
    Some(if encoded & 1 != 0 { -value - 1 } else { value })
}

fn read_u8(data: &[u8], position: &mut usize) -> Option<u8> {
    let b = *data.get(*position)?;
    *position += 1;
    Some(b)
}

fn read_fixed<const N: usize>(data: &[u8], position: &mut usize) -> Option<[u8; N]> {
    let bytes = data.get(*position..*position + N)?;
    *position += N;
    Some(bytes.try_into().unwrap())
}

/// A serialized string (II.23.3): a compressed length, or `-1` for `null`.
fn read_ser_string(data: &[u8], position: &mut usize) -> Option<Option<String>> {
    let len = read_compressed_i32(data, position)?;
    if len < 0 {
        return Some(None);
    }
    let len = len as usize;
    let bytes = data.get(*position..*position + len)?;
    *position += len;
    Some(Some(String::from_utf8_lossy(bytes).into_owned()))
}

/// Whether a constructor parameter would need ECMA boxing to serialize (an
/// `object` or `object[]` parameter). We don't emit boxed fixed arguments, so such
/// attributes are dropped rather than mis-encoded.
fn needs_boxing(sig: &TypeSig) -> bool {
    match sig.unmodified() {
        TypeSig::Primitive(PrimitiveType::Object) => true,
        TypeSig::Class(t) => t.full_name_is("System", "Object"),
        TypeSig::SzArray(element) => needs_boxing(element),
        _ => false,
    }
}

/// Decodes a constant from the default value data (v29 compresses 32 bit values and string lengths).
fn constant(element_type: u8, data: &[u8]) -> Option<Constant> {
    let mut position = 0;
    let fixed = |n: usize| data.get(..n).map(<[u8]>::to_vec);
    let value = match element_type {
        0x02 | 0x04 | 0x05 => fixed(1)?,
        0x03 | 0x06 | 0x07 => fixed(2)?,
        0x08 => read_compressed_i32(data, &mut position)?
            .to_le_bytes()
            .to_vec(),
        0x09 => read_compressed_u32(data, &mut position)?
            .to_le_bytes()
            .to_vec(),
        0x0A | 0x0B | 0x0D => fixed(8)?,
        0x0C => fixed(4)?,
        element::STRING => {
            let length = read_compressed_i32(data, &mut position)?;
            if length < 0 {
                return None;
            }
            let bytes = data.get(position..position + length as usize)?;
            let text = std::str::from_utf8(bytes).ok()?;
            text.encode_utf16().flat_map(u16::to_le_bytes).collect()
        }
        _ => return None,
    };
    Some(Constant {
        element_type: ElementType::from_repr(element_type)?,
        value,
    })
}

impl<'a> Generator<'a> {
    fn new(metadata: &'a Metadata, binary: &'a Binary) -> Self {
        let mut type_image = vec![0; metadata.type_definitions.len()];
        let modules = metadata
            .images
            .iter()
            .enumerate()
            .map(|(i, image)| {
                for t in metadata.image_types(image) {
                    if let Some(slot) = type_image.get_mut(t as usize) {
                        *slot = i;
                    }
                }
                let assembly = &metadata.assemblies[image.assembly_index as usize];
                let file_name = metadata.image_name(image);
                let name = metadata.string(assembly.name_index);
                let mut module = Module::new(name, ModuleKind::Dll, TargetRuntime::NetFramework);
                module.module.get_mut().name = file_name.to_owned();
                if let Some(manifest) = &module.assembly {
                    let mut manifest = manifest.get_mut();
                    manifest.version = version(assembly.version);
                }
                if name == "mscorlib" {
                    module.assembly_refs.clear();
                }
                module.types.clear();
                module
            })
            .collect::<Vec<_>>();

        let names = [
            (0x01, "Void"),
            (0x02, "Boolean"),
            (0x03, "Char"),
            (0x04, "SByte"),
            (0x05, "Byte"),
            (0x06, "Int16"),
            (0x07, "UInt16"),
            (0x08, "Int32"),
            (0x09, "UInt32"),
            (0x0A, "Int64"),
            (0x0B, "UInt64"),
            (0x0C, "Single"),
            (0x0D, "Double"),
            (0x0E, "String"),
            (0x16, "TypedReference"),
            (0x18, "IntPtr"),
            (0x19, "UIntPtr"),
            (0x1C, "Object"),
        ];
        let mut primitive_types = HashMap::new();
        for (i, t) in metadata.type_definitions.iter().enumerate() {
            if metadata.string(t.namespace_index) == "System" && t.declaring_type_index < 0 {
                let name = metadata.string(t.name_index);
                if let Some((element, _)) = names.iter().find(|(_, n)| *n == name) {
                    primitive_types.insert(*element, i as TypeDefinitionIndex);
                }
            }
        }

        let default_member = (0..metadata.type_definitions.len() as i32)
            .find(|i| metadata.type_full_name(*i) == "System.Reflection.DefaultMemberAttribute");

        // Custom attribute data (v29+): the global range table maps each entity's
        // metadata token to an offset into the `attribute_data` blob. The ranges are
        // partitioned per image; a token is only unique within its image.
        let attribute_ranges = metadata.attribute_data_ranges();
        let attribute_tokens = metadata
            .images
            .iter()
            .map(|image| {
                let start = image.custom_attribute_start.max(0) as usize;
                // v29 stores the count; v39 (Unity 6) leaves it zero, so no custom
                // attributes are recovered there until its image layout is confirmed.
                let end = start
                    .saturating_add(image.custom_attribute_count as usize)
                    .min(attribute_ranges.len());
                (start.min(end)..end)
                    .map(|i| (attribute_ranges[i].0, i))
                    .collect()
            })
            .collect();

        Self {
            default_member,
            type_refs: vec![HashMap::new(); modules.len()],
            modules,
            types: Vec::new(),
            type_image,
            primitive_types,
            attribute_ranges,
            attribute_tokens,
            metadata,
            binary,
        }
    }

    fn assembly_ref(&mut self, module: usize, image: usize) -> Entry<AssemblyRef> {
        let image = &self.metadata.images[image];
        let assembly = &self.metadata.assemblies[image.assembly_index as usize];
        let token = if assembly.public_key_token == [0; 8] {
            Vec::new()
        } else {
            assembly.public_key_token.to_vec()
        };
        self.modules[module].add_assembly_ref(
            self.metadata.string(assembly.name_index),
            version(assembly.version),
            &token,
        )
    }

    /// A reference from `module` to type definition `index`.
    fn type_ref(&mut self, module: usize, index: TypeDefinitionIndex) -> TypeDefOrRef {
        if self.type_image[index as usize] == module {
            return TypeDefOrRef::TypeDef(self.types[index as usize].clone());
        }
        if let Some(existing) = self.type_refs[module].get(&index) {
            return TypeDefOrRef::TypeRef(existing.clone());
        }
        let def = &self.metadata.type_definitions[index as usize];
        let name = self.metadata.string(def.name_index).to_owned();
        let scope = match self.metadata.declaring_type_definition(index) {
            Some(parent) => match self.type_ref(module, parent) {
                TypeDefOrRef::TypeRef(r) => ResolutionScope::TypeRef(r),
                _ => unreachable!("the parent is in another image"),
            },
            None => ResolutionScope::AssemblyRef(
                self.assembly_ref(module, self.type_image[index as usize]),
            ),
        };
        let namespace = if matches!(scope, ResolutionScope::TypeRef(_)) {
            String::new()
        } else {
            self.metadata.string(def.namespace_index).to_owned()
        };
        let type_ref = Entry::new(TypeRef::new(scope, namespace, name));
        self.type_refs[module].insert(index, type_ref.clone());
        TypeDefOrRef::TypeRef(type_ref)
    }

    fn generic_number(&self, index: i32) -> u32 {
        self.metadata
            .generic_parameters
            .get(index as usize)
            .map_or(0, |g| g.num as u32)
    }

    fn sig(&mut self, module: usize, ty: &Il2CppType) -> TypeSig {
        let sig = match &ty.kind {
            TypeKind::Primitive(e) => match PrimitiveType::from_repr(*e) {
                Some(p) => TypeSig::Primitive(p),
                None => TypeSig::OBJECT,
            },
            TypeKind::Class(t) => TypeSig::Class(self.type_ref(module, *t)),
            TypeKind::ValueType(t) => TypeSig::ValueType(self.type_ref(module, *t)),
            TypeKind::SzArray(e) => TypeSig::SzArray(Box::new(self.sig(module, e))),
            TypeKind::Array { element, rank } => TypeSig::Array {
                element: Box::new(self.sig(module, element)),
                shape: ArrayShape {
                    rank: (*rank).max(1) as u32,
                    ..Default::default()
                },
            },
            TypeKind::Ptr(e) => TypeSig::Ptr(Box::new(self.sig(module, e))),
            TypeKind::GenericInst { generic, args } => TypeSig::GenericInst {
                generic: Box::new(self.sig(module, generic)),
                args: args.iter().map(|a| self.sig(module, a)).collect(),
            },
            TypeKind::Var(i) => TypeSig::Var(self.generic_number(*i)),
            TypeKind::MVar(i) => TypeSig::MVar(self.generic_number(*i)),
            TypeKind::FnPtr | TypeKind::Unknown(_) => TypeSig::Primitive(PrimitiveType::I),
        };
        if ty.byref {
            TypeSig::ByRef(Box::new(sig))
        } else {
            sig
        }
    }

    fn sig_at(&mut self, module: usize, index: i32) -> TypeSig {
        match self.binary.type_at(index).cloned() {
            Some(ty) => self.sig(module, &ty),
            None => TypeSig::OBJECT,
        }
    }

    fn type_def_or_ref(&mut self, module: usize, index: i32) -> Option<TypeDefOrRef> {
        let ty = self.binary.type_at(index)?.clone();
        Some(match &ty.kind {
            TypeKind::Class(t) | TypeKind::ValueType(t) => self.type_ref(module, *t),
            TypeKind::Primitive(e) => self.type_ref(module, *self.primitive_types.get(e)?),
            _ => {
                let signature = self.sig(module, &ty);
                TypeDefOrRef::TypeSpec(Entry::new(TypeSpec {
                    signature,
                    custom_attributes: Vec::new(),
                }))
            }
        })
    }

    fn generic_params(&mut self, module: usize, container: i32) -> Vec<Entry<GenericParam>> {
        let params = self
            .metadata
            .generic_container_parameters(container)
            .to_vec();
        params
            .iter()
            .map(|g| {
                let start = g.constraints_start.max(0) as usize;
                let constraints = self
                    .metadata
                    .generic_parameter_constraints
                    .get(start..start + g.constraints_count.max(0) as usize)
                    .unwrap_or(&[])
                    .to_vec()
                    .into_iter()
                    .filter_map(|c| self.type_def_or_ref(module, c))
                    .map(|constraint| GenericParamConstraint {
                        constraint,
                        custom_attributes: Vec::new(),
                    })
                    .collect();
                Entry::new(GenericParam {
                    number: g.num,
                    flags: GenericParamFlags::from_bits_retain(g.flags),
                    name: self.metadata.string(g.name_index).to_owned(),
                    constraints,
                    custom_attributes: Vec::new(),
                })
            })
            .collect()
    }

    /// `[DefaultMember(name)]`, which C# needs to see indexers.
    fn default_member_attribute(&mut self, module: usize, name: &str) -> Option<CustomAttribute> {
        let index = self.default_member?;
        let signature = MethodSig::new_instance(TypeSig::VOID, vec![TypeSig::STRING]);
        let constructor = match self.type_ref(module, index) {
            TypeDefOrRef::TypeDef(def) => {
                let ctor = def
                    .get()
                    .methods
                    .iter()
                    .find(|m| m.get().is_ctor() && m.get().signature == signature)
                    .cloned()?;
                CustomAttributeCtor::Method(ctor)
            }
            TypeDefOrRef::TypeRef(r) => CustomAttributeCtor::MemberRef(Entry::new(MemberRef {
                parent: hao::dotnet::entries::MemberRefParent::TypeRef(r),
                name: ".ctor".into(),
                signature: MemberRefSig::Method(signature),
                custom_attributes: Vec::new(),
            })),
            TypeDefOrRef::TypeSpec(_) => return None,
        };
        let value = CustomAttributeValue {
            fixed: vec![name.into()],
            named: Vec::new(),
        };
        CustomAttribute::new(constructor, &value).ok()
    }

    // ---- Recovered custom attributes (v29+) --------------------------------
    //
    // il2cpp v29 reintroduced per-token attribute data into metadata (earlier
    // versions kept only the attribute *types*, with arguments baked into
    // generated C++). The `attribute_data` blob stores, for each token, the
    // applied attributes with their constructor index and full argument values,
    // and `attribute_data_range` maps each token to its slice of that blob. We
    // reconstruct the attributes faithfully from this, re-encoding into ECMA-335
    // custom attribute blobs. Fidelity caveats (we drop rather than guess):
    //  * `object`/`object[]` constructor parameters would need ECMA boxing we
    //    don't emit, so those attributes are skipped (`needs_boxing`).
    //  * named arguments are typed by their recovered *value* (correct unless the
    //    field/property is declared `object`, where the static type is lost).
    //  * generic/array `System.Type` arguments and nested-array element types
    //    aren't reconstructed, so such attributes are skipped.
    // v39 (Unity 6) uses the same blob format, but its image record doesn't yet
    // expose the per-image attribute count, so nothing is recovered there.

    /// The `attribute_data` bytes for `token` in `module`'s image, if any.
    fn attribute_blob(&self, module: usize, token: u32) -> Option<Vec<u8>> {
        let index = *self.attribute_tokens.get(module)?.get(&token)?;
        let blob = self.metadata.attribute_data();
        let start = self.attribute_ranges[index].1 as usize;
        let end = self
            .attribute_ranges
            .get(index + 1)
            .map_or(blob.len(), |r| r.1 as usize)
            .clamp(start, blob.len());
        blob.get(start..end).map(<[u8]>::to_vec)
    }

    /// Resolves an `Il2CppType` index to the type definition it names, if it is a
    /// plain class or value type.
    fn type_def_index_of(&self, il_type_index: i32) -> Option<TypeDefinitionIndex> {
        match &self.binary.type_at(il_type_index)?.kind {
            TypeKind::Class(d) | TypeKind::ValueType(d) => Some(*d),
            _ => None,
        }
    }

    /// The ECMA element type byte backing an enum's values.
    fn enum_underlying(&self, def: TypeDefinitionIndex) -> Option<u8> {
        let ty = self.metadata.type_definitions.get(def as usize)?;
        // v29 points `element_type_index` at the underlying primitive; v39 drops
        // it, so fall back to the enum's instance field (`value__`).
        if ty.element_type_index >= 0 {
            if let Some(TypeKind::Primitive(e)) =
                self.binary.type_at(ty.element_type_index).map(|t| &t.kind)
            {
                return Some(*e);
            }
        }
        match &self.binary.type_at(self.metadata.type_fields(ty).first()?.type_index)?.kind {
            TypeKind::Primitive(e) => Some(*e),
            _ => None,
        }
    }

    /// `Namespace.Outer+Inner`, the reflection form used in serialized attributes.
    fn serialized_type_name(&self, def: TypeDefinitionIndex) -> Option<String> {
        if def < 0 || def as usize >= self.metadata.type_definitions.len() {
            return None;
        }
        Some(self.metadata.type_full_name(def).replace('/', "+"))
    }

    /// A reflection type name for a `System.Type` argument's `Il2CppType` index.
    fn type_argument_name(&self, il_type_index: i32) -> Option<String> {
        match &self.binary.type_at(il_type_index)?.kind {
            TypeKind::Class(d) | TypeKind::ValueType(d) => self.serialized_type_name(*d),
            TypeKind::Primitive(e) => self.serialized_type_name(*self.primitive_types.get(e)?),
            _ => None,
        }
    }

    /// Reads a self-describing type tag (`ReadEncodedTypeEnum`): the element type
    /// byte, and the enum's definition when the tag is an enum (the byte is then the
    /// enum's underlying primitive). `None` when an enum can't be resolved, as its
    /// value width is then unknown.
    fn read_encoded_type(
        &self,
        blob: &[u8],
        pos: &mut usize,
    ) -> Option<(u8, Option<TypeDefinitionIndex>)> {
        let tag = read_u8(blob, pos)?;
        if tag == TAG_ENUM {
            let type_index = read_compressed_i32(blob, pos)?;
            let def = self.type_def_index_of(type_index)?;
            Some((self.enum_underlying(def)?, Some(def)))
        } else {
            Some((tag, None))
        }
    }

    /// Reads one attribute argument (`ReadAttributeDataValue`), always advancing
    /// past a well-formed value. `Arg::Skip` marks a value that parsed but can't be
    /// represented, so the attribute is dropped without mis-encoding it.
    fn read_arg(&self, blob: &[u8], pos: &mut usize) -> Option<Arg> {
        let (tag, enum_def) = self.read_encoded_type(blob, pos)?;
        self.read_value(blob, pos, tag, enum_def)
    }

    fn read_value(
        &self,
        blob: &[u8],
        pos: &mut usize,
        tag: u8,
        enum_def: Option<TypeDefinitionIndex>,
    ) -> Option<Arg> {
        use element::*;
        if let Some(def) = enum_def {
            // Read the underlying value, then wrap it with the enum's name.
            let Arg::Value(inner) = self.read_value(blob, pos, tag, None)? else {
                return Some(Arg::Skip);
            };
            return Some(match self.serialized_type_name(def) {
                Some(type_name) => Arg::Value(CAValue::Enum {
                    type_name,
                    value: Box::new(inner),
                }),
                None => Arg::Skip,
            });
        }
        let value = match tag {
            BOOLEAN => CAValue::Bool(read_u8(blob, pos)? != 0),
            I1 => CAValue::I1(read_u8(blob, pos)? as i8),
            U1 => CAValue::U1(read_u8(blob, pos)?),
            CHAR => CAValue::Char(u16::from_le_bytes(read_fixed(blob, pos)?)),
            I2 => CAValue::I2(i16::from_le_bytes(read_fixed(blob, pos)?)),
            U2 => CAValue::U2(u16::from_le_bytes(read_fixed(blob, pos)?)),
            I4 => CAValue::I4(read_compressed_i32(blob, pos)?),
            U4 => CAValue::U4(read_compressed_u32(blob, pos)?),
            I8 => CAValue::I8(i64::from_le_bytes(read_fixed(blob, pos)?)),
            U8 => CAValue::U8(u64::from_le_bytes(read_fixed(blob, pos)?)),
            R4 => CAValue::R4(f32::from_le_bytes(read_fixed(blob, pos)?)),
            R8 => CAValue::R8(f64::from_le_bytes(read_fixed(blob, pos)?)),
            STRING => CAValue::String(read_ser_string(blob, pos)?),
            TAG_TYPE_INDEX => {
                let type_index = read_compressed_i32(blob, pos)?;
                if type_index < 0 {
                    CAValue::Type(None)
                } else {
                    match self.type_argument_name(type_index) {
                        Some(name) => CAValue::Type(Some(name)),
                        None => return Some(Arg::Skip),
                    }
                }
            }
            SZARRAY => return self.read_array(blob, pos),
            _ => return None,
        };
        Some(Arg::Value(value))
    }

    fn read_array(&self, blob: &[u8], pos: &mut usize) -> Option<Arg> {
        let len = read_compressed_i32(blob, pos)?;
        if len < 0 {
            return Some(Arg::Value(CAValue::Array {
                element: CAType::Object,
                values: None,
            }));
        }
        let (base_tag, base_enum) = self.read_encoded_type(blob, pos)?;
        let different = read_u8(blob, pos)? == 1;
        let mut values = Vec::new();
        let mut skip = false;
        for _ in 0..len as usize {
            let (tag, enum_def) = if different {
                self.read_encoded_type(blob, pos)?
            } else {
                (base_tag, base_enum)
            };
            match self.read_value(blob, pos, tag, enum_def)? {
                Arg::Value(v) => values.push(v),
                Arg::Skip => skip = true,
            }
        }
        if skip {
            return Some(Arg::Skip);
        }
        Some(match self.array_element_type(base_tag, base_enum) {
            Some(element) => Arg::Value(CAValue::Array {
                element,
                values: Some(values),
            }),
            None => Arg::Skip,
        })
    }

    fn array_element_type(&self, tag: u8, enum_def: Option<TypeDefinitionIndex>) -> Option<CAType> {
        use element::*;
        if let Some(def) = enum_def {
            return Some(CAType::Enum {
                type_name: self.serialized_type_name(def)?,
                underlying: PrimitiveType::from_repr(tag)?,
            });
        }
        Some(match tag {
            TAG_TYPE_INDEX => CAType::Type,
            BOOLEAN | CHAR | I1 | U1 | I2 | U2 | I4 | U4 | I8 | U8 | R4 | R8 | STRING => {
                CAType::Primitive(PrimitiveType::from_repr(tag)?)
            }
            _ => return None,
        })
    }

    /// The name of a named-argument field or property, from its declaring type and
    /// member index (`ReadCustomAttributeNamedArgumentClassAndIndex`). Always
    /// advances; the inner `None` means the member couldn't be resolved.
    fn read_named_member(
        &self,
        blob: &[u8],
        pos: &mut usize,
        ctor_type: Option<TypeDefinitionIndex>,
        is_field: bool,
    ) -> Option<Option<String>> {
        let member = read_compressed_i32(blob, pos)?;
        let (decl, index) = if member >= 0 {
            (ctor_type, member as usize)
        } else {
            let type_index = read_compressed_u32(blob, pos)? as TypeDefinitionIndex;
            (Some(type_index), (-(member + 1)) as usize)
        };
        Some(decl.and_then(|d| self.member_name(d, index, is_field)))
    }

    fn member_name(
        &self,
        decl: TypeDefinitionIndex,
        index: usize,
        is_field: bool,
    ) -> Option<String> {
        let ty = self.metadata.type_definitions.get(usize::try_from(decl).ok()?)?;
        let name_index = if is_field {
            let start = ty.field_start.max(0) as usize;
            self.metadata.fields.get(start + index)?.name_index
        } else {
            let start = ty.property_start.max(0) as usize;
            self.metadata.properties.get(start + index)?.name_index
        };
        Some(self.metadata.string(name_index).to_owned())
    }

    /// Parses one attribute's argument block: the fixed arguments, then the named
    /// fields and properties. Must stay byte-aligned, so returns `None` (stopping
    /// the token) on malformed data.
    #[allow(clippy::type_complexity)]
    fn parse_arguments(
        &self,
        blob: &[u8],
        pos: &mut usize,
        ctor_type: Option<TypeDefinitionIndex>,
    ) -> Option<(Vec<Arg>, Vec<(bool, String, Arg)>)> {
        let arg_count = read_compressed_u32(blob, pos)?;
        let field_count = read_compressed_u32(blob, pos)?;
        let property_count = read_compressed_u32(blob, pos)?;
        let mut fixed = Vec::new();
        for _ in 0..arg_count {
            fixed.push(self.read_arg(blob, pos)?);
        }
        let mut named = Vec::new();
        for (is_field, count) in [(true, field_count), (false, property_count)] {
            for _ in 0..count {
                let value = self.read_arg(blob, pos)?;
                match self.read_named_member(blob, pos, ctor_type, is_field)? {
                    Some(name) => named.push((is_field, name, value)),
                    None => named.push((is_field, String::new(), Arg::Skip)),
                }
            }
        }
        Some((fixed, named))
    }

    /// The custom attributes applied to `token` in `module`, reconstructed from the
    /// v29+ `attribute_data` blob.
    fn custom_attributes(&mut self, module: usize, token: u32) -> Vec<CustomAttribute> {
        let Some(blob) = self.attribute_blob(module, token) else {
            return Vec::new();
        };
        let mut pos = 0;
        let Some(count) = read_compressed_u32(&blob, &mut pos) else {
            return Vec::new();
        };
        let ctor_base = pos;
        let mut data_pos = ctor_base + count as usize * 4;
        let mut out = Vec::new();
        for i in 0..count as usize {
            let Some(ctor_bytes) = blob.get(ctor_base + i * 4..ctor_base + i * 4 + 4) else {
                break;
            };
            let ctor_index = i32::from_le_bytes(ctor_bytes.try_into().unwrap());
            let ctor_type = self
                .metadata
                .methods
                .get(usize::try_from(ctor_index).unwrap_or(usize::MAX))
                .map(|m| m.declaring_type);
            let Some((fixed, named)) = self.parse_arguments(&blob, &mut data_pos, ctor_type) else {
                break;
            };
            if let Some(attribute) = self.build_attribute(module, ctor_index, fixed, named) {
                out.push(attribute);
            }
        }
        out
    }

    fn build_attribute(
        &mut self,
        module: usize,
        ctor_index: i32,
        fixed: Vec<Arg>,
        named: Vec<(bool, String, Arg)>,
    ) -> Option<CustomAttribute> {
        let method = self
            .metadata
            .methods
            .get(usize::try_from(ctor_index).ok()?)?
            .clone();
        let decl = method.declaring_type;
        let params: Vec<TypeSig> = self
            .metadata
            .method_parameters(&method)
            .to_vec()
            .iter()
            .map(|p| self.sig_at(module, p.type_index))
            .collect();
        if params.len() != fixed.len() || params.iter().any(needs_boxing) {
            return None;
        }
        let fixed = fixed
            .into_iter()
            .map(|a| match a {
                Arg::Value(v) => Some(v),
                Arg::Skip => None,
            })
            .collect::<Option<Vec<_>>>()?;
        let named = named
            .into_iter()
            .map(|(is_field, name, a)| match a {
                Arg::Value(v) if is_field => Some(CANamedArgument::field(name, v)),
                Arg::Value(v) => Some(CANamedArgument::property(name, v)),
                Arg::Skip => None,
            })
            .collect::<Option<Vec<_>>>()?;
        let constructor = self.attribute_ctor(module, decl, ctor_index, params)?;
        CustomAttribute::new(constructor, &CustomAttributeValue { fixed, named }).ok()
    }

    fn attribute_ctor(
        &mut self,
        module: usize,
        decl: TypeDefinitionIndex,
        ctor_index: i32,
        params: Vec<TypeSig>,
    ) -> Option<CustomAttributeCtor> {
        match self.type_ref(module, decl) {
            TypeDefOrRef::TypeDef(def) => {
                let method_start = self.metadata.type_definitions.get(decl as usize)?.method_start;
                let local = usize::try_from(ctor_index - method_start).ok()?;
                Some(CustomAttributeCtor::Method(
                    def.get().methods.get(local)?.clone(),
                ))
            }
            TypeDefOrRef::TypeRef(r) => Some(CustomAttributeCtor::MemberRef(Entry::new(MemberRef {
                parent: MemberRefParent::TypeRef(r),
                name: ".ctor".into(),
                signature: MemberRefSig::Method(MethodSig::new_instance(TypeSig::VOID, params)),
                custom_attributes: Vec::new(),
            }))),
            TypeDefOrRef::TypeSpec(_) => None,
        }
    }

    fn create_types(&mut self) {
        self.types = (0..self.metadata.type_definitions.len())
            .map(|i| {
                let def = &self.metadata.type_definitions[i];
                Entry::new(TypeDef {
                    flags: TypeAttributes::from_bits_retain(def.flags),
                    name: self.metadata.string(def.name_index).to_owned(),
                    namespace: self.metadata.string(def.namespace_index).to_owned(),
                    ..Default::default()
                })
            })
            .collect();

        for (i, def) in self.metadata.type_definitions.iter().enumerate() {
            let start = def.nested_types_start.max(0) as usize;
            let nested = self
                .metadata
                .nested_types
                .get(start..start + def.nested_type_count as usize)
                .unwrap_or(&[]);
            for n in nested {
                let child = &self.types[*n as usize];
                child.get_mut().declaring_type = self.types[i].downgrade();
                child.get_mut().namespace.clear();
                self.types[i].get_mut().nested_types.push(child.clone());
            }
        }

        // `<Module>` first, as metadata requires.
        for (i, ty) in self.types.iter().enumerate() {
            if ty.get().declaring_type.upgrade().is_none() {
                let module = &mut self.modules[self.type_image[i]];
                if ty.get().name == "<Module>" {
                    module.types.insert(0, ty.clone());
                } else {
                    module.types.push(ty.clone());
                }
            }
        }
        for module in &mut self.modules {
            if module
                .types
                .first()
                .is_none_or(|t| t.get().name != "<Module>")
            {
                module.types.insert(
                    0,
                    Entry::new(TypeDef::new(TypeAttributes::empty(), "", "<Module>", None)),
                );
            }
        }
    }

    fn fill_type(&mut self, index: usize) {
        let module = self.type_image[index];
        let def = self.metadata.type_definitions[index].clone();
        let entry = self.types[index].clone();

        let is_interface = def.flags & TypeAttributes::ClassSemanticsMask.bits()
            == TypeAttributes::Interface.bits();
        let extends = if def.parent_index >= 0 && !is_interface {
            self.type_def_or_ref(module, def.parent_index)
        } else {
            None
        };
        let generic_params = self.generic_params(module, def.generic_container_index);
        let start = def.interfaces_start.max(0) as usize;
        let interfaces = self
            .metadata
            .interfaces
            .get(start..start + def.interfaces_count as usize)
            .unwrap_or(&[])
            .to_vec()
            .into_iter()
            .filter_map(|i| self.type_def_or_ref(module, i))
            .map(InterfaceImpl::new)
            .collect();
        {
            let mut t = entry.get_mut();
            t.extends = extends;
            t.generic_params = generic_params;
            t.interfaces = interfaces;
        }

        let field_start = def.field_start.max(0) as usize;
        for (offset, f) in self.metadata.type_fields(&def).to_vec().iter().enumerate() {
            let ty = self.binary.type_at(f.type_index).cloned();
            let attrs = ty.as_ref().map_or(0, |t| t.attrs);
            let mut flags = FieldFlags::from_bits_retain(attrs);
            flags.remove(
                FieldFlags::HasFieldRVA | FieldFlags::HasFieldMarshal | FieldFlags::HasDefault,
            );
            let signature = FieldSig {
                ty: ty.map(|t| self.sig(module, &t)).unwrap_or(TypeSig::OBJECT),
            };
            let mut field = Field::new(flags, self.metadata.string(f.name_index), signature);
            let field_index = (field_start + offset) as i32;
            if let Some(default) = self
                .metadata
                .field_default_values
                .iter()
                .find(|d| d.field_index == field_index)
            {
                let element = self
                    .binary
                    .type_at(default.type_index)
                    .and_then(|t| match t.kind {
                        TypeKind::Primitive(e) => Some(e),
                        _ => None,
                    });
                field.constant = element.and_then(|e| {
                    constant(e, self.metadata.default_value_data(default.data_index))
                });
                if field.constant.is_some() {
                    field.flags.insert(FieldFlags::HasDefault);
                }
            }
            TypeDef::add_field(&entry, field);
        }

        let mut methods = Vec::new();
        for m in self.metadata.type_methods(&def).to_vec() {
            let mut flags = MethodFlags::from_bits_retain(m.flags);
            let impl_flags = MethodImplFlags::from_bits_retain(m.iflags);
            let parameters = self.metadata.method_parameters(&m).to_vec();
            let params: Vec<TypeSig> = parameters
                .iter()
                .map(|p| self.sig_at(module, p.type_index))
                .collect();
            let generic_params = self.generic_params(module, m.generic_container_index);
            let signature = MethodSig {
                calling_convention: CallingConvention::Default,
                flags: if flags.contains(MethodFlags::Static) {
                    SignatureFlags::empty()
                } else {
                    SignatureFlags::HAS_THIS
                },
                generic_param_count: generic_params.len() as u32,
                ret: self.sig_at(module, m.return_type),
                params,
                varargs: Vec::new(),
            };
            flags.remove(MethodFlags::PinvokeImpl | MethodFlags::UnmanagedExport);
            let mut method = Method::new(flags, self.metadata.string(m.name_index), signature);
            method.impl_flags = impl_flags;
            method.generic_params = generic_params;
            method.params = parameters
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let attrs = self.binary.type_at(p.type_index).map_or(0, |t| t.attrs);
                    let mut flags = ParamFlags::from_bits_retain(attrs);
                    flags.remove(ParamFlags::HasDefault | ParamFlags::HasFieldMarshal);
                    Entry::new(Param {
                        flags,
                        ..Param::new(i as u16 + 1, self.metadata.string(p.name_index))
                    })
                })
                .collect();
            let code_type = impl_flags & MethodImplFlags::CodeTypeMask;
            let has_body = !flags.contains(MethodFlags::Abstract)
                && code_type != MethodImplFlags::Runtime
                && !impl_flags.contains(MethodImplFlags::InternalCall);
            if has_body {
                method.body = Some(MethodBody {
                    instructions: vec![
                        Instruction::simple(OpCode::Ldnull),
                        Instruction::simple(OpCode::Throw),
                    ],
                    ..Default::default()
                });
            }
            methods.push(TypeDef::add_method(&entry, method));
        }

        let property_start = def.property_start.max(0) as usize;
        for p in self
            .metadata
            .properties
            .get(property_start..property_start + def.property_count as usize)
            .unwrap_or(&[])
            .to_vec()
        {
            let method = |i: i32| {
                usize::try_from(i)
                    .ok()
                    .and_then(|i| methods.get(i))
                    .cloned()
            };
            let (getter, setter) = (method(p.get), method(p.set));
            let (has_this, ty, params) = match (&getter, &setter) {
                (Some(g), _) => {
                    let g = g.get();
                    (
                        g.signature.has_this(),
                        g.signature.ret.clone(),
                        g.signature.params.clone(),
                    )
                }
                (None, Some(s)) => {
                    let s = s.get();
                    let mut params = s.signature.params.clone();
                    let ty = params.pop().unwrap_or(TypeSig::OBJECT);
                    (s.signature.has_this(), ty, params)
                }
                (None, None) => continue,
            };
            let property = Property {
                flags: PropertyFlags::from_bits_retain(p.attrs as u16),
                name: self.metadata.string(p.name_index).to_owned(),
                signature: PropertySig {
                    has_this,
                    ty,
                    params,
                },
                getter,
                setter,
                ..Default::default()
            };
            entry.get_mut().properties.push(Entry::new(property));
        }

        let event_start = def.event_start.max(0) as usize;
        for e in self
            .metadata
            .events
            .get(event_start..event_start + def.event_count as usize)
            .unwrap_or(&[])
            .to_vec()
        {
            let method = |i: i32| {
                usize::try_from(i)
                    .ok()
                    .and_then(|i| methods.get(i))
                    .cloned()
            };
            let event = Event {
                flags: EventFlags::empty(),
                name: self.metadata.string(e.name_index).to_owned(),
                event_type: self.type_def_or_ref(module, e.type_index),
                add: method(e.add),
                remove: method(e.remove),
                fire: method(e.raise),
                ..Default::default()
            };
            entry.get_mut().events.push(Entry::new(event));
        }
    }
}

fn version(v: [i32; 4]) -> Version {
    Version {
        major: v[0] as u16,
        minor: v[1] as u16,
        build: v[2] as u16,
        revision: v[3] as u16,
    }
}

/// Builds a declaration assembly for every image in the game. Method bodies are
/// stubs that throw.
pub fn generate(metadata: &Metadata, binary: &Binary) -> Result<Vec<GeneratedAssembly>> {
    let mut generator = Generator::new(metadata, binary);
    generator.create_types();
    for i in 0..metadata.type_definitions.len() {
        generator.fill_type(i);
    }
    // Attach the game's own custom attributes (recovered from the v29+ attribute
    // data) to every entity, by matching metadata tokens. Generated member lists
    // mirror the metadata order, so they zip one-to-one.
    for i in 0..metadata.type_definitions.len() {
        let module = generator.type_image[i];
        let def = metadata.type_definitions[i].clone();
        let entry = generator.types[i].clone();

        let attrs = generator.custom_attributes(module, def.token);
        entry.get_mut().custom_attributes.extend(attrs);

        let methods = entry.get().methods.clone();
        for (method, m_def) in methods.iter().zip(metadata.type_methods(&def)) {
            let attrs = generator.custom_attributes(module, m_def.token);
            method.get_mut().custom_attributes.extend(attrs);
            let params = method.get().params.clone();
            for (param, p_def) in params.iter().zip(metadata.method_parameters(m_def)) {
                let attrs = generator.custom_attributes(module, p_def.token);
                param.get_mut().custom_attributes.extend(attrs);
            }
        }

        let fields = entry.get().fields.clone();
        for (field, f_def) in fields.iter().zip(metadata.type_fields(&def)) {
            let attrs = generator.custom_attributes(module, f_def.token);
            field.get_mut().custom_attributes.extend(attrs);
        }

        let properties = entry.get().properties.clone();
        let prop_start = def.property_start.max(0) as usize;
        let prop_defs = metadata
            .properties
            .get(prop_start..prop_start + def.property_count as usize)
            .unwrap_or(&[]);
        for (property, pr_def) in properties.iter().zip(prop_defs) {
            let attrs = generator.custom_attributes(module, pr_def.token);
            property.get_mut().custom_attributes.extend(attrs);
        }

        let events = entry.get().events.clone();
        let event_start = def.event_start.max(0) as usize;
        let event_defs = metadata
            .events
            .get(event_start..event_start + def.event_count as usize)
            .unwrap_or(&[]);
        for (event, ev_def) in events.iter().zip(event_defs) {
            let attrs = generator.custom_attributes(module, ev_def.token);
            event.get_mut().custom_attributes.extend(attrs);
        }
    }
    for i in 0..metadata.type_definitions.len() {
        let entry = generator.types[i].clone();
        let has_indexer = entry
            .get()
            .properties
            .iter()
            .any(|p| !p.get().signature.params.is_empty());
        // Add the synthetic `[DefaultMember("Item")]` unless the game already
        // carries one (recovered above), so indexers compile in C#.
        let has_default_member = entry
            .get()
            .custom_attributes
            .iter()
            .any(|a| a.type_name() == "System.Reflection.DefaultMemberAttribute");
        if has_indexer && !has_default_member {
            if let Some(attribute) =
                generator.default_member_attribute(generator.type_image[i], "Item")
            {
                entry.get_mut().custom_attributes.push(attribute);
            }
        }
    }
    Ok(metadata
        .images
        .iter()
        .zip(generator.modules)
        .map(|(image, module)| GeneratedAssembly {
            file_name: metadata.image_name(image).to_owned(),
            module,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compressed_integers() {
        let decode = |bytes: &[u8]| read_compressed_i32(bytes, &mut 0).unwrap();
        assert_eq!(decode(&[0x00]), 0);
        assert_eq!(decode(&[0x02]), 1);
        assert_eq!(decode(&[0x01]), -1);
        assert_eq!(decode(&[0xFF]), i32::MIN);
        assert_eq!(
            read_compressed_u32(&[0xF0, 0x78, 0x56, 0x34, 0x12], &mut 0),
            Some(0x1234_5678)
        );
        assert_eq!(read_compressed_u32(&[0x81, 0x02], &mut 0), Some(0x102));
    }

    /// Runs against a real game. Set `HAO_IL2CPP_GAME` to an Il2Cpp game folder
    /// with a v29 `GameAssembly.so`. Skipped otherwise.
    #[test]
    fn recovers_custom_attributes() {
        let Some(dir) = std::env::var_os("HAO_IL2CPP_GAME") else {
            eprintln!("skipping: set HAO_IL2CPP_GAME to an Il2Cpp game folder");
            return;
        };
        let game = crate::game::Game::open(dir).unwrap();
        let assemblies = game.generate().unwrap();

        let mut type_attributes = std::collections::BTreeSet::new();
        let mut member_attributes = 0usize;
        let mut default_member_item = false;

        for assembly in &assemblies {
            for ty in assembly.module.all_types() {
                let t = ty.get();
                for attribute in &t.custom_attributes {
                    type_attributes.insert(attribute.type_name());
                    // The recovered DefaultMember still carries its string argument.
                    if attribute.type_name() == "System.Reflection.DefaultMemberAttribute" {
                        if let Ok(value) = attribute.decode(&assembly.module) {
                            default_member_item |=
                                value.fixed.iter().any(|v| v.as_str() == Some("Item"));
                        }
                    }
                }
                for method in &t.methods {
                    member_attributes += method.get().custom_attributes.len();
                    for param in &method.get().params {
                        member_attributes += param.get().custom_attributes.len();
                    }
                }
                member_attributes += t.fields.iter().map(|f| f.get().custom_attributes.len()).sum::<usize>();
            }
        }

        // The game's own attributes now appear, not just the synthetic DefaultMember:
        // every enum carries [Flags]? no — but many do, and extension methods carry
        // [Extension]. These are cross-assembly, exercising the MemberRef path.
        assert!(
            type_attributes.contains("System.FlagsAttribute"),
            "expected recovered [Flags] attributes among {} distinct type attributes",
            type_attributes.len()
        );
        assert!(
            type_attributes.contains("System.Runtime.CompilerServices.ExtensionAttribute"),
            "expected recovered [Extension] attributes"
        );
        assert!(
            type_attributes.contains("System.Reflection.DefaultMemberAttribute"),
            "DefaultMember should still be present"
        );
        assert!(default_member_item, "DefaultMember(\"Item\") argument should round-trip");
        assert!(member_attributes > 0, "expected recovered method/field/param attributes");
    }
}
