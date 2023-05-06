use hao::{dotnet::ModuleDefMD, Metadata};

fn main() {
    let data = std::fs::read(r#"C:\re\dnspy\bin\dnlib.dll"#).unwrap();

    let md = Metadata::parse(&data).unwrap();

    println!("{:#?}", md.metadata_streams.tables_stream.header);

    let module = ModuleDefMD::from_metadada(&md).unwrap();

    for ty in module.types() {
        println!("{}.{}", ty.namespace, ty.name);
    }

    for ty in module.methods.iter().filter(|x| !x.is_refrenced()) {
        let ty = ty.value();
        println!("methods  {}", ty.name);
    }

    println!("non refrenced methods: {}", module.methods.iter().filter(|x| !x.is_refrenced()).count());

    println!("loaded");
}
