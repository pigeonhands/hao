//! Loads the `heal_mod` example the way the game would with `LD_PRELOAD`, then
//! starts Megabonk's Il2Cpp runtime (without Unity) and checks the mod hooked
//! `PlayerHealth.Heal`. Set `HAO_IL2CPP_GAME` to Megabonk's folder. Skipped otherwise.

#![cfg(target_os = "linux")]

use std::{ffi::CString, io::Read, os::fd::AsRawFd};

#[test]
fn heal_mod_hooks_the_game() {
    let Some(game) = std::env::var_os("HAO_IL2CPP_GAME").map(std::path::PathBuf::from) else {
        eprintln!("skipping: set HAO_IL2CPP_GAME to Megabonk's folder");
        return;
    };
    if !game.join("Megabonk_Data").exists() {
        eprintln!("skipping: the heal_mod example is for Megabonk");
        return;
    }
    let examples = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples");
    let mod_path = examples.join("libheal_mod.so");

    // The mod prints to stderr, so capture file descriptor 2 for the duration.
    let mut captured = tempfile();
    let saved = unsafe { libc::dup(2) };
    unsafe { libc::dup2(captured.as_raw_fd(), 2) };

    unsafe {
        let open = |path: &std::path::Path| {
            let path = CString::new(path.to_str().unwrap()).unwrap();
            let handle = libc::dlopen(path.as_ptr(), libc::RTLD_NOW);
            assert!(!handle.is_null(), "dlopen {:?} failed", path);
            handle
        };
        open(&mod_path);
        let game_assembly = open(&game.join("GameAssembly.so"));
        let set_data_dir: extern "C" fn(*const libc::c_char) =
            std::mem::transmute(libc::dlsym(game_assembly, c"il2cpp_set_data_dir".as_ptr()));
        let init: extern "C" fn(*const libc::c_char) -> i32 =
            std::mem::transmute(libc::dlsym(game_assembly, c"il2cpp_init".as_ptr()));
        let data = CString::new(game.join("Megabonk_Data/il2cpp_data").to_str().unwrap()).unwrap();
        set_data_dir(data.as_ptr());
        assert_ne!(init(c"heal_mod".as_ptr()), 0);
    }

    unsafe { libc::dup2(saved, 2) };
    let mut output = String::new();
    use std::io::Seek;
    captured.rewind().unwrap();
    captured.read_to_string(&mut output).unwrap();
    assert!(
        output.contains("hooked PlayerHealth.Heal"),
        "mod output: {:?}",
        output
    );
}

fn tempfile() -> std::fs::File {
    let path = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("heal_mod_stderr.txt");
    std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(path)
        .unwrap()
}
