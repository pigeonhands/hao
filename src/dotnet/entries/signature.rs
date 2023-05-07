use std::{cell::Ref, fmt::Display, ops::Deref};

pub use crate::dotnet::md::streams::ArraySize;
use crate::{
    dotnet::md::streams::{SignatureCallingConvention, SignatureDef, TypeSigDef},
    error::{HaoError, Result},
};

use super::{
    values::{ModuleDef, ResolutionScopePtr, TypeDef, TypeDefOrRefPtr, TypeRef, TypeSpec},
    Entry, RowEntry,
};

#[derive(Debug, Clone)]
pub enum ResolutionScope {
    Module(Entry<ModuleDef>),
    //ModuleRef,
    //AssemblyRef,
    TypeRef(Entry<TypeRef>),
    NotImplimented,
}

impl ResolutionScope {
    pub(crate) fn from_ent_pointer(ptr: ResolutionScopePtr) -> Option<Self> {
        match ptr {
            ResolutionScopePtr::Module(d) => Some(Self::Module(Entry(d))),
            ResolutionScopePtr::TypeRef(d) => Some(Self::TypeRef(Entry(d))),
            _ => Some(Self::NotImplimented),
        }
    }
    pub(crate) fn from_ent_ptr_must(ptr: ResolutionScopePtr) -> Result<Self> {
        Self::from_ent_pointer(ptr).ok_or_else(|| HaoError::InvalidSignatureForEntry(std::any::type_name::<Self>()))
    }
}

impl Display for ResolutionScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Module(e) => write!(f, "<Module>({})", e.value().name),
            Self::TypeRef(e) => write!(f, "{}", e.value().name),
            Self::NotImplimented => write!(f, "NotImplimented"),
        }
    }
}

#[derive(Clone)]
pub enum TypeDefOrRef {
    TypeDef(Entry<TypeDef>),
    TypeRef(Entry<TypeRef>),
    TypeSpec(Entry<TypeSpec>),
}

impl TypeDefOrRef {
    pub(crate) fn from_ent_pointer(ptr: TypeDefOrRefPtr) -> Option<Self> {
        match ptr {
            TypeDefOrRefPtr::TypeDef(d) => Some(Self::TypeDef(Entry(d))),
            TypeDefOrRefPtr::TypeRef(r) => Some(Self::TypeRef(Entry(r))),
            TypeDefOrRefPtr::TypeSpec(s) => Some(Self::TypeSpec(Entry(s))),
            TypeDefOrRefPtr::None => None,
        }
    }
    pub(crate) fn from_ent_ptr_must(ptr: TypeDefOrRefPtr) -> Result<Self> {
        Self::from_ent_pointer(ptr).ok_or_else(|| HaoError::InvalidSignatureForEntry(std::any::type_name::<Self>()))
    }

    pub fn is_type_ref_and(&self, func: impl FnOnce(Ref<RowEntry<TypeRef>>) -> bool) -> bool {
        match self {
            Self::TypeRef(r) => func(r.value()),
            _ => false,
        }
    }

    pub fn is_type_def_and(&self, func: impl FnOnce(Ref<RowEntry<TypeDef>>) -> bool) -> bool {
        match self {
            Self::TypeDef(r) => func(r.value()),
            _ => false,
        }
    }
}

// impl to stop infiniate reccursion on debug print
impl std::fmt::Debug for TypeDefOrRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TypeDef(e) => write!(
                f,
                "Self::TypeDef(\"{}.{}\")",
                e.value().namespace(),
                e.value().name()
            ),
            Self::TypeRef(e) => write!(
                f,
                "Self::TypeRef(\"{}.{}\")",
                e.value().namespace(),
                e.value().name()
            ),
            Self::TypeSpec(s) => write!(f, "{:?}", s.value().signature()),
        }
    }
}

impl Display for TypeDefOrRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TypeDef(e) => write!(f, "{}", e.value().name()),
            Self::TypeRef(e) => write!(f, "{}", e.value().name()),
            Self::TypeSpec(s) => write!(f, "{}", s.value().signature()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ValueType {
    Void,
    Boolean,
    Char,
    SByte,
    Byte,
    Int16,
    UInt16,
    Int32,
    UInt32,
    Int64,
    UInt64,
    Single,
    Double,
    String,
    TypedRefrence,
    IntPtr,
    UIntPtr,
    Object,

    //Ptr(Box<SignatureType>),
    //ByRef(Box<SignatureType>),
    ValueType(TypeDefOrRef),
    Class(TypeDefOrRef),
    //FnPtr(Box<SignatureDef>),
    SZArray(Box<ValueType>),
    // CModReq(TypeDefOrRefSig),
    // CModOpt(TypeDefOrRefSig),
    //Pinned(Box<SignatureType>),
    Var {
        generic_param_index: u32,
    },
    MVar {
        generic_param_index: u32,
    },
    ValueArray {
        len: u32,
        ty: Box<ValueType>,
    },
    Module(u32, Box<ValueType>),
    GenericInst {
        ty: Box<ValueType>,
        generic_args: Vec<ValueType>,
    },
    Array {
        ty: Box<ValueType>,
        size: ArraySize,
    },
    NotDone(TypeSigDef),
}

impl ValueType {
    pub fn from_type_sig(sig: TypeSigDef) -> Result<Self> {
        let sig = match sig {
            TypeSigDef::Void => Self::Void,
            TypeSigDef::Boolean => Self::Boolean,
            TypeSigDef::Char => Self::Char,
            TypeSigDef::SByte => Self::SByte,
            TypeSigDef::Byte => Self::Byte,
            TypeSigDef::Int16 => Self::Int16,
            TypeSigDef::UInt16 => Self::UInt16,
            TypeSigDef::Int32 => Self::Int32,
            TypeSigDef::UInt32 => Self::UInt32,
            TypeSigDef::Int64 => Self::Int64,
            TypeSigDef::UInt64 => Self::UInt64,
            TypeSigDef::Single => Self::Single,
            TypeSigDef::Double => Self::Double,
            TypeSigDef::String => Self::String,
            TypeSigDef::TypedRefrence => Self::TypedRefrence,
            TypeSigDef::IntPtr => Self::IntPtr,
            TypeSigDef::UIntPtr => Self::IntPtr,
            TypeSigDef::Object => Self::Object,

            TypeSigDef::ValueType(ptr) => Self::ValueType(TypeDefOrRef::from_ent_ptr_must(ptr.0)?),
            TypeSigDef::SZArray(ty) => Self::SZArray(Box::new(Self::from_type_sig(*ty)?)),
            TypeSigDef::Var { generic_param_index } => Self::Var { generic_param_index },
            TypeSigDef::Class(ptr) => Self::Class(TypeDefOrRef::from_ent_ptr_must(ptr.0)?),
            TypeSigDef::ValueArray { len, next_sig: ty } => Self::ValueArray {
                len,
                ty: Box::new(Self::from_type_sig(*ty)?),
            },
            TypeSigDef::GenericInst { ty, generic_args } => Self::GenericInst {
                ty: Box::new(Self::from_type_sig(*ty)?),
                generic_args: generic_args
                    .into_iter()
                    .map(Self::from_type_sig)
                    .collect::<Result<_>>()?,
            },
            TypeSigDef::Array { ty, size } => Self::Array {
                ty: Box::new(Self::from_type_sig(*ty)?),
                size,
            },
            
            t => Self::NotDone(t), // return Err(HaoError::InvalidSignatureForEntry),
        };
        Ok(sig)
    }
}

impl Display for ValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Void => write!(f, "void"),
            Self::Boolean => write!(f, "bool"),
            Self::Char => write!(f, "char"),
            Self::SByte => write!(f, "sbyte"),
            Self::Byte => write!(f, "byte"),
            Self::Int16 => write!(f, "short"),
            Self::UInt16 => write!(f, "ushort"),
            Self::Int32 => write!(f, "int"),
            Self::UInt32 => write!(f, "uint"),
            Self::Int64 => write!(f, "long"),
            Self::UInt64 => write!(f, "ulong"),
            Self::Single => write!(f, "float"),
            Self::Double => write!(f, "double"),
            Self::String => write!(f, "string"),
            Self::TypedRefrence => write!(f, "refrence"),
            Self::IntPtr => write!(f, "IntPtr"),
            Self::UIntPtr => write!(f, "UIntPtr"),
            Self::Object => write!(f, "object"),
            Self::ValueType(val) => write!(f, "{}", val),
            Self::SZArray(ty) => write!(f, "{}[]", ty),
            Self::Class(val) => write!(f, "{}", val),
            Self::ValueArray { .. } => panic!("valuearray?"),
            Self::Var { generic_param_index: generic_params } => write!(f, "T{}", generic_params),
            Self::MVar { generic_param_index: generic_params } => write!(f, "M{}", generic_params),
            Self::GenericInst { ty, generic_args } => {
                write!(f, "{}", ty)?;
                write!(f, "<")?;
                for (index, ty) in generic_args.iter().enumerate() {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", ty)?;
                }
                write!(f, ">")
            },
            Self::Array { ty, size } => match size {
                ArraySize::SingleDimention => write!(f, "{}[]", ty),
                ArraySize::MultiDimention { sizes, .. } => {
                    write!(f, "{}{:?}", ty, sizes.as_slice())
                }
            },
            Self::Module(_, _) => write!(f, "ValueType(Module)"),
            Self::NotDone(x) => write!(f, "{:?}", x),
          // _ => write!(f, "ValueType(Other)"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FieldSignature(pub ValueType);

impl FieldSignature {
    pub fn from_sig_def(sig: SignatureDef) -> Result<Self> {
        let field_sig = match sig.calling_convention {
            SignatureCallingConvention::Field(field) => field,
            _ => return Err(HaoError::InvalidSignatureForEntry(std::any::type_name::<Self>())),
        };

        Ok(Self(ValueType::from_type_sig(field_sig)?))
    }
}

impl Display for FieldSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Deref for FieldSignature {
    type Target = ValueType;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub enum TypeSignature{
    GenericInst{ ty: ValueType, generic_args: Vec<ValueType> },
    ClassVariable { generic_param_index: u32 },
    MethodVariable { generic_param_index: u32 },
    SZArray(ValueType)
}

impl TypeSignature {
    pub fn from_sig_def(sig: TypeSigDef) -> Result<Self> {
        match sig {
            TypeSigDef::GenericInst { ty, generic_args } => Ok(Self::GenericInst {
                ty: ValueType::from_type_sig(*ty)?,
                generic_args: generic_args
                    .into_iter()
                    .map(ValueType::from_type_sig)
                    .collect::<Result<_>>()?,
            }),
            TypeSigDef::Var { generic_param_index } => Ok(Self::ClassVariable { generic_param_index }),
            TypeSigDef::MVar { generic_param_index } => Ok(Self::MethodVariable { generic_param_index }),
            TypeSigDef::SZArray(ty) => Ok(Self::SZArray(ValueType::from_type_sig(*ty)?)),
            n => return Err(HaoError::InvalidSignatureForEntry(std::any::type_name::<Self>())),
        }
    }
}

impl Display for TypeSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GenericInst { ty, generic_args } => {
                write!(f, "{}", ty)?;
                write!(f, "<")?;
                for (index, ty) in generic_args.iter().enumerate() {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", ty)?;
                }
                write!(f, ">")
            },
            Self::ClassVariable { generic_param_index } => write!(f, "GenericVar({})", generic_param_index),
            Self::MethodVariable { generic_param_index } => write!(f, "GenericMethodVar({})", generic_param_index),
            Self::SZArray(ty) => write!(f, "{}[]", ty)
        }
       
    }
}