use std::fmt::{self, Write};

use super::{CSharpWriter, ToCSharpWith, WriteCSharp};
use crate::dotnet::{
    entries::MemberRefParent,
    il::{instruction_offsets, MemberToken, MethodBody, Operand},
};

/// Lists the instructions as `// IL_0000: opcode operand` comment lines.
impl WriteCSharp for MethodBody {
    fn write_csharp<W: Write>(&self, w: &mut CSharpWriter<W>) -> fmt::Result {
        write_il(w, self)
    }
}

fn write_il<W: Write>(w: &mut CSharpWriter<W>, body: &MethodBody) -> fmt::Result {
    let offsets = instruction_offsets(&body.instructions);
    let label = |index: usize| format!("IL_{:04x}", offsets[index]);

    writeln!(w, "// maxstack {}", body.max_stack)?;
    for (i, local) in body.locals.iter().enumerate() {
        writeln!(w, "// local {}: {}", i, local.to_csharp_with(w))?;
    }
    for (index, instruction) in body.instructions.iter().enumerate() {
        write!(w, "// {}: {}", label(index), instruction.opcode)?;
        match &instruction.operand {
            Operand::None => {}
            Operand::Int32(v) => write!(w, " {}", v)?,
            Operand::Int64(v) => write!(w, " {}", v)?,
            Operand::Float32(v) => write!(w, " {}", v)?,
            Operand::Float64(v) => write!(w, " {}", v)?,
            Operand::Target(t) => write!(w, " {}", label(t.0))?,
            Operand::Switch(targets) => {
                let targets: Vec<String> = targets.iter().map(|t| label(t.0)).collect();
                write!(w, " ({})", targets.join(", "))?;
            }
            Operand::Local(i) => write!(w, " V_{}", i)?,
            Operand::Arg(i) => write!(w, " A_{}", i)?,
            Operand::String(s) => write!(w, " {:?}", s)?,
            Operand::Sig(sig) => write!(w, " {}", sig.to_csharp_with(w))?,
            Operand::Token(token) => {
                let name = token_name(w, token);
                write!(w, " {}", name)?
            }
        }
        w.newline()?;
    }
    for handler in &body.exception_handlers {
        writeln!(
            w,
            "// .try {} to {} {} handler {} to {}",
            label(handler.try_start.0),
            label(handler.try_end.0),
            handler.kind_name(),
            label(handler.handler_start.0),
            label(handler.handler_end.0),
        )?;
    }
    Ok(())
}

fn token_name<W: Write>(w: &CSharpWriter<W>, token: &MemberToken) -> String {
    match token {
        MemberToken::TypeDef(t) => t.get().full_name(),
        MemberToken::TypeRef(t) => t.get().full_name(),
        MemberToken::TypeSpec(t) => t.get().signature.to_csharp_with(w),
        MemberToken::Field(f) => qualified(&f.get().declaring_type, &f.get().name),
        MemberToken::Method(m) => qualified(&m.get().declaring_type, &m.get().name),
        MemberToken::MemberRef(m) => {
            let m = m.get();
            let parent = match &m.parent {
                MemberRefParent::ModuleRef(r) => r.get().name.clone(),
                MemberRefParent::Method(r) => r.get().name.clone(),
                MemberRefParent::TypeSpec(t) => t.get().signature.to_csharp_with(w),
                p => p.as_type().map(|t| t.full_name()).unwrap_or_default(),
            };
            format!("{}::{}", parent, m.name)
        }
        MemberToken::MethodSpec(m) => {
            let m = m.get();
            let args: Vec<String> = m.instantiation.iter().map(|a| a.to_csharp_with(w)).collect();
            let method = match &m.method {
                crate::dotnet::entries::MethodDefOrRef::Method(d) => token_name(w, &MemberToken::Method(d.clone())),
                crate::dotnet::entries::MethodDefOrRef::MemberRef(r) => token_name(w, &MemberToken::MemberRef(r.clone())),
            };
            format!("{}<{}>", method, args.join(", "))
        }
    }
}

fn qualified(
    declaring: &crate::dotnet::entries::WeakEntry<crate::dotnet::entries::TypeDef>,
    name: &str,
) -> String {
    match declaring.upgrade() {
        Some(t) => format!("{}::{}", t.get().full_name(), name),
        None => name.to_owned(),
    }
}
