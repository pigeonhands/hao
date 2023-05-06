use super::{
    super::md::streams::{
        tables_stream::{
            coded_tokens::{CodedToken, ResolutionScopeToken, TypeDefOrRefToken},
            FieldFlags, FieldTableRow, MethodFlags, MethodImplFlags, MethodTableRow,
            ModulesTableRow, ParamTableOffset, TypeAttributes, TypeDefTableRow, TypeRefTableRow,
        },
        Signature,
    },
    {Ptr, ReadEntry, RowRange},
};
use crate::{
    dotnet::entries::{GetEntryField, MaybeUninitEntries},
    error::{HaoError, Result},
    io::{EntryReader, ValueReadable},
};
use std::fmt::Debug;

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
    fn from_row(
        &self,
        _: usize,
        row: &Self::RawRow,
        _next: Option<&Self::RawRow>,
    ) -> Result<ModuleDef> {
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

impl<'a> GetEntryField<CodedToken<ResolutionScopeToken>> for MaybeUninitEntries {
    type EntryFieldValue = ResolutionScope;

    fn get_entry_field(
        &self,
        identifier: CodedToken<ResolutionScopeToken>,
    ) -> Result<Self::EntryFieldValue> {
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
    fn from_row(
        &self,
        _: usize,
        row: &Self::RawRow,
        _next: Option<&Self::RawRow>,
    ) -> Result<TypeRef> {
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

impl GetEntryField<CodedToken<TypeDefOrRefToken>> for MaybeUninitEntries {
    type EntryFieldValue = TypeDefOrRef;

    fn get_entry_field(
        &self,
        identifier: CodedToken<TypeDefOrRefToken>,
    ) -> Result<Self::EntryFieldValue> {
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
    fn from_row(
        &self,
        _: usize,
        row: &Self::RawRow,
        _next: Option<&Self::RawRow>,
    ) -> Result<TypeDef> {
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
    fn from_row(
        &self,
        _: usize,
        row: &Self::RawRow,
        _next: Option<&Self::RawRow>,
    ) -> Result<Field> {
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
    fn from_row(
        &self,
        _: usize,
        row: &Self::RawRow,
        _next: Option<&Self::RawRow>,
    ) -> Result<Method> {
        Ok(Method {
            rva: row.rva,
            impl_flags: row.impl_flags,
            flags: row.flags,
            name: self.read(row.name)?,
            signature: self.read(row.signature)?,
            param_list: row.param_list,
        })
    }
}
