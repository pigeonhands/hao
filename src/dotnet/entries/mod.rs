mod containers;
pub mod signature;
pub mod values;
pub mod well_known;
use std::{
    cell::{Ref, RefMut, RefCell},
};

use super::md::streams::tables_stream::metadata::TableLocations;
use crate::{
    dotnet::md::streams::tables_stream::{TableLocation, TablesStreamReader},
    error::Result,
    io::{EntryReader, ReadData},
};
pub use containers::*;
use values::*;

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
    fn from_row(
        &self,
        index: usize,
        row: &Self::RawRow,
        next_row: Option<&Self::RawRow>,
    ) -> Result<T>;
}

pub trait GetEntryField<T> {
    type EntryFieldValue: Clone;
    fn get_entry_field(&self, identifier: T) -> Result<Self::EntryFieldValue>;
}

#[derive(Debug)]
pub struct EntryView<'a, T>(pub (crate) &'a Ptr<T>);

impl<'a, T> EntryView<'a, T> {
    pub fn value(&self) -> Ref<RowEntry<T>> {
        self.0.value()
    }
    pub fn value_mut(&self) -> RefMut<T> {
        self.0.value_mut()
    }

    pub fn try_value(&self) -> Option<Ref<RowEntry<T>>> {
        self.0.try_value()
    }

    pub fn try_value_mut(&self) -> Option<RefMut<T>> {
        self.0.try_value_mut()
    }

    pub fn map<E, F: FnOnce(Ref<RowEntry<T>>) -> E>(&self, func: F) -> E {
        func(self.value())
    }
}

#[derive(Debug, Clone)]
pub struct Entry<T>(pub (crate) Ptr<T>);

impl<'a, T> Entry<T> {
    pub fn value(&self) -> Ref<RowEntry<T>> {
        self.0.value()
    }
    pub fn value_mut(&self) -> RefMut<T> {
        self.0.value_mut()
    }

    pub fn try_value(&self) -> Option<Ref<RowEntry<T>>> {
        self.0.try_value()
    }

    pub fn try_value_mut(&self) -> Option<RefMut<T>> {
        self.0.try_value_mut()
    }

    pub fn map<E, F: FnOnce(Ref<RowEntry<T>>) -> E>(&self, func: F) -> E {
        func(self.value())
    }
}

pub struct EntryCollection<'a, T> {
    rows: &'a [Ptr<T>],
    position: usize,
}

impl<'a, T> EntryCollection<'a, T> {
    pub(crate) fn new(rows: &'a [Ptr<T>]) -> Self {
        Self { rows, position: 0 }
    }

    pub fn values(&self) -> EntryIteratorValue<'a, T> {
        EntryIteratorValue::new(&self.rows[self.position..])
    }

    pub fn values_mut(&self) -> EntryIteratorValueMut<'a, T> {
        EntryIteratorValueMut::new(&self.rows[self.position..])
    }

    pub fn get_rid(&self, rid: usize) -> Option<Ref<'a, RowEntry<T>>> {
        rid.checked_sub(1)
            .map(|rid| self.rows.get(rid))
            .flatten()
            .map(|v| v.value())
    }

    pub fn get_rid_mut(&self, rid: usize) -> Option<RefMut<'a, T>> {
        rid.checked_sub(1)
            .map(|rid| self.rows.get(rid))
            .flatten()
            .map(|v| v.value_mut())
    }
}

impl<'a, T> Iterator for EntryCollection<'a, T> {
    type Item = EntryView<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.rows.get(self.position);
        value.map(|v| {
            self.position += 1;
            EntryView(v)
        })
    }
}

pub struct EntryIteratorValue<'a, T> {
    rows: &'a [Ptr<T>],
    position: usize,
}

impl<'a, T> EntryIteratorValue<'a, T> {
    pub(crate) fn new(rows: &'a [Ptr<T>]) -> Self {
        Self { rows, position: 0 }
    }
}

impl<'a, T> Iterator for EntryIteratorValue<'a, T> {
    type Item = Ref<'a, RowEntry<T>>;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.rows.get(self.position);
        value.map(|v| {
            self.position += 1;
            v.value()
        })
    }
}

pub struct EntryIteratorValueMut<'a, T> {
    rows: &'a [Ptr<T>],
    position: usize,
}

impl<'a, T> EntryIteratorValueMut<'a, T> {
    pub(crate) fn new(rows: &'a [Ptr<T>]) -> Self {
        Self { rows, position: 0 }
    }
}

impl<'a, T> Iterator for EntryIteratorValueMut<'a, T> {
    type Item = RefMut<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.rows.get(self.position);
        value.map(|v| {
            self.position += 1;
            v.value_mut()
        })
    }
}

pub(crate) struct MaybeUninitEntries {
    pub modules: EntList<ModuleDef>,
    pub type_refs: EntList<TypeRef>,
    pub type_defs: EntList<TypeDef>,
    pub fields: EntList<Field>,
    pub methods: EntList<Method>,
    pub params: EntList<Param>,
    pub interface_impls: Vec<RefCell<MaybeUnsetEntry<InterfaceImpl>>>,


    pub(crate) type_specs: EntList<TypeSpec>,
}

impl MaybeUninitEntries {
    pub fn from_rows(locations: &TableLocations) -> Self {
        fn init_ent_list<T>(size: TableLocation) -> EntList<T> {
            let mut v = EntList::with_capacity(size.rows.0 as usize);
            for _ in 0..v.capacity() {
                v.push(Ptr::new_unset());
            }
            v
        }
        fn init_metadata_list<T>(size: TableLocation) -> Vec<RefCell<MaybeUnsetEntry<T>>> {
            let mut v = Vec::with_capacity(size.rows.0 as usize);
            for _ in 0..v.capacity() {
                v.push(RefCell::new(MaybeUnsetEntry::new_unset()));
            }
            v
        }
        Self {
            modules: init_ent_list(locations.module),
            type_refs: init_ent_list(locations.type_ref),
            type_defs: init_ent_list(locations.type_def),
            fields: init_ent_list(locations.field),
            methods: init_ent_list(locations.method),
            params: init_ent_list(locations.param),
            interface_impls: init_metadata_list(locations.interface_impl),



            type_specs: init_ent_list(locations.type_spec),
        }
    }

    pub fn init_rows(&self, locations: &TableLocations, reader: &EntryReader) -> Result<()> {

        fn write_ent<V>(entry: &Ptr<V>, value: V, index: usize) {
            let row = (index + 1) as u32; // // 0 = none. Row id's start at 1.
            entry.set_value(row, value);
        }
        fn write_metadata<V>(entry: &RefCell<MaybeUnsetEntry<V>>, value: V, index: usize) {
            let row = (index + 1) as u32; // // 0 = none. Row id's start at 1.        
            entry.borrow_mut().set_value(row, value);
        }
        
        fn init_ent_list<'a, T, V>(
            uninit_rows: &Vec<T>,
            location: &TableLocation,
            reader: &EntryReader<'a>,
            write_value: fn(entry: &T, V, usize)
        ) -> Result<()>
        where
            EntryReader<'a>: ReadEntry<V>,
            TablesStreamReader<'a>: ReadData<<EntryReader<'a> as ReadEntry<V>>::RawRow>,
        {
            let mut row_iter = reader
                .streams
                .tables_stream
                .row_iter(*location)?
                .enumerate()
                .peekable();

            while let Some((index, row)) = row_iter.next() {
                let row: <EntryReader as ReadEntry<V>>::RawRow = row?;

                let next = match row_iter.peek() {
                    Some((_, Ok(v))) => Some(v),
                    _ => None,
                };
                
                let entry = &uninit_rows[index];
                let val = reader.from_row(index, &row, next)?;
                write_value(entry, val, index);
            }

            Ok(())
        }

        init_ent_list(&self.modules, &locations.module, reader, write_ent)?;
        init_ent_list(&self.type_refs, &locations.type_ref, reader, write_ent)?;
        init_ent_list(&self.type_defs, &locations.type_def, reader, write_ent)?;
        init_ent_list(&self.fields, &locations.field, reader, write_ent)?;
        init_ent_list(&self.methods, &locations.method, reader, write_ent)?;
        init_ent_list(&self.params, &locations.param, reader, write_ent)?;
        init_ent_list(&self.interface_impls, &locations.interface_impl, reader, write_metadata)?;


        init_ent_list(&self.type_specs, &locations.type_spec, reader, write_ent)?;

        Ok(())
    }
}
