//! Reads an assembly, patches its IL with hao, and loads the patched copy into
//! CoreCLR without writing a file. The original is loaded too, to compare.
//!
//! ```sh
//! cargo run -p hao-runtime --example patch_in_memory
//! ```

use hao::{
    dotnet::{
        csharp::WriteCSharp,
        entries::{Method, TypeDef},
        il::{Emitter, Instruction, OpCode, Operand},
        module::{ModuleKind, TargetRuntime},
        signature::{MethodSig, TypeSig},
    },
    Module,
};
use hao_runtime::Runtime;

/// The assembly being patched, standing in for one read from a file:
///
/// ```csharp
/// public static class Game.Player {
///     public static int TakeDamage(int health, int amount) => health - amount;
///     public static int Score(int kills) => kills * 100;
///     // Entry points for native code.
///     [UnmanagedCallersOnly] public static int Hit(int health, int amount) => TakeDamage(health, amount);
///     [UnmanagedCallersOnly] public static int GetScore(int kills) => Score(kills);
/// }
/// ```
fn game_assembly() -> hao::error::Result<Vec<u8>> {
    let mut module = Module::new(
        "Game",
        ModuleKind::Dll,
        TargetRuntime::NetCoreImplementation(8),
    );
    let player = module.add_static_class("Game", "Player")?;
    let uco =
        module.corlib_attribute("System.Runtime.InteropServices.UnmanagedCallersOnlyAttribute")?;
    let signature =
        |params: u16| MethodSig::new_static(TypeSig::INT32, vec![TypeSig::INT32; params as usize]);

    let add = |name, params, body| {
        TypeDef::add_method(
            &player,
            Method::public_static(name, signature(params)).with_body(body),
        )
    };
    let take_damage = add(
        "TakeDamage",
        2,
        Emitter::build(|e| {
            e.ldarg(0).ldarg(1).op(OpCode::Sub).ret();
        }),
    );
    let score = add(
        "Score",
        1,
        Emitter::build(|e| {
            e.ldarg(0).ldc_i4(100).op(OpCode::Mul).ret();
        }),
    );
    for (name, params, target) in [("Hit", 2, take_damage), ("GetScore", 1, score)] {
        let body = Emitter::build(|e| {
            (0..params).for_each(|i| {
                e.ldarg(i);
            });
            e.call(target).ret();
        });
        let entry = Method::public_static(name, signature(params))
            .with_attribute(uco.clone())
            .with_body(body);
        TypeDef::add_method(&player, entry);
    }
    module.write()
}

/// Calls `Hit(100, 30)` and `GetScore(3)` in the loaded assembly `assembly`.
fn play(runtime: &Runtime, assembly: &str) -> hao_runtime::Result<(i32, i32)> {
    let hit = runtime.function_pointer(assembly, "Game.Player", "Hit")?;
    let get_score = runtime.function_pointer(assembly, "Game.Player", "GetScore")?;
    let hit: extern "C" fn(i32, i32) -> i32 = unsafe { std::mem::transmute(hit) };
    let get_score: extern "C" fn(i32) -> i32 = unsafe { std::mem::transmute(get_score) };
    Ok((hit(100, 30), get_score(3)))
}

fn main() -> hao_runtime::Result<()> {
    let bytes = game_assembly().expect("a valid assembly");
    let runtime = Runtime::coreclr()?;
    runtime.load_assembly_bytes("Game", &bytes)?;
    let (health, score) = play(&runtime, "Game")?;
    println!(
        "original: Hit(100, 30) = {}, GetScore(3) = {}",
        health, score
    );

    let mut module = Module::from_bytes(&bytes).expect("a valid assembly");

    // Halve all damage: `amount /= 2;` at the start of TakeDamage.
    let take_damage = module.find_method("Game.Player", "TakeDamage").unwrap();
    take_damage.edit_body(|body| {
        body.prefix(
            Emitter::build(|e| {
                e.ldarg(1).ldc_i4(2).op(OpCode::Div).starg(1);
            })
            .instructions,
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
    module.set_assembly_name("Game.Patched");
    runtime.load_module(&module)?;
    let (health, score) = play(&runtime, "Game.Patched")?;
    println!(
        "patched:  Hit(100, 30) = {}, GetScore(3) = {}",
        health, score
    );
    Ok(())
}
