use super::{
    well_known::{SystemType, WellKnown},
    AssemblyRef, CustomAttribute, Entry, EntryName, Method, ModuleDef, TypeDef,
};
use crate::{
    dotnet::signature::{MemberRefSig, TypeSig},
    sync::Shared,
    Module,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeDefOrRef {
    TypeDef(Entry<TypeDef>),
    TypeRef(Entry<TypeRef>),
    TypeSpec(Entry<TypeSpec>),
}

impl TypeDefOrRef {
    /// `Namespace.Name` (or the signature for type specs).
    pub fn full_name(&self) -> String {
        match self {
            Self::TypeDef(t) => t.get().full_name(),
            Self::TypeRef(t) => t.get().full_name(),
            Self::TypeSpec(t) => format!("{:?}", t.get().signature),
        }
    }

    pub fn name(&self) -> String {
        match self {
            Self::TypeDef(t) => t.get().name.clone(),
            Self::TypeRef(t) => t.get().name.clone(),
            Self::TypeSpec(_) => self.full_name(),
        }
    }

    pub fn namespace(&self) -> String {
        match self {
            Self::TypeDef(t) => t.get().namespace.clone(),
            Self::TypeRef(t) => t.get().namespace.clone(),
            Self::TypeSpec(_) => String::new(),
        }
    }

    pub fn full_name_is(&self, namespace: &str, name: &str) -> bool {
        match self {
            Self::TypeDef(t) => t.get().full_name_is(namespace, name),
            Self::TypeRef(t) => t.get().full_name_is(namespace, name),
            Self::TypeSpec(_) => false,
        }
    }

    pub fn is_system_type(&self, system_type: SystemType) -> bool {
        self.namespace() == "System" && SystemType::from_type_name(&self.name()) == Some(system_type)
    }

    /// The type definition, looking through resolved assembly references.
    pub fn resolve(&self) -> Option<Entry<TypeDef>> {
        match self {
            Self::TypeDef(t) => Some(t.clone()),
            Self::TypeRef(t) => t.get().resolve(),
            Self::TypeSpec(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionScope {
    Module(Entry<ModuleDef>),
    ModuleRef(Entry<ModuleRef>),
    AssemblyRef(Entry<AssemblyRef>),
    TypeRef(Entry<TypeRef>),
}

#[derive(Debug, Clone, Default)]
pub struct TypeRef {
    /// `None` means the type is found through the `ExportedType` table.
    pub scope: Option<ResolutionScope>,
    pub name: String,
    pub namespace: String,
    pub custom_attributes: Vec<CustomAttribute>,
}

impl TypeRef {
    pub fn new(scope: ResolutionScope, namespace: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            scope: Some(scope),
            name: name.into(),
            namespace: namespace.into(),
            custom_attributes: Vec::new(),
        }
    }

    pub fn full_name(&self) -> String {
        match &self.scope {
            Some(ResolutionScope::TypeRef(parent)) => {
                format!("{}/{}", parent.get().full_name(), self.name)
            }
            _ if self.namespace.is_empty() => self.name.clone(),
            _ => format!("{}.{}", self.namespace, self.name),
        }
    }

    pub fn full_name_is(&self, namespace: &str, name: &str) -> bool {
        self.namespace == namespace && self.name == name
    }

    pub fn is_corlib(&self) -> bool {
        matches!(&self.scope, Some(ResolutionScope::AssemblyRef(r)) if r.get().is_corlib())
    }

    pub fn well_known(&self) -> Option<WellKnown> {
        if self.is_corlib() {
            WellKnown::from_full_name(&self.namespace, &self.name)
        } else {
            None
        }
    }

    /// Finds the matching [`TypeDef`] in the referenced module, if it was loaded.
    pub fn resolve(&self) -> Option<Entry<TypeDef>> {
        let module: Shared<Module> = match &self.scope {
            Some(ResolutionScope::AssemblyRef(asm)) => asm.get().resolved.clone()?,
            Some(ResolutionScope::TypeRef(parent)) => {
                let parent = parent.get().resolve()?;
                let parent = parent.get();
                return parent
                    .nested_types
                    .iter()
                    .find(|t| t.get().name == self.name)
                    .cloned();
            }
            _ => return None,
        };
        module.find_type(&self.namespace, &self.name)
    }
}

impl EntryName for TypeRef {
    fn entry_name(&self) -> String {
        self.full_name()
    }
}

#[derive(Debug, Clone, Default)]
pub struct TypeSpec {
    pub signature: TypeSig,
    pub custom_attributes: Vec<CustomAttribute>,
}

impl EntryName for TypeSpec {
    fn entry_name(&self) -> String {
        format!("{:?}", self.signature)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemberRefParent {
    TypeDef(Entry<TypeDef>),
    TypeRef(Entry<TypeRef>),
    ModuleRef(Entry<ModuleRef>),
    Method(Entry<Method>),
    TypeSpec(Entry<TypeSpec>),
}

impl MemberRefParent {
    pub fn as_type(&self) -> Option<TypeDefOrRef> {
        match self {
            Self::TypeDef(t) => Some(TypeDefOrRef::TypeDef(t.clone())),
            Self::TypeRef(t) => Some(TypeDefOrRef::TypeRef(t.clone())),
            Self::TypeSpec(t) => Some(TypeDefOrRef::TypeSpec(t.clone())),
            _ => None,
        }
    }
}

impl Default for MemberRefParent {
    fn default() -> Self {
        Self::TypeRef(Entry::new(TypeRef::default()))
    }
}

/// A reference to a field or method, usually in another assembly.
#[derive(Debug, Clone, Default)]
pub struct MemberRef {
    pub parent: MemberRefParent,
    pub name: String,
    pub signature: MemberRefSig,
    pub custom_attributes: Vec<CustomAttribute>,
}

impl EntryName for MemberRef {
    fn entry_name(&self) -> String {
        let parent = self
            .parent
            .as_type()
            .map(|t| t.full_name())
            .unwrap_or_default();
        format!("{}::{}", parent, self.name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MethodDefOrRef {
    Method(Entry<Method>),
    MemberRef(Entry<MemberRef>),
}

impl MethodDefOrRef {
    pub fn name(&self) -> String {
        match self {
            Self::Method(m) => m.get().name.clone(),
            Self::MemberRef(m) => m.get().name.clone(),
        }
    }
}

impl Default for MethodDefOrRef {
    fn default() -> Self {
        Self::MemberRef(Entry::new(MemberRef::default()))
    }
}

/// An instantiation of a generic method.
#[derive(Debug, Clone, Default)]
pub struct MethodSpec {
    pub method: MethodDefOrRef,
    pub instantiation: Vec<TypeSig>,
    pub custom_attributes: Vec<CustomAttribute>,
}

impl EntryName for MethodSpec {
    fn entry_name(&self) -> String {
        self.method.name()
    }
}

#[derive(Debug, Clone, Default)]
pub struct ModuleRef {
    pub name: String,
    pub custom_attributes: Vec<CustomAttribute>,
}

impl EntryName for ModuleRef {
    fn entry_name(&self) -> String {
        self.name.clone()
    }
}
