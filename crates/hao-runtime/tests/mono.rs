#![cfg(feature = "mono")]

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
use std::ffi::c_void;

fn exports_assembly() -> Vec<u8> {
    let mut module = Module::new("MonoExports", ModuleKind::Dll, TargetRuntime::NetFramework);
    let object = module.corlib_type("System.Object").unwrap();
    let exports = module.add_type(TypeDef::new(
        TypeAttributes::Public | TypeAttributes::Abstract | TypeAttributes::Sealed,
        "Native",
        "Exports",
        Some(object),
    ));
    let add = TypeDef::add_method(
        &exports,
        Method::new(
            MethodFlags::Public | MethodFlags::Static | MethodFlags::HideBySig,
            "Add",
            MethodSig::new_static(TypeSig::INT32, vec![TypeSig::INT32, TypeSig::INT32]),
        ),
    );
    add.get_mut().body = Some(MethodBody {
        instructions: vec![
            Instruction::ldarg(0),
            Instruction::ldarg(1),
            Instruction::simple(OpCode::Add),
            Instruction::ret(),
        ],
        ..Default::default()
    });
    module.write().unwrap()
}

#[test]
fn call_generated_method_through_mono_thunk() {
    let runtime = match Runtime::mono(&MonoOptions::default()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("skipping: {}", e);
            return;
        }
    };
    runtime
        .load_assembly_bytes("MonoExports", &exports_assembly())
        .unwrap();
    let add = runtime
        .function_pointer("MonoExports", "Native.Exports", "Add")
        .unwrap();
    let add: extern "C" fn(i32, i32, *mut *mut c_void) -> i32 = unsafe { std::mem::transmute(add) };
    let mut exception = std::ptr::null_mut();
    assert_eq!(add(40, 2, &mut exception), 42);
    assert!(exception.is_null());
}
