//! Runs against a real game. Set `HAO_IL2CPP_GAME` to an Il2Cpp game folder with a
//! v29 `GameAssembly.so`, otherwise an installed Megabonk is used if found.

use hao_il2cpp::{binary::TypeKind, Binary, ElfImage, Metadata};

struct Game {
    metadata: Metadata,
    binary: Binary,
}

fn game() -> Option<Game> {
    let dir = std::env::var("HAO_IL2CPP_GAME").unwrap_or_else(|_| "/games/SteamLibrary/steamapps/common/Megabonk".into());
    let dir = std::path::Path::new(&dir);
    let binary = dir.join("GameAssembly.so");
    let metadata = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .map(|p| p.join("il2cpp_data/Metadata/global-metadata.dat"))
        .find(|p| p.exists());
    let (Some(metadata), true) = (metadata, binary.exists()) else {
        eprintln!("skipping: no Il2Cpp game at {} (set HAO_IL2CPP_GAME)", dir.display());
        return None;
    };
    let metadata = Metadata::from_path(metadata).unwrap();
    let data = std::fs::read(binary).unwrap();
    let image = ElfImage::parse(&data).unwrap();
    let binary = Binary::read(&image, &metadata).unwrap();
    Some(Game { metadata, binary })
}

fn field_types(game: &Game, type_name: &str) -> Vec<(String, String)> {
    let m = &game.metadata;
    let t = (0..m.type_definitions.len() as i32).find(|i| m.type_full_name(*i) == type_name).unwrap();
    m.type_fields(&m.type_definitions[t as usize])
        .iter()
        .map(|f| (m.string(f.name_index).to_owned(), game.binary.type_at(f.type_index).unwrap().name(m)))
        .collect()
}

#[test]
fn decodes_the_whole_type_table() {
    let Some(game) = game() else { return };
    let definitions = game.metadata.type_definitions.len() as i32;
    for ty in &game.binary.types {
        match &ty.kind {
            TypeKind::Unknown(e) => panic!("unknown element type {:#x}", e),
            TypeKind::Class(i) | TypeKind::ValueType(i) => assert!((0..definitions).contains(i)),
            _ => {}
        }
    }
    // Every type definition's own type points back at it.
    for (i, def) in game.metadata.type_definitions.iter().enumerate() {
        match &game.binary.type_at(def.byval_type_index).unwrap().kind {
            TypeKind::Class(t) | TypeKind::ValueType(t) => assert_eq!(*t, i as i32),
            TypeKind::GenericInst { .. } => {}
            // Built in types such as System.String use their element type.
            TypeKind::Primitive(_) if game.metadata.type_full_name(i as i32).starts_with("System.") => {}
            other => panic!("{} has byval type {:?}", game.metadata.type_full_name(i as i32), other),
        }
    }
}

#[test]
fn field_types_of_framework_types() {
    let Some(game) = game() else { return };
    let string = field_types(&game, "System.String");
    assert!(string.contains(&("_stringLength".into(), "int".into())));
    assert!(string.contains(&("_firstChar".into(), "char".into())));
    assert!(string.contains(&("Empty".into(), "string".into())));

    let dictionary = field_types(&game, "System.Collections.Generic.Dictionary`2");
    assert!(dictionary.contains(&(
        "_entries".into(),
        "System.Collections.Generic.Dictionary/Entry<TKey, TValue>[]".into()
    )));
    assert!(dictionary.contains(&("_comparer".into(), "System.Collections.Generic.IEqualityComparer<TKey>".into())));
}

#[test]
fn method_signatures() {
    let Some(game) = game() else { return };
    let m = &game.metadata;
    let string = (0..m.type_definitions.len() as i32).find(|i| m.type_full_name(*i) == "System.String").unwrap();
    let def = &m.type_definitions[string as usize];
    let signature = |name: &str| -> Vec<String> {
        m.type_methods(def)
            .iter()
            .filter(|method| m.string(method.name_index) == name)
            .map(|method| {
                let params: Vec<String> = m
                    .method_parameters(method)
                    .iter()
                    .map(|p| game.binary.type_at(p.type_index).unwrap().name(m))
                    .collect();
                format!("{} {}({})", game.binary.type_at(method.return_type).unwrap().name(m), name, params.join(", "))
            })
            .collect()
    };
    assert!(signature("Concat").contains(&"string Concat(string, string)".to_owned()));
    assert!(signature("get_Length").contains(&"int get_Length()".to_owned()));
    assert!(signature("Split").iter().any(|s| s == "string[] Split(char[])"), "{:?}", signature("Split"));
}

#[test]
fn generates_declaration_assemblies() {
    let Some(game) = game() else { return };
    let assemblies = hao_il2cpp::generate(&game.metadata, &game.binary).unwrap();
    assert_eq!(assemblies.len(), game.metadata.images.len());

    let mut total_types = 0;
    let mut mscorlib = None;
    for assembly in &assemblies {
        let written = assembly.module.write().unwrap();
        let reloaded = hao::Module::from_bytes(&written).unwrap();
        total_types += reloaded.all_types().iter().filter(|t| t.get().name != "<Module>").count();
        if assembly.file_name == "mscorlib.dll" {
            mscorlib = Some(reloaded);
        }
    }
    let modules = game.metadata.type_definitions.iter().filter(|t| game.metadata.string(t.name_index) == "<Module>").count();
    assert_eq!(total_types, game.metadata.type_definitions.len() - modules);

    let mscorlib = mscorlib.unwrap();
    // Unused constants are stripped by Il2Cpp, but enum members are kept.
    let day = mscorlib.find_type("System", "DayOfWeek").unwrap();
    let constant = |name: &str| day.get().find_field(name).unwrap().get().constant.clone().unwrap().as_i64();
    assert_eq!(constant("Sunday"), Some(0));
    assert_eq!(constant("Saturday"), Some(6));

    let dictionary = mscorlib.find_type("System.Collections.Generic", "Dictionary`2").unwrap();
    let dictionary = dictionary.get();
    assert_eq!(dictionary.generic_params.len(), 2);
    assert!(dictionary
        .custom_attributes
        .iter()
        .any(|a| a.type_name() == "System.Reflection.DefaultMemberAttribute"));
    assert!(dictionary.find_method("TryGetValue").is_some());
}
