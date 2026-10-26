//! Starts a game's Il2Cpp runtime in this process, without Unity, then lists
//! and calls methods from the game's own compiled code.
//!
//! ```sh
//! cargo run -p hao-il2cpp-runtime --example call_methods -- <game folder>
//! ```
//!
//! Only code that doesn't need the Unity engine works this way, such as the
//! .NET class library and most plain game logic.

use hao_il2cpp_runtime::{find_game_assembly, Il2Cpp, Il2CppString, Result};

fn main() -> Result<()> {
    let game = std::path::PathBuf::from(
        std::env::args()
            .nth(1)
            .expect("usage: call_methods <game folder>"),
    );
    let data_dir = std::fs::read_dir(&game)
        .expect("could not read the game folder")
        .filter_map(|e| e.ok().map(|e| e.path().join("il2cpp_data")))
        .find(|p| p.join("Metadata/global-metadata.dat").exists())
        .expect("no <game>_Data/il2cpp_data folder");

    let il2cpp = Il2Cpp::load(find_game_assembly(&game).expect("no game binary in the folder"))?;
    il2cpp.init_standalone(&data_dir)?;

    let string = il2cpp.class("mscorlib", "System", "String")?;
    println!(
        "System.String has {} methods, e.g.:",
        string.methods().count()
    );
    for method in string
        .methods()
        .filter(|m| m.name().starts_with("Concat"))
        .take(5)
    {
        println!("  {}", method.signature());
    }

    // Pick an overload by its signature, then call it. The tuple's types are
    // checked against the signature first.
    let concat = string
        .methods()
        .find(|m| m.signature() == "static System.String Concat(System.String, System.String)")
        .expect("String.Concat(string, string)");
    let joined: Il2CppString = concat.call((il2cpp.string("Kia ")?, il2cpp.string("ora")?))?;
    println!("String.Concat(\"Kia \", \"ora\") = {:?}", joined.to_rust());

    let max = il2cpp
        .class("mscorlib", "System", "Math")?
        .methods()
        .find(|m| m.signature() == "static System.Int32 Max(System.Int32, System.Int32)");
    if let Some(max) = max {
        println!("Math.Max(3, 7) = {}", max.call::<_, i32>((3, 7))?);
    }

    // A wrong type is an error rather than a crash.
    if let Err(e) = concat.call::<_, i32>((1, 2)) {
        println!("calling Concat with ints: {}", e);
    }
    Ok(())
}
