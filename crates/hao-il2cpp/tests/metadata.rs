//! Runs against a real game's metadata. Set `HAO_IL2CPP_METADATA` to a
//! `global-metadata.dat` (v29), otherwise an installed Megabonk is used if found.

use hao_il2cpp::Metadata;

fn metadata() -> Option<Metadata> {
    let path = std::env::var("HAO_IL2CPP_METADATA").ok().unwrap_or_else(|| {
        "/games/SteamLibrary/steamapps/common/Megabonk/Megabonk_Data/il2cpp_data/Metadata/global-metadata.dat".into()
    });
    if !std::path::Path::new(&path).exists() {
        eprintln!("skipping: no il2cpp metadata at {} (set HAO_IL2CPP_METADATA)", path);
        return None;
    }
    Some(Metadata::from_path(path).unwrap())
}

#[test]
fn reads_images_and_types() {
    let Some(metadata) = metadata() else { return };
    let names: Vec<&str> = metadata.images.iter().map(|i| metadata.image_name(i)).collect();
    assert!(names.contains(&"mscorlib.dll"));
    assert!(names.contains(&"Assembly-CSharp.dll"));
    let total: u32 = metadata.images.iter().map(|i| i.type_count).sum();
    assert_eq!(total as usize, metadata.type_definitions.len());
}

#[test]
fn reads_members_of_system_string() {
    let Some(metadata) = metadata() else { return };
    let string = (0..metadata.type_definitions.len() as i32)
        .find(|i| metadata.type_full_name(*i) == "System.String")
        .unwrap();
    let t = &metadata.type_definitions[string as usize];
    let fields: Vec<&str> = metadata.type_fields(t).iter().map(|f| metadata.string(f.name_index)).collect();
    assert!(fields.contains(&"_stringLength") && fields.contains(&"_firstChar"));

    let concat = metadata
        .type_methods(t)
        .iter()
        .find(|m| metadata.string(m.name_index) == "Concat" && m.parameter_count == 2)
        .unwrap();
    let params: Vec<&str> = metadata.method_parameters(concat).iter().map(|p| metadata.string(p.name_index)).collect();
    assert_eq!(params.len(), 2);
}

#[test]
fn nested_type_names() {
    let Some(metadata) = metadata() else { return };
    assert!((0..metadata.type_definitions.len() as i32)
        .any(|i| metadata.type_full_name(i) == "System.Collections.Generic.Dictionary`2/Enumerator"));
}
