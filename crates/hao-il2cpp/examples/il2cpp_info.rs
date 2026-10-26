//! Prints the images of an Il2Cpp game, and the members of types matching a filter.
//!
//! ```sh
//! cargo run -p hao-il2cpp --example il2cpp_info -- <global-metadata.dat> [filter] [GameAssembly.so]
//! ```

use hao_il2cpp::{Binary, ElfImage, Metadata};

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: il2cpp_info <global-metadata.dat> [filter] [GameAssembly.so]");
    let filter = args.next();
    let binary_data = args.next().map(|p| std::fs::read(p).unwrap());

    let metadata = Metadata::from_path(path).unwrap();
    let image = binary_data.as_deref().map(|d| ElfImage::parse(d).unwrap());
    let binary = image.as_ref().map(|i| Binary::read(i, &metadata).unwrap());
    let type_name = |index: i32| match &binary {
        Some(b) => b.type_at(index).map(|t| t.name(&metadata)).unwrap_or_default(),
        None => format!("#{}", index),
    };

    println!(
        "metadata v{}: {} images, {} types, {} methods, {} fields",
        metadata.version,
        metadata.images.len(),
        metadata.type_definitions.len(),
        metadata.methods.len(),
        metadata.fields.len()
    );
    let Some(filter) = filter else {
        for image in &metadata.images {
            println!("  {} ({} types)", metadata.image_name(image), image.type_count);
        }
        return;
    };
    for i in 0..metadata.type_definitions.len() as i32 {
        let name = metadata.type_full_name(i);
        if !name.contains(&filter) {
            continue;
        }
        let t = &metadata.type_definitions[i as usize];
        println!("{}", name);
        for f in metadata.type_fields(t) {
            println!("    {} {};", type_name(f.type_index), metadata.string(f.name_index));
        }
        for m in metadata.type_methods(t) {
            let params: Vec<String> = metadata
                .method_parameters(m)
                .iter()
                .map(|p| format!("{} {}", type_name(p.type_index), metadata.string(p.name_index)))
                .collect();
            println!("    {} {}({});", type_name(m.return_type), metadata.string(m.name_index), params.join(", "));
        }
    }
}
