mod common;

use hao::{
    dotnet::{
        entries::{CANamedArgument, CAValue, CustomAttribute, CustomAttributeCtor, CustomAttributeValue},
        il::MemberToken,
        signature::{MethodSig, TypeSig},
    },
    Module,
};

#[test]
fn decode_fixed_and_named_arguments() {
    let module = Module::from_bytes(&sample!().bytes).unwrap();
    let circle = module.find_type("Sample", "Circle").unwrap();
    let attribute = circle.get().custom_attributes[0].clone();
    let value = attribute.decode(&module).unwrap();

    assert_eq!(value.fixed, [CAValue::from("circle")]);
    assert_eq!(value.named("Weight"), Some(&CAValue::I4(3)));
    assert!(!value.named[0].is_field);
    assert_eq!(value.encode().unwrap(), attribute.value);
}

#[test]
fn decode_enum_from_dependency() {
    let module = Module::from_path(sample!().dir.join("Sample.dll")).unwrap();
    let tag = module.find_type("Sample", "TagAttribute").unwrap();
    let usage = tag
        .get()
        .custom_attributes
        .iter()
        .find(|a| a.type_name() == "System.AttributeUsageAttribute")
        .cloned()
        .unwrap();
    let value = usage.decode(&module).unwrap();
    match &value.fixed[0] {
        CAValue::Enum { type_name, value } => {
            assert_eq!(type_name, "System.AttributeTargets");
            assert_eq!(**value, CAValue::I4(0x7FFF));
        }
        other => panic!("expected an enum, got {:?}", other),
    }
}

#[test]
fn add_attribute() {
    let mut module = Module::from_bytes(&sample!().bytes).unwrap();
    let obsolete = module.import_type_from("System.Runtime", "System.ObsoleteAttribute").unwrap();
    let ctor = module
        .import_method_on(&obsolete, ".ctor", MethodSig::new_instance(TypeSig::VOID, vec![TypeSig::STRING]))
        .unwrap();
    let MemberToken::MemberRef(ctor) = ctor else { panic!() };

    let value = CustomAttributeValue {
        fixed: vec!["patched by hao".into()],
        named: vec![CANamedArgument::property("UrlFormat", "https://example.com/{0}".into())],
    };
    let attribute = CustomAttribute::new(CustomAttributeCtor::MemberRef(ctor), &value).unwrap();
    module
        .find_type("Sample", "Calc")
        .unwrap()
        .get_mut()
        .custom_attributes
        .push(attribute);

    let written = Module::from_bytes(&module.write().unwrap()).unwrap();
    let calc = written.find_type("Sample", "Calc").unwrap();
    let attribute = calc
        .get()
        .custom_attributes
        .iter()
        .find(|a| a.type_name() == "System.ObsoleteAttribute")
        .cloned()
        .unwrap();
    assert_eq!(attribute.decode(&written).unwrap(), value);
}

/// Decodes and re-encodes every custom attribute in the shared framework.
#[test]
#[ignore]
fn round_trip_shared_framework() {
    let dir = hao::resolver::PathAssemblyResolver::find_framework_dir().unwrap();
    let (mut total, mut mismatched) = (0, Vec::new());
    let mut failures = Vec::new();
    for file in std::fs::read_dir(&dir).unwrap() {
        let path = file.unwrap().path();
        if path.extension().is_none_or(|e| e != "dll") {
            continue;
        }
        let Ok(module) = Module::from_path(&path) else { continue };
        let attributes = module
            .all_types()
            .iter()
            .flat_map(|t| {
                let t = t.get();
                let mut all = t.custom_attributes.clone();
                for m in &t.methods {
                    all.extend(m.get().custom_attributes.clone());
                }
                for f in &t.fields {
                    all.extend(f.get().custom_attributes.clone());
                }
                all
            })
            .chain(module.assembly.iter().flat_map(|a| a.get().custom_attributes.clone()))
            .collect::<Vec<_>>();
        for attribute in attributes {
            total += 1;
            match attribute.decode(&module) {
                Ok(value) => {
                    if value.encode().unwrap() != attribute.value {
                        mismatched.push(format!("{}: {}", path.display(), attribute.type_name()));
                    }
                }
                Err(e) => failures.push(format!("{}: {} {}", path.display(), attribute.type_name(), e)),
            }
        }
    }
    println!("{} attributes, {} failed to decode", total, failures.len());
    for f in failures.iter().take(10) {
        println!("  {}", f);
    }
    assert!(mismatched.is_empty(), "re-encoded differently:\n{}", mismatched.join("\n"));
    assert!(failures.is_empty());
}
