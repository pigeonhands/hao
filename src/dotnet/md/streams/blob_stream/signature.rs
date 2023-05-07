use bitflags::bitflags;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;
use std::fmt::Debug;

use super::reader::{BlobStream, SignatureReader};
use crate::dotnet::entries::values::*;
use crate::dotnet::entries::GetEntryField;
use crate::dotnet::md::streams::tables_stream::coded_tokens::{CodedToken, TypeDefOrRefToken};
use crate::error::{HaoError, Result};
use crate::io::ReadData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, FromPrimitive, ToPrimitive)]
pub enum CallingConvention {
    Default = 0x0,
    C = 0x1,
    StdCall = 0x2,
    ThisCall = 0x3,
    FastCall = 0x4,
    VarArg = 0x5,
    Field = 0x6,
    LocalSig = 0x7,
    Property = 0x8,
    Unmanaged = 0x9,
    GenericInst = 0xA,
    NativeVarArg = 0xB,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, FromPrimitive, ToPrimitive)]
pub enum ElementType {
    End = 0x00,
    Void = 0x01,
    Boolean = 0x02,
    Char = 0x03,
    I1 = 0x04,
    U1 = 0x05,
    I2 = 0x06,
    U2 = 0x07,
    I4 = 0x08,
    U4 = 0x09,
    I8 = 0x0A,
    U8 = 0x0B,
    R4 = 0x0C,
    R8 = 0x0D,
    String = 0x0E,
    Ptr = 0x0F,
    ByRef = 0x10,
    ValueType = 0x11,
    Class = 0x12,
    Var = 0x13,
    Array = 0x14,
    GenericInst = 0x15,
    TypedByRef = 0x16,
    ValueArray = 0x17,
    I = 0x18,
    U = 0x19,
    R = 0x1A,
    FnPtr = 0x1B,
    Object = 0x1C,
    SZArray = 0x1D,
    MVar = 0x1E,
    CModReqd = 0x1F,
    CModOpt = 0x20,
    Internal = 0x21,
    Module = 0x3F,
    Sentinel = 0x41,
    Pinned = 0x45,
}

impl<'a> ReadData<ElementType> for BlobStream<'a> {
    fn read(&mut self) -> Result<ElementType> {
        let b = self.read()?;
        ElementType::from_u8(b)
            .ok_or_else(|| HaoError::InvalidSignatureElementType(b, self.heap_position()))
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct SignatureFlags: u8 {
        const Generic		= 0x10;
        const HasThis		= 0x20;
        const ExplicitThis	= 0x40;
        const ReservedByCLR	= 0x80;
    }
}

#[derive(Clone, Debug)]
pub struct SignatureDef {
    pub calling_convention: SignatureCallingConvention,
    pub flags: SignatureFlags,
}

impl<'a> ReadData<SignatureDef> for SignatureReader<'a> {
    fn read(&mut self) -> Result<SignatureDef> {
        self.recursion_inc()?;

        const CALLING_CONVENTION_MASK: u8 = 0x0F;

        let sig_type: u8 = self.read()?;

        let calling_convention = CallingConvention::from_u8(sig_type & CALLING_CONVENTION_MASK)
            .ok_or_else(|| {
                HaoError::InvalidSignatureCallingConvention(sig_type & CALLING_CONVENTION_MASK)
            })?;

        let flags = SignatureFlags::from_bits_retain(sig_type & (!CALLING_CONVENTION_MASK));
        let calling_convention =
            SignatureCallingConvention::from_reader(self, calling_convention, flags)?;

        self.recursion_dec();
        Ok(SignatureDef {
            flags,
            calling_convention,
        })
    }
}

#[derive(Clone, Debug)]
pub enum SignatureCallingConvention {
    Field(TypeSigDef),
    Method(MethodSig),
    LocalSig(LocalSig),
    Property(MethodSig),
    GenericInstMethod(GenericInstMethodSig),
}

impl SignatureCallingConvention {
    pub fn from_reader(
        reader: &mut SignatureReader,
        calling_convention: CallingConvention,
        flags: SignatureFlags,
    ) -> Result<Self> {
        let sig = match calling_convention {
            CallingConvention::Default
            | CallingConvention::C
            | CallingConvention::StdCall
            | CallingConvention::ThisCall
            | CallingConvention::FastCall
            | CallingConvention::VarArg
            | CallingConvention::Unmanaged
            | CallingConvention::NativeVarArg => {
                Self::Method(MethodSig::from_reader(reader, flags)?)
            }
            CallingConvention::Field => Self::Field(reader.read()?),
            CallingConvention::LocalSig => Self::LocalSig(reader.read()?),
            CallingConvention::Property => Self::Property(MethodSig::from_reader(reader, flags)?),
            CallingConvention::GenericInst => Self::GenericInstMethod(reader.read()?),
        };

        Ok(sig)
    }
}

#[derive(Clone)]
pub struct TypeDefOrRefSig(pub(crate) TypeDefOrRefPtr);

// This is to stop infinite reccursion when debug printing
impl Debug for TypeDefOrRefSig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            TypeDefOrRefPtr::None => write!(f, "None"),
            TypeDefOrRefPtr::TypeDef(tref) => match tref.is_set() {
                true => write!(
                    f,
                    "TypeRef(Ptr(\"{}.{}\"))",
                    tref.value().namespace,
                    tref.value().name
                ),
                false => write!(f, "TypeRef(Invalid)"),
            },
            TypeDefOrRefPtr::TypeRef(tdef) => match tdef.is_set() {
                true => write!(
                    f,
                    "TypeRef(Ptr(\"{}.{}\"))",
                    tdef.value().namespace,
                    tdef.value().name
                ),
                false => write!(f, "TypeRef(Invalid)"),
            },
            n => write!(f, "{:?}", n),
        }
    }
}

impl From<TypeDefOrRefPtr> for TypeDefOrRefSig {
    fn from(value: TypeDefOrRefPtr) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone)]
pub enum ArraySize {
    SingleDimention,
    MultiDimention {
        rank: u32,
        sizes: Vec<u32>,
        lower_bounds: Vec<u32>,
    },
}

#[derive(Debug, Clone)]
pub enum TypeSigDef {
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

    Ptr(Box<TypeSigDef>),
    ByRef(Box<TypeSigDef>),
    ValueType(TypeDefOrRefSig),
    Class(TypeDefOrRefSig),
    FnPtr(Box<SignatureDef>),
    SZArray(Box<TypeSigDef>),
    CModReq(TypeDefOrRefSig),
    CModOpt(TypeDefOrRefSig),
    Sentinel,
    Pinned(Box<TypeSigDef>),

    Var {
        generic_param_index: u32,
    },
    MVar {
        generic_param_index: u32,
    },
    ValueArray {
        len: u32,
        next_sig: Box<TypeSigDef>,
    },
    Module {
        index: u32,
        ty: Box<TypeSigDef>,
    },
    GenericInst {
        ty: Box<TypeSigDef>,
        generic_args: Vec<TypeSigDef>,
    },
    Array {
        ty: Box<TypeSigDef>,
        size: ArraySize,
    },
    Internal,

    Unknown(ElementType),
}

impl TypeSigDef {
    const MAX_ARRAY_RANK: u32 = 64;
}

impl<'a> ReadData<TypeSigDef> for SignatureReader<'a> {
    fn read(&mut self) -> Result<TypeSigDef> {
        let element_type: ElementType = self.read()?;

        let value = match element_type {
            ElementType::Void => TypeSigDef::Void,
            ElementType::Boolean => TypeSigDef::Boolean,
            ElementType::Char => TypeSigDef::Char,
            ElementType::I1 => TypeSigDef::Byte,
            ElementType::U1 => TypeSigDef::SByte,
            ElementType::I2 => TypeSigDef::Int16,
            ElementType::U2 => TypeSigDef::UInt16,
            ElementType::I4 => TypeSigDef::Int32,
            ElementType::U4 => TypeSigDef::UInt32,
            ElementType::I8 => TypeSigDef::Int64,
            ElementType::U8 => TypeSigDef::UInt64,
            ElementType::R4 => TypeSigDef::Single,
            ElementType::R8 => TypeSigDef::Double,
            ElementType::String => TypeSigDef::String,
            ElementType::TypedByRef => TypeSigDef::TypedRefrence,
            ElementType::I => TypeSigDef::IntPtr,
            ElementType::U => TypeSigDef::UIntPtr,
            ElementType::Object => TypeSigDef::Object,

            ElementType::Ptr => TypeSigDef::Ptr(Box::new(self.read()?)),
            ElementType::ByRef => TypeSigDef::ByRef(Box::new(self.read()?)),
            ElementType::ValueType => {
                let token: CodedToken<TypeDefOrRefToken> = self.read()?;
                TypeSigDef::ValueType(self.entries.get_entry_field(token)?.into())
            }
            ElementType::Class => {
                let token: CodedToken<TypeDefOrRefToken> = self.read()?;
                TypeSigDef::Class(self.entries.get_entry_field(token)?.into())
            }
            ElementType::FnPtr => TypeSigDef::FnPtr(Box::new(self.read()?)),
            ElementType::SZArray => TypeSigDef::SZArray(Box::new(self.read()?)),
            ElementType::CModReqd => {
                let token: CodedToken<TypeDefOrRefToken> = self.read()?;
                TypeSigDef::CModReq(self.entries.get_entry_field(token)?.into())
            }
            ElementType::CModOpt => {
                let token: CodedToken<TypeDefOrRefToken> = self.read()?;
                TypeSigDef::CModOpt(self.entries.get_entry_field(token)?.into())
            }
            ElementType::Sentinel => TypeSigDef::Sentinel,
            ElementType::Pinned => TypeSigDef::Pinned(Box::new(self.read()?)),

            ElementType::Var => TypeSigDef::Var {
                generic_param_index: self.reader.read_compressed_u32()?,
            },
            ElementType::MVar => TypeSigDef::MVar {
                generic_param_index: self.reader.read_compressed_u32()?,
            },
            ElementType::ValueArray => TypeSigDef::ValueArray {
                len: self.reader.read_compressed_u32()?,
                next_sig: Box::new(self.read()?),
            },
            ElementType::Module => TypeSigDef::Module {
                index: self.reader.read_compressed_u32()?,
                ty: Box::new(self.read()?),
            },
            ElementType::GenericInst => {
                let ty = Box::new(self.read()?);
                let n = self.reader.read_compressed_u32()? as usize;
                let mut generic_args = Vec::with_capacity(n);
                for _ in 0..n {
                    generic_args.push(self.read()?);
                }
                TypeSigDef::GenericInst { ty, generic_args }
            }
            ElementType::Array => {
                let ty = Box::new(self.read()?);
                let rank = self.reader.read_compressed_u32()?;
                if rank > TypeSigDef::MAX_ARRAY_RANK {
                    return Err(HaoError::BadImageFormat(
                        "Tried to read an array with more dimentions than allowed.",
                    ));
                }

                TypeSigDef::Array {
                    ty,
                    size: if rank == 0 {
                        ArraySize::SingleDimention
                    } else {
                        let num = self.reader.read_compressed_u32()?;
                        if num > rank {
                            return Err(HaoError::BadImageFormat(
                                "Tried to read an array with more dimentions than rank.",
                            ));
                        }
                        let mut sizes = Vec::with_capacity(num as usize);
                        for _ in 0..num {
                            sizes.push(self.reader.read_compressed_u32()?);
                        }
                        let mut lower_bounds = Vec::with_capacity(num as usize);
                        for _ in 0..num {
                            lower_bounds.push(self.reader.read_compressed_u32()?);
                        }
                        ArraySize::MultiDimention {
                            rank,
                            sizes,
                            lower_bounds,
                        }
                    },
                }
            }
            ElementType::Internal => TypeSigDef::Internal, // TODO
            ElementType::End | ElementType::R => TypeSigDef::Unknown(element_type),
        };
        Ok(value)
    }
}

#[derive(Debug, Clone)]
pub struct MethodSig {
    pub return_type: Box<TypeSigDef>,
    pub generic_params: Option<u32>,
    pub parameters: Vec<TypeSigDef>,
    pub params_after_sentinel: Option<Vec<TypeSigDef>>,
}

impl MethodSig {
    pub fn from_reader(reader: &mut SignatureReader, flags: SignatureFlags) -> Result<Self> {
        let generic_params = (flags.contains(SignatureFlags::Generic))
            .then(|| reader.reader.read_compressed_u32())
            .transpose()?;

        let num_params = reader.reader.read_compressed_u32()?;
        let return_type = Box::new(reader.read()?);

        if num_params as usize > reader.reader.reader.remaning() {
            return Ok(Self {
                generic_params: None,
                parameters: Vec::new(),
                params_after_sentinel: None,
                return_type,
            });
        }

        let mut parameters = Vec::with_capacity(num_params as usize);
        let mut params_after_sentinel: Option<Vec<TypeSigDef>> = None;

        for _ in 0..num_params {
            let ty = reader.read()?;
            if matches!(ty, TypeSigDef::Sentinel) {
                let senti_vec = if let Some(v) = &mut params_after_sentinel {
                    v
                } else {
                    params_after_sentinel =
                        Some(Vec::with_capacity((num_params as usize) - parameters.len()));
                    params_after_sentinel.as_mut().unwrap()
                };
                senti_vec.push(reader.read()?);
            }

            if let Some(senti_params) = &mut params_after_sentinel {
                senti_params.push(ty);
            } else {
                parameters.push(ty);
            }
        }

        Ok(Self {
            return_type,
            generic_params,
            parameters,
            params_after_sentinel,
        })
    }
}

#[derive(Debug, Clone)]
pub struct LocalSig {
    pub locals: Vec<TypeSigDef>,
}

impl<'a> ReadData<LocalSig> for SignatureReader<'a> {
    fn read(&mut self) -> Result<LocalSig> {
        let count = self.reader.read_compressed_u32()?;
        if count > 0x1000 {
            return Err(HaoError::BadImageFormat(
                "Tried to read an LocalSig with more locals than 0x1000.",
            ));
        }

        let mut locals = Vec::with_capacity(count as usize);
        for _ in 0..count {
            locals.push(self.read()?);
        }
        Ok(LocalSig { locals })
    }
}

#[derive(Debug, Clone)]
pub struct GenericInstMethodSig {
    pub generic_args: Vec<TypeSigDef>,
}

impl<'a> ReadData<GenericInstMethodSig> for SignatureReader<'a> {
    fn read(&mut self) -> Result<GenericInstMethodSig> {
        let count = self.reader.read_compressed_u32()?;
        if count > 0x1000 {
            return Err(HaoError::BadImageFormat(
                "Tried to read an GenericInstMethodSig with more args than 0x1000.",
            ));
        }

        let mut generic_args = Vec::with_capacity(count as usize);
        for _ in 0..count {
            generic_args.push(self.read()?);
        }
        Ok(GenericInstMethodSig { generic_args })
    }
}
