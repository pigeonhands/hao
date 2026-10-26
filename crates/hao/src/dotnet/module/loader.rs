//! Builds the entry graph from the raw metadata tables.
//!
//! Every row is first allocated as a placeholder entry so rows can reference
//! each other in any order, then filled in, then linked to its owner.

use pewter::pe::sections::cor20::Cor20Flags;

use crate::{
    dotnet::{
        entries::*,
        il::{BodyReader, BodyResolver, MemberToken},
        md::streams::{
            tables_stream::{coded_tokens::*, metadata::TableLocations, *},
            MetadataStreams,
        },
        metadata::Metadata,
        signature::{
            ElementType, SignatureReader, StandAloneSig, TokenResolver, TypeSig,
        },
    },
    error::{HaoError, Result},
    io::ReadData,
};

#[derive(Default)]
pub(crate) struct Tables {
    pub module: Vec<Entry<ModuleDef>>,
    pub type_refs: Vec<Entry<TypeRef>>,
    pub type_defs: Vec<Entry<TypeDef>>,
    pub fields: Vec<Entry<Field>>,
    pub methods: Vec<Entry<Method>>,
    pub params: Vec<Entry<Param>>,
    pub member_refs: Vec<Entry<MemberRef>>,
    pub properties: Vec<Entry<Property>>,
    pub events: Vec<Entry<Event>>,
    pub module_refs: Vec<Entry<ModuleRef>>,
    pub type_specs: Vec<Entry<TypeSpec>>,
    pub assembly: Vec<Entry<Assembly>>,
    pub assembly_refs: Vec<Entry<AssemblyRef>>,
    pub files: Vec<Entry<File>>,
    pub exported_types: Vec<Entry<ExportedType>>,
    pub resources: Vec<Entry<ManifestResource>>,
    pub generic_params: Vec<Entry<GenericParam>>,
    pub method_specs: Vec<Entry<MethodSpec>>,
    pub stand_alone_sigs: Vec<BlobStreamOffset>,
    pub top_level_types: Vec<Entry<TypeDef>>,
    pub entry_point: Option<Entry<Method>>,
}

pub(crate) struct Loader<'a> {
    md: &'a Metadata<'a>,
    streams: &'a MetadataStreams<'a>,
    locations: &'a TableLocations,
    t: Tables,
    ptrs: PtrTables,
}

/// Indirection tables only present in uncompressed (`#-`) metadata.
#[derive(Default)]
struct PtrTables {
    fields: Vec<u32>,
    methods: Vec<u32>,
    params: Vec<u32>,
    events: Vec<u32>,
    properties: Vec<u32>,
}

fn get<T>(table: &[Entry<T>], rid: u32, name: &'static str) -> Result<Entry<T>> {
    rid.checked_sub(1)
        .and_then(|i| table.get(i as usize))
        .cloned()
        .ok_or(HaoError::InvalidEntryRefrence(name, rid as usize))
}

fn get_opt<T>(table: &[Entry<T>], rid: u32, name: &'static str) -> Result<Option<Entry<T>>> {
    if rid == 0 {
        Ok(None)
    } else {
        get(table, rid, name).map(Some)
    }
}

/// Entries `[start, next_start)` for list columns like `TypeDef.FieldList`.
/// With a pointer table the list indexes the pointer table instead.
fn range<T>(table: &[Entry<T>], ptrs: &[u32], start: u32, end: Option<u32>) -> Result<Vec<Entry<T>>> {
    let len = if ptrs.is_empty() { table.len() } else { ptrs.len() };
    let start = (start.max(1) - 1) as usize;
    let end = end.map(|e| (e.max(1) - 1) as usize).unwrap_or(len).min(len);
    if start >= end {
        return Ok(Vec::new());
    }
    if ptrs.is_empty() {
        Ok(table[start..end].to_vec())
    } else {
        ptrs[start..end]
            .iter()
            .map(|rid| get(table, *rid, std::any::type_name::<T>()))
            .collect()
    }
}

impl<'a> Loader<'a> {
    pub fn new(md: &'a Metadata<'a>) -> Self {
        let streams = &md.metadata_streams;
        let locations = &streams.tables_stream.header.table_locations;

        fn placeholders<T: Default>(location: TableLocation) -> Vec<Entry<T>> {
            (1..=location.rows.0).map(Entry::placeholder).collect()
        }

        let t = Tables {
            module: placeholders(locations.module),
            type_refs: placeholders(locations.type_ref),
            type_defs: placeholders(locations.type_def),
            fields: placeholders(locations.field),
            methods: placeholders(locations.method),
            params: placeholders(locations.param),
            member_refs: placeholders(locations.member_ref),
            properties: placeholders(locations.property),
            events: placeholders(locations.event),
            module_refs: placeholders(locations.module_ref),
            type_specs: placeholders(locations.type_spec),
            assembly: placeholders(locations.assembly),
            assembly_refs: placeholders(locations.assembly_ref),
            files: placeholders(locations.file),
            exported_types: placeholders(locations.exported_type),
            resources: placeholders(locations.manifest_resource),
            generic_params: placeholders(locations.generic_param),
            method_specs: placeholders(locations.method_spec),
            ..Default::default()
        };

        Self {
            md,
            streams,
            locations,
            t,
            ptrs: PtrTables::default(),
        }
    }

    pub fn load(mut self) -> Result<Tables> {
        let l = self.locations;
        self.ptrs = PtrTables {
            fields: self.rows::<FieldPtrTableRow>(l.field_ptr)?.iter().map(|r| r.field.0).collect(),
            methods: self.rows::<MethodPtrTableRow>(l.method_ptr)?.iter().map(|r| r.method.0).collect(),
            params: self.rows::<ParamPtrTableRow>(l.param_ptr)?.iter().map(|r| r.param.0).collect(),
            events: self.rows::<EventPtrTableRow>(l.event_ptr)?.iter().map(|r| r.event.0).collect(),
            properties: self
                .rows::<PropertyPtrTableRow>(l.property_ptr)?
                .iter()
                .map(|r| r.property.0)
                .collect(),
        };
        let method_rvas = self.fill()?;
        self.link()?;
        self.load_bodies(&method_rvas)?;
        self.t.entry_point = self.entry_point()?;
        Ok(self.t)
    }

    fn rows<T>(&self, location: TableLocation) -> Result<Vec<T>>
    where
        TablesStreamReader<'a>: ReadData<T>,
    {
        self.streams.tables_stream.row_iter(location)?.collect()
    }

    fn string(&self, offset: StringsStreamOffset) -> Result<String> {
        self.streams
            .strings_stream
            .read_string(offset.0)
            .map(String::from)
    }

    fn blob(&self, offset: BlobStreamOffset) -> Result<&'a [u8]> {
        self.streams.blob_stream.read_blob(offset.0)
    }

    fn signature(&self, offset: BlobStreamOffset) -> Result<SignatureReader<'_, Self>> {
        Ok(SignatureReader::new(self.blob(offset)?, self))
    }

    fn guid(&self, offset: GuidStreamOffset) -> Result<uuid::Uuid> {
        self.streams.guid_stream.read_guid(offset.0)
    }

    fn type_def_or_ref(&self, token: CodedToken<TypeDefOrRefToken>) -> Result<Option<TypeDefOrRef>> {
        let t = &self.t;
        Ok(match token.target {
            TypeDefOrRefToken::TypeDef => {
                get_opt(&t.type_defs, token.rid, "TypeDef")?.map(TypeDefOrRef::TypeDef)
            }
            TypeDefOrRefToken::TypeRef => {
                get_opt(&t.type_refs, token.rid, "TypeRef")?.map(TypeDefOrRef::TypeRef)
            }
            TypeDefOrRefToken::TypeSpec => {
                get_opt(&t.type_specs, token.rid, "TypeSpec")?.map(TypeDefOrRef::TypeSpec)
            }
        })
    }

    fn method_def_or_ref(&self, token: CodedToken<MethodDefOrRefToken>) -> Result<MethodDefOrRef> {
        Ok(match token.target {
            MethodDefOrRefToken::Method => {
                MethodDefOrRef::Method(get(&self.t.methods, token.rid, "Method")?)
            }
            MethodDefOrRefToken::MemberRef => {
                MethodDefOrRef::MemberRef(get(&self.t.member_refs, token.rid, "MemberRef")?)
            }
        })
    }

    fn implementation(&self, token: CodedToken<ImplementationToken>) -> Result<Option<Implementation>> {
        let t = &self.t;
        Ok(match token.target {
            ImplementationToken::File => get_opt(&t.files, token.rid, "File")?.map(Implementation::File),
            ImplementationToken::AssemblyRef => {
                get_opt(&t.assembly_refs, token.rid, "AssemblyRef")?.map(Implementation::AssemblyRef)
            }
            ImplementationToken::ExportedType => get_opt(&t.exported_types, token.rid, "ExportedType")?
                .map(Implementation::ExportedType),
        })
    }

    fn fill(&mut self) -> Result<Vec<u32>> {
        let l = self.locations;

        for (entry, row) in self.t.module.iter().zip(self.rows::<ModulesTableRow>(l.module)?) {
            entry.replace(ModuleDef {
                generation: row.generation,
                name: self.string(row.name)?,
                mvid: self.guid(row.mvid)?,
                enc_id: self.guid(row.enc_id)?,
                enc_base_id: self.guid(row.enc_base_id)?,
                custom_attributes: Vec::new(),
            });
        }

        for (entry, row) in self.t.type_refs.iter().zip(self.rows::<TypeRefTableRow>(l.type_ref)?) {
            let rid = row.resolution_scope.rid;
            let t = &self.t;
            let scope = match row.resolution_scope.target {
                ResolutionScopeToken::Module => get_opt(&t.module, rid, "Module")?.map(ResolutionScope::Module),
                ResolutionScopeToken::ModuleRef => {
                    get_opt(&t.module_refs, rid, "ModuleRef")?.map(ResolutionScope::ModuleRef)
                }
                ResolutionScopeToken::AssemblyRef => {
                    get_opt(&t.assembly_refs, rid, "AssemblyRef")?.map(ResolutionScope::AssemblyRef)
                }
                ResolutionScopeToken::TypeRef => {
                    get_opt(&t.type_refs, rid, "TypeRef")?.map(ResolutionScope::TypeRef)
                }
            };
            entry.replace(TypeRef {
                scope,
                name: self.string(row.name)?,
                namespace: self.string(row.namespace)?,
                custom_attributes: Vec::new(),
            });
        }

        let type_rows = self.rows::<TypeDefTableRow>(l.type_def)?;
        for (i, row) in type_rows.iter().enumerate() {
            let next = type_rows.get(i + 1);
            self.t.type_defs[i].replace(TypeDef {
                flags: row.flags,
                name: self.string(row.name)?,
                namespace: self.string(row.namespace)?,
                extends: self.type_def_or_ref(row.extends)?,
                fields: range(&self.t.fields, &self.ptrs.fields, row.field_list.0, next.map(|n| n.field_list.0))?,
                methods: range(&self.t.methods, &self.ptrs.methods, row.method_list.0, next.map(|n| n.method_list.0))?,
                ..Default::default()
            });
        }

        for (entry, row) in self.t.fields.iter().zip(self.rows::<FieldTableRow>(l.field)?) {
            entry.replace(Field {
                flags: row.flags,
                name: self.string(row.name)?,
                signature: self.signature(row.signature)?.read_field_sig()?,
                ..Default::default()
            });
        }

        let method_rows = self.rows::<MethodTableRow>(l.method)?;
        for (i, row) in method_rows.iter().enumerate() {
            let next = method_rows.get(i + 1);
            self.t.methods[i].replace(Method {
                flags: row.flags,
                impl_flags: row.impl_flags,
                name: self.string(row.name)?,
                signature: self.signature(row.signature)?.read_method_sig()?,
                params: range(&self.t.params, &self.ptrs.params, row.param_list.0, next.map(|n| n.param_list.0))?,
                ..Default::default()
            });
        }

        for (entry, row) in self.t.params.iter().zip(self.rows::<ParamTableRow>(l.param)?) {
            entry.replace(Param {
                flags: row.flags,
                sequence: row.sequence,
                name: self.string(row.name)?,
                ..Default::default()
            });
        }

        for (entry, row) in self.t.module_refs.iter().zip(self.rows::<ModuleRefTableRow>(l.module_ref)?) {
            entry.replace(ModuleRef {
                name: self.string(row.name)?,
                custom_attributes: Vec::new(),
            });
        }

        for (entry, row) in self.t.type_specs.iter().zip(self.rows::<TypeSpecTableRow>(l.type_spec)?) {
            entry.replace(TypeSpec {
                signature: self.signature(BlobStreamOffset(row.signature.0))?.read_type()?,
                custom_attributes: Vec::new(),
            });
        }

        for (entry, row) in self.t.member_refs.iter().zip(self.rows::<MemberRefTableRow>(l.member_ref)?) {
            let rid = row.class.rid;
            let t = &self.t;
            let parent = match row.class.target {
                MemberRefParentToken::TypeDef => MemberRefParent::TypeDef(get(&t.type_defs, rid, "TypeDef")?),
                MemberRefParentToken::TypeRef => MemberRefParent::TypeRef(get(&t.type_refs, rid, "TypeRef")?),
                MemberRefParentToken::ModuleRef => {
                    MemberRefParent::ModuleRef(get(&t.module_refs, rid, "ModuleRef")?)
                }
                MemberRefParentToken::Method => MemberRefParent::Method(get(&t.methods, rid, "Method")?),
                MemberRefParentToken::TypeSpec => {
                    MemberRefParent::TypeSpec(get(&t.type_specs, rid, "TypeSpec")?)
                }
            };
            entry.replace(MemberRef {
                parent,
                name: self.string(row.name)?,
                signature: self.signature(row.signature)?.read_member_ref_sig()?,
                custom_attributes: Vec::new(),
            });
        }

        for (entry, row) in self.t.properties.iter().zip(self.rows::<PropertyTableRow>(l.property)?) {
            entry.replace(Property {
                flags: row.prop_flags,
                name: self.string(row.name)?,
                signature: self.signature(row.ty)?.read_property_sig()?,
                ..Default::default()
            });
        }

        for (entry, row) in self.t.events.iter().zip(self.rows::<EventTableRow>(l.event)?) {
            entry.replace(Event {
                flags: row.event_flags,
                name: self.string(row.name)?,
                event_type: self.type_def_or_ref(row.event_type)?,
                ..Default::default()
            });
        }

        for (entry, row) in self.t.assembly.iter().zip(self.rows::<AssemblyTableRow>(l.assembly)?) {
            entry.replace(Assembly {
                hash_alg_id: row.hash_alg_id,
                version: Version {
                    major: row.major_version,
                    minor: row.minor_version,
                    build: row.build_number,
                    revision: row.revision_number,
                },
                flags: row.flags,
                public_key: self.blob(row.public_key)?.to_vec(),
                name: self.string(row.name)?,
                culture: self.string(row.locale)?,
                ..Default::default()
            });
        }

        for (entry, row) in self.t.assembly_refs.iter().zip(self.rows::<AssemblyRefTableRow>(l.assembly_ref)?) {
            entry.replace(AssemblyRef {
                version: Version {
                    major: row.major_version,
                    minor: row.minor_version,
                    build: row.build_number,
                    revision: row.revision_number,
                },
                flags: row.flags,
                public_key_or_token: self.blob(row.public_key_or_token)?.to_vec(),
                name: self.string(row.name)?,
                culture: self.string(row.locale)?,
                hash_value: self.blob(row.hash_value)?.to_vec(),
                ..Default::default()
            });
        }

        for (entry, row) in self.t.files.iter().zip(self.rows::<FileTableRow>(l.file)?) {
            entry.replace(File {
                flags: row.flags,
                name: self.string(row.name)?,
                hash_value: self.blob(row.hash_value)?.to_vec(),
                custom_attributes: Vec::new(),
            });
        }

        for (entry, row) in self.t.exported_types.iter().zip(self.rows::<ExportedTypeTableRow>(l.exported_type)?) {
            entry.replace(ExportedType {
                flags: row.flags,
                type_def_id: row.type_def_id,
                name: self.string(row.type_name)?,
                namespace: self.string(row.type_namespace)?,
                implementation: self.implementation(row.implementation)?,
                custom_attributes: Vec::new(),
            });
        }

        for (entry, row) in self.t.resources.iter().zip(self.rows::<ManifestResourceTableRow>(l.manifest_resource)?) {
            let data = match self.implementation(row.implementation)? {
                Some(implementation) => ResourceData::Linked {
                    implementation,
                    offset: row.offset,
                },
                None => ResourceData::Embedded(self.embedded_resource(row.offset)?),
            };
            entry.replace(ManifestResource {
                flags: row.flags,
                name: self.string(row.name)?,
                data,
                custom_attributes: Vec::new(),
            });
        }

        for (entry, row) in self.t.generic_params.iter().zip(self.rows::<GenericParamTableRow>(l.generic_param)?) {
            entry.replace(GenericParam {
                number: row.number,
                flags: row.flags,
                name: self.string(row.name)?,
                ..Default::default()
            });
        }

        for (entry, row) in self.t.method_specs.iter().zip(self.rows::<MethodSpecTableRow>(l.method_spec)?) {
            entry.replace(MethodSpec {
                method: self.method_def_or_ref(row.method)?,
                instantiation: self.signature(row.instantiation)?.read_method_spec_sig()?,
                custom_attributes: Vec::new(),
            });
        }

        self.t.stand_alone_sigs = self
            .rows::<StandAloneSigTableRow>(l.stand_alone_sig)?
            .into_iter()
            .map(|r| r.signature)
            .collect();

        let map_rows = self.rows::<PropertyMapTableRow>(l.property_map)?;
        for (i, row) in map_rows.iter().enumerate() {
            let next = map_rows.get(i + 1).map(|n| n.property_list.0);
            let parent = get(&self.t.type_defs, row.parent.0, "TypeDef")?;
            parent.get_mut().properties = range(&self.t.properties, &self.ptrs.properties, row.property_list.0, next)?;
        }

        let map_rows = self.rows::<EventMapTableRow>(l.event_map)?;
        for (i, row) in map_rows.iter().enumerate() {
            let next = map_rows.get(i + 1).map(|n| n.event_list.0);
            let parent = get(&self.t.type_defs, row.parent.0, "TypeDef")?;
            parent.get_mut().events = range(&self.t.events, &self.ptrs.events, row.event_list.0, next)?;
        }

        Ok(method_rows.iter().map(|r| r.rva).collect())
    }

    fn embedded_resource(&self, offset: u32) -> Result<Vec<u8>> {
        let dir = &self.md.cor20_header.resources;
        let rva = dir.virtual_address + offset;
        let mut reader = self.md.pe_image.create_reader(rva, None)?;
        let len: u32 = reader.read()?;
        Ok(reader.read_slice(len as usize)?.to_vec())
    }

    fn link(&mut self) -> Result<()> {
        let l = self.locations;

        let mut is_nested = vec![false; self.t.type_defs.len()];
        for row in self.rows::<NestedClassTableRow>(l.nested_class)? {
            let nested = get(&self.t.type_defs, row.nested_class.0, "TypeDef")?;
            let enclosing = get(&self.t.type_defs, row.enclosing_class.0, "TypeDef")?;
            nested.get_mut().declaring_type = enclosing.downgrade();
            enclosing.get_mut().nested_types.push(nested);
            is_nested[row.nested_class.0 as usize - 1] = true;
        }
        self.t.top_level_types = self
            .t
            .type_defs
            .iter()
            .zip(is_nested)
            .filter(|(_, nested)| !nested)
            .map(|(t, _)| t.clone())
            .collect();

        for ty in &self.t.type_defs {
            let weak = ty.downgrade();
            let ty = ty.get();
            for f in &ty.fields {
                f.get_mut().declaring_type = weak.clone();
            }
            for m in &ty.methods {
                m.get_mut().declaring_type = weak.clone();
            }
        }

        let mut interface_impls = Vec::new();
        for row in self.rows::<InterfaceImplTableRow>(l.interface_impl)? {
            let class = get(&self.t.type_defs, row.class.0, "TypeDef")?;
            let interface = self
                .type_def_or_ref(row.interface)?
                .ok_or(HaoError::InvalidEntryRefrence("InterfaceImpl", 0))?;
            let index = class.get().interfaces.len();
            class.get_mut().interfaces.push(InterfaceImpl::new(interface));
            interface_impls.push((class, index));
        }

        for row in self.rows::<ConstantTableRow>(l.constant)? {
            let constant = Constant {
                element_type: ElementType::from_repr(row.ty)
                    .ok_or(HaoError::InvalidSignatureElementType(row.ty, 0))?,
                value: self.blob(row.value)?.to_vec(),
            };
            let rid = row.parent.rid;
            match row.parent.target {
                HasConstantToken::Field => get(&self.t.fields, rid, "Field")?.get_mut().constant = Some(constant),
                HasConstantToken::Param => get(&self.t.params, rid, "Param")?.get_mut().constant = Some(constant),
                HasConstantToken::Property => {
                    get(&self.t.properties, rid, "Property")?.get_mut().constant = Some(constant)
                }
            }
        }

        for row in self.rows::<FieldMarshalTableRow>(l.field_marshal)? {
            let marshal = Some(self.blob(row.native_type)?.to_vec());
            let rid = row.parent.rid;
            match row.parent.target {
                HasFieldMarshalToken::Field => get(&self.t.fields, rid, "Field")?.get_mut().marshal = marshal,
                HasFieldMarshalToken::Param => get(&self.t.params, rid, "Param")?.get_mut().marshal = marshal,
            }
        }

        let mut decl_security = Vec::new();
        for row in self.rows::<DeclSecurityTableRow>(l.decl_security)? {
            let security = DeclSecurity {
                action: row.action,
                permission_set: self.blob(row.permission_set)?.to_vec(),
                custom_attributes: Vec::new(),
            };
            let rid = row.parent.rid;
            let owner = match row.parent.target {
                HasDeclSecurityToken::TypeDef => {
                    let ty = get(&self.t.type_defs, rid, "TypeDef")?;
                    let index = ty.get().security.len();
                    ty.get_mut().security.push(security);
                    SecurityOwner::TypeDef(ty, index)
                }
                HasDeclSecurityToken::Method => {
                    let m = get(&self.t.methods, rid, "Method")?;
                    let index = m.get().security.len();
                    m.get_mut().security.push(security);
                    SecurityOwner::Method(m, index)
                }
                HasDeclSecurityToken::Assembly => {
                    let a = get(&self.t.assembly, rid, "Assembly")?;
                    let index = a.get().security.len();
                    a.get_mut().security.push(security);
                    SecurityOwner::Assembly(a, index)
                }
            };
            decl_security.push(owner);
        }

        for row in self.rows::<ClassLayoutTableRow>(l.class_layout)? {
            get(&self.t.type_defs, row.parent.0, "TypeDef")?.get_mut().class_layout = Some(ClassLayout {
                packing_size: row.packing_size,
                class_size: row.class_size,
            });
        }

        for row in self.rows::<FieldLayoutTableRow>(l.field_layout)? {
            get(&self.t.fields, row.field.0, "Field")?.get_mut().offset = Some(row.offset);
        }

        for row in self.rows::<MethodSemanticsTableRow>(l.method_semantics)? {
            let method = get(&self.t.methods, row.method.0, "Method")?;
            let rid = row.association.rid;
            let s = row.semantic;
            match row.association.target {
                HasSemanticToken::Property => {
                    let prop = get(&self.t.properties, rid, "Property")?;
                    let mut prop = prop.get_mut();
                    if s.contains(MethodSemanticsFlags::Getter) {
                        prop.getter = Some(method);
                    } else if s.contains(MethodSemanticsFlags::Setter) {
                        prop.setter = Some(method);
                    } else {
                        prop.other_methods.push(method);
                    }
                }
                HasSemanticToken::Event => {
                    let event = get(&self.t.events, rid, "Event")?;
                    let mut event = event.get_mut();
                    if s.contains(MethodSemanticsFlags::AddOn) {
                        event.add = Some(method);
                    } else if s.contains(MethodSemanticsFlags::RemoveOn) {
                        event.remove = Some(method);
                    } else if s.contains(MethodSemanticsFlags::Fire) {
                        event.fire = Some(method);
                    } else {
                        event.other_methods.push(method);
                    }
                }
            }
        }

        for row in self.rows::<MethodImplTableRow>(l.method_impl)? {
            let class = get(&self.t.type_defs, row.class.0, "TypeDef")?;
            let method_impl = MethodImpl {
                body: self.method_def_or_ref(row.method_body)?,
                declaration: self.method_def_or_ref(row.method_declaration)?,
            };
            class.get_mut().method_impls.push(method_impl);
        }

        for row in self.rows::<ImplMapTableRow>(l.impl_map)? {
            let impl_map = Some(ImplMap {
                flags: row.mapping_flags,
                import_name: self.string(row.import_name)?,
                scope: get(&self.t.module_refs, row.import_scope.0, "ModuleRef")?,
            });
            let rid = row.member_forwarded.rid;
            match row.member_forwarded.target {
                MemberForwardedToken::Field => get(&self.t.fields, rid, "Field")?.get_mut().impl_map = impl_map,
                MemberForwardedToken::Method => get(&self.t.methods, rid, "Method")?.get_mut().impl_map = impl_map,
            }
        }

        let mut field_rvas = self.rows::<FieldRVATableRow>(l.field_rva)?;
        field_rvas.sort_by_key(|r| r.rva);
        for (i, row) in field_rvas.iter().enumerate() {
            let field = get(&self.t.fields, row.field.0, "Field")?;
            let next_rva = field_rvas.get(i + 1).map(|n| n.rva).filter(|n| *n > row.rva);
            let size = field_initial_value_size(&field.get().signature.ty)
                .or_else(|| next_rva.map(|n| (n - row.rva) as usize))
                .ok_or(HaoError::BadImageFormat("Could not determine FieldRVA data size"))?;
            let data = self.md.pe_image.create_reader(row.rva, None)?.read_slice(size)?.to_vec();
            field.get_mut().initial_value = Some(data);
        }

        let mut generic_params = Vec::new();
        for (entry, row) in self.t.generic_params.iter().zip(self.rows::<GenericParamTableRow>(l.generic_param)?) {
            let rid = row.owner.rid;
            match row.owner.target {
                TypeOrMethodDefToken::TypeDef => get(&self.t.type_defs, rid, "TypeDef")?
                    .get_mut()
                    .generic_params
                    .push(entry.clone()),
                TypeOrMethodDefToken::Method => get(&self.t.methods, rid, "Method")?
                    .get_mut()
                    .generic_params
                    .push(entry.clone()),
            }
            generic_params.push(entry.clone());
        }
        for owner in self.t.type_defs.iter().map(|t| t.get().generic_params.clone())
            .chain(self.t.methods.iter().map(|m| m.get().generic_params.clone()))
        {
            debug_assert!(owner.windows(2).all(|w| w[0].get().number <= w[1].get().number));
        }

        let mut constraints = Vec::new();
        for row in self.rows::<GenericParamConstraintTableRow>(l.generic_param_constraint)? {
            let owner = get(&self.t.generic_params, row.owner.0, "GenericParam")?;
            let constraint = self
                .type_def_or_ref(row.constraint)?
                .ok_or(HaoError::InvalidEntryRefrence("GenericParamConstraint", 0))?;
            let index = owner.get().constraints.len();
            owner.get_mut().constraints.push(GenericParamConstraint {
                constraint,
                custom_attributes: Vec::new(),
            });
            constraints.push((owner, index));
        }

        for row in self.rows::<CustomAttributeTableRow>(l.custom_attribute)? {
            let constructor = match row.ty.target {
                CustomAttributeTypeToken::Method => {
                    CustomAttributeCtor::Method(get(&self.t.methods, row.ty.rid, "Method")?)
                }
                CustomAttributeTypeToken::MemberRef => {
                    CustomAttributeCtor::MemberRef(get(&self.t.member_refs, row.ty.rid, "MemberRef")?)
                }
                _ => return Err(HaoError::InvalidCodedToken(row.ty.rid, "CustomAttributeType")),
            };
            let attribute = CustomAttribute {
                constructor,
                value: self.blob(row.value)?.to_vec(),
            };
            let rid = row.parent.rid;
            let t = &self.t;
            macro_rules! push {
                ($table:expr, $name:literal) => {
                    get(&$table, rid, $name)?.get_mut().custom_attributes.push(attribute)
                };
            }
            let index = |len: usize| {
                rid.checked_sub(1)
                    .map(|i| i as usize)
                    .filter(|i| *i < len)
                    .ok_or(HaoError::InvalidEntryRefrence("CustomAttribute parent", rid as usize))
            };
            match row.parent.target {
                HasCustomAttributeToken::Method => push!(t.methods, "Method"),
                HasCustomAttributeToken::Field => push!(t.fields, "Field"),
                HasCustomAttributeToken::TypeRef => push!(t.type_refs, "TypeRef"),
                HasCustomAttributeToken::TypeDef => push!(t.type_defs, "TypeDef"),
                HasCustomAttributeToken::Param => push!(t.params, "Param"),
                HasCustomAttributeToken::MemberRef => push!(t.member_refs, "MemberRef"),
                HasCustomAttributeToken::Module => push!(t.module, "Module"),
                HasCustomAttributeToken::Property => push!(t.properties, "Property"),
                HasCustomAttributeToken::Event => push!(t.events, "Event"),
                HasCustomAttributeToken::ModuleRef => push!(t.module_refs, "ModuleRef"),
                HasCustomAttributeToken::TypeSpec => push!(t.type_specs, "TypeSpec"),
                HasCustomAttributeToken::Assembly => push!(t.assembly, "Assembly"),
                HasCustomAttributeToken::AssemblyRef => push!(t.assembly_refs, "AssemblyRef"),
                HasCustomAttributeToken::File => push!(t.files, "File"),
                HasCustomAttributeToken::ExportedType => push!(t.exported_types, "ExportedType"),
                HasCustomAttributeToken::ManifestResource => push!(t.resources, "ManifestResource"),
                HasCustomAttributeToken::GenericParam => push!(t.generic_params, "GenericParam"),
                HasCustomAttributeToken::MethodSpec => push!(t.method_specs, "MethodSpec"),
                HasCustomAttributeToken::InterfaceImpl => {
                    let (class, i) = &interface_impls[index(interface_impls.len())?];
                    class.get_mut().interfaces[*i].custom_attributes.push(attribute);
                }
                HasCustomAttributeToken::GenericParamConstraint => {
                    let (owner, i) = &constraints[index(constraints.len())?];
                    owner.get_mut().constraints[*i].custom_attributes.push(attribute);
                }
                HasCustomAttributeToken::DeclSecurity => {
                    decl_security[index(decl_security.len())?].push_attribute(attribute)
                }
                HasCustomAttributeToken::StandAloneSig
                | HasCustomAttributeToken::Unused1
                | HasCustomAttributeToken::Unused2 => {}
            }
        }
        Ok(())
    }

    fn load_bodies(&self, rvas: &[u32]) -> Result<()> {
        for (method, rva) in self.t.methods.iter().zip(rvas) {
            let is_il = {
                let m = method.get();
                m.impl_flags & MethodImplFlags::CodeTypeMask == MethodImplFlags::IL
                    && !m.flags.contains(MethodFlags::Abstract)
            };
            if *rva == 0 || !is_il {
                continue;
            }
            let data = self.md.pe_image.create_reader(*rva, None)?.remaning_slice();
            let body = BodyReader::new(data, self).read()?;
            method.get_mut().body = Some(body);
        }
        Ok(())
    }

    fn entry_point(&self) -> Result<Option<Entry<Method>>> {
        let cor20 = &self.md.cor20_header;
        let token = cor20.entrypoint_token_or_rva;
        if cor20.flags.contains(Cor20Flags::NATIVE_ENTRYPOINT) || token >> 24 != 0x06 {
            return Ok(None);
        }
        get_opt(&self.t.methods, token & 0x00FF_FFFF, "Method")
    }
}

enum SecurityOwner {
    TypeDef(Entry<TypeDef>, usize),
    Method(Entry<Method>, usize),
    Assembly(Entry<Assembly>, usize),
}

impl SecurityOwner {
    fn push_attribute(&self, attribute: CustomAttribute) {
        match self {
            Self::TypeDef(e, i) => e.get_mut().security[*i].custom_attributes.push(attribute),
            Self::Method(e, i) => e.get_mut().security[*i].custom_attributes.push(attribute),
            Self::Assembly(e, i) => e.get_mut().security[*i].custom_attributes.push(attribute),
        }
    }
}

fn field_initial_value_size(ty: &TypeSig) -> Option<usize> {
    match ty.unmodified() {
        TypeSig::Primitive(p) => p.size(),
        TypeSig::ValueType(TypeDefOrRef::TypeDef(def)) => {
            let def = def.get();
            if let Some(layout) = def.class_layout.filter(|l| l.class_size > 0) {
                return Some(layout.class_size as usize);
            }
            if def.is_enum() {
                return def
                    .enum_underlying_field()
                    .and_then(|f| field_initial_value_size(&f.get().signature.ty));
            }
            None
        }
        TypeSig::Ptr(_) | TypeSig::FnPtr(_) => None,
        _ => None,
    }
}

impl TokenResolver for Loader<'_> {
    fn type_def_or_ref(&self, coded: u32) -> Result<TypeDefOrRef> {
        let token = CodedToken::<TypeDefOrRefToken>::decode(coded)?;
        Loader::type_def_or_ref(self, token)?.ok_or(HaoError::InvalidSignature("null type token", 0))
    }
}

impl BodyResolver for Loader<'_> {
    fn member_token(&self, token: u32) -> Result<MemberToken> {
        let rid = token & 0x00FF_FFFF;
        let t = &self.t;
        Ok(match token >> 24 {
            0x01 => MemberToken::TypeRef(get(&t.type_refs, rid, "TypeRef")?),
            0x02 => MemberToken::TypeDef(get(&t.type_defs, rid, "TypeDef")?),
            0x04 => MemberToken::Field(get(&t.fields, rid, "Field")?),
            0x06 => MemberToken::Method(get(&t.methods, rid, "Method")?),
            0x0A => MemberToken::MemberRef(get(&t.member_refs, rid, "MemberRef")?),
            0x1B => MemberToken::TypeSpec(get(&t.type_specs, rid, "TypeSpec")?),
            0x2B => MemberToken::MethodSpec(get(&t.method_specs, rid, "MethodSpec")?),
            _ => return Err(HaoError::InvalidToken(token)),
        })
    }

    fn user_string(&self, token: u32) -> Result<String> {
        if token >> 24 != 0x70 {
            return Err(HaoError::InvalidToken(token));
        }
        self.streams
            .user_strings_stream
            .as_ref()
            .ok_or(HaoError::BadImageFormat("No user strings stream (#US)"))?
            .read_string(token & 0x00FF_FFFF)
    }

    fn stand_alone_sig(&self, token: u32) -> Result<StandAloneSig> {
        if token >> 24 != 0x11 {
            return Err(HaoError::InvalidToken(token));
        }
        let offset = (token & 0x00FF_FFFF)
            .checked_sub(1)
            .and_then(|i| self.t.stand_alone_sigs.get(i as usize))
            .ok_or(HaoError::InvalidToken(token))?;
        self.signature(*offset)?.read_stand_alone_sig()
    }
}
