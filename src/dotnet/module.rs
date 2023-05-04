use std::fmt::Debug;

use super::entries::{EntList, Field, Method, ModuleDef, Ptr, TypeDef, TypeRef};
use super::md::streams::tables_stream::metadata::TableLocations;
use crate::dotnet::entries::ReadEntry;
use crate::dotnet::md::streams::tables_stream::{TableLocation, TablesStreamReader};
use crate::io::ReadData;
use crate::{error::Result, io::EntryReader, Metadata};

pub trait GetEntry<T> {
    type EntryValue;
    fn get_entry(&self, identifier: T) -> Result<Self::EntryValue>;
}

pub struct MaybeUninitEntries {
    pub modules: EntList<ModuleDef>,
    pub type_refs: EntList<TypeRef>,
    pub type_defs: EntList<TypeDef>,
    pub fields: EntList<Field>,
    pub methods: EntList<Method>,
}

impl MaybeUninitEntries {
    pub fn from_rows(locations: &TableLocations) -> Self {
        fn init_vec<T>(size: TableLocation) -> EntList<T> {
            let mut v = EntList::with_capacity(size.rows.0 as usize);
            for _ in 0..v.capacity() {
                v.push(Ptr::new_unset());
            }
            v
        }
        Self {
            modules: init_vec(locations.module),
            type_refs: init_vec(locations.type_ref),
            type_defs: init_vec(locations.type_def),
            fields: init_vec(locations.field),
            methods: init_vec(locations.method),
        }
    }

    pub fn init_rows(&self, locations: &TableLocations, reader: &EntryReader) -> Result<()> {
        fn init_ent_list<'a, E>(
            uninit_rows: &EntList<E>,
            location: &TableLocation,
            reader: &EntryReader<'a>,
        ) -> Result<()>
        where
            EntryReader<'a>: ReadEntry<E>,
            TablesStreamReader<'a>: ReadData<<EntryReader<'a> as ReadEntry<E>>::RawRow>,
        {
            let mut row_iter = reader
                .streams
                .tables_stream
                .row_iter(*location)?
                .enumerate()
                .peekable();

            while let Some((index, row)) = row_iter.next() {
                let row = row?;
                let entry = &uninit_rows[index];

                let next = match row_iter.peek() {
                    Some((_, Ok(v))) => Some(v),
                    _ => None,
                };

                let val = reader.from_row(index, &row, next)?;
                entry.set_value(val);
            }

            Ok(())
        }

        init_ent_list(&self.modules, &locations.module, reader)?;
        init_ent_list(&self.type_refs, &locations.type_ref, reader)?;
        init_ent_list(&self.type_defs, &locations.type_def, reader)?;
        init_ent_list(&self.fields, &locations.field, reader)?;
        init_ent_list(&self.methods, &locations.method, reader)?;

        Ok(())
    }
}

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
}
