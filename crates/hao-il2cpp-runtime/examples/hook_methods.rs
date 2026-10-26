//! Starts a game's Il2Cpp runtime in this process, without Unity, and hooks
//! methods of its compiled code with closures: changing arguments, replacing
//! the result, and removing the hook again.
//!
//! ```sh
//! cargo run -p hao-il2cpp-runtime --example hook_methods -- <game folder>
//! ```
//!
//! Mods do the same from inside the game, in [`on_init`](hao_il2cpp_runtime::on_init).

use std::sync::atomic::{AtomicU32, Ordering};

use hao_il2cpp_runtime::{find_game_assembly, Il2Cpp, Il2CppString, Original, Result};

static CALLS: AtomicU32 = AtomicU32::new(0);

fn main() -> Result<()> {
    let game = std::path::PathBuf::from(
        std::env::args()
            .nth(1)
            .expect("usage: hook_methods <game folder>"),
    );
    let data_dir = std::fs::read_dir(&game)
        .expect("could not read the game folder")
        .filter_map(|e| e.ok().map(|e| e.path().join("il2cpp_data")))
        .find(|p| p.join("Metadata/global-metadata.dat").exists())
        .expect("no <game>_Data/il2cpp_data folder");
    let il2cpp = Il2Cpp::load(find_game_assembly(&game).expect("no game binary in the folder"))?;
    il2cpp.init_standalone(&data_dir)?;

    let string = il2cpp.class("mscorlib", "System", "String")?;
    let concat = string
        .methods()
        .find(|m| m.signature() == "static System.String Concat(System.String, System.String)")
        .expect("String.Concat(string, string)");
    let call_concat = |a: &str, b: &str| -> Result<String> {
        let joined: Il2CppString = concat.call((il2cpp.string(a)?, il2cpp.string(b)?))?;
        Ok(joined.to_rust().unwrap_or_default())
    };
    println!("before:   {}", call_concat("Kia ", "ora")?);

    // Count calls and swap the arguments before running the original.
    let hook = concat.hook(
        |a: Il2CppString, b: Il2CppString, original: Original<_, Il2CppString>| {
            CALLS.fetch_add(1, Ordering::Relaxed);
            original.call((b, a))
        },
    )?;
    println!("swapped:  {}", call_concat("Kia ", "ora")?);
    hook.remove()?;

    // Replace the result entirely.
    let hook = concat.hook(
        |a: Il2CppString, b: Il2CppString, _original: Original<_, Il2CppString>| {
            let text = format!(
                "{}{}",
                a.to_rust().unwrap_or_default(),
                b.to_rust().unwrap_or_default()
            );
            Il2Cpp::get().unwrap().string(&text.to_uppercase()).unwrap()
        },
    )?;
    println!("replaced: {}", call_concat("Kia ", "ora")?);
    hook.remove()?;

    println!("after:    {}", call_concat("Kia ", "ora")?);
    println!(
        "the counting hook ran {} times",
        CALLS.load(Ordering::Relaxed)
    );

    // A closure with the wrong types is refused before anything is patched.
    let wrong = concat.hook(|a: i32, _b: i32, _original: Original<_, i32>| a);
    println!(
        "hooking with ints: {}",
        wrong.err().map(|e| e.to_string()).unwrap_or_default()
    );
    Ok(())
}
