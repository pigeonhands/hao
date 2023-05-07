use hao::{Module, dotnet::{metadata::Metadata, md::streams::tables_stream::FieldFlags}};

fn main() {
    let data = std::fs::read(r#"C:\re\dnspy\bin\dnlib.dll"#).unwrap();

    let md = Metadata::parse(&data).unwrap();

    println!("{:#?}", md.metadata_streams.tables_stream.header);

    let module = Module::from_metadada(&md).unwrap();

    for ty in module.types().values() {
        println!("{} {{", ty);
        if ty.is_enum() {
            for field in ty.fields().values().filter(|x| !x.flags().contains(FieldFlags::SpecialName)) {
                println!("\t{},", field.name());
            }
        }else {
            for field in ty.fields().values() {
                println!("\t{};", field);
            }
        }
       
        println!("}}");

        
        if ty.namespace() == "dnlib.DotNet.Pdb.Dss" && ty.name() == "SymbolScopeImpl" {
            break;
        }
    }
}
