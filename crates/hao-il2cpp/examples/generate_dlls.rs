//! Writes a declaration assembly for every image of an Il2Cpp game.
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
