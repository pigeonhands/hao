//! Builds a class with hao in memory, loads it into CoreCLR and calls it from Rust.
//!
//! ```sh
//! cargo run -p hao-runtime --example generate_and_call
//! ```

use hao::{
    dotnet::{
        entries::{Method, TypeDef},
        il::{Emitter, OpCode},
        module::{ModuleKind, TargetRuntime},
        signature::{MethodSig, TypeSig},
    },
    error::Result,
    Module,
};
use hao_runtime::Runtime;

/// ```csharp
/// public static class Generated.Calc {
///     [UnmanagedCallersOnly] public static int Add(int a, int b) => a + b;
///     [UnmanagedCallersOnly] public static int Max(int a, int b) => Math.Max(a, b);
///     [UnmanagedCallersOnly] public static int Factorial(int n) {
///         int result = 1;
///         while (n > 1) { result *= n; n--; }
///         return result;
///     }
/// }
/// ```
fn calc_module() -> Result<Module> {
    let mut module = Module::new(
        "Generated",
        ModuleKind::Dll,
        TargetRuntime::NetCoreImplementation(8),
    );
    let calc = module.add_static_class("Generated", "Calc")?;
    // CoreCLR can only call methods marked [UnmanagedCallersOnly] from native code.
    let uco =
        module.corlib_attribute("System.Runtime.InteropServices.UnmanagedCallersOnlyAttribute")?;
    let int_method = |name: &str, params: usize| {
        Method::public_static(
            name,
            MethodSig::new_static(TypeSig::INT32, vec![TypeSig::INT32; params]),
        )
        .with_attribute(uco.clone())
    };

    let mut add = Emitter::new();
    add.ldarg(0).ldarg(1).op(OpCode::Add).ret();
    TypeDef::add_method(&calc, int_method("Add", 2).with_body(add.finish()));

    // Calls into the .NET class library.
    let math_max = module.import_corlib_method(
        "System.Math",
        "Max",
        MethodSig::new_static(TypeSig::INT32, vec![TypeSig::INT32; 2]),
    )?;
    let mut max = Emitter::new();
    max.ldarg(0).ldarg(1).call(math_max).ret();
    TypeDef::add_method(&calc, int_method("Max", 2).with_body(max.finish()));

    // Locals and a loop.
    let mut factorial = Emitter::new();
    let result = factorial.declare_local(TypeSig::INT32);
    factorial.ldc_i4(1).stloc(result);
    factorial.while_loop(
        |e| {
            e.ldarg(0).ldc_i4(1).op(OpCode::Cgt);
        },
        |e| {
            e.ldloc(result).ldarg(0).op(OpCode::Mul).stloc(result);
            e.ldarg(0).ldc_i4(1).op(OpCode::Sub).starg(0);
        },
    );
    factorial.ldloc(result).ret();
    TypeDef::add_method(
        &calc,
        int_method("Factorial", 1).with_body(factorial.finish()),
    );
    Ok(module)
}

fn main() -> hao_runtime::Result<()> {
    let runtime = Runtime::coreclr()?;
    runtime.load_module(&calc_module().expect("a valid module"))?;

    let function = |name| runtime.function_pointer("Generated", "Generated.Calc", name);
    let add: extern "C" fn(i32, i32) -> i32 = unsafe { std::mem::transmute(function("Add")?) };
    let max: extern "C" fn(i32, i32) -> i32 = unsafe { std::mem::transmute(function("Max")?) };
    let factorial: extern "C" fn(i32) -> i32 =
        unsafe { std::mem::transmute(function("Factorial")?) };

    println!("Calc.Add(40, 2)    = {}", add(40, 2));
    println!("Calc.Max(-3, 7)    = {}", max(-3, 7));
    println!("Calc.Factorial(10) = {}", factorial(10));
    Ok(())
}
