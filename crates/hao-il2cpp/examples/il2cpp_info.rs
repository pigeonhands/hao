//! Prints the images and a few types from a `global-metadata.dat`.

use hao_il2cpp::Metadata;

fn main() {
    let path = std::env::args().nth(1).expect("usage: il2cpp_info <global-metadata.dat> [type filter]");
    let filter = std::env::args().nth(2);
    let metadata = Metadata::from_path(path).unwrap();
    println!(
        "metadata v{}: {} images, {} types, {} methods, {} fields",
        metadata.version,
        metadata.images.len(),
        metadata.type_definitions.len(),
        metadata.methods.len(),
        metadata.fields.len()
    );
    for image in &metadata.images {
        println!("  {} ({} types)", metadata.image_name(image), image.type_count);
    }
    if let Some(filter) = filter {
        for i in 0..metadata.type_definitions.len() as i32 {
            let name = metadata.type_full_name(i);
            if !name.contains(&filter) {
                continue;
            }
            let t = &metadata.type_definitions[i as usize];
            println!("{}", name);
            for f in metadata.type_fields(t) {
                println!("    field {}", metadata.string(f.name_index));
            }
            for m in metadata.type_methods(t) {
                let params: Vec<&str> = metadata.method_parameters(m).iter().map(|p| metadata.string(p.name_index)).collect();
                println!("    method {}({})", metadata.string(m.name_index), params.join(", "));
            }
        }
    }
}
