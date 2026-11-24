#![cfg(feature = "mono")]

use hao::{
    dotnet::{
        entries::{well_known::SystemType, Method, TypeDef},
        il::{Instruction, MethodBody, OpCode},
        md::streams::tables_stream::{MethodFlags, TypeAttributes},
        module::{ModuleKind, TargetRuntime},
        signature::{MethodSig, TypeSig},
    },
    Module,
};
use hao_runtime::{MonoOptions, Runtime};
use std::ffi::c_void;

fn exports_assembly(name: &str) -> Vec<u8> {
    let mut module = Module::new(name, ModuleKind::Dll, TargetRuntime::NetFramework);
    let object = module.corlib_type(SystemType::Object).unwrap();
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

/// Starts the system Mono, or returns `None` after printing a skip notice.
/// Functional tests only run where a `libmonosgen-2.0` is installed; without
/// one `Runtime::mono` fails and the test is skipped rather than failed.
fn mono() -> Option<Runtime> {
    match Runtime::mono(&MonoOptions::default()) {
        Ok(r) => Some(r),
        Err(e) => {
            eprintln!("skipping: {}", e);
            None
        }
    }
}

#[test]
fn call_generated_method_through_mono_thunk() {
    let Some(runtime) = mono() else { return };
    runtime
        .load_assembly_bytes("MonoExports", &exports_assembly("MonoExports"))
        .unwrap();
    let add = runtime
        .function_pointer("MonoExports", "Native.Exports", "Add")
        .unwrap();
    let add: extern "C" fn(i32, i32, *mut *mut c_void) -> i32 = unsafe { std::mem::transmute(add) };
    let mut exception = std::ptr::null_mut();
    assert_eq!(add(40, 2, &mut exception), 42);
    assert!(exception.is_null());
}

#[test]
fn type_names_lists_generated_type() {
    let Some(runtime) = mono() else { return };
    runtime
        .load_assembly_bytes("MonoTypeNames", &exports_assembly("MonoTypeNames"))
        .unwrap();
    let types = runtime.type_names("MonoTypeNames").unwrap();
    assert!(
        types.iter().any(|t| t == "Native.Exports"),
        "expected Native.Exports in {:?}",
        types
    );
    // Row 1, the synthesised `<Module>` type, must be skipped.
    assert!(!types.iter().any(|t| t == "<Module>"), "{:?}", types);
}

#[test]
fn method_names_lists_generated_method() {
    let Some(runtime) = mono() else { return };
    runtime
        .load_assembly_bytes("MonoMethodNames", &exports_assembly("MonoMethodNames"))
        .unwrap();
    let methods = runtime
        .method_names("MonoMethodNames", "Native.Exports")
        .unwrap();
    assert!(
        methods.iter().any(|m| m == "Add"),
        "expected Add in {:?}",
        methods
    );
}

#[test]
fn find_method_matches_by_arity() {
    let Some(runtime) = mono() else { return };
    runtime
        .load_assembly_bytes("MonoFindMethod", &exports_assembly("MonoFindMethod"))
        .unwrap();
    // Arity 2 resolves `Add(int, int)`; the pointer is managed-calling-convention
    // with no trailing exception argument.
    let add = runtime
        .find_method_from_loaded("MonoFindMethod", "Native.Exports", "Add", 2)
        .unwrap();
    assert!(!add.is_null());
    let add: extern "C" fn(i32, i32) -> i32 = unsafe { std::mem::transmute(add) };
    assert_eq!(add(40, 2), 42);
    // No overload takes 5 parameters.
    assert!(runtime
        .find_method_from_loaded("MonoFindMethod", "Native.Exports", "Add", 5)
        .is_err());
}

#[test]
fn enumerate_null_is_empty() {
    let Some(runtime) = mono() else { return };
    assert_eq!(runtime.enumerate(0).unwrap(), Vec::<usize>::new());
}

#[test]
fn warm_enumerate_resolves_corlib_methods() {
    let Some(runtime) = mono() else { return };
    runtime.warm_enumerate().unwrap();
    // Idempotent.
    runtime.warm_enumerate().unwrap();
}
