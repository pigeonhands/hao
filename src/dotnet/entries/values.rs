use num_traits::Zero;

use super::{
    super::md::streams::{
        tables_stream::{
            coded_tokens::{CodedToken, ResolutionScopeToken, TypeDefOrRefToken},
            FieldFlags, FieldTableRow, MethodFlags, MethodImplFlags, MethodTableRow,
            ModulesTableRow, TypeAttributes, TypeDefTableRow, TypeRefTableRow,
        },
        SignatureDef,
    },
    signature::{
        FieldSignature, MethodSignature, ResolutionScope, TypeDefOrRef, TypeSignature, ValueType,
    },
    well_known::{SystemType, WellKnown},
    EntryCollection, EntryView, {Ptr, ReadEntry, RowRange},
};
use crate::{
    dotnet::{
        entries::{GetEntryField, MaybeUninitEntries},
        md::streams::tables_stream::{
            AssemblyFlags, AssemblyRefTableRow, InterfaceImplTableRow, ModuleRefTableRow,
            ParamFlags, ParamTableRow, TypeSpecTableRow,
        },
    },
    error::{HaoError, Result},
    io::{EntryReader, ValueReadable},
    Module,
};
use std::{
    fmt::{Debug, Display},
    rc::Rc,
};

#[derive(Debug, Clone)]
pub struct ModuleDef {
    pub generation: u16,
    pub name: String,
    pub mvid: uuid::Uuid,
    pub enc_id: uuid::Uuid,
    pub enc_base_id: uuid::Uuid,
}

impl ModuleDef {
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl<'a> ReadEntry<ModuleDef> for EntryReader<'a> {
    type RawRow = ModulesTableRow;
    fn from_row(
        &self,
        _: usize,
        row: &Self::RawRow,
        _next: Option<&Self::RawRow>,
    ) -> Result<ModuleDef> {
        Ok(ModuleDef {
            generation: self.read(row.generation)?,
            name: self.read(row.name)?,
            mvid: self.read(row.mvid)?,
            enc_id: self.read(row.enc_id)?,
            enc_base_id: self.read(row.enc_base_id)?,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) enum ResolutionScopePtr {
    Module(Ptr<ModuleDef>),
    ModuleRef(Ptr<ModuleRef>),
    AssemblyRef(Ptr<AssemblyRef>),
    TypeRef(Ptr<TypeRef>),
    None,
}

impl<'a> GetEntryField<CodedToken<ResolutionScopeToken>> for MaybeUninitEntries {
    type EntryFieldValue = ResolutionScopePtr;

    fn get_entry_field(
        &self,
        identifier: CodedToken<ResolutionScopeToken>,
    ) -> Result<Self::EntryFieldValue> {
        let index = match (identifier.rid as usize).checked_sub(1) {
            Some(v) => v,
            None => return Ok(ResolutionScopePtr::None),
        };

        let val = match identifier.target {
            ResolutionScopeToken::Module => self
                .modules
                .get(index)
                .cloned()
                .map(ResolutionScopePtr::Module),
            ResolutionScopeToken::ModuleRef => self
                .module_ref
                .get(index)
                .cloned()
                .map(ResolutionScopePtr::ModuleRef),
            ResolutionScopeToken::AssemblyRef => self
                .assembly_ref
                .get(index)
                .cloned()
                .map(ResolutionScopePtr::AssemblyRef),
            ResolutionScopeToken::TypeRef => self
                .type_refs
                .get(index)
                .cloned()
                .map(ResolutionScopePtr::TypeRef),
        };
        val.ok_or_else(|| HaoError::InvalidCodedTokenOffset(identifier.rid, "ResolutionScopeToken"))
    }
}

#[derive(Debug, Clone)]
pub struct TypeRef {
    pub(crate) resolution_scope: ResolutionScope,
    pub(crate) name: String,
    pub(crate) namespace: String,
}

impl TypeRef {
    pub fn resolution_scope(&self) -> &ResolutionScope {
        &self.resolution_scope
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn well_known(&self) -> Option<WellKnown> {
        match self.resolution_scope() {
            ResolutionScope::AssemblyRef(r) if r.value().is_corlib() => {
                WellKnown::from_full_name(self.namespace(), self.name())
            }
            _ => return None,
        }
    }

    pub fn full_name_is(&self, namespace: &str, name: &str) -> bool {
        (namespace, name) == (self.namespace(), self.name())
    }

    pub fn is_system_type(&self) -> bool {
        match self.resolution_scope() {
            ResolutionScope::AssemblyRef(r) if r.value().is_corlib() => {
                self.is_system_object() || self.is_system_value_type() || self.is_system_enum()
            }
            _ => false,
        }
    }

    pub fn is_corlib(&self) -> bool {
        match self.resolution_scope() {
            ResolutionScope::AssemblyRef(r) if r.value().is_corlib() => true,
            _ => false,
        }
    }

    pub fn is_system_type_instance(&self, system_type: SystemType) -> bool {
        SystemType::from_full_name(self.namespace(), self.name()) == Some(system_type)
    }

    pub fn is_system_enum(&self) -> bool {
        self.is_system_type_instance(SystemType::Enum)
    }

    pub fn is_system_object(&self) -> bool {
        self.is_system_type_instance(SystemType::Object)
    }

    pub fn is_system_value_type(&self) -> bool {
        self.is_system_type_instance(SystemType::ValueType)
    }
}

impl<'a> ReadEntry<TypeRef> for EntryReader<'a> {
    type RawRow = TypeRefTableRow;
    fn from_row(
        &self,
        _: usize,
        row: &Self::RawRow,
        _next: Option<&Self::RawRow>,
    ) -> Result<TypeRef> {
        Ok(TypeRef {
            resolution_scope: ResolutionScope::from_ent_ptr_must(self.read(row.resolution_scope)?)?,
            name: self.read(row.name)?,
            namespace: self.read(row.namespace)?,
        })
    }
}

impl Display for TypeRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(well_known) = self.well_known() {
            write!(f, "{}", well_known.type_name())
        } else {
            write!(f, "{}", self.name())
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum TypeDefOrRefPtr {
    TypeDef(Ptr<TypeDef>),
    TypeRef(Ptr<TypeRef>),
    TypeSpec(Ptr<TypeSpec>),
    None,
}

impl GetEntryField<CodedToken<TypeDefOrRefToken>> for MaybeUninitEntries {
    type EntryFieldValue = TypeDefOrRefPtr;

    fn get_entry_field(
        &self,
        identifier: CodedToken<TypeDefOrRefToken>,
    ) -> Result<Self::EntryFieldValue> {
        let index = match (identifier.rid as usize).checked_sub(1) {
            Some(v) => v,
            None => return Ok(TypeDefOrRefPtr::None),
        };

        let val = match identifier.target {
            TypeDefOrRefToken::TypeDef => self
                .type_defs
                .get(index)
                .cloned()
                .map(|c| TypeDefOrRefPtr::TypeDef(c)),
            TypeDefOrRefToken::TypeRef => self
                .type_refs
                .get(index)
                .cloned()
                .map(|c| TypeDefOrRefPtr::TypeRef(c)),
            TypeDefOrRefToken::TypeSpec => self
                .type_specs
                .get(index)
                .cloned()
                .map(|c| TypeDefOrRefPtr::TypeSpec(c)),
        };
        val.ok_or_else(|| HaoError::InvalidCodedTokenOffset(identifier.rid, "TypeDefOrRefToken"))
    }
}

#[derive(Debug, Clone)]
pub struct TypeDef {
    pub(crate) flags: TypeAttributes,
    pub(crate) name: String,
    pub(crate) namespace: String,
    pub(crate) extends: Option<TypeDefOrRef>,
    pub(crate) field_list: Vec<Ptr<Field>>,
    pub(crate) method_list: Vec<Ptr<Method>>,

    pub(crate) interface_impl: Vec<TypeDefOrRef>,
}

impl TypeDef {
    pub fn flags(&self) -> TypeAttributes {
        self.flags
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn namespace(&self) -> &str {
        &self.namespace
    }
    pub fn extends(&self) -> &Option<TypeDefOrRef> {
        &self.extends
    }

    /// Returns an [`EntryCollection`] of [`Field`] with all the fields
    /// associated with this type.
    ///
    /// ```
    /// # use hao::Module;
    /// let module = Module::default();
    ///
    /// for ty in module.types().values() {
    ///    for field in ty.fields().values() {
    ///        println!("{} {}", field.signature(), field.name());
    ///    }
    /// }
    /// ```
    pub fn fields(&self) -> EntryCollection<Field> {
        EntryCollection::new(&self.field_list)
    }

    /// Return the methods associated with this entity.
    /// ```
    /// # use hao::Module;
    /// let module = Module::default();
    ///
    /// for ty in module.types().values() {
    ///    for method in ty.methods().values() {
    ///        for param in method.params().values() {
    ///             println!("{}", param.name());
    ///        }
    ///    }
    /// }
    /// ```
    pub fn methods(&self) -> EntryCollection<Method> {
        EntryCollection::new(&self.method_list)
    }
    pub fn full_name_is(&self, namespace: &str, name: &str) -> bool {
        (namespace, name) == (self.namespace(), self.name())
    }
    pub fn is_static(&self) -> bool {
        self.flags.contains(TypeAttributes::AutoLayout)
            && self.flags.contains(TypeAttributes::Class)
            && self.flags.contains(TypeAttributes::Abstract)
            && self.flags.contains(TypeAttributes::Sealed)
            && self
                .extends
                .as_ref()
                .map(|c| match c {
                    TypeDefOrRef::TypeRef(r) => r.value().is_system_object(),
                    _ => false,
                })
                .unwrap_or(false)
    }

    pub fn is_enum(&self) -> bool {
        let flags = self.flags;
        !flags.contains(TypeAttributes::Abstract)
            && flags.contains(TypeAttributes::AutoLayout)
            && flags.contains(TypeAttributes::Class)
            && flags.contains(TypeAttributes::Sealed)
            && self
                .extends
                .as_ref()
                .map(|c| match c {
                    TypeDefOrRef::TypeRef(r) => r.value().is_system_enum(),
                    _ => false,
                })
                .unwrap_or(false)
    }
    pub fn is_interface(&self) -> bool {
        self.extends().is_none()
            && self.flags.contains(TypeAttributes::AutoLayout)
            && self.flags.contains(TypeAttributes::Interface)
    }
    pub fn is_struct(&self) -> bool {
        match &self.extends {
            Some(exent) => {
                if exent.is_type_ref_and(|r| {
                    r.is_system_value_type()
                }) {}
            }
            _ => return false,
        };
        self.flags.contains(TypeAttributes::Class)
            && self.flags.contains(TypeAttributes::Sealed)
            && !self.flags.contains(TypeAttributes::Abstract)
    }
    pub fn is_delegate(&self) -> bool {
        // base is "System", "MulticastDelegate"
        let flags = self.flags;
        !flags.contains(TypeAttributes::AutoLayout)
            && !flags.contains(TypeAttributes::Abstract)
            && flags.contains(TypeAttributes::Class)
            && flags.contains(TypeAttributes::Sealed)
    }
}

impl<'a> ReadEntry<TypeDef> for EntryReader<'a> {
    type RawRow = TypeDefTableRow;
    fn from_row(
        &self,
        _: usize,
        row: &Self::RawRow,
        _next: Option<&Self::RawRow>,
    ) -> Result<TypeDef> {
        Ok(TypeDef {
            flags: row.flags,
            name: self.read(row.name)?,
            namespace: self.read(row.namespace)?,
            extends: TypeDefOrRef::from_ent_pointer(self.read(row.extends)?),
            field_list: self.read(RowRange::new(row.field_list, _next.map(|x| x.field_list)))?,
            method_list: self.read(RowRange::new(row.method_list, _next.map(|x| x.method_list)))?,
            interface_impl: Vec::new(),
        })
    }
}

impl Display for TypeDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.flags.contains(TypeAttributes::Public) {
            write!(f, "public ")?;
        } else if self.flags.contains(TypeAttributes::NotPublic) {
            write!(f, "internal ")?;
        }

        if self.is_enum() {
            let enum_type = self
                .fields()
                .values()
                .filter(|x| x.flags.contains(FieldFlags::SpecialName))
                .map(|x| x.signature.clone().0)
                .next()
                .unwrap_or(ValueType::Void);
            write!(f, "enum {} : {}", self.name(), enum_type)?;
            return Ok(());
        }

        if self.is_static() {
            write!(f, "static class ")?;
        } else {
            if self.is_interface() {
                write!(f, "interface ")?;
            } else if self.is_struct() {
                if self.flags.contains(TypeAttributes::Sealed) {
                    write!(f, "readonly ")?;
                }
                write!(f, "struct ")?;
            } else {
                if self.flags.contains(TypeAttributes::Sealed) {
                    write!(f, "sealed ")?;
                }
                write!(f, "class ")?;
            }
        }

        if !self.namespace().is_empty() {
            write!(f, "{}.", self.namespace())?;
        }
        write!(f, "{}", self.name())?;

        let has_base_class = match self.extends() {
            Some(extend) if !extend.is_type_ref_and(|c| c.is_system_type()) => {
                write!(f, ": {}", extend)?;
                true
            }
            _ => false,
        };

        if !self.interface_impl.is_empty() {
            if has_base_class {
                write!(f, ", ")?;
            } else {
                write!(f, ": ")?;
            }
            for (index, imp) in self.interface_impl.iter().enumerate() {
                if index > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", imp)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Field {
    pub(crate) flags: FieldFlags,
    pub(crate) name: String,
    pub(crate) signature: FieldSignature,
}

impl Field {
    pub fn flags(&self) -> FieldFlags {
        self.flags
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn signature(&self) -> &FieldSignature {
        &self.signature
    }
}

impl<'a> ReadEntry<Field> for EntryReader<'a> {
    type RawRow = FieldTableRow;
    fn from_row(
        &self,
        _: usize,
        row: &Self::RawRow,
        _next: Option<&Self::RawRow>,
    ) -> Result<Field> {
        let signature: SignatureDef = self.read(row.signature)?;

        Ok(Field {
            flags: row.flags,
            name: self.read(row.name)?,
            signature: FieldSignature::from_sig_def(signature)?,
        })
    }
}

impl Display for Field {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.flags.contains(FieldFlags::Public) {
            write!(f, "public ")?;
        }
        if self.flags.contains(FieldFlags::Private) {
            write!(f, "private ")?;
        }
        if self.flags.contains(FieldFlags::Static) {
            write!(f, "static ")?;
        }
        write!(f, "{} {}", self.signature(), self.name())
    }
}

#[derive(Debug, Clone)]
pub struct Method {
    pub(crate) rva: u32,
    pub(crate) impl_flags: MethodImplFlags,
    pub(crate) flags: MethodFlags,
    pub(crate) name: String,
    pub(crate) signature: MethodSignature,
    pub(crate) param_list: Vec<Ptr<Param>>,
}

impl Method {
    pub fn rva(&self) -> u32 {
        todo!("{:?}", self.rva)
    }
    pub fn impl_flags(&self) -> MethodImplFlags {
        self.impl_flags
    }
    pub fn flags(&self) -> MethodFlags {
        self.flags
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn signature(&self) -> &MethodSignature {
        &self.signature
    }

    pub fn is_ctor(&self) -> bool {
        self.flags.contains(MethodFlags::SpecialName) && self.name == ".ctor"
    }

    pub fn is_cctor(&self) -> bool {
        self.flags.contains(MethodFlags::SpecialName) && self.name == ".cctor"
    }

    /// Returns the parameters associated with this method.
    /// ```
    /// # use hao::Module;
    /// let module = Module::default();
    ///
    /// for ty in module.types().values() {
    ///    for method in ty.methods().values() {
    ///        for param in method.params().values() {
    ///             println!("{}", param.name());
    ///        }
    ///    }
    /// }
    /// ```
    pub fn params(&self) -> EntryCollection<Param> {
        EntryCollection::new(self.param_list.as_slice())
    }
}

impl<'a> ReadEntry<Method> for EntryReader<'a> {
    type RawRow = MethodTableRow;
    fn from_row(
        &self,
        _: usize,
        row: &Self::RawRow,
        next: Option<&Self::RawRow>,
    ) -> Result<Method> {
        Ok(Method {
            rva: row.rva,
            impl_flags: row.impl_flags,
            flags: row.flags,
            name: self.read(row.name)?,
            signature: MethodSignature::from_sig_def(self.read(row.signature)?)?,
            param_list: self.read(RowRange::new(row.param_list, next.map(|x| x.param_list)))?,
        })
    }
}

impl Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.flags.contains(MethodFlags::Private) {
            write!(f, "private ")?;
        } else if self.flags.contains(MethodFlags::Public) {
            write!(f, "public ")?;
        }

        if self.flags.contains(MethodFlags::Static) {
            write!(f, "static ")?;
        }

        write!(f, "{} {}", self.signature.return_type, self.name)?;

        if let Some(generic_args) = self.signature.generic_params {
            write!(f, "<")?;
            for i in 0..generic_args {
                if !i.is_zero() {
                    write!(f, ", ")?;
                }
                write!(f, "M{}", i)?;
            }
            write!(f, ">")?;
        }

        write!(f, "(")?;
        for (i, param) in self.signature.parameters.iter().enumerate() {
            if !i.is_zero() {
                write!(f, ", ")?;
            }
            write!(f, "{}", param)?;
        }
        write!(f, ")")?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Param {
    pub flags: ParamFlags,
    pub sequence: u16,
    pub name: String,
}

impl Param {
    pub fn flags(&self) -> ParamFlags {
        self.flags
    }
    pub fn sequence(self) -> u16 {
        self.sequence
    }
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl<'a> ReadEntry<Param> for EntryReader<'a> {
    type RawRow = ParamTableRow;
    fn from_row(
        &self,
        _: usize,
        row: &Self::RawRow,
        _next: Option<&Self::RawRow>,
    ) -> Result<Param> {
        Ok(Param {
            flags: row.flags,
            sequence: row.sequence,
            name: self.read(row.name)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct InterfaceImpl {
    pub(crate) class: Ptr<TypeDef>,
    pub(crate) interface: TypeDefOrRef,
}

impl InterfaceImpl {
    pub fn class(&self) -> EntryView<TypeDef> {
        EntryView(&self.class)
    }
    pub fn interface(&self) -> &TypeDefOrRef {
        &self.interface
    }
}

impl<'a> ReadEntry<InterfaceImpl> for EntryReader<'a> {
    type RawRow = InterfaceImplTableRow;
    fn from_row(
        &self,
        _: usize,
        row: &Self::RawRow,
        _next: Option<&Self::RawRow>,
    ) -> Result<InterfaceImpl> {
        Ok(InterfaceImpl {
            class: self.read(row.class)?,
            interface: TypeDefOrRef::from_ent_ptr_must(self.read(row.interface)?)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ModuleRef {
    pub(crate) name: String,
}

impl ModuleRef {
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl<'a> ReadEntry<ModuleRef> for EntryReader<'a> {
    type RawRow = ModuleRefTableRow;
    fn from_row(
        &self,
        _: usize,
        row: &Self::RawRow,
        _next: Option<&Self::RawRow>,
    ) -> Result<ModuleRef> {
        Ok(ModuleRef {
            name: self.read(row.name)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct TypeSpec {
    pub(crate) signature: TypeSignature,
}

impl TypeSpec {
    pub fn signature(&self) -> &TypeSignature {
        &self.signature
    }
}

impl<'a> ReadEntry<TypeSpec> for EntryReader<'a> {
    type RawRow = TypeSpecTableRow;
    fn from_row(
        &self,
        _: usize,
        row: &Self::RawRow,
        _next: Option<&Self::RawRow>,
    ) -> Result<TypeSpec> {
        Ok(TypeSpec {
            signature: TypeSignature::from_sig_def(self.read(row.signature)?)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AssemblyRef {
    pub(crate) major_version: u16,
    pub(crate) minor_version: u16,
    pub(crate) build_number: u16,
    pub(crate) revision_number: u16,
    pub(crate) flags: AssemblyFlags,
    //pub public_key_or_token: Vec<u8>,
    pub(crate) name: String,
    pub(crate) locale: String,
    //pub hash_value: Vec<u8>,
    pub(crate) refrenced_assembly: Option<Rc<Module>>,
}

impl AssemblyRef {
    const KNOWN_CORLIB_NAMES: &[&'static str] = &[
        "System.Private.CoreLib",
        "netstandard",
        "mscorlib",
        "corefx",
        "System.Runtime",
    ];

    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn version(&self) -> (u16, u16, u16, u16) {
        (
            self.major_version,
            self.minor_version,
            self.build_number,
            self.revision_number,
        )
    }
    pub fn flags(&self) -> AssemblyFlags {
        self.flags
    }
    pub fn locale(&self) -> &str {
        &self.locale
    }

    pub fn is_corlib(&self) -> bool {
        Self::KNOWN_CORLIB_NAMES
            .iter()
            .any(|s| self.name.eq_ignore_ascii_case(s))
    }
}

impl<'a> ReadEntry<AssemblyRef> for EntryReader<'a> {
    type RawRow = AssemblyRefTableRow;
    fn from_row(
        &self,
        _: usize,
        row: &Self::RawRow,
        _next: Option<&Self::RawRow>,
    ) -> Result<AssemblyRef> {
        Ok(AssemblyRef {
            major_version: row.major_version,
            minor_version: row.minor_version,
            build_number: row.build_number,
            revision_number: row.revision_number,
            flags: row.flags,
            name: self.read(row.name)?,
            locale: self.read(row.locale)?,

            refrenced_assembly: None,
        })
    }
}
