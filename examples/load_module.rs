use hao::{Module};

fn main() {
    let module = Module::from_path(r#"C:\re\dnspy\bin\dnlib.dll"#).unwrap();
    println!("loaded {}", module.module().value().name());
}
