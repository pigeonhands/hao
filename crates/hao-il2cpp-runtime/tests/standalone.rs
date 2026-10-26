//! Loads a real game's Il2Cpp runtime into the test process (without Unity) and
//! hooks its code. Set `HAO_IL2CPP_GAME` to the game folder. Skipped otherwise.
//!
//! Everything is in one test because a process can only start Il2Cpp once.

#![cfg(target_os = "linux")]

use std::sync::atomic::{AtomicBool, Ordering};

use hao_il2cpp_runtime::{
    original, Error, Il2Cpp, Il2CppString, Il2CppValue, MethodInfo, Object, Original, TypeInfo,
};

#[repr(C)]
#[derive(Clone, Copy)]
struct Scene {
    handle: i32,
}

unsafe impl Il2CppValue for Scene {
    fn matches(ty: TypeInfo) -> bool {
        ty.is_value_type("UnityEngine.SceneManagement", "Scene")
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Vector2 {
    x: f32,
    y: f32,
}

unsafe impl Il2CppValue for Vector2 {
    fn matches(ty: TypeInfo) -> bool {
        ty.is_value_type("UnityEngine", "Vector2")
    }
}

static INIT_CALLBACK_RAN: AtomicBool = AtomicBool::new(false);

type Concat = extern "C" fn(Il2CppString, Il2CppString, MethodInfo) -> Il2CppString;

/// Calls the original with its arguments swapped.
extern "C" fn swapped_concat(a: Il2CppString, b: Il2CppString, method: MethodInfo) -> Il2CppString {
    let concat = original!(Concat);
    concat(b, a, method)
}

#[test]
fn hooks_il2cpp_code() {
    let Some(game) = std::env::var_os("HAO_IL2CPP_GAME").map(std::path::PathBuf::from) else {
        eprintln!("skipping: set HAO_IL2CPP_GAME to an Il2Cpp game folder");
        return;
    };
    let data_dir = std::fs::read_dir(&game)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path().join("il2cpp_data")))
        .find(|p| p.join("Metadata/global-metadata.dat").exists())
        .unwrap();

    // Registered before the game binary is loaded, as a mod loaded with LD_PRELOAD would be.
    hao_il2cpp_runtime::on_init(|il2cpp| {
        il2cpp.class("mscorlib", "System", "String")?;
        INIT_CALLBACK_RAN.store(true, Ordering::SeqCst);
        Ok(())
    });

    let il2cpp = Il2Cpp::load(hao_il2cpp_runtime::find_game_assembly(&game).unwrap()).unwrap();
    il2cpp.init_standalone(&data_dir).unwrap();
    assert!(
        INIT_CALLBACK_RAN.load(Ordering::SeqCst),
        "the il2cpp_init hook did not run the callback"
    );

    let string = il2cpp.class("mscorlib", "System", "String").unwrap();
    assert_eq!((string.namespace(), string.name()), ("System", "String"));
    // Objects start with a two pointer header (class and monitor).
    assert_eq!(
        string.field_offset("_stringLength").unwrap(),
        2 * std::mem::size_of::<usize>()
    );

    let method = string
        .methods()
        .find(|m| m.signature() == "static System.String Concat(System.String, System.String)")
        .unwrap();
    let concat: Concat = unsafe { std::mem::transmute(method.code()) };
    let call = |a: &str, b: &str| unsafe {
        il2cpp
            .read_string(
                concat(
                    il2cpp.string(a).unwrap(),
                    il2cpp.string(b).unwrap(),
                    method.as_ptr(),
                )
                .0,
            )
            .unwrap()
    };
    assert_eq!(call("Kia ", "ora"), "Kia ora");

    let hook = unsafe { method.hook_raw(swapped_concat as *mut u8) }.unwrap();
    assert_eq!(call("Kia ", "ora"), "oraKia ");

    hook.remove().unwrap();
    assert_eq!(call("Kia ", "ora"), "Kia ora");

    // Typed hooks: the closure's types are checked against the method.
    let hook = method
        .hook(
            |a: Il2CppString, b: Il2CppString, original: Original<_, Il2CppString>| {
                let joined = original.call((a, b)).to_rust().unwrap();
                Il2CppString::new(Il2Cpp::get().unwrap(), &joined.to_uppercase()).unwrap()
            },
        )
        .unwrap();
    assert_eq!(call("Kia ", "ora"), "KIA ORA");
    let mismatch = method.hook(|a: Il2CppString, _b: i32, _original: Original<_, Il2CppString>| a);
    assert!(
        matches!(mismatch, Err(Error::Signature(_))),
        "{:?}",
        mismatch.err()
    );
    assert!(matches!(
        method.hook(|a: Object, _original: Original<_, Object>| a),
        Err(Error::Signature(_))
    ));
    hook.remove().unwrap();

    let called: Il2CppString = method
        .call((il2cpp.string("a").unwrap(), il2cpp.string("b").unwrap()))
        .unwrap();
    assert_eq!(called.to_rust().unwrap(), "ab");
    assert!(matches!(
        method.call::<_, i32>((il2cpp.string("a").unwrap(), il2cpp.string("b").unwrap())),
        Err(Error::Signature(_))
    ));

    // A managed exception thrown under a hook unwinds through it to il2cpp_runtime_invoke.
    let parse = il2cpp
        .class("mscorlib", "System", "Int32")
        .unwrap()
        .method("Parse", 1)
        .unwrap();
    assert!(
        parse.signature().contains("Parse(System.String)"),
        "{}",
        parse.signature()
    );
    let parse_hook = parse
        .hook(|s: Il2CppString, original: Original<_, i32>| original.call((s,)) + 1)
        .unwrap();
    let invoke = |text: &str| {
        let mut args = [il2cpp.string(text).unwrap().0.as_ptr()];
        let mut exception = std::ptr::null_mut();
        let result = unsafe {
            (il2cpp.api().il2cpp_runtime_invoke)(
                parse.as_ptr(),
                std::ptr::null_mut(),
                args.as_mut_ptr(),
                &mut exception,
            )
        };
        (result, exception)
    };
    let (boxed, exception) = invoke("41");
    assert!(exception.is_null());
    // A boxed int's value follows the object header.
    assert_eq!(
        unsafe {
            hao_il2cpp_runtime::read_field::<i32>(Object(boxed), 2 * std::mem::size_of::<usize>())
        },
        42
    );
    let (_, exception) = invoke("not a number");
    assert!(!exception.is_null());
    assert_eq!(
        unsafe { il2cpp.class_of(Object(exception)) }.name(),
        "FormatException"
    );
    parse_hook.remove().unwrap();

    // Structs declared in Rust, and enums as their underlying integer.
    let scenes = il2cpp
        .class(
            "UnityEngine.CoreModule",
            "UnityEngine.SceneManagement",
            "SceneManager",
        )
        .unwrap();
    let loaded = scenes.method("Internal_SceneLoaded", 2).unwrap();
    loaded.check_types::<(Scene, i32), ()>().unwrap();
    assert!(loaded.check_types::<(i32, i32), ()>().is_err());
    assert!(loaded.check_types::<(Scene, Vector2), ()>().is_err());
    let rect = il2cpp
        .class("UnityEngine.CoreModule", "UnityEngine", "RectTransform")
        .unwrap();
    rect.method("set_anchoredPosition", 1)
        .unwrap()
        .check_types::<(Object, Vector2), ()>()
        .unwrap();
    let game_object = il2cpp
        .class("UnityEngine.CoreModule", "UnityEngine", "GameObject")
        .unwrap();
    assert_eq!(
        unsafe { il2cpp.class_of(game_object.type_object()) }.name(),
        "RuntimeType"
    );
    assert_eq!(
        unsafe { il2cpp.class_of(game_object.new_object()) }.name(),
        "GameObject"
    );
    let pinned = hao_il2cpp_runtime::GcHandle::new(il2cpp, il2cpp.string("kept").unwrap().0);
    assert_eq!(
        unsafe { il2cpp.read_string(pinned.target()) }.as_deref(),
        Some("kept")
    );

    // A mod injected after the runtime has started: on_init runs straight away.
    static LATE_RAN: AtomicBool = AtomicBool::new(false);
    hao_il2cpp_runtime::on_init(|_| {
        LATE_RAN.store(true, Ordering::SeqCst);
        Ok(())
    });
    assert!(
        LATE_RAN.load(Ordering::SeqCst),
        "on_init after init should run immediately"
    );
}
