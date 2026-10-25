//! CIL method bodies (ECMA-335 II.25.4, III).

mod opcodes;
mod reader;
mod writer;

pub use opcodes::*;
pub(crate) use reader::{BodyReader, BodyResolver};
pub(crate) use writer::{encode_body, BodyEncoder};
pub use writer::compute_max_stack;

use crate::dotnet::{
    entries::{Entry, Field, MemberRef, Method, MethodSpec, TypeDef, TypeDefOrRef, TypeRef, TypeSpec},
    signature::{MethodSig, TypeSig},
};

/// Index of an instruction in [`MethodBody::instructions`].
/// Equal to `instructions.len()` for the end of the method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Label(pub usize);

/// A metadata token operand, resolved to its entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemberToken {
    TypeDef(Entry<TypeDef>),
    TypeRef(Entry<TypeRef>),
    TypeSpec(Entry<TypeSpec>),
    Field(Entry<Field>),
    Method(Entry<Method>),
    MemberRef(Entry<MemberRef>),
    MethodSpec(Entry<MethodSpec>),
}

impl From<TypeDefOrRef> for MemberToken {
    fn from(value: TypeDefOrRef) -> Self {
        match value {
            TypeDefOrRef::TypeDef(t) => Self::TypeDef(t),
            TypeDefOrRef::TypeRef(t) => Self::TypeRef(t),
            TypeDefOrRef::TypeSpec(t) => Self::TypeSpec(t),
        }
    }
}

impl MemberToken {
    pub fn as_type(&self) -> Option<TypeDefOrRef> {
        Some(match self {
            Self::TypeDef(t) => TypeDefOrRef::TypeDef(t.clone()),
            Self::TypeRef(t) => TypeDefOrRef::TypeRef(t.clone()),
            Self::TypeSpec(t) => TypeDefOrRef::TypeSpec(t.clone()),
            _ => return None,
        })
    }

    /// The signature of a method operand, used to work out stack usage.
    pub fn method_sig(&self) -> Option<MethodSig> {
        match self {
            Self::Method(m) => Some(m.get().signature.clone()),
            Self::MemberRef(m) => match &m.get().signature {
                crate::dotnet::signature::MemberRefSig::Method(sig) => Some(sig.clone()),
                _ => None,
            },
            Self::MethodSpec(m) => match &m.get().method {
                crate::dotnet::entries::MethodDefOrRef::Method(m) => {
                    Some(m.get().signature.clone())
                }
                crate::dotnet::entries::MethodDefOrRef::MemberRef(m) => {
                    match &m.get().signature {
                        crate::dotnet::signature::MemberRefSig::Method(sig) => Some(sig.clone()),
                        _ => None,
                    }
                }
            },
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    None,
    Int32(i32),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    Target(Label),
    Switch(Vec<Label>),
    Local(u16),
    Arg(u16),
    Token(MemberToken),
    String(String),
    Sig(MethodSig),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Instruction {
    pub opcode: OpCode,
    pub operand: Operand,
}

impl Instruction {
    pub fn new(opcode: OpCode, operand: Operand) -> Self {
        Self { opcode, operand }
    }

    pub fn simple(opcode: OpCode) -> Self {
        Self::new(opcode, Operand::None)
    }

    /// Encoded size in bytes.
    pub fn size(&self) -> usize {
        self.opcode.size()
            + match &self.operand {
                Operand::Switch(targets) => targets.len() * 4,
                _ => 0,
            }
    }
}

/// Byte offset of every instruction, plus one extra for the end of the code.
pub fn instruction_offsets(instructions: &[Instruction]) -> Vec<u32> {
    let mut offsets = Vec::with_capacity(instructions.len() + 1);
    let mut offset = 0;
    for instruction in instructions {
        offsets.push(offset);
        offset += instruction.size() as u32;
    }
    offsets.push(offset);
    offsets
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandlerKind {
    Catch(TypeDefOrRef),
    Filter(Label),
    Finally,
    Fault,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExceptionHandler {
    pub kind: HandlerKind,
    pub try_start: Label,
    pub try_end: Label,
    pub handler_start: Label,
    pub handler_end: Label,
}

impl ExceptionHandler {
    pub fn kind_name(&self) -> &'static str {
        match self.kind {
            HandlerKind::Catch(_) => "catch",
            HandlerKind::Filter(_) => "filter",
            HandlerKind::Finally => "finally",
            HandlerKind::Fault => "fault",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MethodBody {
    pub max_stack: u16,
    pub init_locals: bool,
    pub locals: Vec<TypeSig>,
    pub instructions: Vec<Instruction>,
    pub exception_handlers: Vec<ExceptionHandler>,
}
