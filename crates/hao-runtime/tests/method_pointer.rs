//! Calling methods that are NOT `[UnmanagedCallersOnly]`: hao-runtime JITs them
//! by reflection and hands back a native entry point. Also checks that IL
//! patched before loading takes effect when the method is JIT-compiled.

#![cfg(feature = "coreclr")]

use hao::{
    dotnet::{
        entries::{Method, TypeDef},
        il::{Emitter, OpCode},
        module::{ModuleKind, TargetRuntime},
        signature::{MethodSig, TypeSig},
    },
    Module,
};
use hao_runtime::Runtime;

/// ```csharp
/// public static class Plain.Math {
///     public static int Add(int a, int b) => a + b;          // no [UnmanagedCallersOnly]
///     public static int Triple(int n) => n * 2;              // patched to n * 3 below
/// }
/// ```
fn plain_module() -> Module {
    let mut module = Module::new(
        "Plain",
        ModuleKind::Dll,
        TargetRuntime::NetCoreImplementation(9),
    );
    let math = module.add_static_class("Plain", "Math").unwrap();
    let signature = |params| MethodSig::new_static(TypeSig::INT32, vec![TypeSig::INT32; params]);
    TypeDef::add_method(
        &math,
        Method::public_static("Add", signature(2)).with_body(Emitter::build(|e| {
            e.ldarg(0).ldarg(1).op(OpCode::Add).ret();
        })),
    );
    TypeDef::add_method(
        &math,
        Method::public_static("Triple", signature(1)).with_body(Emitter::build(|e| {
            e.ldarg(0).ldc_i4(2).op(OpCode::Mul).ret();
        })),
    );
    module
}

#[test]
fn calls_methods_without_unmanaged_callers_only() {
    if hao_runtime::find_dotnet_root().is_none() {
        eprintln!("skipping: .NET is not installed");
        return;
    }
    let runtime = Runtime::coreclr().unwrap();

    let module = plain_module();
    // Patch Triple to multiply by 3 before loading; the JIT compiles the patched IL.
    module
        .find_method("Plain.Math", "Triple")
        .unwrap()
        .edit_body(|body| {
            let index = body.position(|i| i.opcode == OpCode::Ldc_I4_2).unwrap();
            body.instructions[index] = hao::dotnet::il::Instruction::ldc_i4(3);
        });
    runtime.load_module(&module).unwrap();

    // Neither method is [UnmanagedCallersOnly], so function_pointer can't call them...
    assert!(runtime
        .function_pointer("Plain", "Plain.Math", "Add")
        .is_err());

    // ...but method_pointer JITs and returns a callable entry point.
    let add: extern "C" fn(i32, i32) -> i32 = unsafe {
        std::mem::transmute(
            runtime
                .method_pointer("Plain", "Plain.Math", "Add")
                .unwrap(),
        )
    };
    assert_eq!(add(40, 2), 42);

    let triple: extern "C" fn(i32) -> i32 = unsafe {
        std::mem::transmute(
            runtime
                .method_pointer("Plain", "Plain.Math", "Triple")
                .unwrap(),
        )
    };
    assert_eq!(triple(10), 30, "the patched IL should run");

    assert!(runtime
        .method_pointer("Plain", "Plain.Math", "Missing")
        .is_err());
}
