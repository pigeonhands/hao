//! CIL method bodies (ECMA-335 II.25.4, III).

mod opcodes;
mod reader;

pub use opcodes::*;
pub(crate) use reader::{BodyReader, BodyResolver};

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

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MethodBody {
    pub max_stack: u16,
    pub init_locals: bool,
    pub locals: Vec<TypeSig>,
    pub instructions: Vec<Instruction>,
    pub exception_handlers: Vec<ExceptionHandler>,
}
