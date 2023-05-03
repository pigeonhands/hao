use super::{md::streams::{tables_stream::{
    coded_tokens::{CodedToken, ResolutionScopeToken, TypeDefOrRefToken},
    FieldFlags, FieldTableRow, ModulesTableRow, TypeAttributes, TypeDefTableRow,
    TypeRefTableRow,
}, Signature}, module::{MaybeUninitEntries, GetEntry}};
use crate::{
    error::{HaoError, Result},
    io::{EntryReader, ValueReadable},
};
use std::{cell::RefCell, fmt::Debug, rc::Rc};

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

#[derive(Clone)]
pub struct Ptr<T>(Rc<RefCell<Option<T>>>);
pub(crate) type EntList<T> = Vec<Ptr<T>>;

impl<T: Debug> Debug for Ptr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = self.0.as_ref().borrow();
        match v.as_ref() {
            Some(v) => v.fmt(f),
            None => write!(f, "Ptr::new_unset()"),
        }
    }
}

pub trait ReadEntry<T>: Sized {
    type RawRow;
    fn from_row(&self, index: usize, row: &Self::RawRow) -> Result<T>;
}

impl<T> Ptr<T> {
    pub fn new_unset() -> Self {
        Self(Rc::new(RefCell::new(None)))
    }

    pub fn set_value(&self, val: T) {
        self.0.replace(Some(val));
    }

    pub fn value(&self) -> std::cell::Ref<Option<T>> {
        self.0.borrow()
    }
}

#[derive(Debug, Clone)]
pub struct ModuleDef {
    pub generation: u16,
    pub name: String,
    pub mvid: uuid::Uuid,
    pub enc_id: uuid::Uuid,
    pub enc_base_id: uuid::Uuid,
}

impl<'a> ReadEntry<ModuleDef> for EntryReader<'a> {
    type RawRow = ModulesTableRow;
    fn from_row(&self, _: usize, row: &Self::RawRow) -> Result<ModuleDef> {
        Ok(ModuleDef {
            generation: self.read(row.generation)?,
            name: self.read(row.name)?,
            mvid: self.read(row.mvid)?,
            enc_id: self.read(row.enc_id)?,
            enc_base_id: self.read(row.enc_base_id)?,
        })
    }
}

#[derive(Debug, Clone)]
pub enum ResolutionScope {
    Module(Ptr<ModuleDef>),
    //ModuleRef,
    //AssemblyRef,
    //TypeRef,
    NotImplimented,
    None,
}

impl<'a> GetEntry<CodedToken<ResolutionScopeToken>> for MaybeUninitEntries {
    type EntryValue = ResolutionScope;

    fn get_entry(&self, identifier: CodedToken<ResolutionScopeToken>) -> Result<Self::EntryValue> {
        let index = match (identifier.rid as usize).checked_sub(1) {
            Some(v) => v,
            None => return Ok(ResolutionScope::None),
        };

        let val = match identifier.target {
            ResolutionScopeToken::Module => self
                .modules
                .get(index)
                .cloned()
                .map(ResolutionScope::Module),
            _ => Some(ResolutionScope::NotImplimented),
        };
        val.ok_or_else(|| HaoError::InvalidCodedTokenOffset(identifier.rid, "ResolutionScopeToken"))
    }
}

#[derive(Debug, Clone)]
pub struct TypeRef {
    pub resolution_scope: ResolutionScope,
    pub name: String,
    pub namespace: String,
}

impl<'a> ReadEntry<TypeRef> for EntryReader<'a> {
    type RawRow = TypeRefTableRow;
    fn from_row(&self, _: usize, row: &Self::RawRow) -> Result<TypeRef> {
        Ok(TypeRef {
            resolution_scope: self.read(row.resolution_scope)?,
            name: self.read(row.name)?,
            namespace: self.read(row.namespace)?,
        })
    }
}

#[derive(Debug, Clone)]
pub enum TypeDefOrRef {
    TypeDef(Ptr<TypeDef>),
    TypeRef(Ptr<TypeRef>),
    //TypeSpec(Ptr<()>),
    NotImplimented,
    None,
}

impl GetEntry<CodedToken<TypeDefOrRefToken>> for MaybeUninitEntries {
    type EntryValue = TypeDefOrRef;

    fn get_entry(&self, identifier: CodedToken<TypeDefOrRefToken>) -> Result<Self::EntryValue> {
        let index = match (identifier.rid as usize).checked_sub(1) {
            Some(v) => v,
            None => return Ok(TypeDefOrRef::None),
        };

        let val = match identifier.target {
            TypeDefOrRefToken::TypeDef => self
                .type_defs
                .get(index)
                .cloned()
                .map(TypeDefOrRef::TypeDef),
            TypeDefOrRefToken::TypeRef => self
                .type_refs
                .get(index)
                .cloned()
                .map(TypeDefOrRef::TypeRef),
            _ => Some(TypeDefOrRef::NotImplimented),
        };
        val.ok_or_else(|| HaoError::InvalidCodedTokenOffset(identifier.rid, "TypeDefOrRefToken"))
    }
}

#[derive(Debug, Clone)]
pub struct TypeDef {
    pub flags: TypeAttributes,
    pub name: String,
    pub namespace: String,
    pub extends: TypeDefOrRef,
    pub field_list: Vec<Ptr<Field>>,
    //pub method_list: (),
}

impl<'a> ReadEntry<TypeDef> for EntryReader<'a> {
    type RawRow = TypeDefTableRow;
    fn from_row(&self, index: usize, row: &Self::RawRow) -> Result<TypeDef> {
        let fields_list = RowRange::new(
            row.field_list,
            self.raw_rows()
                .type_def
                .get(index + 1)
                .map(|i| i.field_list),
        );

        Ok(TypeDef {
            flags: row.flags,
            name: self.read(row.name)?,
            namespace: self.read(row.namespace)?,
            extends: self.read(row.extends)?,
            field_list: self.read(fields_list)?,
            //method_list: todo!("method_list")
        })
    }
}

#[derive(Debug, Clone)]
pub struct Field {
    pub flags: FieldFlags,
    pub name: String,
    pub signature: Signature,
}

impl<'a> ReadEntry<Field> for EntryReader<'a> {
    type RawRow = FieldTableRow;
    fn from_row(&self, _: usize, row: &Self::RawRow) -> Result<Field> {
        Ok(Field {
            flags: row.flags,
            name: self.read(row.name)?,
            signature: self.read(row.signature)?,
        })
    }
}
