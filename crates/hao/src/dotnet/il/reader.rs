use std::collections::HashMap;

use super::*;
use crate::{
    dotnet::signature::StandAloneSig,
    error::{HaoError, Result},
};

pub(crate) trait BodyResolver {
    fn member_token(&self, token: u32) -> Result<MemberToken>;
    fn user_string(&self, token: u32) -> Result<String>;
    fn stand_alone_sig(&self, token: u32) -> Result<StandAloneSig>;
}

pub(crate) const TINY_FORMAT: u8 = 0x2;
pub(crate) const FAT_FORMAT: u16 = 0x3;
pub(crate) const MORE_SECTS: u16 = 0x8;
pub(crate) const INIT_LOCALS: u16 = 0x10;
pub(crate) const SECT_EH_TABLE: u8 = 0x1;
pub(crate) const SECT_FAT_FORMAT: u8 = 0x40;
pub(crate) const SECT_MORE_SECTS: u8 = 0x80;

pub(crate) struct BodyReader<'a, R: BodyResolver> {
    data: &'a [u8],
    position: usize,
    resolver: &'a R,
}

enum RawOperand {
    Resolved(Operand),
    Target(i64),
    Switch(Vec<i64>),
}

impl<'a, R: BodyResolver> BodyReader<'a, R> {
    pub fn new(data: &'a [u8], resolver: &'a R) -> Self {
        Self {
            data,
            position: 0,
            resolver,
        }
    }

    fn error(&self, msg: &'static str) -> HaoError {
        HaoError::InvalidMethodBody(msg, self.position)
    }

    fn bytes<const N: usize>(&mut self) -> Result<[u8; N]> {
        let slice = self
            .data
            .get(self.position..self.position + N)
            .ok_or_else(|| self.error("unexpected end of method body"))?;
        self.position += N;
        Ok(slice.try_into().unwrap())
    }

    fn u8(&mut self) -> Result<u8> {
        self.bytes::<1>().map(|b| b[0])
    }

    fn u16(&mut self) -> Result<u16> {
        self.bytes().map(u16::from_le_bytes)
    }

    fn u32(&mut self) -> Result<u32> {
        self.bytes().map(u32::from_le_bytes)
    }

    pub fn read(mut self) -> Result<MethodBody> {
        let first = self.u8()?;
        let (max_stack, code_size, flags, locals_token) = if first & 0x3 == TINY_FORMAT {
            (8, (first >> 2) as usize, 0u16, 0u32)
        } else {
            self.position = 0;
            let flags_and_size = self.u16()?;
            if flags_and_size & 0x3 != FAT_FORMAT {
                return Err(self.error("unknown method header format"));
            }
            let header_size = (flags_and_size >> 12) as usize * 4;
            let max_stack = self.u16()?;
            let code_size = self.u32()? as usize;
            let locals_token = self.u32()?;
            self.position = header_size.max(12);
            (max_stack, code_size, flags_and_size & 0x0FFF, locals_token)
        };

        let locals = if locals_token != 0 {
            match self.resolver.stand_alone_sig(locals_token)? {
                StandAloneSig::Locals(locals) => locals,
                StandAloneSig::Method(_) => {
                    return Err(self.error("locals token is not a local signature"))
                }
            }
        } else {
            Vec::new()
        };

        let code_start = self.position;
        let code_end = code_start + code_size;
        if code_end > self.data.len() {
            return Err(self.error("code size is larger than the available data"));
        }

        let mut offsets = Vec::new();
        let mut raw = Vec::new();
        while self.position < code_end {
            offsets.push((self.position - code_start) as u32);
            raw.push(self.read_instruction(code_start)?);
        }
        if self.position != code_end {
            return Err(self.error("instruction overruns the method body"));
        }

        let mut index_of: HashMap<u32, usize> =
            offsets.iter().enumerate().map(|(i, o)| (*o, i)).collect();
        index_of.insert(code_size as u32, offsets.len());

        let label = |offset: i64| -> Result<Label> {
            u32::try_from(offset)
                .ok()
                .and_then(|o| index_of.get(&o))
                .map(|i| Label(*i))
                .ok_or(HaoError::InvalidMethodBody(
                    "branch target is not on an instruction boundary",
                    offset as usize,
                ))
        };

        let instructions = raw
            .into_iter()
            .map(|(opcode, operand)| {
                let operand = match operand {
                    RawOperand::Resolved(op) => op,
                    RawOperand::Target(t) => Operand::Target(label(t)?),
                    RawOperand::Switch(ts) => {
                        Operand::Switch(ts.into_iter().map(label).collect::<Result<_>>()?)
                    }
                };
                Ok(Instruction { opcode, operand })
            })
            .collect::<Result<Vec<_>>>()?;

        let mut exception_handlers = Vec::new();
        if flags & MORE_SECTS != 0 {
            self.position = (code_end + 3) & !3;
            loop {
                let kind = self.u8()?;
                if kind & SECT_EH_TABLE == 0 {
                    return Err(self.error("unsupported method data section"));
                }
                let fat = kind & SECT_FAT_FORMAT != 0;
                let data_size = if fat {
                    let b = self.bytes::<3>()?;
                    u32::from_le_bytes([b[0], b[1], b[2], 0]) as usize
                } else {
                    let size = self.u8()? as usize;
                    self.u16()?;
                    size
                };
                let clause_size = if fat { 24 } else { 12 };
                let count = data_size.saturating_sub(4) / clause_size;
                for _ in 0..count {
                    let (flags, try_offset, try_length, handler_offset, handler_length) = if fat {
                        (
                            self.u32()?,
                            self.u32()?,
                            self.u32()?,
                            self.u32()?,
                            self.u32()?,
                        )
                    } else {
                        (
                            self.u16()? as u32,
                            self.u16()? as u32,
                            self.u8()? as u32,
                            self.u16()? as u32,
                            self.u8()? as u32,
                        )
                    };
                    let class_or_filter = self.u32()?;
                    let kind = match flags {
                        0 => HandlerKind::Catch(
                            self.resolver
                                .member_token(class_or_filter)?
                                .as_type()
                                .ok_or_else(|| self.error("catch type is not a type token"))?,
                        ),
                        1 => HandlerKind::Filter(label(class_or_filter as i64)?),
                        2 => HandlerKind::Finally,
                        4 => HandlerKind::Fault,
                        _ => return Err(self.error("unknown exception clause kind")),
                    };
                    exception_handlers.push(ExceptionHandler {
                        kind,
                        try_start: label(try_offset as i64)?,
                        try_end: label((try_offset + try_length) as i64)?,
                        handler_start: label(handler_offset as i64)?,
                        handler_end: label((handler_offset + handler_length) as i64)?,
                    });
                }
                if kind & SECT_MORE_SECTS == 0 {
                    break;
                }
                self.position = (self.position + 3) & !3;
            }
        }

        Ok(MethodBody {
            max_stack,
            init_locals: flags & INIT_LOCALS != 0,
            locals,
            instructions,
            exception_handlers,
        })
    }

    fn read_instruction(&mut self, code_start: usize) -> Result<(OpCode, RawOperand)> {
        let first = self.u8()? as u16;
        let value = if first == 0xFE {
            0xFE00 | self.u8()? as u16
        } else {
            first
        };
        let opcode =
            OpCode::from_repr(value).ok_or(HaoError::InvalidOpCode(value, self.position))?;

        let relative = |this: &Self, delta: i64| (this.position - code_start) as i64 + delta;

        let operand = match opcode.operand_type() {
            OperandType::None => RawOperand::Resolved(Operand::None),
            OperandType::ShortI => {
                let b = self.u8()?;
                let v = if opcode == OpCode::Ldc_I4_S {
                    b as i8 as i32
                } else {
                    b as i32
                };
                RawOperand::Resolved(Operand::Int32(v))
            }
            OperandType::I => RawOperand::Resolved(Operand::Int32(self.u32()? as i32)),
            OperandType::I8 => {
                RawOperand::Resolved(Operand::Int64(i64::from_le_bytes(self.bytes()?)))
            }
            OperandType::ShortR => {
                RawOperand::Resolved(Operand::Float32(f32::from_le_bytes(self.bytes()?)))
            }
            OperandType::R => {
                RawOperand::Resolved(Operand::Float64(f64::from_le_bytes(self.bytes()?)))
            }
            OperandType::ShortBranch => {
                let delta = self.u8()? as i8 as i64;
                RawOperand::Target(relative(self, delta))
            }
            OperandType::Branch => {
                let delta = self.u32()? as i32 as i64;
                RawOperand::Target(relative(self, delta))
            }
            OperandType::Switch => {
                let count = self.u32()? as usize;
                if count > (self.data.len() - self.position) / 4 {
                    return Err(self.error("switch has more targets than data"));
                }
                let deltas = (0..count)
                    .map(|_| self.u32().map(|d| d as i32 as i64))
                    .collect::<Result<Vec<_>>>()?;
                let base = (self.position - code_start) as i64;
                RawOperand::Switch(deltas.into_iter().map(|d| base + d).collect())
            }
            OperandType::ShortVar => {
                let index = self.u8()? as u16;
                RawOperand::Resolved(var_operand(opcode, index))
            }
            OperandType::Var => {
                let index = self.u16()?;
                RawOperand::Resolved(var_operand(opcode, index))
            }
            OperandType::Method | OperandType::Field | OperandType::Type | OperandType::Tok => {
                let token = self.u32()?;
                RawOperand::Resolved(Operand::Token(self.resolver.member_token(token)?))
            }
            OperandType::String => {
                let token = self.u32()?;
                RawOperand::Resolved(Operand::String(self.resolver.user_string(token)?))
            }
            OperandType::Sig => {
                let token = self.u32()?;
                match self.resolver.stand_alone_sig(token)? {
                    StandAloneSig::Method(sig) => RawOperand::Resolved(Operand::Sig(sig)),
                    StandAloneSig::Locals(_) => {
                        return Err(self.error("calli with a local signature"))
                    }
                }
            }
        };
        Ok((opcode, operand))
    }
}

fn var_operand(opcode: OpCode, index: u16) -> Operand {
    match opcode {
        OpCode::Ldarg_S
        | OpCode::Ldarga_S
        | OpCode::Starg_S
        | OpCode::Ldarg
        | OpCode::Ldarga
        | OpCode::Starg => Operand::Arg(index),
        _ => Operand::Local(index),
    }
}
