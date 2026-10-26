//! Attaching to a Mono runtime that was started outside hao, as a mod injected
//! into a Unity Mono game would. The test starts Mono itself (standing in for
//! the game), then attaches with `Runtime::mono_attach` and calls a method.

#![cfg(feature = "mono")]

use std::{
    ffi::{c_char, c_void, CString},
    path::Path,
};

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

fn mono_library() -> Option<&'static Path> {
    [
        "/usr/lib/libmonosgen-2.0.so",
        "/usr/lib/libmonosgen-2.0.so.1",
        "/usr/local/lib/libmonosgen-2.0.so",
    ]
    .into_iter()
    .map(Path::new)
    .find(|p| p.exists())
}

/// `static class Native.Exports { static int Mul(int a, int b) => a * b; }`
fn exports_assembly() -> Vec<u8> {
    let mut module = Module::new(
        "AttachExports",
        ModuleKind::Dll,
        TargetRuntime::NetFramework,
    );
    let exports = module.add_static_class("Native", "Exports").unwrap();
    let body = Emitter::build(|e| {
        e.ldarg(0).ldarg(1).op(OpCode::Mul).ret();
    });
    TypeDef::add_method(
        &exports,
        Method::public_static(
            "Mul",
            MethodSig::new_static(TypeSig::INT32, vec![TypeSig::INT32; 2]),
        )
        .with_body(body),
    );
    module.write().unwrap()
}

#[test]
fn attaches_to_a_running_mono() {
    let Some(library) = mono_library() else {
        eprintln!("skipping: system Mono not found");
        return;
    };

    // Stand in for the game: start Mono ourselves, without going through hao's
    // Runtime. Keep the handle alive for the whole test so Mono stays loaded.
    let _mono = unsafe { libloading::Library::new(library).unwrap() };
    unsafe {
        let jit_init: libloading::Symbol<
            unsafe extern "C" fn(*const c_char, *const c_char) -> *mut c_void,
        > = _mono.get(b"mono_jit_init_version\0").unwrap();
        let name = CString::new("game").unwrap();
        let version = CString::new("v4.0.30319").unwrap();
        assert!(
            !jit_init(name.as_ptr(), version.as_ptr()).is_null(),
            "could not start Mono"
        );
    }

    // The mod attaches to the Mono already running in this process.
    let options = MonoOptions {
        library: Some(library.to_owned()),
        ..Default::default()
    };
    let runtime = Runtime::mono_attach(&options).unwrap();
    runtime
        .load_assembly_bytes("AttachExports", &exports_assembly())
        .unwrap();
    let mul = runtime
        .function_pointer("AttachExports", "Native.Exports", "Mul")
        .unwrap();
    let mul: extern "C" fn(i32, i32, *mut *mut c_void) -> i32 = unsafe { std::mem::transmute(mul) };
    let mut exception = std::ptr::null_mut();
    assert_eq!(mul(6, 7, &mut exception), 42);
    assert!(exception.is_null());
}
