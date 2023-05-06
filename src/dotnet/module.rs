use std::fmt::Debug;

use super::entries::{EntryIterator, MaybeUninitEntries};
use crate::dotnet::entries::{values::*, EntList};
use crate::{error::Result, io::EntryReader, Metadata};

#[derive(Debug, Clone)]
pub struct ModuleDefMD {
    pub modules: EntList<ModuleDef>,
    pub type_refs: EntList<TypeRef>,
    pub type_defs: EntList<TypeDef>,
    pub fields: EntList<Field>,
    pub methods: EntList<Method>,
}

impl ModuleDefMD {
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

        Ok(Self {
            modules: entries.modules,
            type_refs: entries.type_refs,
            type_defs: entries.type_defs,
            fields: entries.fields,
            methods: entries.methods,
        })
    }

    pub fn types(&self) -> EntryIterator<TypeDef> {
        EntryIterator::new(&self.type_defs)
    }
}
