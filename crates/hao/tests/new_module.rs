mod common;

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
    module.add_assembly_ref(
        "System.Console",
        Version {
            major: 9,
            ..Default::default()
        },
        &TargetRuntime::NetCore(9).framework_token(),
    );

    let object = module.corlib_type("System.Object").unwrap();
    let object_ctor = module
        .import_method_on(
            &object,
            ".ctor",
            MethodSig::new_instance(TypeSig::VOID, vec![]),
        )
        .unwrap();
    let console = module
        .import_type_from("System.Console", "System.Console")
        .unwrap();
    let write_line = module
        .import_method_on(
            &console,
            "WriteLine",
            MethodSig::new_static(TypeSig::VOID, vec![TypeSig::STRING]),
        )
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
    let greeter = module.add_type(TypeDef::new(
        public_class,
        "Hello",
        "Greeter",
        Some(object.clone()),
    ));
    let name = TypeDef::add_field(
        &greeter,
        Field::new(
            FieldFlags::Private,
            "name",
            FieldSig {
                ty: TypeSig::STRING,
            },
        ),
    );

    let ctor_flags = MethodFlags::Public
        | MethodFlags::HideBySig
        | MethodFlags::SpecialName
        | MethodFlags::RTSpecialName;
    let ctor = TypeDef::add_method(
        &greeter,
        Method::new(
            ctor_flags,
            ".ctor",
            MethodSig::new_instance(TypeSig::VOID, vec![TypeSig::STRING]),
        ),
    );
    {
        let mut c = ctor.get_mut();
        c.params
            .push(hao::dotnet::entries::Entry::new(Param::new(1, "name")));
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
        Method::new(
            MethodFlags::Public | MethodFlags::HideBySig,
            "Greet",
            MethodSig::new_instance(TypeSig::STRING, vec![]),
        ),
    );
    greet.get_mut().body = body(vec![
        Instruction::ldstr("Hello, "),
        Instruction::ldarg(0),
        Instruction::new(
            OpCode::Ldfld,
            Operand::Token(greeter.get().fields[0].clone().into()),
        ),
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
    assert_eq!(
        reloaded
            .find_method("Hello.Greeter", "Greet")
            .unwrap()
            .get()
            .name,
        "Greet"
    );
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
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "Hello, hao\n");
}

/// A new assembly calling a method defined in another loaded assembly.
#[test]
fn call_into_another_module() {
    let sample = sample!();
    let library = Module::from_bytes(&sample.bytes).unwrap();
    let add = library.find_method("Sample.Calc", "Add").unwrap();

    let mut module = Module::new("Caller", ModuleKind::Console, TargetRuntime::NetCore(9));
    module.add_assembly_ref(
        "System.Console",
        Version {
            major: 9,
            ..Default::default()
        },
        &TargetRuntime::NetCore(9).framework_token(),
    );
    let add = module.import_foreign_method(&add, &library).unwrap();
    let console = module
        .import_type_from("System.Console", "System.Console")
        .unwrap();
    let write_line = module
        .import_method_on(
            &console,
            "WriteLine",
            MethodSig::new_static(TypeSig::VOID, vec![TypeSig::INT32]),
        )
        .unwrap();
    let object = module.corlib_type("System.Object").unwrap();

    let program = module.add_type(TypeDef::new(
        TypeAttributes::Public | TypeAttributes::Abstract | TypeAttributes::Sealed,
        "",
        "Program",
        Some(object),
    ));
    let main = TypeDef::add_method(
        &program,
        Method::new(
            MethodFlags::Public | MethodFlags::Static,
            "Main",
            MethodSig::new_static(TypeSig::VOID, vec![]),
        ),
    );
    main.get_mut().body = body(vec![
        Instruction::ldc_i4(40),
        Instruction::ldc_i4(2),
        Instruction::call(add),
        Instruction::call(write_line),
        Instruction::ret(),
    ]);
    module.entry_point = Some(main);
    assert!(module.find_assembly_ref("Sample").is_some());

    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("caller");
    std::fs::create_dir_all(&dir).unwrap();
    module.write_to_path(dir.join("Caller.dll")).unwrap();
    std::fs::write(dir.join("Sample.dll"), &sample.bytes).unwrap();
    std::fs::copy(
        sample.dir.join("Sample.runtimeconfig.json"),
        dir.join("Caller.runtimeconfig.json"),
    )
    .unwrap();
    let output = Command::new("dotnet")
        .arg(dir.join("Caller.dll"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "42\n");
}

#[test]
fn builder_helpers() {
    use hao::dotnet::{
        entries::CustomAttributeCtor,
        il::{Emitter, HandlerKind},
    };

    let mut module = Module::new(
        "Helpers",
        ModuleKind::Dll,
        TargetRuntime::NetCoreImplementation(8),
    );
    let exports = module.add_static_class("Native", "Exports").unwrap();
    let uco = module
        .corlib_attribute("System.Runtime.InteropServices.UnmanagedCallersOnlyAttribute")
        .unwrap();
    let exception = module.corlib_type("System.Exception").unwrap();
    let max = module
        .import_corlib_method(
            "System.Math",
            "Max",
            MethodSig::new_static(TypeSig::INT32, vec![TypeSig::INT32; 2]),
        )
        .unwrap();

    // try { result = Math.Max(a, b); } catch (Exception) { result = -1; } return result;
    let mut e = Emitter::new();
    let result = e.declare_local(TypeSig::INT32);
    e.try_catch(
        exception,
        |e| {
            e.ldarg(0).ldarg(1).call(&max).stloc(result);
        },
        |e| {
            e.pop().ldc_i4(-1).stloc(result);
        },
    );
    e.ldloc(result).ret();
    TypeDef::add_method(
        &exports,
        Method::public_static(
            "SafeMax",
            MethodSig::new_static(TypeSig::INT32, vec![TypeSig::INT32; 2]),
        )
        .with_attribute(uco)
        .with_body(e.finish()),
    );
    module.set_assembly_name("Helpers.Renamed");

    let reloaded = Module::from_bytes(&module.write().unwrap()).unwrap();
    assert_eq!(
        reloaded.assembly.as_ref().unwrap().get().name,
        "Helpers.Renamed"
    );
    let ty = reloaded.find_type("Native", "Exports").unwrap();
    assert!(ty.get().is_static() && ty.get().is_visible());
    let method = reloaded.find_method("Native.Exports", "SafeMax").unwrap();
    let method = method.get();
    assert!(method.is_public() && method.is_static());
    assert_eq!(
        method.custom_attributes[0].type_name(),
        "System.Runtime.InteropServices.UnmanagedCallersOnlyAttribute"
    );
    assert!(matches!(
        method.custom_attributes[0].constructor,
        CustomAttributeCtor::MemberRef(_)
    ));
    let body = method.body.as_ref().unwrap();
    assert_eq!(body.exception_handlers.len(), 1);
    assert!(matches!(
        body.exception_handlers[0].kind,
        HandlerKind::Catch(_)
    ));
}
