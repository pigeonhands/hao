//! Shows an Il2Cpp game's assemblies, or a type's members.
//!
//! ```sh
//! cargo run --release -p hao-il2cpp --example game_info -- <game folder>
//! cargo run --release -p hao-il2cpp --example game_info -- <game folder> System.String
//! ```

use hao_il2cpp::Game;

fn main() -> hao_il2cpp::Result<()> {
    let mut args = std::env::args().skip(1);
    let dir = args.next().expect("usage: game_info <game folder> [type name]");
    let game = Game::open(dir)?;

    let Some(type_name) = args.next() else {
        println!("{} types in {} assemblies:", game.types().count(), game.image_names().count());
        for name in game.image_names() {
            println!("  {}", name);
        }
        return Ok(());
    };

    let Some(ty) = game.find_type(&type_name) else {
        println!("no type named {}; similar names:", type_name);
        for t in game.types().filter(|t| t.name().contains(type_name.rsplit('.').next().unwrap_or(&type_name))).take(10) {
            println!("  {}", t.full_name());
        }
        return Ok(());
    };

    print!("// {}\nclass {}", ty.image_name(), ty.full_name());
    if let Some(base) = ty.base_type_name() {
        print!(" : {}", base);
    }
    println!("\n{{");
    for field in ty.fields() {
        println!("    {};", field);
    }
    for method in ty.methods() {
        println!("    {};", method);
    }
    for nested in ty.nested_types() {
        println!("    class {}", nested.name());
    }
    println!("}}");
    Ok(())
}
