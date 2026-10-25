use std::fmt::Debug;

use strum::FromRepr;

use crate::{
    dotnet::md::streams::tables_stream::metadata::TableRowCount,
    error::{HaoError, Result},
    io::ReadData,
};

use super::{TableRows, TablesStreamReader, ValueSize};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct CodedToken<T: CodedTokenTarget> {
    pub rid: u32,
    pub target: T,
}

impl<T: CodedTokenTarget> CodedToken<T> {
    pub fn new(target: T, rid: u32) -> Self {
        Self { rid, target }
    }

    pub fn encode(&self) -> u32 {
        (self.rid << T::BITS) | self.target.tag()
    }

    pub fn decode(coded_value: u32) -> Result<Self> {
        let offset_mask = 0xFFFFFFFF << T::BITS;
        let target_mask = !offset_mask;

        let table_offset = coded_value >> T::BITS;
        let target = coded_value & target_mask;

        Ok(CodedToken {
            rid: table_offset,
            target: T::from_tag(target).ok_or_else(|| {
                HaoError::InvalidCodedToken(coded_value, std::any::type_name::<T>())
            })?,
        })
    }
}

impl<'a, T: CodedTokenTarget> ReadData<CodedToken<T>> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<CodedToken<T>> {
        let coded_value = match T::token_size(&self.header.coded_token_sizes) {
            ValueSize::Big => self.read()?,
            ValueSize::Small => {
                let small: u16 = self.read()?;
                small as u32
            }
        };

        CodedToken::decode(coded_value)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CodedTokenSizes {
    pub type_def_or_ref: ValueSize,
    pub has_constant: ValueSize,
    pub has_custom_attribute: ValueSize,
    pub has_field_marshal: ValueSize,
    pub has_decl_security: ValueSize,
    pub member_ref_parent: ValueSize,
    pub has_semantic: ValueSize,
    pub method_def_or_ref: ValueSize,
    pub member_forwarded: ValueSize,
    pub implementation: ValueSize,
    pub custom_attribute_type: ValueSize,
    pub resolution_scope: ValueSize,
    pub type_or_method_def: ValueSize,
    pub has_custom_debug_information: ValueSize,
}

impl CodedTokenSizes {
    pub fn from_header(rows: &TableRows) -> Self {
        fn size_from_rows(bits: usize, rows: &[TableRowCount]) -> ValueSize {
            let max_row = rows.iter().map(|i| i.0).max().unwrap_or(0);
            if (max_row << bits) > u16::MAX as u32 {
                ValueSize::Big
            } else {
                ValueSize::Small
            }
        }

        Self {
            type_def_or_ref: size_from_rows(
                TypeDefOrRefToken::BITS,
                &[rows.type_def, rows.type_ref, rows.type_spec],
            ),

            has_constant: size_from_rows(
                HasConstantToken::BITS,
                &[rows.field, rows.param, rows.property],
            ),

            has_custom_attribute: size_from_rows(
                HasCustomAttributeToken::BITS,
                &[
                    rows.method,
                    rows.field,
                    rows.type_ref,
                    rows.type_def,
                    rows.param,
                    rows.interface_impl,
                    rows.member_ref,
                    rows.module,
                    rows.decl_security,
                    rows.property,
                    rows.event,
                    rows.stand_alone_sig,
                    rows.module_ref,
                    rows.type_spec,
                    rows.assembly,
                    rows.assembly_ref,
                    rows.file,
                    rows.exported_type,
                    rows.manifest_resource,
                    rows.generic_param,
                    rows.generic_param_constraint,
                    rows.method_spec,
                    rows.module,
                    rows.module,
                ],
            ),

            has_field_marshal: size_from_rows(
                HasFieldMarshalToken::BITS,
                &[rows.field, rows.param],
            ),

            has_decl_security: size_from_rows(
                HasDeclSecurityToken::BITS,
                &[rows.type_def, rows.method, rows.assembly],
            ),

            member_ref_parent: size_from_rows(
                MemberRefParentToken::BITS,
                &[
                    rows.type_def,
                    rows.type_ref,
                    rows.module_ref,
                    rows.method,
                    rows.type_spec,
                ],
            ),

            has_semantic: size_from_rows(HasSemanticToken::BITS, &[rows.event, rows.property]),

            method_def_or_ref: size_from_rows(
                MethodDefOrRefToken::BITS,
                &[rows.method, rows.member_ref],
            ),

            member_forwarded: size_from_rows(
                MemberForwardedToken::BITS,
                &[rows.field, rows.method],
            ),

            implementation: size_from_rows(
                ImplementationToken::BITS,
                &[rows.file, rows.assembly_ref, rows.exported_type],
            ),

            custom_attribute_type: size_from_rows(
                CustomAttributeTypeToken::BITS,
                &[
                    rows.module,
                    rows.module,
                    rows.method,
                    rows.member_ref,
                    rows.module,
                ],
            ),

            resolution_scope: size_from_rows(
                ResolutionScopeToken::BITS,
                &[
                    rows.module,
                    rows.module_ref,
                    rows.assembly_ref,
                    rows.type_ref,
                ],
            ),

            type_or_method_def: size_from_rows(
                TypeOrMethodDefToken::BITS,
                &[rows.type_def, rows.method],
            ),

            has_custom_debug_information: size_from_rows(
                HasCustomDebugInformationToken::BITS,
                &[
                    rows.method,
                    rows.field,
                    rows.type_ref,
                    rows.type_def,
                    rows.param,
                    rows.interface_impl,
                    rows.member_ref,
                    rows.module,
                    rows.decl_security,
                    rows.property,
                    rows.event,
                    rows.stand_alone_sig,
                    rows.module_ref,
                    rows.type_spec,
                    rows.assembly,
                    rows.assembly_ref,
                    rows.file,
                    rows.exported_type,
                    rows.manifest_resource,
                    rows.generic_param,
                    rows.generic_param_constraint,
                    rows.method_spec,
                    rows.document,
                    rows.local_scope,
                    rows.local_variable,
                    rows.local_constant,
                    rows.import_scope,
                ],
            ),
        }
    }
}

pub trait CodedTokenTarget: Sized + Copy {
    const BITS: usize;
    fn token_size(sizes: &CodedTokenSizes) -> ValueSize;
    fn from_tag(tag: u32) -> Option<Self>;
    fn tag(self) -> u32;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
#[repr(u32)]
pub enum TypeDefOrRefToken {
    TypeDef,
    TypeRef,
    TypeSpec,
}
impl CodedTokenTarget for TypeDefOrRefToken {
    fn from_tag(tag: u32) -> Option<Self> {
        Self::from_repr(tag)
    }
    fn tag(self) -> u32 {
        self as u32
    }
    const BITS: usize = 2;
    fn token_size(sizes: &CodedTokenSizes) -> ValueSize {
        sizes.type_def_or_ref
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
#[repr(u32)]
pub enum HasConstantToken {
    Field,
    Param,
    Property,
}
impl CodedTokenTarget for HasConstantToken {
    fn from_tag(tag: u32) -> Option<Self> {
        Self::from_repr(tag)
    }
    fn tag(self) -> u32 {
        self as u32
    }
    const BITS: usize = 2;
    fn token_size(sizes: &CodedTokenSizes) -> ValueSize {
        sizes.has_constant
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
#[repr(u32)]
pub enum HasCustomAttributeToken {
    Method,
    Field,
    TypeRef,
    TypeDef,
    Param,
    InterfaceImpl,
    MemberRef,
    Module,
    DeclSecurity,
    Property,
    Event,
    StandAloneSig,
    ModuleRef,
    TypeSpec,
    Assembly,
    AssemblyRef,
    File,
    ExportedType,
    ManifestResource,
    GenericParam,
    GenericParamConstraint,
    MethodSpec,
    Unused1,
    Unused2,
}
impl CodedTokenTarget for HasCustomAttributeToken {
    fn from_tag(tag: u32) -> Option<Self> {
        Self::from_repr(tag)
    }
    fn tag(self) -> u32 {
        self as u32
    }
    const BITS: usize = 5;
    fn token_size(sizes: &CodedTokenSizes) -> ValueSize {
        sizes.has_custom_attribute
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
#[repr(u32)]
pub enum HasFieldMarshalToken {
    Field,
    Param,
}
impl CodedTokenTarget for HasFieldMarshalToken {
    fn from_tag(tag: u32) -> Option<Self> {
        Self::from_repr(tag)
    }
    fn tag(self) -> u32 {
        self as u32
    }
    const BITS: usize = 1;
    fn token_size(sizes: &CodedTokenSizes) -> ValueSize {
        sizes.has_field_marshal
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
#[repr(u32)]
pub enum HasDeclSecurityToken {
    TypeDef,
    Method,
    Assembly,
}
impl CodedTokenTarget for HasDeclSecurityToken {
    fn from_tag(tag: u32) -> Option<Self> {
        Self::from_repr(tag)
    }
    fn tag(self) -> u32 {
        self as u32
    }
    const BITS: usize = 2;
    fn token_size(sizes: &CodedTokenSizes) -> ValueSize {
        sizes.has_decl_security
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
#[repr(u32)]
pub enum MemberRefParentToken {
    TypeDef,
    TypeRef,
    ModuleRef,
    Method,
    TypeSpec,
}
impl CodedTokenTarget for MemberRefParentToken {
    fn from_tag(tag: u32) -> Option<Self> {
        Self::from_repr(tag)
    }
    fn tag(self) -> u32 {
        self as u32
    }
    const BITS: usize = 3;
    fn token_size(sizes: &CodedTokenSizes) -> ValueSize {
        sizes.member_ref_parent
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
#[repr(u32)]
pub enum HasSemanticToken {
    Event,
    Property,
}
impl CodedTokenTarget for HasSemanticToken {
    fn from_tag(tag: u32) -> Option<Self> {
        Self::from_repr(tag)
    }
    fn tag(self) -> u32 {
        self as u32
    }
    const BITS: usize = 1;
    fn token_size(sizes: &CodedTokenSizes) -> ValueSize {
        sizes.has_semantic
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
#[repr(u32)]
pub enum MethodDefOrRefToken {
    Method,
    MemberRef,
}
impl CodedTokenTarget for MethodDefOrRefToken {
    fn from_tag(tag: u32) -> Option<Self> {
        Self::from_repr(tag)
    }
    fn tag(self) -> u32 {
        self as u32
    }
    const BITS: usize = 1;
    fn token_size(sizes: &CodedTokenSizes) -> ValueSize {
        sizes.method_def_or_ref
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
#[repr(u32)]
pub enum MemberForwardedToken {
    Field,
    Method,
}
impl CodedTokenTarget for MemberForwardedToken {
    fn from_tag(tag: u32) -> Option<Self> {
        Self::from_repr(tag)
    }
    fn tag(self) -> u32 {
        self as u32
    }
    const BITS: usize = 1;
    fn token_size(sizes: &CodedTokenSizes) -> ValueSize {
        sizes.member_forwarded
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
#[repr(u32)]
pub enum ImplementationToken {
    File,
    AssemblyRef,
    ExportedType,
}
impl CodedTokenTarget for ImplementationToken {
    fn from_tag(tag: u32) -> Option<Self> {
        Self::from_repr(tag)
    }
    fn tag(self) -> u32 {
        self as u32
    }
    const BITS: usize = 2;
    fn token_size(sizes: &CodedTokenSizes) -> ValueSize {
        sizes.implementation
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
#[repr(u32)]
pub enum CustomAttributeTypeToken {
    Unused1,
    Unused2,
    Method,
    MemberRef,
    Unused3,
}
impl CodedTokenTarget for CustomAttributeTypeToken {
    fn from_tag(tag: u32) -> Option<Self> {
        Self::from_repr(tag)
    }
    fn tag(self) -> u32 {
        self as u32
    }
    const BITS: usize = 3;
    fn token_size(sizes: &CodedTokenSizes) -> ValueSize {
        sizes.custom_attribute_type
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
#[repr(u32)]
pub enum ResolutionScopeToken {
    Module,
    ModuleRef,
    AssemblyRef,
    TypeRef,
}
impl CodedTokenTarget for ResolutionScopeToken {
    fn from_tag(tag: u32) -> Option<Self> {
        Self::from_repr(tag)
    }
    fn tag(self) -> u32 {
        self as u32
    }
    const BITS: usize = 2;
    #[inline(always)]
    fn token_size(sizes: &CodedTokenSizes) -> ValueSize {
        sizes.resolution_scope
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
#[repr(u32)]
pub enum TypeOrMethodDefToken {
    TypeDef,
    Method,
}
impl CodedTokenTarget for TypeOrMethodDefToken {
    fn from_tag(tag: u32) -> Option<Self> {
        Self::from_repr(tag)
    }
    fn tag(self) -> u32 {
        self as u32
    }
    const BITS: usize = 1;
    fn token_size(sizes: &CodedTokenSizes) -> ValueSize {
        sizes.type_or_method_def
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
#[repr(u32)]
pub enum HasCustomDebugInformationToken {
    Method,
    Field,
    TypeRef,
    TypeDef,
    Param,
    InterfaceImpl,
    MemberRef,
    Module,
    DeclSecurity,
    Property,
    Event,
    StandAloneSig,
    ModuleRef,
    TypeSpec,
    Assembly,
    AssemblyRef,
    File,
    ExportedType,
    ManifestResource,
    GenericParam,
    GenericParamConstraint,
    MethodSpec,
    Document,
    LocalScope,
    LocalVariable,
    LocalConstant,
    ImportScope,
}
impl CodedTokenTarget for HasCustomDebugInformationToken {
    fn from_tag(tag: u32) -> Option<Self> {
        Self::from_repr(tag)
    }
    fn tag(self) -> u32 {
        self as u32
    }
    const BITS: usize = 5;
    fn token_size(sizes: &CodedTokenSizes) -> ValueSize {
        sizes.has_custom_debug_information
    }
}
