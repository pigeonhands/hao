use super::{CustomAttribute, DeclSecurity, Entry, EntryName};
use crate::{
    dotnet::md::streams::tables_stream::{
        AssemblyFlags, FileFlags, ManifestResourceFlags, TypeAttributes,
    },
    sync::Shared,
    Module,
};

#[derive(Debug, Clone, Default)]
pub struct ModuleDef {
    pub generation: u16,
    pub name: String,
    pub mvid: uuid::Uuid,
    pub enc_id: uuid::Uuid,
    pub enc_base_id: uuid::Uuid,
    pub custom_attributes: Vec<CustomAttribute>,
}

impl EntryName for ModuleDef {
    fn entry_name(&self) -> String {
        self.name.clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
    pub build: u16,
    pub revision: u16,
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}.{}", self.major, self.minor, self.build, self.revision)
    }
}

/// The assembly manifest of the current module.
#[derive(Debug, Clone, Default)]
pub struct Assembly {
    pub hash_alg_id: u32,
    pub version: Version,
    pub flags: AssemblyFlags,
    pub public_key: Vec<u8>,
    pub name: String,
    pub culture: String,
    pub security: Vec<DeclSecurity>,
    pub custom_attributes: Vec<CustomAttribute>,
}

impl EntryName for Assembly {
    fn entry_name(&self) -> String {
        self.name.clone()
    }
}

#[derive(Debug, Clone, Default)]
pub struct AssemblyRef {
    pub version: Version,
    pub flags: AssemblyFlags,
    pub public_key_or_token: Vec<u8>,
    pub name: String,
    pub culture: String,
    pub hash_value: Vec<u8>,
    pub custom_attributes: Vec<CustomAttribute>,
    /// Set by [`Module::load_dependancies`].
    pub resolved: Option<Shared<Module>>,
}

impl AssemblyRef {
    const KNOWN_CORLIB_NAMES: &'static [&'static str] = &[
        "System.Private.CoreLib",
        "netstandard",
        "mscorlib",
        "corefx",
        "System.Runtime",
    ];

    pub fn is_corlib(&self) -> bool {
        Self::KNOWN_CORLIB_NAMES
            .iter()
            .any(|s| self.name.eq_ignore_ascii_case(s))
    }
}

impl EntryName for AssemblyRef {
    fn entry_name(&self) -> String {
        self.name.clone()
    }
}

#[derive(Debug, Clone, Default)]
pub struct File {
    pub flags: FileFlags,
    pub name: String,
    pub hash_value: Vec<u8>,
    pub custom_attributes: Vec<CustomAttribute>,
}

impl EntryName for File {
    fn entry_name(&self) -> String {
        self.name.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Implementation {
    File(Entry<File>),
    AssemblyRef(Entry<AssemblyRef>),
    ExportedType(Entry<ExportedType>),
}

/// A type exported (or forwarded) by this assembly.
#[derive(Debug, Clone, Default)]
pub struct ExportedType {
    pub flags: TypeAttributes,
    pub type_def_id: u32,
    pub name: String,
    pub namespace: String,
    pub implementation: Option<Implementation>,
    pub custom_attributes: Vec<CustomAttribute>,
}

impl EntryName for ExportedType {
    fn entry_name(&self) -> String {
        format!("{}.{}", self.namespace, self.name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceData {
    Embedded(Vec<u8>),
    Linked {
        implementation: Implementation,
        offset: u32,
    },
}

impl Default for ResourceData {
    fn default() -> Self {
        Self::Embedded(Vec::new())
    }
}

#[derive(Debug, Clone, Default)]
pub struct ManifestResource {
    pub flags: ManifestResourceFlags,
    pub name: String,
    pub data: ResourceData,
    pub custom_attributes: Vec<CustomAttribute>,
}

impl EntryName for ManifestResource {
    fn entry_name(&self) -> String {
        self.name.clone()
    }
}
