//! Hooks managed methods with Rust functions, like Harmony prefixes and
//! postfixes: hao rewrites the methods' IL to call into Rust through a native
//! function pointer, then the hooked assembly is loaded into CoreCLR.
//!
//! ```sh
//! cargo run -p hao-runtime --example hook_managed
//! ```
//!
//! The hooks are added before the code is loaded, because the runtime compiles
//! each method once. To hook an assembly an app loads itself, edit its bytes
//! before they're loaded.

use std::sync::atomic::{AtomicU32, Ordering};

use hao::{
    dotnet::{
        entries::{Method, TypeDef},
        il::{Emitter, Instruction, OpCode},
        module::{ModuleKind, TargetRuntime},
        signature::{MethodSig, TypeSig},
    },
    Module,
};
use hao_runtime::Runtime;

static DAMAGE_HOOK_CALLS: AtomicU32 = AtomicU32::new(0);

/// Prefix for `TakeDamage(int health, int amount)`: returns the amount to use instead.
extern "C" fn before_take_damage(health: i32, amount: i32) -> i32 {
    DAMAGE_HOOK_CALLS.fetch_add(1, Ordering::Relaxed);
    let reduced = amount / 4;
    println!(
        "  [rust] TakeDamage(health: {}, amount: {}) -> amount {}",
        health, amount, reduced
    );
    reduced
}

/// Postfix for `Score(int kills)`: gets the result and returns the one to use instead.
extern "C" fn after_score(score: i32) -> i32 {
    println!("  [rust] Score returned {}, adding a 50 point bonus", score);
    score + 50
}

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

fn main() -> hao_runtime::Result<()> {
    let module =
        Module::from_bytes(&game_assembly().expect("a valid assembly")).expect("a valid assembly");

    // amount = before_take_damage(health, amount);
    module
        .find_method("Game.Player", "TakeDamage")
        .unwrap()
        .edit_body(|body| {
            let signature = MethodSig::native(TypeSig::INT32, vec![TypeSig::INT32, TypeSig::INT32]);
            body.prefix(
                Emitter::build(|e| {
                    e.ldarg(0)
                        .ldarg(1)
                        .call_native(before_take_damage as *const () as usize, signature)
                        .starg(1);
                })
                .instructions,
            );
        });

    // return after_score(<original result>);
    module
        .find_method("Game.Player", "Score")
        .unwrap()
        .edit_body(|body| {
            body.postfix(|| {
                let signature = MethodSig::native(TypeSig::INT32, vec![TypeSig::INT32]);
                Instruction::call_native(after_score as *const () as usize, signature).to_vec()
            });
        });

    let runtime = Runtime::coreclr()?;
    runtime.load_module(&module)?;
    let hit: extern "C" fn(i32, i32) -> i32 =
        unsafe { std::mem::transmute(runtime.function_pointer("Game", "Game.Player", "Hit")?) };
    let score: extern "C" fn(i32) -> i32 = unsafe {
        std::mem::transmute(runtime.function_pointer("Game", "Game.Player", "GetScore")?)
    };

    println!("Hit(100, 40):");
    println!("  = {}", hit(100, 40));
    println!("GetScore(3):");
    println!("  = {}", score(3));
    println!(
        "the damage hook ran {} times",
        DAMAGE_HOOK_CALLS.load(Ordering::Relaxed)
    );
    Ok(())
}
