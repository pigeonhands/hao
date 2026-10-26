use pewter::pe::sections::cor20::ImageCor20Header;

use super::{
    loader::{Loader, Tables},
    resolver::{AssemblyLoadResult, AssemblyResolver, PathAssemblyResolver},
};
use crate::{
    dotnet::{entries::*, metadata::Metadata},
    error::{HaoError, Result},
};

/// A loaded .net module.
///
/// ```no_run
/// # use hao::Module;
/// let module = Module::from_path("Example.Net.dll").unwrap();
///
/// for ty in module.all_types() {
///     for method in &ty.get().methods {
///         println!("{}", method.get().name);
///     }
/// }
/// ```
///
/// Entries reference each other freely, so dropping a `Module` clears every
/// entry it owns to break those cycles. Entries should not outlive their module.
#[derive(Debug, Default)]
pub struct Module {
    pub module: Entry<ModuleDef>,
    pub assembly: Option<Entry<Assembly>>,
    /// Top level types. The first one is the global `<Module>` type.
    pub types: Vec<Entry<TypeDef>>,
    pub type_refs: Vec<Entry<TypeRef>>,
    pub type_specs: Vec<Entry<TypeSpec>>,
    pub member_refs: Vec<Entry<MemberRef>>,
    pub method_specs: Vec<Entry<MethodSpec>>,
    pub module_refs: Vec<Entry<ModuleRef>>,
    pub assembly_refs: Vec<Entry<AssemblyRef>>,
    pub files: Vec<Entry<File>>,
    pub exported_types: Vec<Entry<ExportedType>>,
    pub resources: Vec<Entry<ManifestResource>>,
    pub entry_point: Option<Entry<Method>>,
    /// Metadata version string, e.g. `v4.0.30319`.
    pub runtime_version: String,
    pub(crate) image: Vec<u8>,
    pub(crate) cor20: ImageCor20Header,
    /// Used when writing a module that wasn't loaded from an image.
    pub(crate) kind: super::ModuleKind,
}

impl Module {
    /// Loads a module and resolves its dependencies with a [`PathAssemblyResolver`]
    /// rooted at the module's directory.
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let path = path.as_ref();
        let module = Self::from_path_no_resolve(path)?;
        let dir = path.parent().unwrap_or_else(|| std::path::Path::new("."));
        module.load_dependancies(&mut PathAssemblyResolver::new(dir))?;
        Ok(module)
    }

    /// Loads a module without resolving its dependencies.
    pub fn from_path_no_resolve(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let data = std::fs::read(path.as_ref()).map_err(HaoError::IoError)?;
        Self::from_bytes(&data)
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        Self::from_metadata(&Metadata::parse(data)?)
    }

    /// Loads a module from parsed metadata. Dependencies are not resolved.
    pub fn from_metadata(metadata: &Metadata) -> Result<Self> {
        let Tables {
            module,
            type_refs,
            assembly,
            top_level_types,
            type_specs,
            member_refs,
            method_specs,
            module_refs,
            assembly_refs,
            files,
            exported_types,
            resources,
            entry_point,
            ..
        } = Loader::new(metadata).load()?;

        Ok(Self {
            module: module
                .into_iter()
                .next()
                .ok_or(HaoError::BadImageFormat("No module row"))?,
            assembly: assembly.into_iter().next(),
            types: top_level_types,
            type_refs,
            type_specs,
            member_refs,
            method_specs,
            module_refs,
            assembly_refs,
            files,
            exported_types,
            resources,
            entry_point,
            runtime_version: metadata.version.clone(),
            image: metadata.data.to_vec(),
            cor20: metadata.cor20_header.clone(),
            kind: super::ModuleKind::Dll,
        })
    }

    /// Loads referenced assemblies with `resolver`.
    pub fn load_dependancies(&self, resolver: &mut impl AssemblyResolver) -> Result<()> {
        for asm in &self.assembly_refs {
            if asm.get().resolved.is_some() {
                continue;
            }
            let name = asm.get().name.clone();
            asm.get_mut().resolved = match resolver.load(&name)? {
                AssemblyLoadResult::Ignore => None,
                AssemblyLoadResult::Loaded(module) => Some(module),
            };
        }
        Ok(())
    }

    pub fn name(&self) -> String {
        self.module.get().name.clone()
    }

    /// All types including nested ones, depth first.
    pub fn all_types(&self) -> Vec<Entry<TypeDef>> {
        self.types.iter().flat_map(TypeDef::with_nested).collect()
    }

    pub fn all_fields(&self) -> Vec<Entry<Field>> {
        self.all_types()
            .iter()
            .flat_map(|t| t.get().fields.clone())
            .collect()
    }

    pub fn all_methods(&self) -> Vec<Entry<Method>> {
        self.all_types()
            .iter()
            .flat_map(|t| t.get().methods.clone())
            .collect()
    }

    /// Finds a top level type by namespace and name.
    pub fn find_type(&self, namespace: &str, name: &str) -> Option<Entry<TypeDef>> {
        self.types
            .iter()
            .find(|t| t.get().full_name_is(namespace, name))
            .cloned()
    }

    /// Like [`Module::find_type`], but also follows type forwarders to other loaded assemblies.
    pub fn resolve_type(&self, namespace: &str, name: &str) -> Option<Entry<TypeDef>> {
        self.resolve_type_depth(namespace, name, 0)
    }

    fn resolve_type_depth(&self, namespace: &str, name: &str, depth: usize) -> Option<Entry<TypeDef>> {
        if let Some(ty) = self.find_type(namespace, name) {
            return Some(ty);
        }
        if depth > 16 {
            return None;
        }
        let exported = self
            .exported_types
            .iter()
            .find(|e| e.get().namespace == namespace && e.get().name == name)?;
        match &exported.get().implementation {
            Some(Implementation::AssemblyRef(asm)) => {
                let module = asm.get().resolved.clone()?;
                module.resolve_type_depth(namespace, name, depth + 1)
            }
            _ => None,
        }
    }

    /// Finds a type by its full name, using `/` for nested types (`Ns.Outer/Inner`).
    pub fn find_type_by_full_name(&self, full_name: &str) -> Option<Entry<TypeDef>> {
        let mut parts = full_name.split('/');
        let top = parts.next()?;
        let (namespace, name) = top.rsplit_once('.').unwrap_or(("", top));
        let mut ty = self.find_type(namespace, name)?;
        for nested in parts {
            let next = ty.get().nested_types.iter().find(|t| t.get().name == nested).cloned()?;
            ty = next;
        }
        Some(ty)
    }

    /// The `<Module>` type holding global fields and methods.
    pub fn global_type(&self) -> Option<Entry<TypeDef>> {
        self.types.first().cloned()
    }

    /// The bytes of the image this module was loaded from.
    pub fn original_image(&self) -> &[u8] {
        &self.image
    }
}

impl Drop for Module {
    fn drop(&mut self) {
        for ty in self.all_types() {
            let (fields, methods, properties, events, generic_params) = {
                let t = ty.get();
                (
                    t.fields.clone(),
                    t.methods.clone(),
                    t.properties.clone(),
                    t.events.clone(),
                    t.generic_params.clone(),
                )
            };
            for m in &methods {
                for p in m.get().params.clone() {
                    p.clear();
                }
                for g in m.get().generic_params.clone() {
                    g.clear();
                }
                m.clear();
            }
            fields.iter().for_each(Entry::clear);
            properties.iter().for_each(Entry::clear);
            events.iter().for_each(Entry::clear);
            generic_params.iter().for_each(Entry::clear);
        }
        for ty in self.all_types() {
            ty.clear();
        }
        self.type_refs.iter().for_each(Entry::clear);
        self.type_specs.iter().for_each(Entry::clear);
        self.member_refs.iter().for_each(Entry::clear);
        self.method_specs.iter().for_each(Entry::clear);
        self.module_refs.iter().for_each(Entry::clear);
        self.assembly_refs.iter().for_each(Entry::clear);
        self.files.iter().for_each(Entry::clear);
        self.exported_types.iter().for_each(Entry::clear);
        self.resources.iter().for_each(Entry::clear);
        self.module.clear();
        if let Some(a) = &self.assembly {
            a.clear();
        }
    }
}
