use hao::{Metadata, dotnet::ModuleDefMD};

fn main() {
    let data = std::fs::read(r#"C:\re\dnspy\bin\dnlib.dll"#).unwrap();

    let md = Metadata::parse(&data).unwrap();

    println!("loaded");
    println!("{:#?}", md.metadata_streams.tables_stream.header);

    let module = ModuleDefMD::from_metadada(&md).unwrap();
    println!("{:#?}", module);
}
