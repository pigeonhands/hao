#![cfg(feature = "coreclr")]

use hao::{
    dotnet::{
        entries::{CustomAttribute, CustomAttributeCtor, CustomAttributeValue, Method, TypeDef},
        il::{Instruction, MemberToken, MethodBody, OpCode},
        md::streams::tables_stream::{MethodFlags, TypeAttributes},
        module::{ModuleKind, TargetRuntime},
        signature::{MethodSig, TypeSig},
    },
    Module,
};
use hao_runtime::Runtime;

/// `static class Native.Exports { [UnmanagedCallersOnly] static int Add(int a, int b) => a + b; }`
fn exports_assembly(name: &str, runtime: TargetRuntime) -> Vec<u8> {
    let mut module = Module::new(name, ModuleKind::Dll, runtime);
    let object = module.corlib_type("System.Object").unwrap();
    let attribute_name = "System.Runtime.InteropServices.UnmanagedCallersOnlyAttribute";
    let attribute = if let TargetRuntime::NetCore(major) = runtime {
        let version = hao::dotnet::entries::Version { major, ..Default::default() };
        module.add_assembly_ref("System.Runtime.InteropServices", version, &runtime.framework_token());
        module.import_type_from("System.Runtime.InteropServices", attribute_name).unwrap()
    } else {
        module.corlib_type(attribute_name).unwrap()
    };
    let MemberToken::MemberRef(ctor) = module
        .import_method_on(&attribute, ".ctor", MethodSig::new_instance(TypeSig::VOID, vec![]))
        .unwrap()
    else {
        unreachable!()
    };

    let exports = module.add_type(TypeDef::new(
        TypeAttributes::Public | TypeAttributes::Abstract | TypeAttributes::Sealed,
        "Native",
        "Exports",
        Some(object),
    ));
    let add = TypeDef::add_method(
        &exports,
        Method::new(
            MethodFlags::Public | MethodFlags::Static | MethodFlags::HideBySig,
            "Add",
            MethodSig::new_static(TypeSig::INT32, vec![TypeSig::INT32, TypeSig::INT32]),
        ),
    );
    let mut add = add.get_mut();
    add.custom_attributes.push(
        CustomAttribute::new(CustomAttributeCtor::MemberRef(ctor), &CustomAttributeValue::default()).unwrap(),
    );
    add.body = Some(MethodBody {
        instructions: vec![
            Instruction::ldarg(0),
            Instruction::ldarg(1),
            Instruction::simple(OpCode::Add),
            Instruction::ret(),
        ],
        ..Default::default()
    });
    drop(add);
    module.write().unwrap()
}

#[test]
fn call_generated_unmanaged_callers_only_method() {
    if hao_runtime::find_dotnet_root().is_none() {
        eprintln!("skipping: .NET is not installed");
        return;
    }
    let runtime = Runtime::coreclr().unwrap();
    for (name, target) in [
        ("NativeExports", TargetRuntime::NetCore(8)),
        ("NativeExportsCoreLib", TargetRuntime::NetCoreImplementation(8)),
    ] {
        runtime.load_assembly_bytes(name, &exports_assembly(name, target)).unwrap();
        let add = runtime.function_pointer(name, "Native.Exports", "Add").unwrap();
        let add: extern "C" fn(i32, i32) -> i32 = unsafe { std::mem::transmute(add) };
        assert_eq!(add(40, 2), 42);
    }

    assert!(Runtime::coreclr().unwrap().function_pointer("NativeExports", "Native.Exports", "Missing").is_err());
}

