use super::{reader::*, *};
use crate::error::{HaoError, Result};

pub(crate) trait BodyEncoder {
    fn member_token(&mut self, token: &MemberToken) -> Result<u32>;
    fn user_string(&mut self, value: &str) -> Result<u32>;
    fn method_sig_token(&mut self, sig: &MethodSig) -> Result<u32>;
    fn locals_token(&mut self, locals: &[TypeSig]) -> Result<u32>;
}

/// Encodes a method body including its header and exception clauses.
/// `returns_value` is used for the `ret` stack effect when computing max stack.
pub(crate) fn encode_body(
    body: &MethodBody,
    returns_value: bool,
    encoder: &mut impl BodyEncoder,
) -> Result<Vec<u8>> {
    let instructions = fix_operand_forms(&body.instructions)?;
    let offsets = instruction_offsets(&instructions);
    let code_size = *offsets.last().unwrap() as usize;
    let offset_of = |label: Label| -> Result<u32> {
        offsets
            .get(label.0)
            .copied()
            .ok_or(HaoError::WriteError(format!(
                "label {} is out of range",
                label.0
            )))
    };

    let mut code = Vec::with_capacity(code_size);
    for (index, instruction) in instructions.iter().enumerate() {
        let opcode = instruction.opcode;
        if opcode.is_two_byte() {
            code.push(0xFE);
        }
        code.push(opcode.value() as u8);
        let next = offsets[index + 1] as i64;

        match (opcode.operand_type(), &instruction.operand) {
            (OperandType::None, Operand::None) => {}
            (OperandType::ShortI, Operand::Int32(v)) => code.push(*v as u8),
            (OperandType::I, Operand::Int32(v)) => code.extend_from_slice(&v.to_le_bytes()),
            (OperandType::I8, Operand::Int64(v)) => code.extend_from_slice(&v.to_le_bytes()),
            (OperandType::ShortR, Operand::Float32(v)) => code.extend_from_slice(&v.to_le_bytes()),
            (OperandType::R, Operand::Float64(v)) => code.extend_from_slice(&v.to_le_bytes()),
            (OperandType::ShortBranch, Operand::Target(t)) => {
                code.push((offset_of(*t)? as i64 - next) as i8 as u8)
            }
            (OperandType::Branch, Operand::Target(t)) => {
                code.extend_from_slice(&((offset_of(*t)? as i64 - next) as i32).to_le_bytes())
            }
            (OperandType::Switch, Operand::Switch(targets)) => {
                code.extend_from_slice(&(targets.len() as u32).to_le_bytes());
                for t in targets {
                    code.extend_from_slice(&((offset_of(*t)? as i64 - next) as i32).to_le_bytes());
                }
            }
            (OperandType::ShortVar, Operand::Local(i) | Operand::Arg(i)) => code.push(*i as u8),
            (OperandType::Var, Operand::Local(i) | Operand::Arg(i)) => {
                code.extend_from_slice(&i.to_le_bytes())
            }
            (
                OperandType::Method | OperandType::Field | OperandType::Type | OperandType::Tok,
                Operand::Token(token),
            ) => code.extend_from_slice(&encoder.member_token(token)?.to_le_bytes()),
            (OperandType::String, Operand::String(s)) => {
                code.extend_from_slice(&encoder.user_string(s)?.to_le_bytes())
            }
            (OperandType::Sig, Operand::Sig(sig)) => {
                code.extend_from_slice(&encoder.method_sig_token(sig)?.to_le_bytes())
            }
            (_, operand) => {
                return Err(HaoError::WriteError(format!(
                    "invalid operand {:?} for {}",
                    operand, opcode
                )))
            }
        }
    }
    debug_assert_eq!(code.len(), code_size);

    let max_stack = compute_max_stack(&instructions, &body.exception_handlers, returns_value)?
        .max(body.max_stack);

    let mut out = Vec::new();
    let tiny = code_size < 64
        && max_stack <= 8
        && body.locals.is_empty()
        && body.exception_handlers.is_empty()
        && !body.init_locals;

    if tiny {
        out.push(((code_size as u8) << 2) | TINY_FORMAT);
        out.extend_from_slice(&code);
        return Ok(out);
    }

    let locals_token = if body.locals.is_empty() {
        0
    } else {
        encoder.locals_token(&body.locals)?
    };
    let mut flags = FAT_FORMAT | (3 << 12);
    if body.init_locals {
        flags |= INIT_LOCALS;
    }
    if !body.exception_handlers.is_empty() {
        flags |= MORE_SECTS;
    }
    out.extend_from_slice(&flags.to_le_bytes());
    out.extend_from_slice(&max_stack.to_le_bytes());
    out.extend_from_slice(&(code_size as u32).to_le_bytes());
    out.extend_from_slice(&locals_token.to_le_bytes());
    out.extend_from_slice(&code);

    if !body.exception_handlers.is_empty() {
        while out.len() % 4 != 0 {
            out.push(0);
        }
        write_exception_handlers(&mut out, &body.exception_handlers, &offset_of, encoder)?;
    }
    Ok(out)
}

struct Clause {
    flags: u32,
    try_offset: u32,
    try_length: u32,
    handler_offset: u32,
    handler_length: u32,
    class_or_filter: u32,
}

fn write_exception_handlers(
    out: &mut Vec<u8>,
    handlers: &[ExceptionHandler],
    offset_of: &impl Fn(Label) -> Result<u32>,
    encoder: &mut impl BodyEncoder,
) -> Result<()> {
    let clauses = handlers
        .iter()
        .map(|h| {
            let (flags, class_or_filter) = match &h.kind {
                HandlerKind::Catch(ty) => (0, encoder.member_token(&ty.clone().into())?),
                HandlerKind::Filter(start) => (1, offset_of(*start)?),
                HandlerKind::Finally => (2, 0),
                HandlerKind::Fault => (4, 0),
            };
            let try_offset = offset_of(h.try_start)?;
            let handler_offset = offset_of(h.handler_start)?;
            Ok(Clause {
                flags,
                try_offset,
                try_length: offset_of(h.try_end)? - try_offset,
                handler_offset,
                handler_length: offset_of(h.handler_end)? - handler_offset,
                class_or_filter,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let small = clauses.len() * 12 + 4 <= 0xFF
        && clauses.iter().all(|c| {
            c.try_offset <= 0xFFFF
                && c.handler_offset <= 0xFFFF
                && c.try_length <= 0xFF
                && c.handler_length <= 0xFF
        });

    if small {
        out.push(SECT_EH_TABLE);
        out.push((clauses.len() * 12 + 4) as u8);
        out.extend_from_slice(&[0, 0]);
        for c in clauses {
            out.extend_from_slice(&(c.flags as u16).to_le_bytes());
            out.extend_from_slice(&(c.try_offset as u16).to_le_bytes());
            out.push(c.try_length as u8);
            out.extend_from_slice(&(c.handler_offset as u16).to_le_bytes());
            out.push(c.handler_length as u8);
            out.extend_from_slice(&c.class_or_filter.to_le_bytes());
        }
    } else {
        let size = (clauses.len() * 24 + 4) as u32;
        out.push(SECT_EH_TABLE | SECT_FAT_FORMAT);
        out.extend_from_slice(&size.to_le_bytes()[..3]);
        for c in clauses {
            for v in [
                c.flags,
                c.try_offset,
                c.try_length,
                c.handler_offset,
                c.handler_length,
                c.class_or_filter,
            ] {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
    }
    Ok(())
}

fn long_var_form(opcode: OpCode) -> Option<OpCode> {
    Some(match opcode {
        OpCode::Ldarg_S => OpCode::Ldarg,
        OpCode::Ldarga_S => OpCode::Ldarga,
        OpCode::Starg_S => OpCode::Starg,
        OpCode::Ldloc_S => OpCode::Ldloc,
        OpCode::Ldloca_S => OpCode::Ldloca,
        OpCode::Stloc_S => OpCode::Stloc,
        _ => return None,
    })
}

/// Promotes short forms whose operands no longer fit (branch offsets, variable
/// indices, `ldc.i4.s` values).
fn fix_operand_forms(instructions: &[Instruction]) -> Result<Vec<Instruction>> {
    let mut instructions = instructions.to_vec();
    for instruction in &mut instructions {
        match (&instruction.operand, instruction.opcode) {
            (Operand::Local(i) | Operand::Arg(i), op) if *i > 0xFF => {
                if let Some(long) = long_var_form(op) {
                    instruction.opcode = long;
                }
            }
            (Operand::Int32(v), OpCode::Ldc_I4_S) if i8::try_from(*v).is_err() => {
                instruction.opcode = OpCode::Ldc_I4;
            }
            _ => {}
        }
    }

    loop {
        let offsets = instruction_offsets(&instructions);
        let mut changed = false;
        for (index, instruction) in instructions.iter_mut().enumerate() {
            let Operand::Target(target) = instruction.operand else {
                continue;
            };
            let Some(long) = instruction.opcode.long_branch() else {
                continue;
            };
            let target = *offsets.get(target.0).ok_or(HaoError::WriteError(format!(
                "label {} is out of range",
                target.0
            )))? as i64;
            let delta = target - offsets[index + 1] as i64;
            if i8::try_from(delta).is_err() {
                instruction.opcode = long;
                changed = true;
            }
        }
        if !changed {
            return Ok(instructions);
        }
    }
}

fn stack_effect(instruction: &Instruction, returns_value: bool) -> (i32, i32) {
    let behaviour = instruction.opcode.stack_behaviour();
    let sig = match &instruction.operand {
        Operand::Token(t) => t.method_sig(),
        Operand::Sig(sig) => Some(sig.clone()),
        _ => None,
    };
    let param_count = |sig: &MethodSig| (sig.params.len() + sig.varargs.len()) as i32;

    let pop = match (behaviour.pop, instruction.opcode, &sig) {
        (Some(n), _, _) => n as i32,
        (None, OpCode::Ret, _) => returns_value as i32,
        (None, OpCode::Newobj, Some(sig)) => param_count(sig),
        (None, OpCode::Calli, Some(sig)) => param_count(sig) + sig.has_this() as i32 + 1,
        (None, _, Some(sig)) => param_count(sig) + sig.has_this() as i32,
        (None, _, None) => 0,
    };
    let push = match (behaviour.push, &sig) {
        (Some(n), _) => n as i32,
        (None, Some(sig)) => !sig.ret.is_void() as i32,
        (None, None) => 0,
    };
    (pop, push)
}

/// The maximum evaluation stack depth reached by `instructions`.
pub fn compute_max_stack(
    instructions: &[Instruction],
    handlers: &[ExceptionHandler],
    returns_value: bool,
) -> Result<u16> {
    let mut heights: Vec<Option<i32>> = vec![None; instructions.len() + 1];
    let mut work = vec![(0usize, 0i32)];
    for h in handlers {
        let start_height = match h.kind {
            HandlerKind::Catch(_) | HandlerKind::Filter(_) => 1,
            _ => 0,
        };
        work.push((h.handler_start.0, start_height));
        if let HandlerKind::Filter(f) = h.kind {
            work.push((f.0, 1));
        }
    }

    let mut max = 0;
    while let Some((mut index, mut height)) = work.pop() {
        loop {
            if index >= instructions.len() {
                break;
            }
            if heights[index].is_some() {
                break;
            }
            heights[index] = Some(height);
            let instruction = &instructions[index];
            let (pop, push) = stack_effect(instruction, returns_value);
            height = (height - pop).max(0) + push;
            max = max.max(height);

            match instruction.opcode.flow_control() {
                FlowControl::Branch => {
                    let Operand::Target(t) = instruction.operand else {
                        break;
                    };
                    if matches!(instruction.opcode, OpCode::Leave | OpCode::Leave_S) {
                        height = 0;
                    }
                    index = t.0;
                    continue;
                }
                FlowControl::CondBranch => match &instruction.operand {
                    Operand::Target(t) => work.push((t.0, height)),
                    Operand::Switch(ts) => work.extend(ts.iter().map(|t| (t.0, height))),
                    _ => {}
                },
                FlowControl::Return | FlowControl::Throw => break,
                _ => {}
            }
            index += 1;
        }
    }
    u16::try_from(max).map_err(|_| HaoError::WriteError("max stack overflow".into()))
}
