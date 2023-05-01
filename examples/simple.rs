use hao::Metadata;

fn main() {
    let data = std::fs::read(r#"C:\re\dnspy\bin\dnlib.dll"#).unwrap();

    let md = Metadata::parse(&data).unwrap();

    println!("loaded");
    println!("{:#?}", md.metadata_streams.tables_stream.header);
    println!("{:#?}", md.metadata_streams.tables_stream.values.assembly);
    println!("{:#?}", md.metadata_streams.tables_stream.values.assembly_ref.first());
    println!("{:#?}", md.metadata_streams.tables_stream.values.nested_class.first());
    println!("{:#?}", md.metadata_streams.tables_stream.values.generic_param.first());
    println!("{:#?}", md.metadata_streams.tables_stream.values.method_spec.first());
    println!("{:#?}", md.metadata_streams.tables_stream.values.generic_param_constraint.first());
}
