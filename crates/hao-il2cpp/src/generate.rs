//! Generates declaration assemblies ("dummy DLLs") from Il2Cpp metadata: every
//! type, field, method signature, property and event, with stub method bodies.

use std::collections::HashMap;

use hao::{
    dotnet::{
        entries::{
            AssemblyRef, CustomAttribute, CustomAttributeCtor, CustomAttributeValue, Constant, Entry, Event, Field, GenericParam, GenericParamConstraint, InterfaceImpl, Method,
            Param, Property, ResolutionScope, TypeDef, TypeDefOrRef, TypeRef, TypeSpec, Version,
        },
        entries::MemberRef,
        il::{Instruction, MethodBody, OpCode},
        md::streams::tables_stream::{
            EventFlags, FieldFlags, GenericParamFlags, MethodFlags, MethodImplFlags, ParamFlags, PropertyFlags,
            TypeAttributes,
        },
        module::{ModuleKind, TargetRuntime},
        signature::{
            ArrayShape, CallingConvention, ElementType, FieldSig, MemberRefSig, MethodSig, PrimitiveType, PropertySig,
            SignatureFlags, TypeSig,
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
}

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

/// Decodes a constant from the default value data (v29 compresses 32 bit values and string lengths).
fn constant(element_type: u8, data: &[u8]) -> Option<Constant> {
    let mut position = 0;
    let fixed = |n: usize| data.get(..n).map(<[u8]>::to_vec);
    let value = match element_type {
        0x02 | 0x04 | 0x05 => fixed(1)?,
        0x03 | 0x06 | 0x07 => fixed(2)?,
        0x08 => read_compressed_i32(data, &mut position)?.to_le_bytes().to_vec(),
        0x09 => read_compressed_u32(data, &mut position)?.to_le_bytes().to_vec(),
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

        Self {
            default_member,
            type_refs: vec![HashMap::new(); modules.len()],
            modules,
            types: Vec::new(),
            type_image,
            primitive_types,
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
        self.modules[module].add_assembly_ref(self.metadata.string(assembly.name_index), version(assembly.version), &token)
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
            None => ResolutionScope::AssemblyRef(self.assembly_ref(module, self.type_image[index as usize])),
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
        let params = self.metadata.generic_container_parameters(container).to_vec();
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
                let ctor = def.get().methods.iter().find(|m| m.get().is_ctor() && m.get().signature == signature).cloned()?;
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
            let nested = self.metadata.nested_types.get(start..start + def.nested_type_count as usize).unwrap_or(&[]);
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
            if module.types.first().is_none_or(|t| t.get().name != "<Module>") {
                module.types.insert(0, Entry::new(TypeDef::new(TypeAttributes::empty(), "", "<Module>", None)));
            }
        }
    }

    fn fill_type(&mut self, index: usize) {
        let module = self.type_image[index];
        let def = self.metadata.type_definitions[index].clone();
        let entry = self.types[index].clone();

        let is_interface = def.flags & TypeAttributes::ClassSemanticsMask.bits() == TypeAttributes::Interface.bits();
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
            flags.remove(FieldFlags::HasFieldRVA | FieldFlags::HasFieldMarshal | FieldFlags::HasDefault);
            let signature = FieldSig {
                ty: ty.map(|t| self.sig(module, &t)).unwrap_or(TypeSig::OBJECT),
            };
            let mut field = Field::new(flags, self.metadata.string(f.name_index), signature);
            let field_index = (field_start + offset) as i32;
            if let Some(default) = self.metadata.field_default_values.iter().find(|d| d.field_index == field_index) {
                let element = self.binary.type_at(default.type_index).and_then(|t| match t.kind {
                    TypeKind::Primitive(e) => Some(e),
                    _ => None,
                });
                field.constant = element.and_then(|e| constant(e, self.metadata.default_value_data(default.data_index)));
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
            let params: Vec<TypeSig> = parameters.iter().map(|p| self.sig_at(module, p.type_index)).collect();
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
                    instructions: vec![Instruction::simple(OpCode::Ldnull), Instruction::simple(OpCode::Throw)],
                    ..Default::default()
                });
            }
            methods.push(TypeDef::add_method(&entry, method));
        }

        let property_start = def.property_start.max(0) as usize;
        for p in self.metadata.properties.get(property_start..property_start + def.property_count as usize).unwrap_or(&[]).to_vec() {
            let method = |i: i32| usize::try_from(i).ok().and_then(|i| methods.get(i)).cloned();
            let (getter, setter) = (method(p.get), method(p.set));
            let (has_this, ty, params) = match (&getter, &setter) {
                (Some(g), _) => {
                    let g = g.get();
                    (g.signature.has_this(), g.signature.ret.clone(), g.signature.params.clone())
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
                signature: PropertySig { has_this, ty, params },
                getter,
                setter,
                ..Default::default()
            };
            entry.get_mut().properties.push(Entry::new(property));
        }

        let event_start = def.event_start.max(0) as usize;
        for e in self.metadata.events.get(event_start..event_start + def.event_count as usize).unwrap_or(&[]).to_vec() {
            let method = |i: i32| usize::try_from(i).ok().and_then(|i| methods.get(i)).cloned();
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

/// Builds a declaration assembly for every image in the game.
pub fn generate(metadata: &Metadata, binary: &Binary) -> Result<Vec<GeneratedAssembly>> {
    let mut generator = Generator::new(metadata, binary);
    generator.create_types();
    for i in 0..metadata.type_definitions.len() {
        generator.fill_type(i);
    }
    for i in 0..metadata.type_definitions.len() {
        let entry = generator.types[i].clone();
        let has_indexer = entry.get().properties.iter().any(|p| !p.get().signature.params.is_empty());
        if has_indexer {
            if let Some(attribute) = generator.default_member_attribute(generator.type_image[i], "Item") {
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
        assert_eq!(read_compressed_u32(&[0xF0, 0x78, 0x56, 0x34, 0x12], &mut 0), Some(0x1234_5678));
        assert_eq!(read_compressed_u32(&[0x81, 0x02], &mut 0), Some(0x102));
    }
}
