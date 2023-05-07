use std::fmt::Debug;

use super::entries::{EntryCollection, MaybeUninitEntries, EntryView};
use crate::dotnet::{entries::{values::*, EntList}, metadata::Metadata};
use crate::error::HaoError;
use crate::{error::Result, io::EntryReader};

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
    pub(crate) modules: EntList<ModuleDef>,
    pub(crate) type_refs: EntList<TypeRef>,
    pub(crate) type_defs: EntList<TypeDef>,
    pub(crate) fields: EntList<Field>,
    pub(crate) methods: EntList<Method>,
    pub(crate) params: EntList<Param>,



   pub(crate) type_specs: EntList<TypeSpec>,
}

impl Module {
    pub fn from_path(path: &std::path::Path) -> Result<Self> {
        let data = std::fs::read(path).map_err(HaoError::IoError)?;
        let md = Metadata::parse(&data)?;
        Module::from_metadada(&md)
    }

    pub fn from_metadada(metadada: &Metadata) -> Result<Self> {
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


            type_specs: entries.type_specs,
        })
    }
    
    /// Returns the module infomation of the current module as a [`EntryView`].
    pub fn module(&self) -> EntryView<ModuleDef> {
        EntryView(&self.modules[0])
    }

    /// Returns an [`EntryCollection`] of [`TypeRef`] of the type refrences 
    /// inside the current module.
    pub fn type_refs(&self) -> EntryCollection<TypeRef> {
        EntryCollection::new(&self.type_refs)
    }

    /// Returns an [`EntryCollection`] of [`TypeDef`] of the type refrences 
    /// inside the current module.
    pub fn types(&self) -> EntryCollection<TypeDef> {
        EntryCollection::new(&self.type_defs)
    }

    /// Returns an [`EntryCollection`] of [`Field`] of all the fields
    /// defined in the module regardless of their parent type.
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

    /// Returns an [`EntryCollection`] of [`Method`] of all the methods
    /// defined in the module regardless of their parent type.
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

    /// Returns an [`EntryCollection`] of [`Param`] of all the parameters
    /// defined in the module regardless of their parent method.
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
}
