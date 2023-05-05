use super::{
    md::streams::{
        tables_stream::{
            coded_tokens::{CodedToken, ResolutionScopeToken, TypeDefOrRefToken},
            FieldFlags, FieldTableRow, MethodFlags, MethodImplFlags, MethodTableRow,
            ModulesTableRow, TypeAttributes, TypeDefTableRow, TypeRefTableRow, ParamTableOffset,
        },
        Signature,
    },
    module::{GetEntry, MaybeUninitEntries},
};
use crate::{
    error::{HaoError, Result},
    io::{EntryReader, ValueReadable},
};
use std::{cell::{RefCell, Ref}, fmt::Debug, rc::Rc, ops::{Deref, DerefMut}, mem::MaybeUninit};

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

#[derive(Debug)]
pub (crate) struct MaybeUnsetEntry<T:Sized> {
    value: MaybeUninit<T>,
    is_set: bool
}

impl<T:Sized> MaybeUnsetEntry<T> {
    pub fn new_unset() -> Self {
        Self {
            value: MaybeUninit::uninit(),
            is_set: false
        }
    }

    pub fn is_set(&self) -> bool {
        self.is_set
    }

    pub fn set_value(&mut self, value: T) {
        if self.is_set() {
            unsafe{ self.value.assume_init_drop(); }
        }
        self.value.write(value);
        self.is_set = true;
    }

}

impl<T:Sized> Drop for MaybeUnsetEntry<T> {
    fn drop(&mut self) {
        if self.is_set() {
            unsafe { self.value.assume_init_drop(); }
        }
    }
}

impl<T> AsRef<T> for MaybeUnsetEntry<T> {
    fn as_ref(&self) -> &T {
        assert!(self.is_set());
        unsafe { self.value.assume_init_ref() }
    }
}

impl<T> AsMut<T> for MaybeUnsetEntry<T> {
    fn as_mut(&mut self) -> &mut T {
        assert!(self.is_set());
        unsafe { self.value.assume_init_mut() }
    }
}

impl<T:Sized> Deref for MaybeUnsetEntry<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T:Sized> DerefMut for MaybeUnsetEntry<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}


#[derive(Clone)]
pub struct Ptr<T>(Rc<RefCell<MaybeUnsetEntry<T>>>);
pub(crate) type EntList<T> = Vec<Ptr<T>>;

impl<T> Ptr<T> {
    pub fn new_unset() -> Self {
        Self(Rc::new(RefCell::new(MaybeUnsetEntry::new_unset())))
    }

    pub fn is_set(&self) -> bool {
        self.0.borrow().is_set()
    }

    pub fn set_value(&self, value: T) {
        let mut val_ref = self.0.borrow_mut();
        val_ref.set_value(value);
    }

    pub fn value(&self) -> Ref<T> {
        let r = self.0.borrow();
        Ref::map(r, |x| x.as_ref())
    }

    pub fn clone_value(&self) -> T 
    where T: Clone {
        self.0.borrow().clone()
    }
}


impl<T: Debug> Debug for Ptr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = self.0.as_ref().borrow();
        if v.is_set() {
            v.as_ref().fmt(f)
        }else{
            write!(f, "Ptr::new_unset()")
        }
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
    fn from_row(&self, _: usize, row: &Self::RawRow, _next: Option<&Self::RawRow>) -> Result<ModuleDef> {
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
    fn from_row(&self, _: usize, row: &Self::RawRow, _next: Option<&Self::RawRow>) -> Result<TypeRef> {
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
    pub method_list: Vec<Ptr<Method>>,
}

impl<'a> ReadEntry<TypeDef> for EntryReader<'a> {
    type RawRow = TypeDefTableRow;
    fn from_row(&self, _: usize, row: &Self::RawRow, _next: Option<&Self::RawRow>) -> Result<TypeDef> {
        Ok(TypeDef {
            flags: row.flags,
            name: self.read(row.name)?,
            namespace: self.read(row.namespace)?,
            extends: self.read(row.extends)?,
            field_list: self.read(RowRange::new(row.field_list, _next.map(|x| x.field_list)))?,
            method_list: self.read(RowRange::new(row.method_list, _next.map(|x| x.method_list)))?,
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
    fn from_row(&self, _: usize, row: &Self::RawRow, _next: Option<&Self::RawRow>) -> Result<Field> {
        Ok(Field {
            flags: row.flags,
            name: self.read(row.name)?,
            signature: self.read(row.signature)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Method {
    pub rva: u32,
    pub impl_flags: MethodImplFlags,
    pub flags: MethodFlags,
    pub name: String,
    pub signature: Signature,
    pub param_list: ParamTableOffset,
}

impl<'a> ReadEntry<Method> for EntryReader<'a> {
    type RawRow = MethodTableRow;
    fn from_row(&self, _: usize, row: &Self::RawRow, _next: Option<&Self::RawRow>) -> Result<Method> {
        Ok(Method {
            rva: row.rva,
            impl_flags: row.impl_flags,
            flags: row.flags,
            name: self.read(row.name)?,
            signature: self.read(row.signature)?,
            param_list: row.param_list
        })
    }
}
