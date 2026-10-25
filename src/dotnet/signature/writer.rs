use super::*;
use crate::error::{HaoError, Result};

pub(crate) trait TokenEncoder {
    /// Returns the `TypeDefOrRef` coded token for `ty`.
    fn type_def_or_ref(&mut self, ty: &TypeDefOrRef) -> Result<u32>;
}

pub(crate) struct SignatureWriter<'a, E: TokenEncoder> {
    pub buffer: Vec<u8>,
    encoder: &'a mut E,
}

impl<'a, E: TokenEncoder> SignatureWriter<'a, E> {
    pub fn new(encoder: &'a mut E) -> Self {
        Self {
            buffer: Vec::new(),
            encoder,
        }
    }

    pub fn finish(self) -> Vec<u8> {
        self.buffer
    }

    pub fn write_compressed_u32(&mut self, value: u32) -> Result<()> {
        encode_compressed_u32(&mut self.buffer, value)
    }

    pub fn write_compressed_i32(&mut self, value: i32) -> Result<()> {
        let (raw, bits) = match value {
            -0x40..=0x3F => ((value as u32) & 0x7F, 7),
            -0x2000..=0x1FFF => ((value as u32) & 0x3FFF, 14),
            -0x1000_0000..=0x0FFF_FFFF => ((value as u32) & 0x1FFF_FFFF, 29),
            _ => return Err(HaoError::ValueTooLarge("compressed signed integer")),
        };
        let mask = (1u32 << bits) - 1;
        let rotated = ((raw << 1) & mask) | (raw >> (bits - 1));
        match bits {
            7 => self.buffer.push(rotated as u8),
            14 => self
                .buffer
                .extend_from_slice(&((rotated as u16) | 0x8000).to_be_bytes()),
            _ => self
                .buffer
                .extend_from_slice(&(rotated | 0xC000_0000).to_be_bytes()),
        }
        Ok(())
    }

    fn element(&mut self, element_type: ElementType) {
        self.buffer.push(element_type as u8);
    }

    fn type_def_or_ref(&mut self, ty: &TypeDefOrRef) -> Result<()> {
        let coded = self.encoder.type_def_or_ref(ty)?;
        self.write_compressed_u32(coded)
    }

    pub fn write_type(&mut self, sig: &TypeSig) -> Result<()> {
        match sig {
            TypeSig::Primitive(p) => self.buffer.push(*p as u8),
            TypeSig::ValueType(t) => {
                self.element(ElementType::ValueType);
                self.type_def_or_ref(t)?;
            }
            TypeSig::Class(t) => {
                self.element(ElementType::Class);
                self.type_def_or_ref(t)?;
            }
            TypeSig::Ptr(t) => {
                self.element(ElementType::Ptr);
                self.write_type(t)?;
            }
            TypeSig::ByRef(t) => {
                self.element(ElementType::ByRef);
                self.write_type(t)?;
            }
            TypeSig::Pinned(t) => {
                self.element(ElementType::Pinned);
                self.write_type(t)?;
            }
            TypeSig::SzArray(t) => {
                self.element(ElementType::SzArray);
                self.write_type(t)?;
            }
            TypeSig::Var(i) => {
                self.element(ElementType::Var);
                self.write_compressed_u32(*i)?;
            }
            TypeSig::MVar(i) => {
                self.element(ElementType::MVar);
                self.write_compressed_u32(*i)?;
            }
            TypeSig::FnPtr(m) => {
                self.element(ElementType::FnPtr);
                self.write_method_sig(m)?;
            }
            TypeSig::Modified {
                required,
                modifier,
                ty,
            } => {
                self.element(if *required {
                    ElementType::CModReqd
                } else {
                    ElementType::CModOpt
                });
                self.type_def_or_ref(modifier)?;
                self.write_type(ty)?;
            }
            TypeSig::GenericInst { generic, args } => {
                self.element(ElementType::GenericInst);
                self.write_type(generic)?;
                self.write_compressed_u32(args.len() as u32)?;
                for arg in args {
                    self.write_type(arg)?;
                }
            }
            TypeSig::Array { element, shape } => {
                self.element(ElementType::Array);
                self.write_type(element)?;
                self.write_compressed_u32(shape.rank)?;
                self.write_compressed_u32(shape.sizes.len() as u32)?;
                for size in &shape.sizes {
                    self.write_compressed_u32(*size)?;
                }
                self.write_compressed_u32(shape.lo_bounds.len() as u32)?;
                for bound in &shape.lo_bounds {
                    self.write_compressed_i32(*bound)?;
                }
            }
        }
        Ok(())
    }

    pub fn write_method_sig(&mut self, sig: &MethodSig) -> Result<()> {
        let mut flags = sig.flags;
        flags.set(SignatureFlags::GENERIC, sig.generic_param_count > 0);
        self.buffer.push(sig.calling_convention as u8 | flags.bits());
        if sig.generic_param_count > 0 {
            self.write_compressed_u32(sig.generic_param_count)?;
        }
        self.write_compressed_u32((sig.params.len() + sig.varargs.len()) as u32)?;
        self.write_type(&sig.ret)?;
        for param in &sig.params {
            self.write_type(param)?;
        }
        if !sig.varargs.is_empty() {
            self.element(ElementType::Sentinel);
            for param in &sig.varargs {
                self.write_type(param)?;
            }
        }
        Ok(())
    }

    pub fn write_field_sig(&mut self, sig: &FieldSig) -> Result<()> {
        self.buffer.push(CallingConvention::Field as u8);
        self.write_type(&sig.ty)
    }

    pub fn write_member_ref_sig(&mut self, sig: &MemberRefSig) -> Result<()> {
        match sig {
            MemberRefSig::Method(m) => self.write_method_sig(m),
            MemberRefSig::Field(f) => self.write_field_sig(f),
        }
    }

    pub fn write_property_sig(&mut self, sig: &PropertySig) -> Result<()> {
        let flags = if sig.has_this {
            SignatureFlags::HAS_THIS
        } else {
            SignatureFlags::empty()
        };
        self.buffer.push(CallingConvention::Property as u8 | flags.bits());
        self.write_compressed_u32(sig.params.len() as u32)?;
        self.write_type(&sig.ty)?;
        for param in &sig.params {
            self.write_type(param)?;
        }
        Ok(())
    }

    pub fn write_local_sig(&mut self, locals: &[TypeSig]) -> Result<()> {
        self.buffer.push(CallingConvention::LocalSig as u8);
        self.write_compressed_u32(locals.len() as u32)?;
        for local in locals {
            self.write_type(local)?;
        }
        Ok(())
    }

    pub fn write_method_spec_sig(&mut self, args: &[TypeSig]) -> Result<()> {
        self.buffer.push(CallingConvention::GenericInst as u8);
        self.write_compressed_u32(args.len() as u32)?;
        for arg in args {
            self.write_type(arg)?;
        }
        Ok(())
    }
}

pub(crate) fn encode_compressed_u32(buffer: &mut Vec<u8>, value: u32) -> Result<()> {
    match value {
        0..=0x7F => buffer.push(value as u8),
        0x80..=0x3FFF => buffer.extend_from_slice(&((value as u16) | 0x8000).to_be_bytes()),
        0x4000..=0x1FFF_FFFF => buffer.extend_from_slice(&(value | 0xC000_0000).to_be_bytes()),
        _ => return Err(HaoError::ValueTooLarge("compressed integer")),
    }
    Ok(())
}
