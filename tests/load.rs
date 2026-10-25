use hao::{
    dotnet::{
        entries::{ResourceData, TypeDefOrRef},
        il::{OpCode, Operand},
        signature::{PrimitiveType, TypeSig},
    },
    Module,
};

const SAMPLE: &[u8] = include_bytes!("fixtures/Sample.dll");

fn sample() -> Module {
    Module::from_bytes(SAMPLE).unwrap()
}

#[test]
fn loads_types() {
    let module = sample();
    assert_eq!(module.name(), "Sample.dll");
    assert_eq!(module.types[0].get().name, "<Module>");

    let calc = module.find_type("Sample", "Calc").unwrap();
    let calc = calc.get();
    assert!(calc.is_static());
    let add = calc.find_method("Add").unwrap();
    let add = add.get();
    assert_eq!(add.signature.ret, TypeSig::Primitive(PrimitiveType::I4));
    assert_eq!(add.signature.params.len(), 2);
    assert_eq!(add.param_name(0), "a");

    let body = add.body.as_ref().unwrap();
    let ops: Vec<OpCode> = body.instructions.iter().map(|i| i.opcode).collect();
    assert_eq!(ops, [OpCode::Ldarg_0, OpCode::Ldarg_1, OpCode::Add, OpCode::Ret]);
}

#[test]
fn nested_and_generic_types() {
    let module = sample();
    let boxed = module.find_type_by_full_name("Sample.Box`1/Nested").unwrap();
    assert_eq!(boxed.get().full_name(), "Sample.Box`1/Nested");

    let generic = module.find_type("Sample", "Box`1").unwrap();
    let generic = generic.get();
    assert_eq!(generic.generic_params.len(), 1);
    assert_eq!(generic.generic_params[0].get().name, "T");
    assert_eq!(generic.generic_params[0].get().constraints.len(), 1);
    assert_eq!(generic.events.len(), 1);
    let event = generic.events[0].get();
    assert!(event.add.is_some() && event.remove.is_some());
}

#[test]
fn enums_structs_and_layout() {
    let module = sample();
    let colour = module.find_type("Sample", "Colour").unwrap();
    let colour = colour.get();
    assert!(colour.is_enum() && colour.is_value_type());
    let underlying = colour.enum_underlying_field().unwrap();
    assert_eq!(
        underlying.get().signature.ty,
        TypeSig::Primitive(PrimitiveType::U1)
    );
    let blue = colour.find_field("Blue").unwrap();
    assert_eq!(blue.get().constant.as_ref().unwrap().as_i64(), Some(4));

    let union = module.find_type("Sample", "Union").unwrap();
    let union = union.get();
    assert_eq!(union.class_layout.unwrap().class_size, 8);
    assert_eq!(union.find_field("High").unwrap().get().offset, Some(4));

    let point = module.find_type("Sample", "Point").unwrap();
    assert!(point.get().is_value_type());
    let shape = module.find_type("Sample", "IShape").unwrap();
    assert!(shape.get().is_interface());
    assert_eq!(shape.get().properties.len(), 1);
}

#[test]
fn custom_attributes_resources_and_pinvoke() {
    let module = sample();
    let circle = module.find_type("Sample", "Circle").unwrap();
    let names: Vec<String> = circle
        .get()
        .custom_attributes
        .iter()
        .map(|a| a.type_name())
        .collect();
    assert_eq!(names, ["Sample.TagAttribute"]);

    let resource = module.resources[0].get();
    assert_eq!(resource.name, "Sample.data.txt");
    assert_eq!(
        resource.data,
        ResourceData::Embedded(b"embedded resource payload\n".to_vec())
    );

    let native = module.find_type("Sample", "Native").unwrap();
    let get_pid = native.get().find_method("GetPid").unwrap();
    let get_pid = get_pid.get();
    let impl_map = get_pid.impl_map.as_ref().unwrap();
    assert_eq!(impl_map.import_name, "getpid");
    assert_eq!(impl_map.scope.get().name, "libc");
}

#[test]
fn field_rva_switch_and_exception_handlers() {
    let module = sample();
    let primes: Vec<u8> = module
        .all_fields()
        .iter()
        .filter_map(|f| f.get().initial_value.clone())
        .find(|v| v.len() == 32)
        .unwrap();
    assert_eq!(&primes[..8], &[2, 0, 0, 0, 3, 0, 0, 0]);

    let calc = module.find_type("Sample", "Calc").unwrap();
    let classify = calc.get().find_method("Classify").unwrap();
    let classify = classify.get();
    let body = classify.body.as_ref().unwrap();
    let switch = body
        .instructions
        .iter()
        .find(|i| i.opcode == OpCode::Switch)
        .unwrap();
    assert!(matches!(&switch.operand, Operand::Switch(t) if t.len() == 4));

    let guarded = calc.get().find_method("Guarded").unwrap();
    let guarded = guarded.get();
    let handlers = &guarded.body.as_ref().unwrap().exception_handlers;
    assert_eq!(handlers.len(), 2);

    let main = module.entry_point.clone().unwrap();
    assert_eq!(main.get().name, "Main");
    let ldstr = main
        .get()
        .body
        .as_ref()
        .unwrap()
        .instructions
        .iter()
        .find_map(|i| match &i.operand {
            Operand::String(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(ldstr, "Hello from hao");
}

#[test]
fn type_refs_resolve_to_corlib() {
    let module = sample();
    let calc = module.find_type("Sample", "Calc").unwrap();
    let extends = calc.get().extends.clone().unwrap();
    assert!(matches!(&extends, TypeDefOrRef::TypeRef(r) if r.get().is_corlib()));
    assert_eq!(extends.full_name(), "System.Object");
}

/// Loads every assembly in the installed shared framework.
#[test]
#[ignore]
fn load_shared_framework() {
    let root = std::path::Path::new("/usr/share/dotnet/shared/Microsoft.NETCore.App");
    let mut failures = Vec::new();
    let mut count = 0;
    for version in std::fs::read_dir(root).unwrap() {
        for file in std::fs::read_dir(version.unwrap().path()).unwrap() {
            let path = file.unwrap().path();
            if path.extension().is_some_and(|e| e == "dll") {
                match Module::from_path_no_resolve(&path) {
                    Ok(_) => count += 1,
                    Err(hao::error::HaoError::NotDotNetBinary) => {}
                    Err(e) => failures.push(format!("{}: {}", path.display(), e)),
                }
            }
        }
    }
    assert!(failures.is_empty(), "{} loaded, failures:\n{}", count, failures.join("\n"));
    println!("loaded {} assemblies", count);
}

#[cfg(feature = "sync")]
#[test]
fn module_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Module>();
}
