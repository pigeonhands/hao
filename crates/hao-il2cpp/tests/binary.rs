//! Runs against a real game. Set `HAO_IL2CPP_GAME` to an Il2Cpp game folder with a
//! v29 `GameAssembly.so`. Skipped otherwise.

use hao_il2cpp::{binary::TypeKind, Game};

fn game() -> Option<Game> {
    let Some(dir) = std::env::var_os("HAO_IL2CPP_GAME") else {
        eprintln!("skipping: set HAO_IL2CPP_GAME to an Il2Cpp game folder");
        return None;
    };
    Some(Game::open(dir).unwrap())
}

fn fields(game: &Game, type_name: &str) -> Vec<String> {
    game.find_type(type_name)
        .unwrap()
        .fields()
        .map(|f| f.to_string())
        .collect()
}

#[test]
fn decodes_the_whole_type_table() {
    let Some(game) = game() else { return };
    let definitions = game.metadata().type_definitions.len() as i32;
    for ty in &game.binary().types {
        match &ty.kind {
            TypeKind::Unknown(e) => panic!("unknown element type {:#x}", e),
            TypeKind::Class(i) | TypeKind::ValueType(i) => assert!((0..definitions).contains(i)),
            _ => {}
        }
    }
    // Every type definition's own type points back at it.
    for ty in game.types() {
        match &game
            .binary()
            .type_at(ty.definition().byval_type_index)
            .unwrap()
            .kind
        {
            TypeKind::Class(t) | TypeKind::ValueType(t) => assert_eq!(*t, ty.index()),
            TypeKind::GenericInst { .. } => {}
            // Built in types such as System.String use their element type.
            TypeKind::Primitive(_) if ty.namespace() == "System" => {}
            other => panic!("{} has byval type {:?}", ty.full_name(), other),
        }
    }
}

#[test]
fn field_types_of_framework_types() {
    let Some(game) = game() else { return };
    let string = fields(&game, "System.String");
    assert!(string.contains(&"int _stringLength".to_owned()));
    assert!(string.contains(&"char _firstChar".to_owned()));
    assert!(string.contains(&"static string Empty".to_owned()));

    let dictionary = fields(&game, "System.Collections.Generic.Dictionary`2");
    assert!(dictionary.contains(
        &"System.Collections.Generic.Dictionary/Entry<TKey, TValue>[] _entries".to_owned()
    ));
    assert!(dictionary
        .contains(&"System.Collections.Generic.IEqualityComparer<TKey> _comparer".to_owned()));
}

#[test]
fn method_signatures() {
    let Some(game) = game() else { return };
    let string = game.find_type("System.String").unwrap();
    let overloads = |name: &str| -> Vec<String> {
        string
            .methods()
            .filter(|m| m.name() == name)
            .map(|m| {
                format!(
                    "{} {}({})",
                    m.return_type_name(),
                    name,
                    m.parameter_type_names().join(", ")
                )
            })
            .collect()
    };
    assert!(overloads("Concat").contains(&"string Concat(string, string)".to_owned()));
    assert!(overloads("get_Length").contains(&"int get_Length()".to_owned()));
    assert!(overloads("Split").contains(&"string[] Split(char[])".to_owned()));
    assert!(string.find_method("IsNullOrEmpty").unwrap().is_static());
}

#[test]
fn generates_declaration_assemblies() {
    let Some(game) = game() else { return };
    let out = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("il2cpp_assemblies");
    let paths = game.write_assemblies(&out).unwrap();
    assert_eq!(paths.len(), game.image_names().count());

    let modules: Vec<hao::Module> = paths
        .iter()
        .map(|p| hao::Module::from_path_no_resolve(p).unwrap())
        .collect();
    let generated_types: usize = modules
        .iter()
        .map(|m| {
            m.all_types()
                .iter()
                .filter(|t| t.get().name != "<Module>")
                .count()
        })
        .sum();
    assert_eq!(
        generated_types,
        game.types().filter(|t| t.name() != "<Module>").count()
    );

    let mscorlib = modules.iter().find(|m| m.name() == "mscorlib.dll").unwrap();
    // Unused constants are stripped by Il2Cpp, but enum members are kept.
    let day = mscorlib.find_type("System", "DayOfWeek").unwrap();
    let constant = |name: &str| {
        day.get()
            .find_field(name)
            .unwrap()
            .get()
            .constant
            .clone()
            .unwrap()
            .as_i64()
    };
    assert_eq!(constant("Sunday"), Some(0));
    assert_eq!(constant("Saturday"), Some(6));

    let dictionary = mscorlib
        .find_type("System.Collections.Generic", "Dictionary`2")
        .unwrap();
    let dictionary = dictionary.get();
    assert_eq!(dictionary.generic_params.len(), 2);
    assert!(dictionary
        .custom_attributes
        .iter()
        .any(|a| a.type_name() == "System.Reflection.DefaultMemberAttribute"));
    assert!(dictionary.find_method("TryGetValue").is_some());
}
