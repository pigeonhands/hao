pub mod values;
mod containers;
use std::cell::Ref;

pub use containers::*;
use crate::{
    error::{Result}, dotnet::md::streams::tables_stream::{TablesStreamReader, TableLocation}, io::{EntryReader, ReadData},
};
use values::*;
use super::md::streams::tables_stream::metadata::TableLocations;

#[derive(Copy, Clone, Debug)]
pub struct RowRange<T> {
    pub start: T,
    pub end: Option<T>,
}

impl<T> RowRange<T> {
    pub fn new(start: T, end: Option<T>) -> Self {
        Self { start, end }
    }

}

pub trait ReadEntry<T>: Sized {
    type RawRow;
    fn from_row(&self, index: usize, row: &Self::RawRow, next_row: Option<&Self::RawRow>) -> Result<T>;
}

pub trait GetEntryField<T> {
    type EntryFieldValue;
    fn get_entry_field(&self, identifier: T) -> Result<Self::EntryFieldValue>;
}

pub struct EntryIterator<'a, T> {
    rows: &'a [Ptr<T>],
    position: usize,
}

impl<'a, T> EntryIterator<'a, T> {
    pub (crate) fn new(rows: &'a [Ptr<T>]) -> Self {
        Self { rows, position: 0 }
    }
}

impl<'a, T> Iterator for EntryIterator<'a, T> {
    type Item = Ref<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.rows.get(self.position);
        value.map(|v| {
            self.position += 1;
            v.value()
        })
    }
}

pub (crate) struct MaybeUninitEntries {
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
            for index in 0..v.capacity() {
                v.push(Ptr::new_unset(index as u32));
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
