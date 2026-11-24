//! Executes bodies emitted by `callchain!`, `managed_body!` and
//! `dotnet_class!` in a live CoreCLR — behavioral coverage beyond the
//! structural expansion tests in the hao crate.

use hao_runtime::hao::dotnet::entries::well_known::{
    AppDomainMethod, MarshalMethod, StringExtMethod, StringMethod, SystemReflectionType,
    SystemType, WellKnownMethod,
};
use hao_runtime::hao::dotnet::il::ChainImports;
use hao_runtime::hao::dotnet::module::{ModuleKind, TargetRuntime};
use hao_runtime::hao::Module;
use hao_runtime::Runtime;

fn runtime() -> Option<Runtime> {
    if hao_runtime::find_dotnet_root().is_none() {
        eprintln!("skipping: .NET is not installed");
        return None;
    }
    Some(Runtime::coreclr().unwrap())
}

fn new_module(name: &str) -> Module {
    Module::new(name, ModuleKind::Dll, TargetRuntime::NetCoreImplementation(8))
}

extern "C" fn add_ten(x: u64) -> u64 {
    x + 10
}
extern "C" fn double(x: u64) -> u64 {
    x * 2
}
extern "C" fn always_zero(_x: u64) -> u64 {
    0
}

fn ptr(f: extern "C" fn(u64) -> u64) -> usize {
    f as usize
}

#[repr(C)]
struct ChainHeader {
    value: i64,
}

#[test]
fn callchain_executes() {
    let Some(rt) = runtime() else { return };
    let mut module = new_module("MacroChainTest");
    let imports = ChainImports::resolve(&mut module).unwrap();

    hao_runtime::hao::dotnet_class! { &mut module =>
        static class "Macro.Test".Chain {
            #[unmanaged_callers_only]
            fn Run(x: i64, out_ptr: nint) -> i64 = chain {
                imports: &imports,
                methods {
                    f1: ptr(add_ten),
                    f2: ptr(double),
                },
                errors { err_zero = 7 },
                chain {
                    let a = f1(x) else err_zero;
                    let b = f2(a);
                },
                returns out_ptr: ChainHeader { value: b },
            };
            #[unmanaged_callers_only]
            fn RunZero(x: i64, out_ptr: nint) -> i64 = chain {
                imports: &imports,
                methods {
                    f1: ptr(always_zero),
                    f2: ptr(double),
                },
                errors { err_zero = 7 },
                chain {
                    let a = f1(x) else err_zero;
                    let b = f2(a);
                },
                returns out_ptr: ChainHeader { value: b },
            };
        }
    }
    .unwrap();

    rt.load_module(&module).unwrap();
    type Fn = unsafe extern "system" fn(i64, *mut ChainHeader) -> i64;

    let run: Fn = unsafe {
        std::mem::transmute(
            rt.function_pointer("MacroChainTest", "Macro.Test.Chain", "Run")
                .unwrap(),
        )
    };
    let mut header = ChainHeader { value: -1 };
    let rc = unsafe { run(5, &mut header) };
    assert_eq!(rc, 0);
    assert_eq!(header.value, (5 + 10) * 2);

    let run_zero: Fn = unsafe {
        std::mem::transmute(
            rt.function_pointer("MacroChainTest", "Macro.Test.Chain", "RunZero")
                .unwrap(),
        )
    };
    let mut header = ChainHeader { value: -1 };
    let rc = unsafe { run_zero(5, &mut header) };
    assert_eq!(rc, 7, "the null-check error exit fires");
}

#[test]
fn managed_body_executes() {
    let Some(rt) = runtime() else { return };
    let mut module = new_module("MacroManagedTest");
    let exception = module.corlib_type(SystemType::Exception).unwrap();
    let to_string = module
        .import_well_known_method(WellKnownMethod::Marshal(MarshalMethod::PtrToStringAuto))
        .unwrap();
    let get_length = module
        .import_well_known_method(WellKnownMethod::String(StringMethod::GetLength))
        .unwrap();

    hao_runtime::hao::dotnet_class! { &mut module =>
        static class "Macro.Test".Managed {
            #[unmanaged_callers_only]
            fn StrLen(text: nint) -> i32 = managed {
                exception: exception.clone(),
                methods {
                    to_string: static &to_string,
                    get_length: virtual &get_length,
                },
                result: i32 = -1,
                body {
                    let s: string = to_string(text);
                    result = s.get_length();
                },
            };
        }
    }
    .unwrap();

    rt.load_module(&module).unwrap();
    type Fn = unsafe extern "system" fn(*const std::ffi::c_char) -> i32;
    let str_len: Fn = unsafe {
        std::mem::transmute(
            rt.function_pointer("MacroManagedTest", "Macro.Test.Managed", "StrLen")
                .unwrap(),
        )
    };
    let text = std::ffi::CString::new("hello hao").unwrap();
    assert_eq!(unsafe { str_len(text.as_ptr()) }, 9);
}

#[test]
fn managed_body_control_flow_executes() {
    let Some(rt) = runtime() else { return };
    let mut module = new_module("MacroScanTest");
    let exception = module.corlib_type(SystemType::Exception).unwrap();
    let app_domain = module.corlib_type(SystemType::AppDomain).unwrap();
    let assembly = module.corlib_type(SystemReflectionType::Assembly).unwrap();
    let assembly_name = module
        .corlib_type(SystemReflectionType::AssemblyName)
        .unwrap();
    let wk = |m: &mut Module, method| m.import_well_known_method(method).unwrap();
    let to_string = wk(
        &mut module,
        WellKnownMethod::Marshal(MarshalMethod::PtrToStringAuto),
    );
    let get_current_domain = wk(
        &mut module,
        WellKnownMethod::AppDomain(AppDomainMethod::GetCurrentDomain),
    );
    let get_assemblies = wk(
        &mut module,
        WellKnownMethod::AppDomain(AppDomainMethod::GetAssemblies),
    );
    let get_asm_name = wk(
        &mut module,
        WellKnownMethod::Assembly(hao_runtime::hao::dotnet::entries::well_known::AssemblyMethod::GetName),
    );
    let get_name_prop = wk(
        &mut module,
        WellKnownMethod::AssemblyName(
            hao_runtime::hao::dotnet::entries::well_known::AssemblyNameMethod::GetName,
        ),
    );
    let string_equals = wk(
        &mut module,
        WellKnownMethod::StringExt(StringExtMethod::OpEquality),
    );

    hao_runtime::hao::dotnet_class! { &mut module =>
        static class "Macro.Test".Scan {
            #[unmanaged_callers_only]
            fn IsLoaded(name: nint) -> i32 = managed {
                exception: exception.clone(),
                types {
                    AppDomain: class(app_domain.clone()),
                    Assembly: class(assembly.clone()),
                    Assemblies: array(Assembly),
                    AssemblyName: class(assembly_name.clone()),
                },
                methods {
                    to_string: static &to_string,
                    get_current_domain: static &get_current_domain,
                    get_assemblies: virtual &get_assemblies,
                    get_asm_name: virtual &get_asm_name,
                    get_name_prop: virtual &get_name_prop,
                    string_equals: static &string_equals,
                },
                result: i32 = 0,
                body {
                    let target: string = to_string(name);
                    let domain: AppDomain = get_current_domain();
                    let assemblies: Assemblies = domain.get_assemblies();
                    for current: Assembly in assemblies {
                        let an: AssemblyName = current.get_asm_name();
                        let current_name: string = an.get_name_prop();
                        if string_equals(current_name, target) {
                            result = const 1;
                            return;
                        }
                    }
                },
            };
        }
    }
    .unwrap();

    rt.load_module(&module).unwrap();
    type Fn = unsafe extern "system" fn(*const std::ffi::c_char) -> i32;
    let is_loaded: Fn = unsafe {
        std::mem::transmute(
            rt.function_pointer("MacroScanTest", "Macro.Test.Scan", "IsLoaded")
                .unwrap(),
        )
    };
    let corlib = std::ffi::CString::new("System.Private.CoreLib").unwrap();
    assert_eq!(unsafe { is_loaded(corlib.as_ptr()) }, 1);
    let nonsense = std::ffi::CString::new("Definitely.Not.Loaded").unwrap();
    assert_eq!(unsafe { is_loaded(nonsense.as_ptr()) }, 0);
}
