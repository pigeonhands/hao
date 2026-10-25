use super::*;
use crate::error::{HaoError, Result};

pub(crate) trait TokenResolver {
    /// Resolves a `TypeDefOrRef` coded token as stored in a signature.
    fn type_def_or_ref(&self, coded: u32) -> Result<TypeDefOrRef>;
}

pub(crate) struct SignatureReader<'a, R: TokenResolver> {
    data: &'a [u8],
    position: usize,
    resolver: &'a R,
    depth: usize,
}

impl<'a, R: TokenResolver> SignatureReader<'a, R> {
    const MAX_DEPTH: usize = 64;
    const MAX_COUNT: u32 = 0x10000;

    pub fn new(data: &'a [u8], resolver: &'a R) -> Self {
        Self {
            data,
            position: 0,
            resolver,
            depth: 0,
        }
    }

    fn error(&self, msg: &'static str) -> HaoError {
        HaoError::InvalidSignature(msg, self.position)
    }

    fn peek_u8(&self) -> Result<u8> {
        self.data
            .get(self.position)
            .copied()
            .ok_or_else(|| self.error("unexpected end of signature"))
    }

    fn read_u8(&mut self) -> Result<u8> {
        let b = self.peek_u8()?;
        self.position += 1;
        Ok(b)
    }

    pub fn read_compressed_u32(&mut self) -> Result<u32> {
        let (value, len) = decode_compressed_u32(&self.data[self.position.min(self.data.len())..])
            .ok_or_else(|| self.error("invalid compressed integer"))?;
        self.position += len;
        Ok(value)
    }

    pub fn read_compressed_i32(&mut self) -> Result<i32> {
        let start = self.position;
        let raw = self.read_compressed_u32()?;
        let negative = raw & 1 != 0;
        let value = (raw >> 1) as i32;
        Ok(match (negative, self.position - start) {
            (false, _) => value,
            (true, 1) => value - 0x40,
            (true, 2) => value - 0x2000,
            (true, _) => value - 0x1000_0000,
        })
    }

    fn read_count(&mut self) -> Result<u32> {
        let count = self.read_compressed_u32()?;
        if count > Self::MAX_COUNT || count as usize > self.data.len() {
            return Err(self.error("count too large"));
        }
        Ok(count)
    }

    fn element_type(&mut self) -> Result<ElementType> {
        let b = self.read_u8()?;
        ElementType::from_repr(b).ok_or(HaoError::InvalidSignatureElementType(b, self.position - 1))
    }

    fn type_def_or_ref(&mut self) -> Result<TypeDefOrRef> {
        let coded = self.read_compressed_u32()?;
        self.resolver.type_def_or_ref(coded)
    }

    fn nested<T>(&mut self, f: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        self.depth += 1;
        if self.depth > Self::MAX_DEPTH {
            return Err(HaoError::RecursionLimitReached);
        }
        let result = f(self);
        self.depth -= 1;
        result
    }

    pub fn read_type(&mut self) -> Result<TypeSig> {
        self.nested(Self::read_type_inner)
    }

    fn boxed_type(&mut self) -> Result<Box<TypeSig>> {
        self.read_type().map(Box::new)
    }

    fn read_type_inner(&mut self) -> Result<TypeSig> {
        let element_type = self.element_type()?;
        if let Some(primitive) = PrimitiveType::from_repr(element_type as u8) {
            return Ok(TypeSig::Primitive(primitive));
        }

        let sig = match element_type {
            ElementType::ValueType => TypeSig::ValueType(self.type_def_or_ref()?),
            ElementType::Class => TypeSig::Class(self.type_def_or_ref()?),
            ElementType::Ptr => TypeSig::Ptr(self.boxed_type()?),
            ElementType::ByRef => TypeSig::ByRef(self.boxed_type()?),
            ElementType::Pinned => TypeSig::Pinned(self.boxed_type()?),
            ElementType::SzArray => TypeSig::SzArray(self.boxed_type()?),
            ElementType::Var => TypeSig::Var(self.read_compressed_u32()?),
            ElementType::MVar => TypeSig::MVar(self.read_compressed_u32()?),
            ElementType::FnPtr => {
                let sig = self.nested(|r| r.read_method_sig())?;
                TypeSig::FnPtr(Box::new(sig))
            }
            ElementType::CModReqd | ElementType::CModOpt => TypeSig::Modified {
                required: element_type == ElementType::CModReqd,
                modifier: self.type_def_or_ref()?,
                ty: self.boxed_type()?,
            },
            ElementType::GenericInst => {
                let generic = self.boxed_type()?;
                if !matches!(*generic, TypeSig::ValueType(_) | TypeSig::Class(_)) {
                    return Err(self.error("generic instance of a non class/valuetype"));
                }
                let count = self.read_count()?;
                let args = (0..count)
                    .map(|_| self.read_type())
                    .collect::<Result<_>>()?;
                TypeSig::GenericInst { generic, args }
            }
            ElementType::Array => {
                let element = self.boxed_type()?;
                let rank = self.read_compressed_u32()?;
                if rank == 0 || rank > 32 {
                    return Err(self.error("invalid array rank"));
                }
                let num_sizes = self.read_count()?;
                let sizes = (0..num_sizes)
                    .map(|_| self.read_compressed_u32())
                    .collect::<Result<_>>()?;
                let num_lo_bounds = self.read_count()?;
                let lo_bounds = (0..num_lo_bounds)
                    .map(|_| self.read_compressed_i32())
                    .collect::<Result<_>>()?;
                TypeSig::Array {
                    element,
                    shape: ArrayShape {
                        rank,
                        sizes,
                        lo_bounds,
                    },
                }
            }
            _ => return Err(HaoError::InvalidSignatureElementType(element_type as u8, self.position - 1)),
        };
        Ok(sig)
    }

    fn read_header(&mut self) -> Result<(CallingConvention, SignatureFlags)> {
        let b = self.read_u8()?;
        let cc = CallingConvention::from_repr(b & 0x0F)
            .ok_or(HaoError::InvalidSignatureCallingConvention(b))?;
        Ok((cc, SignatureFlags::from_bits_retain(b & 0xF0)))
    }

    fn read_method_body(&mut self, calling_convention: CallingConvention, flags: SignatureFlags) -> Result<MethodSig> {
        let generic_param_count = if flags.contains(SignatureFlags::GENERIC) {
            self.read_compressed_u32()?
        } else {
            0
        };
        let param_count = self.read_count()?;
        let ret = self.read_type()?;

        let mut params = Vec::with_capacity(param_count as usize);
        let mut varargs = Vec::new();
        let mut after_sentinel = false;
        for _ in 0..param_count {
            if self.peek_u8()? == ElementType::Sentinel as u8 {
                self.position += 1;
                after_sentinel = true;
            }
            let ty = self.read_type()?;
            if after_sentinel {
                varargs.push(ty);
            } else {
                params.push(ty);
            }
        }

        Ok(MethodSig {
            calling_convention,
            flags,
            generic_param_count,
            ret,
            params,
            varargs,
        })
    }

    pub fn read_method_sig(&mut self) -> Result<MethodSig> {
        let (cc, flags) = self.read_header()?;
        match cc {
            CallingConvention::Field
            | CallingConvention::LocalSig
            | CallingConvention::Property
            | CallingConvention::GenericInst => Err(self.error("expected a method signature")),
            cc => self.read_method_body(cc, flags),
        }
    }

    pub fn read_field_sig(&mut self) -> Result<FieldSig> {
        match self.read_header()? {
            (CallingConvention::Field, _) => Ok(FieldSig {
                ty: self.read_type()?,
            }),
            _ => Err(self.error("expected a field signature")),
        }
    }

    pub fn read_member_ref_sig(&mut self) -> Result<MemberRefSig> {
        match self.peek_u8()? & 0x0F {
            0x06 => self.read_field_sig().map(MemberRefSig::Field),
            _ => self.read_method_sig().map(MemberRefSig::Method),
        }
    }

    pub fn read_property_sig(&mut self) -> Result<PropertySig> {
        let (cc, flags) = self.read_header()?;
        if cc != CallingConvention::Property {
            return Err(self.error("expected a property signature"));
        }
        let count = self.read_count()?;
        let ty = self.read_type()?;
        let params = (0..count).map(|_| self.read_type()).collect::<Result<_>>()?;
        Ok(PropertySig {
            has_this: flags.contains(SignatureFlags::HAS_THIS),
            ty,
            params,
        })
    }

    pub fn read_local_sig(&mut self) -> Result<Vec<TypeSig>> {
        match self.read_header()? {
            (CallingConvention::LocalSig, _) => {
                let count = self.read_count()?;
                (0..count).map(|_| self.read_type()).collect()
            }
            _ => Err(self.error("expected a local variable signature")),
        }
    }

    pub fn read_method_spec_sig(&mut self) -> Result<Vec<TypeSig>> {
        match self.read_header()? {
            (CallingConvention::GenericInst, _) => {
                let count = self.read_count()?;
                (0..count).map(|_| self.read_type()).collect()
            }
            _ => Err(self.error("expected a generic method instantiation")),
        }
    }

    /// Either a method or local signature, as referenced by a `StandAloneSig`.
    pub fn read_stand_alone_sig(&mut self) -> Result<StandAloneSig> {
        if self.peek_u8()? & 0x0F == CallingConvention::LocalSig as u8 {
            self.read_local_sig().map(StandAloneSig::Locals)
        } else {
            self.read_method_sig().map(StandAloneSig::Method)
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum StandAloneSig {
    Locals(Vec<TypeSig>),
    Method(MethodSig),
}

/// Returns the value and the number of bytes it took.
pub(crate) fn decode_compressed_u32(data: &[u8]) -> Option<(u32, usize)> {
    let b0 = *data.first()?;
    if b0 & 0x80 == 0 {
        Some((b0 as u32, 1))
    } else if b0 & 0xC0 == 0x80 {
        let b1 = *data.get(1)?;
        Some(((((b0 & 0x3F) as u32) << 8) | b1 as u32, 2))
    } else if b0 & 0xE0 == 0xC0 {
        let bytes = data.get(1..4)?;
        Some((
            (((b0 & 0x1F) as u32) << 24)
                | ((bytes[0] as u32) << 16)
                | ((bytes[1] as u32) << 8)
                | bytes[2] as u32,
            4,
        ))
    } else {
        None
    }
}
