use hao::{dotnet::{ModuleDefMD}, Metadata};

fn main() {
    let data = std::fs::read(r#"C:\re\dnspy\bin\dnlib.dll"#).unwrap();

    let md = Metadata::parse(&data).unwrap();

    println!("{:#?}", md.metadata_streams.tables_stream.header);
    println!("{:#?}", md.metadata_streams.tables_stream.header.table_locations);

    let module = ModuleDefMD::from_metadada(&md).unwrap();
    println!("{:#?}", module);

    for _ in module.methods {

    }

    println!("loaded");
}
