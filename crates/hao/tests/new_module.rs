use std::process::Command;

use hao::{
    dotnet::{
        entries::{Field, Method, Param, TypeDef, Version},
        il::{Instruction, MethodBody, OpCode, Operand},
        md::streams::tables_stream::{FieldFlags, MethodFlags, TypeAttributes},
        module::{ModuleKind, TargetRuntime},
        signature::{FieldSig, MethodSig, TypeSig},
    },
    Module,
};

fn body(instructions: Vec<Instruction>) -> Option<MethodBody> {
    Some(MethodBody {
        instructions,
        ..Default::default()
    })
}

fn hello_world() -> Module {
    let mut module = Module::new("Hello", ModuleKind::Console, TargetRuntime::NetCore(9));
    module.add_assembly_ref("System.Console", Version { major: 9, ..Default::default() }, &TargetRuntime::NetCore(9).framework_token());

    let object = module.corlib_type("System.Object").unwrap();
    let object_ctor = module
        .import_method_on(&object, ".ctor", MethodSig::new_instance(TypeSig::VOID, vec![]))
        .unwrap();
    let console = module.import_type_from("System.Console", "System.Console").unwrap();
    let write_line = module
        .import_method_on(&console, "WriteLine", MethodSig::new_static(TypeSig::VOID, vec![TypeSig::STRING]))
        .unwrap();
    let string = module.corlib_type("System.String").unwrap();
    let concat = module
        .import_method_on(
            &string,
            "Concat",
            MethodSig::new_static(TypeSig::STRING, vec![TypeSig::STRING, TypeSig::STRING]),
        )
        .unwrap();

    let public_class = TypeAttributes::Public | TypeAttributes::BeforeFieldInit;
    let greeter = module.add_type(TypeDef::new(public_class, "Hello", "Greeter", Some(object.clone())));
    let name = TypeDef::add_field(&greeter, Field::new(FieldFlags::Private, "name", FieldSig { ty: TypeSig::STRING }));

    let ctor_flags = MethodFlags::Public | MethodFlags::HideBySig | MethodFlags::SpecialName | MethodFlags::RTSpecialName;
    let ctor = TypeDef::add_method(
        &greeter,
        Method::new(ctor_flags, ".ctor", MethodSig::new_instance(TypeSig::VOID, vec![TypeSig::STRING])),
    );
    {
        let mut c = ctor.get_mut();
        c.params.push(hao::dotnet::entries::Entry::new(Param::new(1, "name")));
        c.body = body(vec![
            Instruction::ldarg(0),
            Instruction::call(object_ctor),
            Instruction::ldarg(0),
            Instruction::ldarg(1),
            Instruction::new(OpCode::Stfld, Operand::Token(name.into())),
            Instruction::ret(),
        ]);
    }

    let greet = TypeDef::add_method(
        &greeter,
        Method::new(MethodFlags::Public | MethodFlags::HideBySig, "Greet", MethodSig::new_instance(TypeSig::STRING, vec![])),
    );
    greet.get_mut().body = body(vec![
        Instruction::ldstr("Hello, "),
        Instruction::ldarg(0),
        Instruction::new(OpCode::Ldfld, Operand::Token(greeter.get().fields[0].clone().into())),
        Instruction::call(concat),
        Instruction::ret(),
    ]);

    let program = module.add_type(TypeDef::new(
        TypeAttributes::Public | TypeAttributes::Abstract | TypeAttributes::Sealed,
        "Hello",
        "Program",
        Some(object),
    ));
    let main = TypeDef::add_method(
        &program,
        Method::new(
            MethodFlags::Public | MethodFlags::Static | MethodFlags::HideBySig,
            "Main",
            MethodSig::new_static(TypeSig::VOID, vec![]),
        ),
    );
    main.get_mut().body = body(vec![
        Instruction::ldstr("hao"),
        Instruction::newobj(ctor),
        Instruction::callvirt(greet),
        Instruction::call(write_line),
        Instruction::ret(),
    ]);
    module.entry_point = Some(main);
    module
}

#[test]
fn write_new_module() {
    let module = hello_world();
    let data = module.write().unwrap();

    let reloaded = Module::from_bytes(&data).unwrap();
    assert_eq!(reloaded.name(), "Hello.exe");
    assert_eq!(reloaded.find_method("Hello.Greeter", "Greet").unwrap().get().name, "Greet");
    assert_eq!(reloaded.entry_point.as_ref().unwrap().get().name, "Main");

    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("new_module");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("Hello.dll"), &data).unwrap();
    std::fs::write(
        dir.join("Hello.runtimeconfig.json"),
        r#"{"runtimeOptions":{"tfm":"net9.0","framework":{"name":"Microsoft.NETCore.App","version":"9.0.0"}}}"#,
    )
    .unwrap();
    let Ok(output) = Command::new("dotnet").arg(dir.join("Hello.dll")).output() else {
        eprintln!("skipping run: dotnet not installed");
        return;
    };
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "Hello, hao\n");
}
