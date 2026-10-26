use super::*;

/// Builds a method body with forward branch labels and try blocks.
///
/// ```
/// use hao::dotnet::il::{Emitter, Instruction, OpCode};
///
/// let mut e = Emitter::new();
/// let done = e.label();
/// e.emit(Instruction::ldarg(0));
/// e.branch(OpCode::Brtrue, done);
/// e.emit(Instruction::ldc_i4(1));
/// e.emit(Instruction::pop());
/// e.mark(done);
/// e.emit(Instruction::ret());
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
