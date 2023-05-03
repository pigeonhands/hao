use std::fmt::Debug;

use super::entries::{EntList, ModuleDef, Ptr, TypeDef, TypeRef, Field};
use super::md::streams::tables_stream::{TableRows, TablesValues};
use crate::dotnet::entries::ReadEntry;
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
}

impl MaybeUninitEntries {
    pub fn from_rows(rows: &TableRows) -> Self {
        fn init_vec<T>(size: u32) -> EntList<T> {
            let mut v = EntList::with_capacity(size as usize);
            for _ in 0..v.capacity() {
                v.push(Ptr::new_unset());
            }
            v
        }

        Self {
            modules: init_vec(rows.module),
            type_refs: init_vec(rows.type_ref),
            type_defs: init_vec(rows.type_def),
            fields: init_vec(rows.field)
        }
    }

    pub fn init_rows(&self, raw_rows: &TablesValues, reader: &EntryReader) -> Result<()> {
        fn init_ent_list<'a, E>(
            uninit_rows: &EntList<E>,
            raw_rows: &[<EntryReader<'a> as ReadEntry<E>>::RawRow],
            reader: &EntryReader<'a>,
        ) -> Result<()>
        where
            EntryReader<'a>: ReadEntry<E>,
        {
            for (index, (entry, row)) in uninit_rows.iter().zip(raw_rows.into_iter()).enumerate() {
                let val = reader.from_row(index, row)?;
                entry.set_value(val);
            }

            Ok(())
        }

        init_ent_list(&self.modules, &raw_rows.module, reader)?;
        init_ent_list(&self.type_refs, &raw_rows.type_ref, reader)?;
        init_ent_list(&self.type_defs, &raw_rows.type_def, reader)?;
        init_ent_list(&self.fields, &raw_rows.field, reader)?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ModuleDefMD {
    pub modules: EntList<ModuleDef>,
    pub type_refs: EntList<TypeRef>,
    pub type_defs: EntList<TypeDef>,
    pub fields: EntList<Field>,
}

impl ModuleDefMD {
    pub fn from_metadada(metadada: &Metadata) -> Result<Self> {
        let entries = {
            let maybe_entries = MaybeUninitEntries::from_rows(
                &metadada.metadata_streams.tables_stream.header.table_rows,
            );
            let raw_rows = &metadada.metadata_streams.tables_stream.values;
            let reader = EntryReader::from_metadata(&metadada.metadata_streams, &maybe_entries);

            maybe_entries.init_rows(raw_rows, &reader)?;
            maybe_entries
        };

        let MaybeUninitEntries {
            modules,
            type_refs,
            type_defs,
            fields
        } = entries;

        Ok(Self {
            modules,
            type_refs,
            type_defs,
            fields
        })
    }
}
