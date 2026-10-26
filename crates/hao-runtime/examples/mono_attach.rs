//! Attaching to a Mono runtime already running in the process, the way a mod
//! injected into a Unity Mono game would. In a real mod the game has already
//! started Mono; here the example starts one itself to stand in for the game.
//!
//! ```sh
//! cargo run -p hao-runtime --features mono --example mono_attach
//! ```

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

fn main() {
    let Some(library) = mono_library() else {
        eprintln!("Mono is not installed; nothing to attach to");
        return;
    };

    // Stand in for the game: start Mono. Keep the handle alive so it stays loaded.
    let host = unsafe { libloading::Library::new(library).unwrap() };
    unsafe {
        let jit_init: libloading::Symbol<
            unsafe extern "C" fn(*const c_char, *const c_char) -> *mut c_void,
        > = host.get(b"mono_jit_init_version\0").unwrap();
        let (name, version) = (
            CString::new("game").unwrap(),
            CString::new("v4.0.30319").unwrap(),
        );
        assert!(
            !jit_init(name.as_ptr(), version.as_ptr()).is_null(),
            "could not start Mono"
        );
    }
    println!("a Mono runtime is now running in this process");

    // What a mod does: attach to the running Mono and use it.
    let options = MonoOptions {
        library: Some(library.to_owned()),
        ..Default::default()
    };
    let runtime = Runtime::mono_attach(&options).expect("attach to Mono");
    runtime
        .load_assembly_bytes("AttachExports", &exports_assembly())
        .unwrap();
    let mul = runtime
        .function_pointer("AttachExports", "Native.Exports", "Mul")
        .unwrap();
    let mul: extern "C" fn(i32, i32, *mut *mut c_void) -> i32 = unsafe { std::mem::transmute(mul) };

    let mut exception = std::ptr::null_mut();
    println!(
        "attached; Native.Exports.Mul(6, 7) = {}",
        mul(6, 7, &mut exception)
    );
}
