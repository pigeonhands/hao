//! The Mono backend (the runtime Unity's Mono games use): builds a class with
//! hao, loads it and calls it, including a call that throws.
//!
//! ```sh
//! cargo run -p hao-runtime --features mono --example mono
//! ```
//!
//! Mono can call any static method, without `[UnmanagedCallersOnly]`. Its
//! thunks take an extra last argument that receives a thrown exception.

use std::ffi::c_void;

use hao::{
    dotnet::{
        entries::{Method, TypeDef},
        il::{Emitter, OpCode},
        module::{ModuleKind, TargetRuntime},
        signature::{MethodSig, TypeSig},
    },
    Module,
};
use hao_runtime::{MonoOptions, Runtime};

/// `public static class Generated.Calc { static int Add(int a, int b); static int Divide(int a, int b); }`
fn calc_assembly() -> hao::error::Result<Vec<u8>> {
    let mut module = Module::new("MonoCalc", ModuleKind::Dll, TargetRuntime::NetFramework);
    let calc = module.add_static_class("Generated", "Calc")?;
    for (name, opcode) in [("Add", OpCode::Add), ("Divide", OpCode::Div)] {
        let signature = MethodSig::new_static(TypeSig::INT32, vec![TypeSig::INT32; 2]);
        let body = Emitter::build(|e| {
            e.ldarg(0).ldarg(1).op(opcode).ret();
        });
        TypeDef::add_method(
            &calc,
            Method::public_static(name, signature).with_body(body),
        );
    }
    module.write()
}

type Thunk = extern "C" fn(i32, i32, *mut *mut c_void) -> i32;

fn main() -> hao_runtime::Result<()> {
    let runtime = Runtime::mono(&MonoOptions::default())?;
    runtime.load_assembly_bytes("MonoCalc", &calc_assembly().expect("a valid assembly"))?;
    let thunk = |name| -> hao_runtime::Result<Thunk> {
        Ok(unsafe {
            std::mem::transmute::<*const c_void, Thunk>(runtime.function_pointer(
                "MonoCalc",
                "Generated.Calc",
                name,
            )?)
        })
    };
    let (add, divide) = (thunk("Add")?, thunk("Divide")?);

    let mut exception = std::ptr::null_mut();
    println!("Calc.Add(40, 2)    = {}", add(40, 2, &mut exception));
    println!("Calc.Divide(84, 2) = {}", divide(84, 2, &mut exception));

    divide(1, 0, &mut exception);
    println!("Calc.Divide(1, 0) threw: {}", !exception.is_null());
    Ok(())
}
