//! Hooks managed methods with Rust functions, like Harmony prefixes and
//! postfixes: hao rewrites the methods' IL to call into Rust through a native
//! function pointer (`calli`), then the hooked assembly is loaded into CoreCLR.
//!
//! ```sh
//! cargo run -p hao-runtime --example hook_managed
//! ```
//!
//! The hooks are added before the code is loaded, because the runtime compiles
//! each method once. To hook an assembly an app loads itself, edit its bytes
//! before they're loaded.

mod common;

use std::sync::atomic::{AtomicU32, Ordering};

use hao::{
    dotnet::{
        il::{Instruction, OpCode, Operand},
        signature::{CallingConvention, MethodSig, TypeSig},
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

/// Instructions that call `function`, a native function with signature `ret(params)`.
fn call_native(function: usize, ret: TypeSig, params: Vec<TypeSig>) -> [Instruction; 3] {
    let signature = MethodSig {
        calling_convention: CallingConvention::C,
        ret,
        params,
        ..Default::default()
    };
    [
        Instruction::new(OpCode::Ldc_I8, Operand::Int64(function as i64)),
        Instruction::simple(OpCode::Conv_I),
        Instruction::new(OpCode::Calli, Operand::Sig(signature)),
    ]
}

fn main() -> hao_runtime::Result<()> {
    let module = Module::from_bytes(&common::game_assembly()).expect("a valid assembly");

    // amount = before_take_damage(health, amount);
    module
        .find_method("Game.Player", "TakeDamage")
        .unwrap()
        .edit_body(|body| {
            let mut prefix = vec![Instruction::ldarg(0), Instruction::ldarg(1)];
            prefix.extend(call_native(
                before_take_damage as *const () as usize,
                TypeSig::INT32,
                vec![TypeSig::INT32, TypeSig::INT32],
            ));
            prefix.push(Instruction::new(OpCode::Starg_S, Operand::Arg(1)));
            body.insert(0, prefix);
        });

    // return after_score(<original result>);
    module
        .find_method("Game.Player", "Score")
        .unwrap()
        .edit_body(|body| {
            let rets: Vec<usize> = (0..body.instructions.len())
                .filter(|&i| body.instructions[i].opcode == OpCode::Ret)
                .collect();
            for ret in rets.into_iter().rev() {
                // The `ret` becomes the postfix's first instruction, so branches to it run the postfix.
                let [load, convert, call] = call_native(
                    after_score as *const () as usize,
                    TypeSig::INT32,
                    vec![TypeSig::INT32],
                );
                body.instructions[ret] = load;
                body.insert(ret + 1, [convert, call, Instruction::ret()]);
            }
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
