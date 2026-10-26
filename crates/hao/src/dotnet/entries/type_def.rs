use super::{
    well_known::SystemType, Constant, CustomAttribute, DeclSecurity, Entry, EntryName, ImplMap,
    MethodDefOrRef, TypeDefOrRef, WeakEntry,
};
use crate::dotnet::{
    il::MethodBody,
    md::streams::tables_stream::{
        EventFlags, FieldFlags, GenericParamFlags, MethodFlags, MethodImplFlags, ParamFlags,
        PropertyFlags, TypeAttributes,
    },
    signature::{FieldSig, MethodSig, PropertySig},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClassLayout {
    pub packing_size: u16,
    pub class_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceImpl {
    pub interface: TypeDefOrRef,
    pub custom_attributes: Vec<CustomAttribute>,
}

impl InterfaceImpl {
    pub fn new(interface: TypeDefOrRef) -> Self {
        Self {
            interface,
            custom_attributes: Vec::new(),
        }
    }
}

/// An explicit interface method implementation (II.22.27).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodImpl {
    pub body: MethodDefOrRef,
    pub declaration: MethodDefOrRef,
}

#[derive(Debug, Clone, Default)]
pub struct TypeDef {
    pub flags: TypeAttributes,
    pub name: String,
    pub namespace: String,
    pub extends: Option<TypeDefOrRef>,
    pub fields: Vec<Entry<Field>>,
    pub methods: Vec<Entry<Method>>,
    pub properties: Vec<Entry<Property>>,
    pub events: Vec<Entry<Event>>,
    pub nested_types: Vec<Entry<TypeDef>>,
    pub generic_params: Vec<Entry<GenericParam>>,
    pub interfaces: Vec<InterfaceImpl>,
    pub method_impls: Vec<MethodImpl>,
    pub class_layout: Option<ClassLayout>,
    pub security: Vec<DeclSecurity>,
    pub custom_attributes: Vec<CustomAttribute>,
    pub declaring_type: WeakEntry<TypeDef>,
}

impl TypeDef {
    pub fn new(
        flags: TypeAttributes,
        namespace: impl Into<String>,
        name: impl Into<String>,
        extends: Option<TypeDefOrRef>,
    ) -> Self {
        Self {
            flags,
            name: name.into(),
            namespace: namespace.into(),
            extends,
            ..Default::default()
        }
    }

    pub fn full_name(&self) -> String {
        if let Some(parent) = self.declaring_type.upgrade() {
            format!("{}/{}", parent.get().full_name(), self.name)
        } else if self.namespace.is_empty() {
            self.name.clone()
        } else {
            format!("{}.{}", self.namespace, self.name)
        }
    }

    pub fn full_name_is(&self, namespace: &str, name: &str) -> bool {
        self.namespace == namespace && self.name == name
    }

    pub fn extends_system_type(&self, system_type: SystemType) -> bool {
        self.extends
            .as_ref()
            .is_some_and(|e| e.is_system_type(system_type))
    }

    pub fn is_interface(&self) -> bool {
        self.flags & TypeAttributes::ClassSemanticsMask == TypeAttributes::Interface
    }

    pub fn is_enum(&self) -> bool {
        self.extends_system_type(SystemType::Enum)
    }

    pub fn is_value_type(&self) -> bool {
        self.is_enum()
            || (self.extends_system_type(SystemType::ValueType)
                && !self.full_name_is("System", "Enum"))
    }

    pub fn is_delegate(&self) -> bool {
        self.extends_system_type(SystemType::MulticastDelegate)
    }

    pub fn is_static(&self) -> bool {
        !self.is_interface()
            && self.flags.contains(TypeAttributes::Abstract | TypeAttributes::Sealed)
    }

    pub fn is_nested(&self) -> bool {
        self.declaring_type.upgrade().is_some()
    }

    /// The underlying type of an enum (the `value__` field).
    pub fn enum_underlying_field(&self) -> Option<Entry<Field>> {
        self.fields
            .iter()
            .find(|f| {
                let f = f.get();
                !f.flags.contains(FieldFlags::Static) && f.flags.contains(FieldFlags::RTSpecialName)
            })
            .cloned()
    }

    pub fn find_method(&self, name: &str) -> Option<Entry<Method>> {
        self.methods.iter().find(|m| m.get().name == name).cloned()
    }

    pub fn find_field(&self, name: &str) -> Option<Entry<Field>> {
        self.fields.iter().find(|f| f.get().name == name).cloned()
    }

    pub fn add_field(this: &Entry<TypeDef>, mut field: Field) -> Entry<Field> {
        field.declaring_type = this.downgrade();
        let field = Entry::new(field);
        this.get_mut().fields.push(field.clone());
        field
    }

    pub fn add_method(this: &Entry<TypeDef>, mut method: Method) -> Entry<Method> {
        method.declaring_type = this.downgrade();
        let method = Entry::new(method);
        this.get_mut().methods.push(method.clone());
        method
    }

    pub fn add_nested_type(this: &Entry<TypeDef>, mut ty: TypeDef) -> Entry<TypeDef> {
        ty.declaring_type = this.downgrade();
        let ty = Entry::new(ty);
        this.get_mut().nested_types.push(ty.clone());
        ty
    }

    /// This type and all nested types, depth first.
    pub fn with_nested(this: &Entry<TypeDef>) -> Vec<Entry<TypeDef>> {
        let mut out = vec![this.clone()];
        for nested in &this.get().nested_types {
            out.extend(Self::with_nested(nested));
        }
        out
    }
}

impl EntryName for TypeDef {
    fn entry_name(&self) -> String {
        self.full_name()
    }
}

#[derive(Debug, Clone, Default)]
pub struct Field {
    pub flags: FieldFlags,
    pub name: String,
    pub signature: FieldSig,
    pub constant: Option<Constant>,
    /// Explicit offset from `FieldLayout`.
    pub offset: Option<u32>,
    pub marshal: Option<Vec<u8>>,
    /// Initial data from `FieldRVA`.
    pub initial_value: Option<Vec<u8>>,
    pub impl_map: Option<ImplMap>,
    pub custom_attributes: Vec<CustomAttribute>,
    pub declaring_type: WeakEntry<TypeDef>,
}

impl Field {
    pub fn new(flags: FieldFlags, name: impl Into<String>, signature: FieldSig) -> Self {
        Self {
            flags,
            name: name.into(),
            signature,
            ..Default::default()
        }
    }

    pub fn is_static(&self) -> bool {
        self.flags.contains(FieldFlags::Static)
    }
}

impl EntryName for Field {
    fn entry_name(&self) -> String {
        self.name.clone()
    }
}

#[derive(Debug, Clone, Default)]
pub struct Method {
    pub flags: MethodFlags,
    pub impl_flags: MethodImplFlags,
    pub name: String,
    pub signature: MethodSig,
    pub params: Vec<Entry<Param>>,
    pub body: Option<MethodBody>,
    pub generic_params: Vec<Entry<GenericParam>>,
    pub impl_map: Option<ImplMap>,
    pub security: Vec<DeclSecurity>,
    pub custom_attributes: Vec<CustomAttribute>,
    pub declaring_type: WeakEntry<TypeDef>,
}

impl Method {
    pub fn new(flags: MethodFlags, name: impl Into<String>, signature: MethodSig) -> Self {
        Self {
            flags,
            name: name.into(),
            signature,
            ..Default::default()
        }
    }

    pub fn is_static(&self) -> bool {
        self.flags.contains(MethodFlags::Static)
    }

    pub fn is_ctor(&self) -> bool {
        self.flags.contains(MethodFlags::RTSpecialName) && self.name == ".ctor"
    }

    pub fn is_cctor(&self) -> bool {
        self.flags.contains(MethodFlags::RTSpecialName) && self.name == ".cctor"
    }

    /// The `Param` row for parameter `index` (0 is the return value, 1 the first parameter).
    pub fn param(&self, sequence: u16) -> Option<Entry<Param>> {
        self.params
            .iter()
            .find(|p| p.get().sequence == sequence)
            .cloned()
    }

    /// Name of parameter `index` (zero based, excluding `this`).
    pub fn param_name(&self, index: usize) -> String {
        self.param(index as u16 + 1)
            .map(|p| p.get().name.clone())
            .unwrap_or_else(|| format!("arg{}", index))
    }
}

impl EntryName for Method {
    fn entry_name(&self) -> String {
        match self.declaring_type.upgrade() {
            Some(t) => format!("{}::{}", t.get().full_name(), self.name),
            None => self.name.clone(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Param {
    pub flags: ParamFlags,
    pub sequence: u16,
    pub name: String,
    pub constant: Option<Constant>,
    pub marshal: Option<Vec<u8>>,
    pub custom_attributes: Vec<CustomAttribute>,
}

impl Param {
    pub fn new(sequence: u16, name: impl Into<String>) -> Self {
        Self {
            sequence,
            name: name.into(),
            ..Default::default()
        }
    }
}

impl EntryName for Param {
    fn entry_name(&self) -> String {
        self.name.clone()
    }
}

#[derive(Debug, Clone, Default)]
pub struct Property {
    pub flags: PropertyFlags,
    pub name: String,
    pub signature: PropertySig,
    pub constant: Option<Constant>,
    pub getter: Option<Entry<Method>>,
    pub setter: Option<Entry<Method>>,
    pub other_methods: Vec<Entry<Method>>,
    pub custom_attributes: Vec<CustomAttribute>,
}

impl EntryName for Property {
    fn entry_name(&self) -> String {
        self.name.clone()
    }
}

#[derive(Debug, Clone, Default)]
pub struct Event {
    pub flags: EventFlags,
    pub name: String,
    pub event_type: Option<TypeDefOrRef>,
    pub add: Option<Entry<Method>>,
    pub remove: Option<Entry<Method>>,
    pub fire: Option<Entry<Method>>,
    pub other_methods: Vec<Entry<Method>>,
    pub custom_attributes: Vec<CustomAttribute>,
}

impl EntryName for Event {
    fn entry_name(&self) -> String {
        self.name.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericParamConstraint {
    pub constraint: TypeDefOrRef,
    pub custom_attributes: Vec<CustomAttribute>,
}

#[derive(Debug, Clone, Default)]
pub struct GenericParam {
    pub number: u16,
    pub flags: GenericParamFlags,
    pub name: String,
    pub constraints: Vec<GenericParamConstraint>,
    pub custom_attributes: Vec<CustomAttribute>,
}

impl EntryName for GenericParam {
    fn entry_name(&self) -> String {
        self.name.clone()
    }
}
