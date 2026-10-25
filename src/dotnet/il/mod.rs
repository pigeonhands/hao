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

macro_rules! member_token_from {
    ($($variant:ident),*) => {$(
        impl From<Entry<$variant>> for MemberToken {
            fn from(value: Entry<$variant>) -> Self {
                Self::$variant(value)
            }
        }
    )*};
}

member_token_from!(TypeDef, TypeRef, TypeSpec, Field, Method, MemberRef, MethodSpec);

impl From<crate::dotnet::entries::MethodDefOrRef> for MemberToken {
    fn from(value: crate::dotnet::entries::MethodDefOrRef) -> Self {
        match value {
            crate::dotnet::entries::MethodDefOrRef::Method(m) => Self::Method(m),
            crate::dotnet::entries::MethodDefOrRef::MemberRef(m) => Self::MemberRef(m),
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

impl From<&str> for Operand {
    fn from(value: &str) -> Self {
        Self::String(value.into())
    }
}

impl From<String> for Operand {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<MemberToken> for Operand {
    fn from(value: MemberToken) -> Self {
        Self::Token(value)
    }
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

    pub fn pop() -> Self {
        Self::simple(OpCode::Pop)
    }

    pub fn ret() -> Self {
        Self::simple(OpCode::Ret)
    }

    pub fn ldstr(value: impl Into<String>) -> Self {
        Self::new(OpCode::Ldstr, Operand::String(value.into()))
    }

    pub fn call(method: impl Into<MemberToken>) -> Self {
        Self::new(OpCode::Call, Operand::Token(method.into()))
    }

    pub fn callvirt(method: impl Into<MemberToken>) -> Self {
        Self::new(OpCode::Callvirt, Operand::Token(method.into()))
    }

    pub fn newobj(constructor: impl Into<MemberToken>) -> Self {
        Self::new(OpCode::Newobj, Operand::Token(constructor.into()))
    }

    /// `ldc.i4` using the shortest encoding for `value`.
    pub fn ldc_i4(value: i32) -> Self {
        const SHORT: [OpCode; 10] = [
            OpCode::Ldc_I4_M1,
            OpCode::Ldc_I4_0,
            OpCode::Ldc_I4_1,
            OpCode::Ldc_I4_2,
            OpCode::Ldc_I4_3,
            OpCode::Ldc_I4_4,
            OpCode::Ldc_I4_5,
            OpCode::Ldc_I4_6,
            OpCode::Ldc_I4_7,
            OpCode::Ldc_I4_8,
        ];
        match value {
            -1..=8 => Self::simple(SHORT[(value + 1) as usize]),
            -128..=127 => Self::new(OpCode::Ldc_I4_S, Operand::Int32(value)),
            _ => Self::new(OpCode::Ldc_I4, Operand::Int32(value)),
        }
    }

    /// `ldarg` using the shortest encoding for `index`.
    pub fn ldarg(index: u16) -> Self {
        match index {
            0 => Self::simple(OpCode::Ldarg_0),
            1 => Self::simple(OpCode::Ldarg_1),
            2 => Self::simple(OpCode::Ldarg_2),
            3 => Self::simple(OpCode::Ldarg_3),
            4..=255 => Self::new(OpCode::Ldarg_S, Operand::Arg(index)),
            _ => Self::new(OpCode::Ldarg, Operand::Arg(index)),
        }
    }

    /// `ldloc` using the shortest encoding for `index`.
    pub fn ldloc(index: u16) -> Self {
        match index {
            0 => Self::simple(OpCode::Ldloc_0),
            1 => Self::simple(OpCode::Ldloc_1),
            2 => Self::simple(OpCode::Ldloc_2),
            3 => Self::simple(OpCode::Ldloc_3),
            4..=255 => Self::new(OpCode::Ldloc_S, Operand::Local(index)),
            _ => Self::new(OpCode::Ldloc, Operand::Local(index)),
        }
    }

    /// `stloc` using the shortest encoding for `index`.
    pub fn stloc(index: u16) -> Self {
        match index {
            0 => Self::simple(OpCode::Stloc_0),
            1 => Self::simple(OpCode::Stloc_1),
            2 => Self::simple(OpCode::Stloc_2),
            3 => Self::simple(OpCode::Stloc_3),
            4..=255 => Self::new(OpCode::Stloc_S, Operand::Local(index)),
            _ => Self::new(OpCode::Stloc, Operand::Local(index)),
        }
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

impl MethodBody {
    fn remap_labels(&mut self, f: impl Fn(usize) -> usize) {
        let f = |l: &mut Label| l.0 = f(l.0);
        for instruction in &mut self.instructions {
            match &mut instruction.operand {
                Operand::Target(t) => f(t),
                Operand::Switch(targets) => targets.iter_mut().for_each(f),
                _ => {}
            }
        }
        for h in &mut self.exception_handlers {
            f(&mut h.try_start);
            f(&mut h.try_end);
            f(&mut h.handler_start);
            f(&mut h.handler_end);
            if let HandlerKind::Filter(start) = &mut h.kind {
                f(start);
            }
        }
    }

    /// Inserts instructions before `index`. Branches and exception ranges that
    /// pointed at `index` or later keep pointing at the same instructions.
    pub fn insert(&mut self, index: usize, instructions: impl IntoIterator<Item = Instruction>) {
        let new: Vec<Instruction> = instructions.into_iter().collect();
        let count = new.len();
        self.remap_labels(|l| if l >= index { l + count } else { l });
        self.instructions.splice(index..index, new);
    }

    /// Removes the instruction at `index`. Branches to it now target the next instruction.
    pub fn remove(&mut self, index: usize) -> Instruction {
        self.remap_labels(|l| if l > index { l - 1 } else { l });
        self.instructions.remove(index)
    }

    /// Replaces every operand equal to `from`, returning how many were replaced.
    pub fn replace_operand(&mut self, from: &Operand, to: Operand) -> usize {
        let mut count = 0;
        for instruction in &mut self.instructions {
            if instruction.operand == *from {
                instruction.operand = to.clone();
                count += 1;
            }
        }
        count
    }

    /// Index of the first instruction matching `f`.
    pub fn position(&self, f: impl Fn(&Instruction) -> bool) -> Option<usize> {
        self.instructions.iter().position(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_remove_keep_branch_targets() {
        let mut body = MethodBody {
            instructions: vec![
                Instruction::simple(OpCode::Nop),
                Instruction::new(OpCode::Br_S, Operand::Target(Label(0))),
                Instruction::simple(OpCode::Ret),
            ],
            exception_handlers: vec![ExceptionHandler {
                kind: HandlerKind::Finally,
                try_start: Label(0),
                try_end: Label(2),
                handler_start: Label(2),
                handler_end: Label(3),
            }],
            ..Default::default()
        };
        body.insert(0, [Instruction::simple(OpCode::Ldnull), Instruction::simple(OpCode::Pop)]);
        assert_eq!(body.instructions[3].operand, Operand::Target(Label(2)));
        assert_eq!(body.exception_handlers[0].handler_end, Label(5));

        body.remove(2);
        assert_eq!(body.instructions[2].operand, Operand::Target(Label(2)));
        assert_eq!(body.exception_handlers[0].try_start, Label(2));
    }
}
