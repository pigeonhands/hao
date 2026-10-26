//! Rebuilds the metadata of a [`Module`] and writes it into a copy of the
//! original PE image.
//!
//! The new metadata, method bodies, field data and resources go into a new
//! section. The original sections are kept as they are so native stubs, imports
//! and relocations stay valid.

mod heaps;
mod tables;

use std::collections::HashMap;

use pewter::pe::{
    coff::{ImageFileCharacteristics, ImageFileMachine},
    definition::{align_up, PEImageDef},
    optional_header::{data_directories::ImageDataDirectory, ImageSubsystem, OptionalHeader},
    sections::{
        cor20::{Cor20Flags, ImageCor20Header},
        SectionFlags,
    },
};

use heaps::*;
use tables::*;

use crate::{
    dotnet::module::ModuleKind,
    dotnet::{
        entries::*,
        il::{encode_body, BodyEncoder, MemberToken},
        md::{
            streams::tables_stream::{
                coded_tokens::*,
                metadata::{TableRowCount, TableRows},
                *,
            },
            MDStreamFlags,
        },
        signature::{MethodSig, SignatureWriter, TokenEncoder, TypeSig},
    },
    error::{HaoError, Result},
    Module,
};

const SECTION_NAME: &str = ".hao";

struct Rids<T> {
    map: HashMap<usize, u32>,
    list: Vec<Entry<T>>,
}

impl<T> Default for Rids<T> {
    fn default() -> Self {
        Self {
            map: HashMap::new(),
            list: Vec::new(),
        }
    }
}

impl<T> Rids<T> {
    fn get(&self, entry: &Entry<T>) -> Option<u32> {
        self.map.get(&entry.key()).copied()
    }

    fn add(&mut self, entry: &Entry<T>) -> u32 {
        if let Some(rid) = self.get(entry) {
            return rid;
        }
        self.list.push(entry.clone());
        let rid = self.list.len() as u32;
        self.map.insert(entry.key(), rid);
        rid
    }
}

#[derive(Default)]
struct Rows {
    module: Vec<ModulesTableRow>,
    type_ref: Vec<TypeRefTableRow>,
    type_def: Vec<TypeDefTableRow>,
    field: Vec<FieldTableRow>,
    method: Vec<MethodTableRow>,
    param: Vec<ParamTableRow>,
    interface_impl: Vec<InterfaceImplTableRow>,
    member_ref: Vec<MemberRefTableRow>,
    constant: Vec<ConstantTableRow>,
    custom_attribute: Vec<CustomAttributeTableRow>,
    field_marshal: Vec<FieldMarshalTableRow>,
    decl_security: Vec<DeclSecurityTableRow>,
    class_layout: Vec<ClassLayoutTableRow>,
    field_layout: Vec<FieldLayoutTableRow>,
    stand_alone_sig: Vec<StandAloneSigTableRow>,
    event_map: Vec<EventMapTableRow>,
    event: Vec<EventTableRow>,
    property_map: Vec<PropertyMapTableRow>,
    property: Vec<PropertyTableRow>,
    method_semantics: Vec<MethodSemanticsTableRow>,
    method_impl: Vec<MethodImplTableRow>,
    module_ref: Vec<ModuleRefTableRow>,
    type_spec: Vec<TypeSpecTableRow>,
    impl_map: Vec<ImplMapTableRow>,
    field_rva: Vec<FieldRVATableRow>,
    assembly: Vec<AssemblyTableRow>,
    assembly_ref: Vec<AssemblyRefTableRow>,
    file: Vec<FileTableRow>,
    exported_type: Vec<ExportedTypeTableRow>,
    manifest_resource: Vec<ManifestResourceTableRow>,
    nested_class: Vec<NestedClassTableRow>,
    generic_param: Vec<GenericParamTableRow>,
    method_spec: Vec<MethodSpecTableRow>,
    generic_param_constraint: Vec<GenericParamConstraintTableRow>,
}

struct ModuleWriter<'m> {
    module: &'m Module,
    strings: StringsHeap,
    blobs: BlobHeap,
    guids: GuidHeap,
    user_strings: UserStringHeap,

    type_defs: Rids<TypeDef>,
    fields: Rids<Field>,
    methods: Rids<Method>,
    params: Rids<Param>,
    properties: Rids<Property>,
    events: Rids<Event>,
    generic_params: Rids<GenericParam>,
    type_refs: Rids<TypeRef>,
    type_specs: Rids<TypeSpec>,
    member_refs: Rids<MemberRef>,
    method_specs: Rids<MethodSpec>,
    module_refs: Rids<ModuleRef>,
    assembly_refs: Rids<AssemblyRef>,
    files: Rids<File>,
    exported_types: Rids<ExportedType>,
    stand_alone_sigs: HashMap<u32, u32>,

    rows: Rows,
    code: Vec<u8>,
    field_data: Vec<u8>,
    resources: Vec<u8>,
    section_rva: u32,
    pending_field_rvas: Vec<(u32, Vec<u8>)>,
}

fn err(msg: impl Into<String>) -> HaoError {
    HaoError::WriteError(msg.into())
}

impl<'m> ModuleWriter<'m> {
    fn new(module: &'m Module, section_rva: u32) -> Self {
        Self {
            module,
            strings: Default::default(),
            blobs: Default::default(),
            guids: Default::default(),
            user_strings: Default::default(),
            type_defs: Default::default(),
            fields: Default::default(),
            methods: Default::default(),
            params: Default::default(),
            properties: Default::default(),
            events: Default::default(),
            generic_params: Default::default(),
            type_refs: Default::default(),
            type_specs: Default::default(),
            member_refs: Default::default(),
            method_specs: Default::default(),
            module_refs: Default::default(),
            assembly_refs: Default::default(),
            files: Default::default(),
            exported_types: Default::default(),
            stand_alone_sigs: HashMap::new(),
            rows: Default::default(),
            code: Vec::new(),
            field_data: Vec::new(),
            resources: Vec::new(),
            section_rva,
            pending_field_rvas: Vec::new(),
        }
    }

    fn string(&mut self, value: &str) -> StringsStreamOffset {
        StringsStreamOffset(self.strings.add(value))
    }

    fn blob(&mut self, value: &[u8]) -> BlobStreamOffset {
        BlobStreamOffset(self.blobs.add(value))
    }

    fn sig_blob(
        &mut self,
        f: impl FnOnce(&mut SignatureWriter<Self>) -> Result<()>,
    ) -> Result<BlobStreamOffset> {
        let mut w = SignatureWriter::new(self);
        f(&mut w)?;
        let bytes = w.finish();
        Ok(self.blob(&bytes))
    }

    fn type_def_rid(&self, ty: &Entry<TypeDef>) -> Result<u32> {
        self.type_defs
            .get(ty)
            .ok_or_else(|| err(format!("type {:?} is not part of the module", ty)))
    }

    fn field_rid(&self, field: &Entry<Field>) -> Result<u32> {
        self.fields
            .get(field)
            .ok_or_else(|| err(format!("field {:?} is not part of the module", field)))
    }

    fn method_rid(&self, method: &Entry<Method>) -> Result<u32> {
        self.methods
            .get(method)
            .ok_or_else(|| err(format!("method {:?} is not part of the module", method)))
    }

    fn type_def_or_ref(&mut self, ty: &TypeDefOrRef) -> Result<CodedToken<TypeDefOrRefToken>> {
        Ok(match ty {
            TypeDefOrRef::TypeDef(t) => {
                CodedToken::new(TypeDefOrRefToken::TypeDef, self.type_def_rid(t)?)
            }
            TypeDefOrRef::TypeRef(t) => {
                CodedToken::new(TypeDefOrRefToken::TypeRef, self.type_refs.add(t))
            }
            TypeDefOrRef::TypeSpec(t) => {
                CodedToken::new(TypeDefOrRefToken::TypeSpec, self.type_specs.add(t))
            }
        })
    }

    fn method_def_or_ref(&mut self, m: &MethodDefOrRef) -> Result<CodedToken<MethodDefOrRefToken>> {
        Ok(match m {
            MethodDefOrRef::Method(m) => {
                CodedToken::new(MethodDefOrRefToken::Method, self.method_rid(m)?)
            }
            MethodDefOrRef::MemberRef(m) => {
                CodedToken::new(MethodDefOrRefToken::MemberRef, self.member_refs.add(m))
            }
        })
    }

    fn implementation(&mut self, i: &Option<Implementation>) -> CodedToken<ImplementationToken> {
        match i {
            None => CodedToken::new(ImplementationToken::File, 0),
            Some(Implementation::File(f)) => {
                CodedToken::new(ImplementationToken::File, self.files.add(f))
            }
            Some(Implementation::AssemblyRef(a)) => {
                CodedToken::new(ImplementationToken::AssemblyRef, self.assembly_refs.add(a))
            }
            Some(Implementation::ExportedType(e)) => CodedToken::new(
                ImplementationToken::ExportedType,
                self.exported_types.add(e),
            ),
        }
    }

    fn custom_attributes(
        &mut self,
        parent: HasCustomAttributeToken,
        rid: u32,
        attributes: &[CustomAttribute],
    ) -> Result<()> {
        for attribute in attributes {
            let ty = match &attribute.constructor {
                CustomAttributeCtor::Method(m) => {
                    CodedToken::new(CustomAttributeTypeToken::Method, self.method_rid(m)?)
                }
                CustomAttributeCtor::MemberRef(m) => {
                    CodedToken::new(CustomAttributeTypeToken::MemberRef, self.member_refs.add(m))
                }
            };
            let value = self.blob(&attribute.value);
            self.rows.custom_attribute.push(CustomAttributeTableRow {
                parent: CodedToken::new(parent, rid),
                ty,
                value,
            });
        }
        Ok(())
    }

    fn constant(&mut self, parent: HasConstantToken, rid: u32, constant: &Option<Constant>) {
        if let Some(c) = constant {
            let value = self.blob(&c.value);
            self.rows.constant.push(ConstantTableRow {
                ty: c.element_type as u8,
                padding: 0,
                parent: CodedToken::new(parent, rid),
                value,
            });
        }
    }

    fn marshal(&mut self, parent: HasFieldMarshalToken, rid: u32, marshal: &Option<Vec<u8>>) {
        if let Some(m) = marshal {
            let native_type = self.blob(m);
            self.rows.field_marshal.push(FieldMarshalTableRow {
                parent: CodedToken::new(parent, rid),
                native_type,
            });
        }
    }

    fn impl_map(&mut self, target: MemberForwardedToken, rid: u32, impl_map: &Option<ImplMap>) {
        if let Some(m) = impl_map {
            let import_name = self.string(&m.import_name);
            let import_scope = ModuleRefTableOffset(self.module_refs.add(&m.scope));
            self.rows.impl_map.push(ImplMapTableRow {
                mapping_flags: m.flags,
                member_forwarded: CodedToken::new(target, rid),
                import_name,
                import_scope,
            });
        }
    }

    fn stand_alone_sig(&mut self, signature: BlobStreamOffset) -> u32 {
        let next = self.rows.stand_alone_sig.len() as u32 + 1;
        let rid = *self.stand_alone_sigs.entry(signature.0).or_insert(next);
        if rid == next {
            self.rows
                .stand_alone_sig
                .push(StandAloneSigTableRow { signature });
        }
        0x1100_0000 | rid
    }

    fn build(&mut self) -> Result<Vec<(HasDeclSecurityToken, u32, DeclSecurity)>> {
        let module = self.module;
        let types = module.all_types();

        for ty in &types {
            self.type_defs.add(ty);
            let ty = ty.get();
            for f in &ty.fields {
                self.fields.add(f);
            }
            for m in &ty.methods {
                self.methods.add(m);
                let mut params = m.get().params.clone();
                params.sort_by_key(|p| p.get().sequence);
                for p in &params {
                    self.params.add(p);
                }
            }
            for p in &ty.properties {
                self.properties.add(p);
            }
            for e in &ty.events {
                self.events.add(e);
            }
        }
        for e in &module.assembly_refs {
            self.assembly_refs.add(e);
        }
        for e in &module.module_refs {
            self.module_refs.add(e);
        }
        for e in &module.type_refs {
            self.type_refs.add(e);
        }
        for e in &module.type_specs {
            self.type_specs.add(e);
        }
        for e in &module.member_refs {
            self.member_refs.add(e);
        }
        for e in &module.method_specs {
            self.method_specs.add(e);
        }
        for e in &module.files {
            self.files.add(e);
        }
        for e in &module.exported_types {
            self.exported_types.add(e);
        }

        {
            let m = module.module.get();
            let row = ModulesTableRow {
                generation: m.generation,
                name: self.string(&m.name),
                mvid: GuidStreamOffset(self.guids.add(&m.mvid)),
                enc_id: GuidStreamOffset(self.guids.add(&m.enc_id)),
                enc_base_id: GuidStreamOffset(self.guids.add(&m.enc_base_id)),
            };
            self.rows.module.push(row);
            self.custom_attributes(HasCustomAttributeToken::Module, 1, &m.custom_attributes)?;
        }

        let mut field_cursor = 1;
        let mut method_cursor = 1;
        for ty in &types {
            let ty = ty.get();
            let extends = match &ty.extends {
                Some(e) => self.type_def_or_ref(e)?,
                None => CodedToken::new(TypeDefOrRefToken::TypeDef, 0),
            };
            let row = TypeDefTableRow {
                flags: ty.flags,
                name: self.string(&ty.name),
                namespace: self.string(&ty.namespace),
                extends,
                field_list: FieldTableOffset(field_cursor),
                method_list: MethodTableOffset(method_cursor),
            };
            self.rows.type_def.push(row);
            field_cursor += ty.fields.len() as u32;
            method_cursor += ty.methods.len() as u32;
        }

        for field in self.fields.list.clone() {
            let f = field.get();
            let signature = self.sig_blob(|w| w.write_field_sig(&f.signature))?;
            let row = FieldTableRow {
                flags: f.flags,
                name: self.string(&f.name),
                signature,
            };
            self.rows.field.push(row);
        }

        let mut param_cursor = 1;
        for method in self.methods.list.clone() {
            let m = method.get();
            let signature = self.sig_blob(|w| w.write_method_sig(&m.signature))?;
            let rva = match &m.body {
                Some(body) => {
                    let bytes = encode_body(body, !m.signature.ret.is_void(), self)?;
                    let align = if bytes.first().is_some_and(|b| b & 3 == 2) {
                        1
                    } else {
                        4
                    };
                    self.code.resize(align_up(self.code.len(), align), 0);
                    let rva = self.section_rva + self.code.len() as u32;
                    self.code.extend_from_slice(&bytes);
                    rva
                }
                None => 0,
            };
            let row = MethodTableRow {
                rva,
                impl_flags: m.impl_flags,
                flags: m.flags,
                name: self.string(&m.name),
                signature,
                param_list: ParamTableOffset(param_cursor),
            };
            self.rows.method.push(row);
            param_cursor += m.params.len() as u32;
        }

        for param in self.params.list.clone() {
            let p = param.get();
            let row = ParamTableRow {
                flags: p.flags,
                sequence: p.sequence,
                name: self.string(&p.name),
            };
            self.rows.param.push(row);
        }

        let mut owners: Vec<(CodedToken<TypeOrMethodDefToken>, Entry<GenericParam>)> = Vec::new();
        for (i, ty) in types.iter().enumerate() {
            for gp in &ty.get().generic_params {
                owners.push((
                    CodedToken::new(TypeOrMethodDefToken::TypeDef, i as u32 + 1),
                    gp.clone(),
                ));
            }
        }
        for (i, m) in self.methods.list.iter().enumerate() {
            for gp in &m.get().generic_params {
                owners.push((
                    CodedToken::new(TypeOrMethodDefToken::Method, i as u32 + 1),
                    gp.clone(),
                ));
            }
        }
        owners.sort_by_key(|(owner, gp)| (owner.encode(), gp.get().number));
        for (owner, gp) in &owners {
            self.generic_params.add(gp);
            let g = gp.get();
            let row = GenericParamTableRow {
                number: g.number,
                flags: g.flags,
                owner: *owner,
                name: self.string(&g.name),
                kind: None,
            };
            self.rows.generic_param.push(row);
        }

        let mut security = Vec::new();

        for (i, ty_entry) in types.iter().enumerate() {
            let rid = i as u32 + 1;
            let ty = ty_entry.get();
            if let Some(parent) = ty.declaring_type.upgrade() {
                let enclosing = self.type_def_rid(&parent)?;
                self.rows.nested_class.push(NestedClassTableRow {
                    nested_class: TypeDefTableOffset(rid),
                    enclosing_class: TypeDefTableOffset(enclosing),
                });
            }
            for interface in &ty.interfaces {
                let token = self.type_def_or_ref(&interface.interface)?;
                self.rows.interface_impl.push(InterfaceImplTableRow {
                    class: TypeDefTableOffset(rid),
                    interface: token,
                });
                let impl_rid = self.rows.interface_impl.len() as u32;
                self.custom_attributes(
                    HasCustomAttributeToken::InterfaceImpl,
                    impl_rid,
                    &interface.custom_attributes,
                )?;
            }
            if let Some(layout) = ty.class_layout {
                self.rows.class_layout.push(ClassLayoutTableRow {
                    packing_size: layout.packing_size,
                    class_size: layout.class_size,
                    parent: TypeDefTableOffset(rid),
                });
            }
            for method_impl in &ty.method_impls {
                let row = MethodImplTableRow {
                    class: TypeDefTableOffset(rid),
                    method_body: self.method_def_or_ref(&method_impl.body)?,
                    method_declaration: self.method_def_or_ref(&method_impl.declaration)?,
                };
                self.rows.method_impl.push(row);
            }
            if let Some(first) = ty.properties.first() {
                self.rows.property_map.push(PropertyMapTableRow {
                    parent: TypeDefTableOffset(rid),
                    property_list: PropertyTableOffset(self.properties.get(first).unwrap()),
                });
            }
            if let Some(first) = ty.events.first() {
                self.rows.event_map.push(EventMapTableRow {
                    parent: TypeDefTableOffset(rid),
                    event_list: EventTableOffset(self.events.get(first).unwrap()),
                });
            }
            for s in &ty.security {
                security.push((HasDeclSecurityToken::TypeDef, rid, s.clone()));
            }
            self.custom_attributes(HasCustomAttributeToken::TypeDef, rid, &ty.custom_attributes)?;
        }

        for (i, field) in self.fields.list.clone().iter().enumerate() {
            let rid = i as u32 + 1;
            let f = field.get();
            self.constant(HasConstantToken::Field, rid, &f.constant);
            self.marshal(HasFieldMarshalToken::Field, rid, &f.marshal);
            self.impl_map(MemberForwardedToken::Field, rid, &f.impl_map);
            if let Some(offset) = f.offset {
                self.rows.field_layout.push(FieldLayoutTableRow {
                    offset,
                    field: FieldTableOffset(rid),
                });
            }
            if let Some(data) = &f.initial_value {
                self.pending_field_rvas.push((rid, data.clone()));
            }
            self.custom_attributes(HasCustomAttributeToken::Field, rid, &f.custom_attributes)?;
        }

        for (i, method) in self.methods.list.clone().iter().enumerate() {
            let rid = i as u32 + 1;
            let m = method.get();
            self.impl_map(MemberForwardedToken::Method, rid, &m.impl_map);
            for s in &m.security {
                security.push((HasDeclSecurityToken::Method, rid, s.clone()));
            }
            self.custom_attributes(HasCustomAttributeToken::Method, rid, &m.custom_attributes)?;
        }

        for (i, param) in self.params.list.clone().iter().enumerate() {
            let rid = i as u32 + 1;
            let p = param.get();
            self.constant(HasConstantToken::Param, rid, &p.constant);
            self.marshal(HasFieldMarshalToken::Param, rid, &p.marshal);
            self.custom_attributes(HasCustomAttributeToken::Param, rid, &p.custom_attributes)?;
        }

        for (i, property) in self.properties.list.clone().iter().enumerate() {
            let rid = i as u32 + 1;
            let p = property.get();
            let signature = self.sig_blob(|w| w.write_property_sig(&p.signature))?;
            let row = PropertyTableRow {
                prop_flags: p.flags,
                name: self.string(&p.name),
                ty: signature,
            };
            self.rows.property.push(row);
            self.constant(HasConstantToken::Property, rid, &p.constant);
            let association = CodedToken::new(HasSemanticToken::Property, rid);
            let semantics = p
                .setter
                .iter()
                .map(|m| (MethodSemanticsFlags::Setter, m))
                .chain(p.getter.iter().map(|m| (MethodSemanticsFlags::Getter, m)))
                .chain(
                    p.other_methods
                        .iter()
                        .map(|m| (MethodSemanticsFlags::Other, m)),
                );
            for (semantic, method) in semantics {
                self.rows.method_semantics.push(MethodSemanticsTableRow {
                    semantic,
                    method: MethodTableOffset(self.method_rid(method)?),
                    association,
                });
            }
            self.custom_attributes(HasCustomAttributeToken::Property, rid, &p.custom_attributes)?;
        }

        for (i, event) in self.events.list.clone().iter().enumerate() {
            let rid = i as u32 + 1;
            let e = event.get();
            let event_type = match &e.event_type {
                Some(t) => self.type_def_or_ref(t)?,
                None => CodedToken::new(TypeDefOrRefToken::TypeDef, 0),
            };
            let row = EventTableRow {
                event_flags: e.flags,
                name: self.string(&e.name),
                event_type,
            };
            self.rows.event.push(row);
            let association = CodedToken::new(HasSemanticToken::Event, rid);
            let semantics = e
                .add
                .iter()
                .map(|m| (MethodSemanticsFlags::AddOn, m))
                .chain(e.remove.iter().map(|m| (MethodSemanticsFlags::RemoveOn, m)))
                .chain(e.fire.iter().map(|m| (MethodSemanticsFlags::Fire, m)))
                .chain(
                    e.other_methods
                        .iter()
                        .map(|m| (MethodSemanticsFlags::Other, m)),
                );
            for (semantic, method) in semantics {
                self.rows.method_semantics.push(MethodSemanticsTableRow {
                    semantic,
                    method: MethodTableOffset(self.method_rid(method)?),
                    association,
                });
            }
            self.custom_attributes(HasCustomAttributeToken::Event, rid, &e.custom_attributes)?;
        }

        for (i, gp) in self.generic_params.list.clone().iter().enumerate() {
            let rid = i as u32 + 1;
            let g = gp.get();
            for constraint in &g.constraints {
                let token = self.type_def_or_ref(&constraint.constraint)?;
                self.rows
                    .generic_param_constraint
                    .push(GenericParamConstraintTableRow {
                        owner: GenericParamTableOffset(rid),
                        constraint: token,
                    });
                let constraint_rid = self.rows.generic_param_constraint.len() as u32;
                self.custom_attributes(
                    HasCustomAttributeToken::GenericParamConstraint,
                    constraint_rid,
                    &constraint.custom_attributes,
                )?;
            }
            self.custom_attributes(
                HasCustomAttributeToken::GenericParam,
                rid,
                &g.custom_attributes,
            )?;
        }

        if let Some(assembly) = &module.assembly {
            let a = assembly.get();
            let row = AssemblyTableRow {
                hash_alg_id: a.hash_alg_id,
                major_version: a.version.major,
                minor_version: a.version.minor,
                build_number: a.version.build,
                revision_number: a.version.revision,
                flags: a.flags,
                public_key: self.blob(&a.public_key),
                name: self.string(&a.name),
                locale: self.string(&a.culture),
            };
            self.rows.assembly.push(row);
            for s in &a.security {
                security.push((HasDeclSecurityToken::Assembly, 1, s.clone()));
            }
            self.custom_attributes(HasCustomAttributeToken::Assembly, 1, &a.custom_attributes)?;
        }

        for (i, resource) in module.resources.iter().enumerate() {
            let r = resource.get();
            let (offset, implementation) = match &r.data {
                ResourceData::Embedded(data) => {
                    self.resources.resize(align_up(self.resources.len(), 8), 0);
                    let offset = self.resources.len() as u32;
                    self.resources
                        .extend_from_slice(&(data.len() as u32).to_le_bytes());
                    self.resources.extend_from_slice(data);
                    (offset, CodedToken::new(ImplementationToken::File, 0))
                }
                ResourceData::Linked {
                    implementation,
                    offset,
                } => (*offset, self.implementation(&Some(implementation.clone()))),
            };
            let row = ManifestResourceTableRow {
                offset,
                flags: r.flags,
                name: self.string(&r.name),
                implementation,
            };
            self.rows.manifest_resource.push(row);
            self.custom_attributes(
                HasCustomAttributeToken::ManifestResource,
                i as u32 + 1,
                &r.custom_attributes,
            )?;
        }

        Ok(security)
    }

    /// Builds rows for reference tables, including entries registered while
    /// building other rows, until nothing new is added.
    fn flush_references(&mut self) -> Result<()> {
        loop {
            let mut progress = false;

            while self.rows.assembly_ref.len() < self.assembly_refs.list.len() {
                let rid = self.rows.assembly_ref.len() as u32 + 1;
                let entry = self.assembly_refs.list[rid as usize - 1].clone();
                let a = entry.get();
                let row = AssemblyRefTableRow {
                    major_version: a.version.major,
                    minor_version: a.version.minor,
                    build_number: a.version.build,
                    revision_number: a.version.revision,
                    flags: a.flags,
                    public_key_or_token: self.blob(&a.public_key_or_token),
                    name: self.string(&a.name),
                    locale: self.string(&a.culture),
                    hash_value: self.blob(&a.hash_value),
                };
                self.rows.assembly_ref.push(row);
                self.custom_attributes(
                    HasCustomAttributeToken::AssemblyRef,
                    rid,
                    &a.custom_attributes,
                )?;
                progress = true;
            }

            while self.rows.module_ref.len() < self.module_refs.list.len() {
                let rid = self.rows.module_ref.len() as u32 + 1;
                let entry = self.module_refs.list[rid as usize - 1].clone();
                let m = entry.get();
                let row = ModuleRefTableRow {
                    name: self.string(&m.name),
                };
                self.rows.module_ref.push(row);
                self.custom_attributes(
                    HasCustomAttributeToken::ModuleRef,
                    rid,
                    &m.custom_attributes,
                )?;
                progress = true;
            }

            while self.rows.type_ref.len() < self.type_refs.list.len() {
                let rid = self.rows.type_ref.len() as u32 + 1;
                let entry = self.type_refs.list[rid as usize - 1].clone();
                let t = entry.get();
                let resolution_scope = match &t.scope {
                    None => CodedToken::new(ResolutionScopeToken::Module, 0),
                    Some(ResolutionScope::Module(_)) => {
                        CodedToken::new(ResolutionScopeToken::Module, 1)
                    }
                    Some(ResolutionScope::ModuleRef(m)) => {
                        CodedToken::new(ResolutionScopeToken::ModuleRef, self.module_refs.add(m))
                    }
                    Some(ResolutionScope::AssemblyRef(a)) => CodedToken::new(
                        ResolutionScopeToken::AssemblyRef,
                        self.assembly_refs.add(a),
                    ),
                    Some(ResolutionScope::TypeRef(r)) => {
                        CodedToken::new(ResolutionScopeToken::TypeRef, self.type_refs.add(r))
                    }
                };
                let row = TypeRefTableRow {
                    resolution_scope,
                    name: self.string(&t.name),
                    namespace: self.string(&t.namespace),
                };
                self.rows.type_ref.push(row);
                self.custom_attributes(
                    HasCustomAttributeToken::TypeRef,
                    rid,
                    &t.custom_attributes,
                )?;
                progress = true;
            }

            while self.rows.type_spec.len() < self.type_specs.list.len() {
                let rid = self.rows.type_spec.len() as u32 + 1;
                let entry = self.type_specs.list[rid as usize - 1].clone();
                let t = entry.get();
                let signature = self.sig_blob(|w| w.write_type(&t.signature))?;
                self.rows.type_spec.push(TypeSpecTableRow {
                    signature: BlobStreamOffsetTypeSpec(signature.0),
                });
                self.custom_attributes(
                    HasCustomAttributeToken::TypeSpec,
                    rid,
                    &t.custom_attributes,
                )?;
                progress = true;
            }

            while self.rows.member_ref.len() < self.member_refs.list.len() {
                let rid = self.rows.member_ref.len() as u32 + 1;
                let entry = self.member_refs.list[rid as usize - 1].clone();
                let m = entry.get();
                let class = match &m.parent {
                    MemberRefParent::TypeDef(t) => {
                        CodedToken::new(MemberRefParentToken::TypeDef, self.type_def_rid(t)?)
                    }
                    MemberRefParent::TypeRef(t) => {
                        CodedToken::new(MemberRefParentToken::TypeRef, self.type_refs.add(t))
                    }
                    MemberRefParent::ModuleRef(t) => {
                        CodedToken::new(MemberRefParentToken::ModuleRef, self.module_refs.add(t))
                    }
                    MemberRefParent::Method(t) => {
                        CodedToken::new(MemberRefParentToken::Method, self.method_rid(t)?)
                    }
                    MemberRefParent::TypeSpec(t) => {
                        CodedToken::new(MemberRefParentToken::TypeSpec, self.type_specs.add(t))
                    }
                };
                let signature = self.sig_blob(|w| w.write_member_ref_sig(&m.signature))?;
                let row = MemberRefTableRow {
                    class,
                    name: self.string(&m.name),
                    signature,
                };
                self.rows.member_ref.push(row);
                self.custom_attributes(
                    HasCustomAttributeToken::MemberRef,
                    rid,
                    &m.custom_attributes,
                )?;
                progress = true;
            }

            while self.rows.method_spec.len() < self.method_specs.list.len() {
                let rid = self.rows.method_spec.len() as u32 + 1;
                let entry = self.method_specs.list[rid as usize - 1].clone();
                let m = entry.get();
                let method = self.method_def_or_ref(&m.method)?;
                let instantiation = self.sig_blob(|w| w.write_method_spec_sig(&m.instantiation))?;
                self.rows.method_spec.push(MethodSpecTableRow {
                    method,
                    instantiation,
                });
                self.custom_attributes(
                    HasCustomAttributeToken::MethodSpec,
                    rid,
                    &m.custom_attributes,
                )?;
                progress = true;
            }

            while self.rows.file.len() < self.files.list.len() {
                let rid = self.rows.file.len() as u32 + 1;
                let entry = self.files.list[rid as usize - 1].clone();
                let f = entry.get();
                let row = FileTableRow {
                    flags: f.flags,
                    name: self.string(&f.name),
                    hash_value: self.blob(&f.hash_value),
                };
                self.rows.file.push(row);
                self.custom_attributes(HasCustomAttributeToken::File, rid, &f.custom_attributes)?;
                progress = true;
            }

            while self.rows.exported_type.len() < self.exported_types.list.len() {
                let rid = self.rows.exported_type.len() as u32 + 1;
                let entry = self.exported_types.list[rid as usize - 1].clone();
                let e = entry.get();
                let implementation = self.implementation(&e.implementation);
                let row = ExportedTypeTableRow {
                    flags: e.flags,
                    type_def_id: e.type_def_id,
                    type_name: self.string(&e.name),
                    type_namespace: self.string(&e.namespace),
                    implementation,
                };
                self.rows.exported_type.push(row);
                self.custom_attributes(
                    HasCustomAttributeToken::ExportedType,
                    rid,
                    &e.custom_attributes,
                )?;
                progress = true;
            }

            if !progress {
                return Ok(());
            }
        }
    }

    fn finish_tables(
        &mut self,
        mut security: Vec<(HasDeclSecurityToken, u32, DeclSecurity)>,
    ) -> Result<()> {
        security.sort_by_key(|(target, rid, _)| CodedToken::new(*target, *rid).encode());
        for (target, rid, s) in security {
            let permission_set = self.blob(&s.permission_set);
            self.rows.decl_security.push(DeclSecurityTableRow {
                action: s.action,
                parent: CodedToken::new(target, rid),
                permission_set,
            });
            let security_rid = self.rows.decl_security.len() as u32;
            self.custom_attributes(
                HasCustomAttributeToken::DeclSecurity,
                security_rid,
                &s.custom_attributes,
            )?;
        }

        self.flush_references()?;

        let field_data_rva = self.section_rva + align_up(self.code.len(), 8) as u32;
        for (rid, data) in std::mem::take(&mut self.pending_field_rvas) {
            self.field_data
                .resize(align_up(self.field_data.len(), 8), 0);
            let rva = field_data_rva + self.field_data.len() as u32;
            self.field_data.extend_from_slice(&data);
            self.rows.field_rva.push(FieldRVATableRow {
                rva,
                field: FieldTableOffset(rid),
            });
        }

        let r = &mut self.rows;
        r.constant.sort_by_key(|row| row.parent.encode());
        r.custom_attribute.sort_by_key(|row| row.parent.encode());
        r.field_marshal.sort_by_key(|row| row.parent.encode());
        r.method_semantics
            .sort_by_key(|row| row.association.encode());
        r.impl_map.sort_by_key(|row| row.member_forwarded.encode());
        r.field_layout.sort_by_key(|row| row.field.0);
        r.field_rva.sort_by_key(|row| row.field.0);
        r.class_layout.sort_by_key(|row| row.parent.0);
        r.nested_class.sort_by_key(|row| row.nested_class.0);
        r.method_impl.sort_by_key(|row| row.class.0);
        Ok(())
    }

    fn table_rows(&self) -> TableRows {
        let r = &self.rows;
        let c = |n: usize| TableRowCount(n as u32);
        TableRows {
            module: c(r.module.len()),
            type_ref: c(r.type_ref.len()),
            type_def: c(r.type_def.len()),
            field: c(r.field.len()),
            method: c(r.method.len()),
            param: c(r.param.len()),
            interface_impl: c(r.interface_impl.len()),
            member_ref: c(r.member_ref.len()),
            constant: c(r.constant.len()),
            custom_attribute: c(r.custom_attribute.len()),
            field_marshal: c(r.field_marshal.len()),
            decl_security: c(r.decl_security.len()),
            class_layout: c(r.class_layout.len()),
            field_layout: c(r.field_layout.len()),
            stand_alone_sig: c(r.stand_alone_sig.len()),
            event_map: c(r.event_map.len()),
            event: c(r.event.len()),
            property_map: c(r.property_map.len()),
            property: c(r.property.len()),
            method_semantics: c(r.method_semantics.len()),
            method_impl: c(r.method_impl.len()),
            module_ref: c(r.module_ref.len()),
            type_spec: c(r.type_spec.len()),
            impl_map: c(r.impl_map.len()),
            field_rva: c(r.field_rva.len()),
            assembly: c(r.assembly.len()),
            assembly_ref: c(r.assembly_ref.len()),
            file: c(r.file.len()),
            exported_type: c(r.exported_type.len()),
            manifest_resource: c(r.manifest_resource.len()),
            nested_class: c(r.nested_class.len()),
            generic_param: c(r.generic_param.len()),
            method_spec: c(r.method_spec.len()),
            generic_param_constraint: c(r.generic_param_constraint.len()),
            ..Default::default()
        }
    }

    fn tables_stream(&self) -> Vec<u8> {
        let rows = self.table_rows();
        let coded = CodedTokenSizes::from_header(&rows);
        let mut flags = MDStreamFlags::empty();
        if self.strings.data.len() > 0xFFFF {
            flags |= MDStreamFlags::BigStrings;
        }
        if self.guids.data.len() / 16 > 0xFFFF {
            flags |= MDStreamFlags::BigGUID;
        }
        if self.blobs.data.len() > 0xFFFF {
            flags |= MDStreamFlags::BigBlob;
        }

        let mut w = RowWriter {
            buf: Vec::new(),
            rows: &rows,
            flags,
            coded: &coded,
        };

        let r = &self.rows;
        let mut valid = 0u64;
        let mut counts = Vec::new();
        macro_rules! tables {
            ($($bit:literal => $field:ident),* $(,)?) => {
                $(
                    if !r.$field.is_empty() {
                        valid |= 1 << $bit;
                        counts.push(r.$field.len() as u32);
                    }
                )*
                $(
                    for row in &r.$field {
                        row.write_row(&mut w);
                    }
                )*
            };
        }
        tables! {
            0x00 => module,
            0x01 => type_ref,
            0x02 => type_def,
            0x04 => field,
            0x06 => method,
            0x08 => param,
            0x09 => interface_impl,
            0x0A => member_ref,
            0x0B => constant,
            0x0C => custom_attribute,
            0x0D => field_marshal,
            0x0E => decl_security,
            0x0F => class_layout,
            0x10 => field_layout,
            0x11 => stand_alone_sig,
            0x12 => event_map,
            0x14 => event,
            0x15 => property_map,
            0x17 => property,
            0x18 => method_semantics,
            0x19 => method_impl,
            0x1A => module_ref,
            0x1B => type_spec,
            0x1C => impl_map,
            0x1D => field_rva,
            0x20 => assembly,
            0x23 => assembly_ref,
            0x26 => file,
            0x27 => exported_type,
            0x28 => manifest_resource,
            0x29 => nested_class,
            0x2A => generic_param,
            0x2B => method_spec,
            0x2C => generic_param_constraint,
        }

        const SORTED: u64 = 0x0000_1600_3301_FA00;
        let mut out = Vec::new();
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&[2, 0, flags.bits(), 1]);
        out.extend_from_slice(&valid.to_le_bytes());
        out.extend_from_slice(&SORTED.to_le_bytes());
        for count in counts {
            out.extend_from_slice(&count.to_le_bytes());
        }
        out.extend_from_slice(&w.buf);
        out
    }

    fn metadata(&self) -> Vec<u8> {
        fn padded(mut data: Vec<u8>) -> Vec<u8> {
            data.resize(align_up(data.len(), 4), 0);
            data
        }
        let streams: [(&str, Vec<u8>); 5] = [
            ("#~", padded(self.tables_stream())),
            ("#Strings", padded(self.strings.data.clone())),
            ("#US", padded(self.user_strings.data.clone())),
            ("#GUID", self.guids.data.clone()),
            ("#Blob", padded(self.blobs.data.clone())),
        ];

        let mut version = self.module.runtime_version.as_bytes().to_vec();
        version.push(0);
        let version = padded(version);

        let header_size = 16
            + version.len()
            + 4
            + streams
                .iter()
                .map(|(name, _)| 8 + align_up(name.len() + 1, 4))
                .sum::<usize>();

        let mut out = Vec::new();
        out.extend_from_slice(&0x424A_5342u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(version.len() as u32).to_le_bytes());
        out.extend_from_slice(&version);
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&(streams.len() as u16).to_le_bytes());

        let mut offset = header_size;
        for (name, data) in &streams {
            out.extend_from_slice(&(offset as u32).to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            let mut name = name.as_bytes().to_vec();
            name.push(0);
            out.extend_from_slice(&padded(name));
            offset += data.len();
        }
        debug_assert_eq!(out.len(), header_size);
        for (_, data) in streams {
            out.extend_from_slice(&data);
        }
        out
    }
}

impl TokenEncoder for ModuleWriter<'_> {
    fn type_def_or_ref(&mut self, ty: &TypeDefOrRef) -> Result<u32> {
        ModuleWriter::type_def_or_ref(self, ty).map(|t| t.encode())
    }
}

impl BodyEncoder for ModuleWriter<'_> {
    fn member_token(&mut self, token: &MemberToken) -> Result<u32> {
        Ok(match token {
            MemberToken::TypeRef(t) => 0x0100_0000 | self.type_refs.add(t),
            MemberToken::TypeDef(t) => 0x0200_0000 | self.type_def_rid(t)?,
            MemberToken::Field(f) => 0x0400_0000 | self.field_rid(f)?,
            MemberToken::Method(m) => 0x0600_0000 | self.method_rid(m)?,
            MemberToken::MemberRef(m) => 0x0A00_0000 | self.member_refs.add(m),
            MemberToken::TypeSpec(t) => 0x1B00_0000 | self.type_specs.add(t),
            MemberToken::MethodSpec(m) => 0x2B00_0000 | self.method_specs.add(m),
        })
    }

    fn user_string(&mut self, value: &str) -> Result<u32> {
        let offset = self.user_strings.add(value);
        if self.user_strings.len() > 0x00FF_FFFF {
            return Err(err("#US heap is too large"));
        }
        Ok(0x7000_0000 | offset)
    }

    fn method_sig_token(&mut self, sig: &MethodSig) -> Result<u32> {
        let blob = self.sig_blob(|w| w.write_method_sig(sig))?;
        Ok(self.stand_alone_sig(blob))
    }

    fn locals_token(&mut self, locals: &[TypeSig]) -> Result<u32> {
        let blob = self.sig_blob(|w| w.write_local_sig(locals))?;
        Ok(self.stand_alone_sig(blob))
    }
}

pub(crate) fn write_module(module: &Module) -> Result<Vec<u8>> {
    let mut cor20 = module.cor20.clone();
    if !cor20.vtable_fixups.is_null() {
        return Err(err(
            "mixed mode assemblies (vtable fixups) are not supported",
        ));
    }

    let mut image = if module.image.is_empty() {
        new_image(module.kind)
    } else {
        PEImageDef::parse(&module.image).map_err(HaoError::PeError)?
    };
    let cor20_rva = image
        .optional_header
        .data_directories
        .clr_runtime_header
        .virtual_address;
    let section_rva = image
        .sections
        .next_virtual_address(image.section_alignment()) as u32;

    let mut writer = ModuleWriter::new(module, section_rva);
    let security = writer.build()?;
    writer.finish_tables(security)?;

    let mut data = std::mem::take(&mut writer.code);
    data.resize(align_up(data.len(), 8), 0);
    data.extend_from_slice(&writer.field_data);
    data.resize(align_up(data.len(), 8), 0);

    let resources_rva = section_rva + data.len() as u32;
    let resources_len = writer.resources.len() as u32;
    data.extend_from_slice(&writer.resources);
    data.resize(align_up(data.len(), 4), 0);

    let metadata = writer.metadata();
    let metadata_rva = section_rva + data.len() as u32;
    data.extend_from_slice(&metadata);

    cor20.metadata = ImageDataDirectory {
        virtual_address: metadata_rva,
        size: metadata.len() as u32,
    };
    cor20.resources = if resources_len > 0 {
        ImageDataDirectory {
            virtual_address: resources_rva,
            size: resources_len,
        }
    } else {
        ImageDataDirectory::default()
    };
    cor20.flags.remove(Cor20Flags::STRONG_NAME_SIGNED);
    // ReadyToRun code refers to the old metadata, so drop it and run from IL.
    if !cor20.managed_native_header.is_null() {
        cor20.managed_native_header = ImageDataDirectory::default();
        cor20.flags.remove(Cor20Flags::IL_LIBRARY);
        cor20.flags.insert(Cor20Flags::IL_ONLY);
        image.coff_header.machine = native_machine(image.coff_header.machine);
        let dirs = &mut image.optional_header.data_directories;
        dirs.exception_table = ImageDataDirectory::default();
        if image.optional_header.windows_specific_fields.is_pe32_plus() {
            dirs.base_relocation_table = ImageDataDirectory::default();
        }
    }
    if !cor20.flags.contains(Cor20Flags::NATIVE_ENTRYPOINT) {
        cor20.entrypoint_token_or_rva = match &module.entry_point {
            Some(m) => 0x0600_0000 | writer.method_rid(m)?,
            None if cor20.entrypoint_token_or_rva >> 24 == 0x26 => cor20.entrypoint_token_or_rva,
            None => 0,
        };
    }

    let section = image.new_section(
        SECTION_NAME,
        SectionFlags::CNT_CODE | SectionFlags::MEM_EXECUTE | SectionFlags::MEM_READ,
    );
    section.data = data;

    let mut cor20_bytes = Vec::new();
    pewter::io::Writer::write(&mut cor20_bytes, &cor20).map_err(HaoError::PeError)?;
    image
        .write_at_rva(cor20_rva, &cor20_bytes)
        .map_err(HaoError::PeError)?;
    image.strip_certificate();

    image.write_file().map_err(HaoError::PeError)
}

/// An AnyCPU, IL only image holding just a placeholder cor20 header.
fn new_image(kind: ModuleKind) -> PEImageDef {
    let mut characteristics =
        ImageFileCharacteristics::EXECUTABLE_IMAGE | ImageFileCharacteristics::LARGE_ADDRESS_AWARE;
    let (subsystem, image_base) = match kind {
        ModuleKind::Dll => {
            characteristics |= ImageFileCharacteristics::FILE_DLL;
            (ImageSubsystem::WindowsCui, 0x1000_0000)
        }
        ModuleKind::Console => (ImageSubsystem::WindowsCui, 0x40_0000),
        ModuleKind::Windows => (ImageSubsystem::WindowsGui, 0x40_0000),
    };
    let mut image = PEImageDef::new(
        ImageFileMachine::I386,
        characteristics,
        OptionalHeader::new_pe32(subsystem, image_base),
    );
    let text = image.new_section(
        ".text",
        SectionFlags::CNT_CODE | SectionFlags::MEM_EXECUTE | SectionFlags::MEM_READ,
    );
    let rva = text.add_data(&[0; ImageCor20Header::SIZE]) as u32;
    image.optional_header.data_directories.clr_runtime_header = ImageDataDirectory {
        virtual_address: rva,
        size: ImageCor20Header::SIZE as u32,
    };
    image
}

/// ReadyToRun images XOR the machine type with an OS specific value.
fn native_machine(machine: ImageFileMachine) -> ImageFileMachine {
    const OS_XOR: [u16; 5] = [0x7B79, 0x4644, 0xADC4, 0x1993, 0x1992];
    if let ImageFileMachine::Other(value) = machine {
        for xor in OS_XOR {
            let candidate = ImageFileMachine::from_u16(value ^ xor);
            if !matches!(candidate, ImageFileMachine::Other(_)) {
                return candidate;
            }
        }
    }
    machine
}

impl Module {
    /// Writes the module to a new PE image based on the one it was loaded from.
    pub fn write(&self) -> Result<Vec<u8>> {
        write_module(self)
    }

    pub fn write_to_path(&self, path: impl AsRef<std::path::Path>) -> Result<()> {
        let data = self.write()?;
        std::fs::write(path, data).map_err(HaoError::IoError)
    }
}
