use hao::{dotnet::md::streams::tables_stream::FieldFlags, Module};

fn main() {
    let module = Module::from_path(r#"C:\re\dnspy\bin\dnlib.dll"#).unwrap();
    println!("loaded");

    for ty in module.types().values() {
        println!("{} {{", ty);
        if ty.is_enum() {
            for field in ty
                .fields()
                .values()
                .filter(|x| !x.flags().contains(FieldFlags::SpecialName))
            {
                println!("\t{},", field.name());
            }
        } else {
            for field in ty.fields().values() {
                println!("\t{};", field);
            }
        }

        println!("\n");

        for method in ty.methods().values() {
            println!("\t{};", method);
        }

        println!("}}");
    }
}
