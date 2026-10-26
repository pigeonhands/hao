//! Writes a declaration assembly ("dummy DLL") for every image of an Il2Cpp game.
//!
//! An Il2Cpp game ships no .NET assemblies: its C# was compiled to C++ and then
//! to native code in `GameAssembly`. What's left of the original assemblies is
//! `global-metadata.dat`, with every type, method and field name, but no code.
//!
//! This rebuilds one ordinary .NET assembly per original assembly from that
//! metadata: every class, struct, enum, interface, field, constant, property,
//! event and method signature, with method bodies that just throw. They're
//! useful for:
//!
//! - browsing the game in ILSpy, dnSpy or Rider to find the classes, methods and
//!   fields a mod should hook or read, with their exact signatures;
//! - compiling C# against the game's API, e.g. for tooling or to check a mod's
//!   references, since the declarations match the real types;
//! - comparing game versions to see what an update changed.
//!
//! They contain no game logic: for that you'd disassemble `GameAssembly`.
//!
//! ```sh
//! cargo run --release -p hao-il2cpp --example generate_dlls -- <game folder> <output folder>
//! ```

use hao_il2cpp::Game;

fn main() -> hao_il2cpp::Result<()> {
    let mut args = std::env::args().skip(1);
    let (Some(game), Some(out)) = (args.next(), args.next()) else {
        panic!("usage: generate_dlls <game folder> <output folder>");
    };
    let paths = Game::open(game)?.write_assemblies(&out)?;
    println!("wrote {} assemblies to {}", paths.len(), out);
    Ok(())
}
