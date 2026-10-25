use hao::Module;

fn main() {
    let path = std::env::args().nth(1).expect("usage: simple <assembly>");
    let module = Module::from_path_no_resolve(path).unwrap();

    for ty in module.all_types() {
        let ty = ty.get();
        println!("{}", ty.full_name());
        for field in &ty.fields {
            println!("\tfield {}", field.get().name);
        }
        for method in &ty.methods {
            let method = method.get();
            let size = method.body.as_ref().map_or(0, |b| b.instructions.len());
            println!("\tmethod {} ({} instructions)", method.name, size);
        }
    }
}
