//! Loads a real game's Il2Cpp runtime into the test process (without Unity) and
//! hooks its code. Set `HAO_IL2CPP_GAME` to the game folder. Skipped otherwise.
//!
//! Everything is in one test because a process can only start Il2Cpp once.

#![cfg(target_os = "linux")]

use std::sync::atomic::{AtomicBool, Ordering};

use hao_il2cpp_runtime::{original, Il2Cpp, MethodInfo, Object};

static INIT_CALLBACK_RAN: AtomicBool = AtomicBool::new(false);

type Concat = extern "C" fn(Object, Object, MethodInfo) -> Object;

/// Calls the original with its arguments swapped.
extern "C" fn swapped_concat(a: Object, b: Object, method: MethodInfo) -> Object {
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
    assert!(INIT_CALLBACK_RAN.load(Ordering::SeqCst), "the il2cpp_init hook did not run the callback");

    let string = il2cpp.class("mscorlib", "System", "String").unwrap();
    assert_eq!((string.namespace(), string.name()), ("System", "String"));
    // Objects start with a two pointer header (class and monitor).
    assert_eq!(string.field_offset("_stringLength").unwrap(), 2 * std::mem::size_of::<usize>());

    let method = string.method("Concat", 2).unwrap();
    let concat: Concat = unsafe { std::mem::transmute(method.code()) };
    let call = |a: &str, b: &str| unsafe {
        il2cpp
            .read_string(concat(il2cpp.string(a).unwrap(), il2cpp.string(b).unwrap(), method.as_ptr()))
            .unwrap()
    };
    assert_eq!(call("Kia ", "ora"), "Kia ora");

    let hook = unsafe { method.hook(swapped_concat as *mut u8) }.unwrap();
    assert_eq!(call("Kia ", "ora"), "oraKia ");

    hook.remove().unwrap();
    assert_eq!(call("Kia ", "ora"), "Kia ora");
}
