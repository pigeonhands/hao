//! Reads an assembly, patches its IL with hao, and loads the patched copy into
//! CoreCLR without writing a file. The original is loaded too, to compare.
//!
//! ```sh
//! cargo run -p hao-runtime --example patch_in_memory
//! ```
//!
//! The assembly here is built by `common::game_assembly`; bytes read from a
//! file work the same way.

mod common;

use hao::{
    dotnet::{
        csharp::WriteCSharp,
        il::{Instruction, OpCode, Operand},
    },
    Module,
};
use hao_runtime::Runtime;

type Hit = extern "C" fn(i32, i32) -> i32;
type GetScore = extern "C" fn(i32) -> i32;

fn entry_points(runtime: &Runtime, assembly: &str) -> hao_runtime::Result<(Hit, GetScore)> {
    unsafe {
        Ok((
            std::mem::transmute::<*const std::ffi::c_void, Hit>(runtime.function_pointer(
                assembly,
                "Game.Player",
                "Hit",
            )?),
            std::mem::transmute::<*const std::ffi::c_void, GetScore>(runtime.function_pointer(
                assembly,
                "Game.Player",
                "GetScore",
            )?),
        ))
    }
}

fn main() -> hao_runtime::Result<()> {
    let bytes = common::game_assembly();
    let runtime = Runtime::coreclr()?;
    runtime.load_assembly_bytes("Game", &bytes)?;
    let (hit, score) = entry_points(&runtime, "Game")?;
    println!(
        "original: Hit(100, 30) = {}, GetScore(3) = {}",
        hit(100, 30),
        score(3)
    );

    let module = Module::from_bytes(&bytes).expect("a valid assembly");

    // Halve all damage: `amount /= 2;` at the start of TakeDamage.
    let take_damage = module.find_method("Game.Player", "TakeDamage").unwrap();
    take_damage.edit_body(|body| {
        body.insert(
            0,
            [
                Instruction::ldarg(1),
                Instruction::ldc_i4(2),
                Instruction::simple(OpCode::Div),
                Instruction::new(OpCode::Starg_S, Operand::Arg(1)),
            ],
        )
    });
    println!(
        "\npatched TakeDamage:\n{}",
        take_damage.get().body.as_ref().unwrap().csharp()
    );

    // Ten times the points per kill.
    let score_method = module.find_method("Game.Player", "Score").unwrap();
    score_method.edit_body(|body| {
        let index = body.position(|i| i.operand == Operand::Int32(100)).unwrap();
        body.instructions[index] = Instruction::ldc_i4(1000);
    });
    println!(
        "patched Score:\n{}",
        score_method.get().body.as_ref().unwrap().csharp()
    );

    // A different name, since the original is still loaded.
    common::rename(&module, "Game.Patched");
    runtime.load_module(&module)?;
    let (hit, score) = entry_points(&runtime, "Game.Patched")?;
    println!(
        "patched:  Hit(100, 30) = {}, GetScore(3) = {}",
        hit(100, 30),
        score(3)
    );
    Ok(())
}
