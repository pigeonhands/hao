//! Writes a declaration assembly for every image of an Il2Cpp game.
//!
//! ```sh
//! cargo run --release -p hao-il2cpp --example generate_dlls -- <game folder> <output folder>
//! ```

use hao_il2cpp::{generate, Binary, ElfImage, Metadata};

fn main() {
    let mut args = std::env::args().skip(1);
    let (game, out) = match (args.next(), args.next()) {
        (Some(g), Some(o)) => (std::path::PathBuf::from(g), std::path::PathBuf::from(o)),
        _ => panic!("usage: generate_dlls <game folder> <output folder>"),
    };
    let metadata_path = std::fs::read_dir(&game)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .map(|p| p.join("il2cpp_data/Metadata/global-metadata.dat"))
        .find(|p| p.exists())
        .expect("no il2cpp_data/Metadata/global-metadata.dat");

    let start = std::time::Instant::now();
    let metadata = Metadata::from_path(metadata_path).unwrap();
    let data = std::fs::read(game.join("GameAssembly.so")).unwrap();
    let image = ElfImage::parse(&data).unwrap();
    let binary = Binary::read(&image, &metadata).unwrap();
    println!("read metadata and binary in {:?}", start.elapsed());

    let assemblies = generate(&metadata, &binary).unwrap();
    println!("generated {} assemblies in {:?}", assemblies.len(), start.elapsed());
    std::fs::create_dir_all(&out).unwrap();
    for assembly in &assemblies {
        let path = out.join(&assembly.file_name);
        if let Err(e) = assembly.module.write_to_path(&path) {
            println!("  {}: {}", assembly.file_name, e);
        }
    }
    println!("wrote {} in {:?}", out.display(), start.elapsed());
}
