//! Builds a class with hao in memory, loads it into CoreCLR and calls it from Rust.
//!
//! ```sh
//! cargo run -p hao-runtime --example generate_and_call
//! ```

mod common;

use hao::{
    dotnet::{
        entries::TypeDef,
        il::{Emitter, Instruction, OpCode, Operand},
        md::streams::tables_stream::TypeAttributes,
        module::{ModuleKind, TargetRuntime},
        signature::{MethodSig, TypeSig},
    },
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
fn calc_module() -> Module {
    let mut module = Module::new(
        "Generated",
        ModuleKind::Dll,
        TargetRuntime::NetCoreImplementation(8),
    );
    let object = module.corlib_type("System.Object").unwrap();
    let calc = module.add_type(TypeDef::new(
        TypeAttributes::Public | TypeAttributes::Abstract | TypeAttributes::Sealed,
        "Generated",
        "Calc",
        Some(object),
    ));

    let add = common::static_method(
        &calc,
        "Add",
        2,
        vec![
            Instruction::ldarg(0),
            Instruction::ldarg(1),
            Instruction::simple(OpCode::Add),
            Instruction::ret(),
        ],
    );

    // Calls into the .NET class library.
    let math = module.corlib_type("System.Math").unwrap();
    let math_max = module
        .import_method_on(
            &math,
            "Max",
            MethodSig::new_static(TypeSig::INT32, vec![TypeSig::INT32, TypeSig::INT32]),
        )
        .unwrap();
    let max = common::static_method(
        &calc,
        "Max",
        2,
        vec![
            Instruction::ldarg(0),
            Instruction::ldarg(1),
            Instruction::call(math_max),
            Instruction::ret(),
        ],
    );

    // Branches and locals with the emitter.
    let mut e = Emitter::new();
    let result = e.declare_local(TypeSig::INT32);
    let (top, done) = (e.label(), e.label());
    e.emit(Instruction::ldc_i4(1))
        .emit(Instruction::stloc(result));
    e.mark(top);
    e.emit(Instruction::ldarg(0)).emit(Instruction::ldc_i4(1));
    e.branch(OpCode::Ble, done);
    e.emit(Instruction::ldloc(result))
        .emit(Instruction::ldarg(0))
        .op(OpCode::Mul)
        .emit(Instruction::stloc(result));
    e.emit(Instruction::ldarg(0))
        .emit(Instruction::ldc_i4(1))
        .op(OpCode::Sub)
        .emit(Instruction::new(OpCode::Starg_S, Operand::Arg(0)));
    e.branch(OpCode::Br, top);
    e.mark(done);
    e.emit(Instruction::ldloc(result)).emit(Instruction::ret());
    let factorial = common::static_method(&calc, "Factorial", 1, vec![]);
    factorial.get_mut().body = Some(e.finish());

    for method in [&add, &max, &factorial] {
        common::unmanaged_callers_only(&mut module, method);
    }
    module
}

fn main() -> hao_runtime::Result<()> {
    let runtime = Runtime::coreclr()?;
    runtime.load_module(&calc_module())?;

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
