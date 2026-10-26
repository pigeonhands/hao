use hao::Module;

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: load_module <assembly>");
    let module = Module::from_path(path).unwrap();
    println!("loaded {}", module.name());
}
