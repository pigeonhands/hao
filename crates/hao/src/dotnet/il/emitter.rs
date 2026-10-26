use super::*;

/// Builds a method body with forward branch labels and try blocks.
///
/// ```
/// use hao::dotnet::il::{Emitter, OpCode};
///
/// let mut e = Emitter::new();
/// let done = e.label();
/// e.ldarg(0).branch(OpCode::Brtrue, done);
/// e.ldc_i4(1).pop();
/// e.mark(done);
/// e.ret();
/// let body = e.finish();
/// assert_eq!(body.instructions.len(), 5);
/// ```
#[derive(Debug, Default)]
pub struct Emitter {
    body: MethodBody,
    labels: Vec<Option<usize>>,
    pending: Vec<(usize, PendingTarget)>,
    handlers: Vec<PendingHandler>,
}

/// A position in the code, usable before it is marked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmitLabel(usize);

#[derive(Debug)]
enum PendingTarget {
    Single(EmitLabel),
    Switch(Vec<EmitLabel>),
}

#[derive(Debug)]
struct PendingHandler {
    kind: Option<TypeDefOrRef>,
    try_start: EmitLabel,
    try_end: EmitLabel,
    handler_start: EmitLabel,
    handler_end: EmitLabel,
}

impl Emitter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn label(&mut self) -> EmitLabel {
        self.labels.push(None);
        EmitLabel(self.labels.len() - 1)
    }

    /// Points `label` at the next emitted instruction.
    pub fn mark(&mut self, label: EmitLabel) {
        self.labels[label.0] = Some(self.body.instructions.len());
    }

    pub fn emit(&mut self, instruction: Instruction) -> &mut Self {
        self.body.instructions.push(instruction);
        self
    }

    pub fn op(&mut self, opcode: OpCode) -> &mut Self {
        self.emit(Instruction::simple(opcode))
    }

    pub fn ldarg(&mut self, index: u16) -> &mut Self {
        self.emit(Instruction::ldarg(index))
    }

    pub fn starg(&mut self, index: u16) -> &mut Self {
        let opcode = if index <= u8::MAX as u16 {
            OpCode::Starg_S
        } else {
            OpCode::Starg
        };
        self.emit(Instruction::new(opcode, Operand::Arg(index)))
    }

    pub fn ldloc(&mut self, index: u16) -> &mut Self {
        self.emit(Instruction::ldloc(index))
    }

    pub fn stloc(&mut self, index: u16) -> &mut Self {
        self.emit(Instruction::stloc(index))
    }

    /// The address of a local, e.g. to call a method on a struct.
    pub fn ldloca(&mut self, index: u16) -> &mut Self {
        let opcode = if index <= u8::MAX as u16 {
            OpCode::Ldloca_S
        } else {
            OpCode::Ldloca
        };
        self.emit(Instruction::new(opcode, Operand::Local(index)))
    }

    pub fn ldc_i4(&mut self, value: i32) -> &mut Self {
        self.emit(Instruction::ldc_i4(value))
    }

    pub fn ldstr(&mut self, value: impl Into<String>) -> &mut Self {
        self.emit(Instruction::ldstr(value))
    }

    pub fn ldnull(&mut self) -> &mut Self {
        self.op(OpCode::Ldnull)
    }

    pub fn dup(&mut self) -> &mut Self {
        self.op(OpCode::Dup)
    }

    pub fn pop(&mut self) -> &mut Self {
        self.op(OpCode::Pop)
    }

    pub fn call(&mut self, method: impl Into<MemberToken>) -> &mut Self {
        self.emit(Instruction::call(method))
    }

    pub fn callvirt(&mut self, method: impl Into<MemberToken>) -> &mut Self {
        self.emit(Instruction::callvirt(method))
    }

    pub fn newobj(&mut self, constructor: impl Into<MemberToken>) -> &mut Self {
        self.emit(Instruction::newobj(constructor))
    }

    pub fn castclass(&mut self, ty: impl Into<MemberToken>) -> &mut Self {
        self.emit(Instruction::new(
            OpCode::Castclass,
            Operand::Token(ty.into()),
        ))
    }

    pub fn ret(&mut self) -> &mut Self {
        self.emit(Instruction::ret())
    }

    /// Emits `try { body } catch (catch_type) { handler }`. The exception is on
    /// the stack when `handler` starts, and both blocks continue after the catch.
    pub fn try_catch(
        &mut self,
        catch_type: TypeDefOrRef,
        body: impl FnOnce(&mut Self),
        handler: impl FnOnce(&mut Self),
    ) -> &mut Self {
        let (try_start, handler_start, end) = (self.label(), self.label(), self.label());
        self.mark(try_start);
        body(self);
        self.branch(OpCode::Leave, end);
        self.mark(handler_start);
        handler(self);
        self.branch(OpCode::Leave, end);
        self.mark(end);
        self.exception_handler(
            Some(catch_type),
            try_start,
            handler_start,
            handler_start,
            end,
        );
        self
    }

    pub fn branch(&mut self, opcode: OpCode, target: EmitLabel) -> &mut Self {
        self.pending
            .push((self.body.instructions.len(), PendingTarget::Single(target)));
        self.emit(Instruction::new(opcode, Operand::Target(Label(0))))
    }

    pub fn switch(&mut self, targets: Vec<EmitLabel>) -> &mut Self {
        self.pending
            .push((self.body.instructions.len(), PendingTarget::Switch(targets)));
        self.emit(Instruction::new(
            OpCode::Switch,
            Operand::Switch(Vec::new()),
        ))
    }

    /// Adds a local variable, returning its index.
    pub fn declare_local(&mut self, ty: TypeSig) -> u16 {
        self.body.locals.push(ty);
        self.body.init_locals = true;
        (self.body.locals.len() - 1) as u16
    }

    /// Records a `try`/`catch` region. `catch_type` of `None` makes it a `finally`.
    pub fn exception_handler(
        &mut self,
        catch_type: Option<TypeDefOrRef>,
        try_start: EmitLabel,
        try_end: EmitLabel,
        handler_start: EmitLabel,
        handler_end: EmitLabel,
    ) {
        self.handlers.push(PendingHandler {
            kind: catch_type,
            try_start,
            try_end,
            handler_start,
            handler_end,
        });
    }

    /// Resolves labels. Panics if a used label was never marked.
    pub fn finish(mut self) -> MethodBody {
        let labels = self.labels;
        let resolve = |l: EmitLabel| Label(labels[l.0].expect("label used but never marked"));
        for (index, target) in self.pending {
            self.body.instructions[index].operand = match target {
                PendingTarget::Single(l) => Operand::Target(resolve(l)),
                PendingTarget::Switch(ls) => Operand::Switch(ls.into_iter().map(resolve).collect()),
            };
        }
        for h in self.handlers {
            self.body.exception_handlers.push(ExceptionHandler {
                kind: match h.kind {
                    Some(ty) => HandlerKind::Catch(ty),
                    None => HandlerKind::Finally,
                },
                try_start: resolve(h.try_start),
                try_end: resolve(h.try_end),
                handler_start: resolve(h.handler_start),
                handler_end: resolve(h.handler_end),
            });
        }
        self.body
    }
}
