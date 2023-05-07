use std::fmt::Debug;

use crate::dotnet::entries::{EntryCollection, EntryView, MaybeUninitEntries};
use crate::dotnet::{
    entries::{values::*, EntList},
    metadata::Metadata,
};

use crate::error::HaoError;
use crate::{error::Result, io::EntryReader};

use super::resolver::PathAssemblyResolver;
use super::resolver::{AssemblyResolver, AssemblyLoadResult};

/// Represents a loaded .net module.
/// ```no_run
/// # use hao::Module;
/// let module = Module::from_file(r#"Example.Net.dll"#);
///
/// for ty in module.types().values() {
///    for method in ty.methods().values() {
///        println!("{}", method.name());
///    }
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct Module {
    // All below are from metatada tables
    pub(crate) modules: EntList<ModuleDef>,
    pub(crate) type_refs: EntList<TypeRef>,
    pub(crate) type_defs: EntList<TypeDef>,
    pub(crate) fields: EntList<Field>,
    pub(crate) methods: EntList<Method>,
    pub(crate) params: EntList<Param>,

    pub(crate) module_ref: EntList<ModuleRef>,
    pub(crate) type_specs: EntList<TypeSpec>,

    pub(crate) assembly_ref: EntList<AssemblyRef>,
}

impl Module {
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let data = std::fs::read(path.as_ref()).map_err(HaoError::IoError)?;
        let md = Metadata::parse(&data)?;
        let asm = Self::from_metadata(&md)?;

        dbg!(&md.metadata_streams.tables_stream.header.table_locations.field);
        let mut resolver: PathAssemblyResolver = PathAssemblyResolver::new(path.as_ref());
        asm.load_dependancies(&mut resolver)?;
        Ok(asm)
    }

    pub fn from_path_no_resolve(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let data = std::fs::read(path.as_ref()).map_err(HaoError::IoError)?;
        let md = Metadata::parse(&data)?;
        //dbg!(&md.metadata_streams.tables_stream.header.table_locations.field);

        let asm = Self::from_metadata(&md)?;
        Ok(asm)
    }

    pub fn from_metadata(metadada: &Metadata) -> Result<Self> {
        let entries = {
            let locations = &metadada
                .metadata_streams
                .tables_stream
                .header
                .table_locations;

            let maybe_entries = MaybeUninitEntries::from_rows(locations);
            let reader = EntryReader::from_metadata(&metadada.metadata_streams, &maybe_entries);
            maybe_entries.init_rows(locations, &reader)?;
            maybe_entries
        };

        for if_impl in entries.interface_impls {
            let inner = if_impl.into_inner();
            let InterfaceImpl { class, interface } = inner.assume_init_value();
            class.value_mut().interface_impl.push(interface);
        }

        Ok(Self {
            modules: entries.modules,
            type_refs: entries.type_refs,
            type_defs: entries.type_defs,
            fields: entries.fields,
            methods: entries.methods,
            params: entries.params,

            module_ref: entries.module_ref,
            type_specs: entries.type_specs,

            assembly_ref: entries.assembly_ref,
        })
    }

    /// Attempts to load the refrenced assemblies using the given resolver
    /// this will panic if there is a refrence holding any of the 
    /// [`AssemblyRef`] in this module.
    pub fn load_dependancies(&self, resolver: &mut impl AssemblyResolver) -> Result<()> {
        for asm in self.assembly_ref.iter() {
            let mut asm = asm.value_mut();
            if asm.refrenced_assembly.is_some() {
                continue;
            }
            asm.refrenced_assembly = match resolver.load(&asm.name)? {
                AssemblyLoadResult::Ignore => None,
                AssemblyLoadResult::Loaded(asm) => Some(asm)
            };
        }
        Ok(())
    }

    /// Returns the module infomation of the current module as a [`EntryView`].
    pub fn module(&self) -> EntryView<ModuleDef> {
        EntryView(&self.modules[0])
    }

    /// Returns an [`EntryCollection`] of [`TypeRef`] with  all the type refrences
    /// inside the current module.
    pub fn type_refs(&self) -> EntryCollection<TypeRef> {
        EntryCollection::new(&self.type_refs)
    }

    /// Returns an [`EntryCollection`] of [`TypeDef`] with all the type refrences
    /// inside the current module.
    pub fn types(&self) -> EntryCollection<TypeDef> {
        EntryCollection::new(&self.type_defs)
    }

    /// Returns an [`EntryCollection`] of [`Field`] with all the fields
    /// defined in the module regardless of the parent type.
    ///
    /// If you want the associated type, use [`TypeDef::fields()`].
    /// ```
    /// # use hao::Module;
    /// let module = Module::default();
    /// for ty in module.types().values() {
    ///    for field in ty.fields().values() {
    ///        println!("{} {}", field.signature(), field.name());
    ///    }
    /// }
    /// ```
    pub fn all_fields(&self) -> EntryCollection<Field> {
        EntryCollection::new(&self.fields)
    }

    /// Returns an [`EntryCollection`] of [`Method`] with all the methods
    /// defined in the module regardless of the parent type.
    ///
    /// If you want the associated type, use [`TypeDef::methods()`].
    /// ```
    /// # use hao::Module;
    /// let module = Module::default();
    /// for ty in module.types().values() {
    ///    for method in ty.methods().values() {
    ///        println!("{}", method.name());
    ///    }
    /// }
    /// ```
    pub fn all_methods(&self) -> EntryCollection<Method> {
        EntryCollection::new(&self.methods)
    }

    /// Returns an [`EntryCollection`] of [`Param`] with all the parameters
    /// defined in the module regardless of the parent method.
    ///
    /// If you want the associated method, use [`Method::params()`].
    /// ```
    /// # use hao::Module;
    /// let module = Module::default();
    /// for ty in module.types().values() {
    ///    for method in ty.methods().values() {
    ///        for param in method.params().values() {
    ///             println!("{}", param.name());
    ///        }
    ///    }
    /// }
    /// ```
    pub fn all_params(&self) -> EntryCollection<Param> {
        EntryCollection::new(&self.params)
    }

    /// Returns an [`EntryCollection`] of [`TypeSpec`] with all the type specs
    /// defined in the module.
    pub fn all_type_specs(&self) -> EntryCollection<TypeSpec> {
        EntryCollection::new(&self.type_specs)
    }

    /// Returns an [`EntryCollection`] of [`ModuleRef`] with all the modules
    /// refrenced in the module.
    pub fn module_ref(&self) -> EntryCollection<ModuleRef> {
        EntryCollection::new(&self.module_ref)
    }

    /// Returns an [`EntryCollection`] of [`AssemblyRef`] with all the assemblies
    /// refrenced in the module.
    pub fn assembly_ref(&self) -> EntryCollection<AssemblyRef> {
        EntryCollection::new(&self.assembly_ref)
    }
}
