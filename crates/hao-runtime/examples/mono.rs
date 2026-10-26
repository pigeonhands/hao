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
        il::{Instruction, MethodBody, OpCode},
        md::streams::tables_stream::{MethodFlags, TypeAttributes},
        module::{ModuleKind, TargetRuntime},
        signature::{MethodSig, TypeSig},
    },
    Module,
};
use hao_runtime::{MonoOptions, Runtime};

/// `public static class Generated.Calc { static int Add(int a, int b); static int Divide(int a, int b); }`
fn calc_assembly() -> Vec<u8> {
    let mut module = Module::new("MonoCalc", ModuleKind::Dll, TargetRuntime::NetFramework);
    let object = module.corlib_type("System.Object").unwrap();
    let calc = module.add_type(TypeDef::new(
        TypeAttributes::Public | TypeAttributes::Abstract | TypeAttributes::Sealed,
        "Generated",
        "Calc",
        Some(object),
    ));
    for (name, opcode) in [("Add", OpCode::Add), ("Divide", OpCode::Div)] {
        let method = TypeDef::add_method(
            &calc,
            Method::new(
                MethodFlags::Public | MethodFlags::Static | MethodFlags::HideBySig,
                name,
                MethodSig::new_static(TypeSig::INT32, vec![TypeSig::INT32, TypeSig::INT32]),
            ),
        );
        method.get_mut().body = Some(MethodBody {
            instructions: vec![
                Instruction::ldarg(0),
                Instruction::ldarg(1),
                Instruction::simple(opcode),
                Instruction::ret(),
            ],
            ..Default::default()
        });
    }
    module.write().unwrap()
}

type Thunk = extern "C" fn(i32, i32, *mut *mut c_void) -> i32;

fn main() -> hao_runtime::Result<()> {
    let runtime = Runtime::mono(&MonoOptions::default())?;
    runtime.load_assembly_bytes("MonoCalc", &calc_assembly())?;
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
