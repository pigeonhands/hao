//! Calls methods that are NOT `[UnmanagedCallersOnly]`. hao-runtime looks them
//! up by reflection, JIT-compiles them and returns a native entry point, so a
//! plain static method (or one whose IL you patched before loading) is callable
//! from Rust without decorating it.
//!
//! ```sh
//! cargo run -p hao-runtime --example call_any_method
//! ```

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

/// ```csharp
/// public static class Plain.Calc {
///     public static int Add(int a, int b) => a + b;   // no [UnmanagedCallersOnly]
///     public static int Double(int n) => n + n;       // patched to n * 10 below
/// }
/// ```
fn calc_module() -> Module {
    let mut module = Module::new(
        "Plain",
        ModuleKind::Dll,
        TargetRuntime::NetCoreImplementation(9),
    );
    let calc = module.add_static_class("Plain", "Calc").unwrap();
    let signature = |params| MethodSig::new_static(TypeSig::INT32, vec![TypeSig::INT32; params]);
    TypeDef::add_method(
        &calc,
        Method::public_static("Add", signature(2)).with_body(Emitter::build(|e| {
            e.ldarg(0).ldarg(1).op(OpCode::Add).ret();
        })),
    );
    TypeDef::add_method(
        &calc,
        Method::public_static("Double", signature(1)).with_body(Emitter::build(|e| {
            e.ldarg(0).ldarg(0).op(OpCode::Add).ret();
        })),
    );
    module
}

fn main() -> hao_runtime::Result<()> {
    let runtime = Runtime::coreclr()?;

    let module = calc_module();
    // Patch Double into "times 10" before loading. The method is JIT-compiled
    // when we take its pointer, so the patched IL is what runs.
    module
        .find_method("Plain.Calc", "Double")
        .unwrap()
        .edit_body(|body| {
            body.instructions = vec![
                Instruction::ldarg(0),
                Instruction::ldc_i4(10),
                Instruction::simple(OpCode::Mul),
                Instruction::ret(),
            ];
        });
    runtime.load_module(&module)?;

    // function_pointer only works on [UnmanagedCallersOnly] methods:
    match runtime.function_pointer("Plain", "Plain.Calc", "Add") {
        Err(e) => println!("function_pointer(Add): {}", e),
        Ok(_) => println!("function_pointer(Add): unexpectedly succeeded"),
    }

    // method_pointer JITs and returns a callable entry point for any static method:
    let add: extern "C" fn(i32, i32) -> i32 =
        unsafe { std::mem::transmute(runtime.method_pointer("Plain", "Plain.Calc", "Add")?) };
    let double: extern "C" fn(i32) -> i32 =
        unsafe { std::mem::transmute(runtime.method_pointer("Plain", "Plain.Calc", "Double")?) };

    println!("Calc.Add(40, 2)  = {}", add(40, 2));
    println!("Calc.Double(5)   = {} (patched to times 10)", double(5));
    Ok(())
}
