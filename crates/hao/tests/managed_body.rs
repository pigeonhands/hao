//! Pins the `managed_body!` expansion contract against a hand-built
//! `Emitter` body, and round-trips a `dotnet_class!`-integrated body.

use hao::dotnet::entries::well_known::{
    MarshalMethod, MethodBaseMethod, RuntimeHelpersMethod, RuntimeMethodHandleMethod,
    SystemReflectionType, SystemType, TypeExtMethod, TypeMethod, WellKnownMethod,
};
use hao::dotnet::il::{Emitter, OpCode};
use hao::dotnet::module::{ModuleKind, TargetRuntime};
use hao::dotnet::signature::TypeSig;
use hao::Module;

/// Instance | Static | Public | NonPublic
const BINDING_FLAGS: i32 = 4 | 8 | 16 | 32;

fn test_module() -> Module {
    Module::new(
        "ManagedBodyTest",
        ModuleKind::Dll,
        TargetRuntime::NetCoreImplementation(9),
    )
}

#[test]
fn matches_hand_built_body() {
    let mut module = test_module();
    let exception = module.corlib_type(SystemType::Exception).unwrap();
    let type_ty = module.corlib_type(SystemType::Type).unwrap();
    let method_info = module
        .corlib_type(SystemReflectionType::MethodInfo)
        .unwrap();
    let handle_ty = module.corlib_type(SystemType::RuntimeMethodHandle).unwrap();
    let wk = |module: &mut Module, m| module.import_well_known_method(m).unwrap();
    let to_string = wk(
        &mut module,
        WellKnownMethod::Marshal(MarshalMethod::PtrToStringAuto),
    );
    let get_type = wk(
        &mut module,
        WellKnownMethod::TypeExt(TypeExtMethod::GetTypeStatic),
    );
    let get_method = wk(&mut module, WellKnownMethod::TypeExt(TypeExtMethod::GetMethodBF));
    let get_full_name = wk(&mut module, WellKnownMethod::Type(TypeMethod::GetFullName));
    let method_handle = wk(
        &mut module,
        WellKnownMethod::MethodBase(MethodBaseMethod::GetMethodHandle),
    );
    let prepare = wk(
        &mut module,
        WellKnownMethod::RuntimeHelpers(RuntimeHelpersMethod::PrepareMethod),
    );
    let function_pointer = wk(
        &mut module,
        WellKnownMethod::RuntimeMethodHandle(RuntimeMethodHandleMethod::GetFunctionPointer),
    );

    let generated = hao::managed_body! {
        exception: exception.clone(),
        types {
            Type: class(type_ty.clone()),
            MethodInfo: class(method_info.clone()),
            MethodHandle: valuetype(handle_ty.clone()),
        },
        methods {
            to_string: static &to_string,
            get_type: static &get_type,
            get_method: virtual &get_method,
            get_full_name: virtual &get_full_name,
            method_handle: virtual &method_handle,
            prepare: static &prepare,
            function_pointer: instance &function_pointer,
        },
        params(full_name: nint, name: nint),
        result: nint = 0,
        body {
            let type_str: string = to_string(full_name);
            let found: Type = get_type(type_str);
            _ = found.get_full_name();
            let name_str: string = to_string(name);
            let method: MethodInfo = found.get_method(name_str, const BINDING_FLAGS);
            let handle: MethodHandle = method.method_handle();
            prepare(handle);
            raw |e, r| {
                let _ = r;
                e.dup().pop();
            }
            result = handle.function_pointer();
        },
    };

    let expected = Emitter::build(|e| {
        // params
        let full_name = e.declare_local(TypeSig::NATIVE_INT);
        e.ldarg(0).stloc(full_name);
        let name = e.declare_local(TypeSig::NATIVE_INT);
        e.ldarg(1).stloc(name);
        // result
        let result = e.declare_local(TypeSig::NATIVE_INT);
        e.ldc_i4(0).op(OpCode::Conv_I).stloc(result);
        e.try_catch(
            exception,
            |e| {
                let type_str = e.declare_local(TypeSig::STRING);
                e.ldloc(full_name).call(&to_string).stloc(type_str);
                let found = e.declare_local(TypeSig::class(type_ty.clone()));
                e.ldloc(type_str).call(&get_type).stloc(found);
                e.ldloc(found).callvirt(&get_full_name).pop();
                let name_str = e.declare_local(TypeSig::STRING);
                e.ldloc(name).call(&to_string).stloc(name_str);
                let method = e.declare_local(TypeSig::class(method_info.clone()));
                e.ldloc(found)
                    .ldloc(name_str)
                    .ldc_i4(BINDING_FLAGS)
                    .callvirt(&get_method)
                    .stloc(method);
                let handle = e.declare_local(TypeSig::value_type(handle_ty.clone()));
                e.ldloc(method).callvirt(&method_handle).stloc(handle);
                e.ldloc(handle).call(&prepare);
                e.dup().pop();
                e.ldloca(handle).call(&function_pointer);
                e.op(OpCode::Conv_I).stloc(result);
            },
            |e| {
                e.pop();
            },
        );
        e.ldloc(result).ret();
    });

    assert_eq!(generated.locals, expected.locals);
    assert_eq!(generated.instructions, expected.instructions);
    assert_eq!(generated.exception_handlers, expected.exception_handlers);
}

#[test]
fn control_flow_matches_hand_built_body() {
    use hao::dotnet::entries::well_known::{AppDomainMethod, StringExtMethod};

    let mut module = test_module();
    let exception = module.corlib_type(SystemType::Exception).unwrap();
    let app_domain = module.corlib_type(SystemType::AppDomain).unwrap();
    let assembly = module.corlib_type(SystemReflectionType::Assembly).unwrap();
    let get_current_domain = module
        .import_well_known_method(WellKnownMethod::AppDomain(AppDomainMethod::GetCurrentDomain))
        .unwrap();
    let get_assemblies = module
        .import_well_known_method(WellKnownMethod::AppDomain(AppDomainMethod::GetAssemblies))
        .unwrap();
    let string_equals = module
        .import_well_known_method(WellKnownMethod::StringExt(StringExtMethod::OpEquality))
        .unwrap();
    let get_length = module
        .import_well_known_method(WellKnownMethod::String(
            hao::dotnet::entries::well_known::StringMethod::GetLength,
        ))
        .unwrap();
    let concat = module
        .import_well_known_method(WellKnownMethod::String(
            hao::dotnet::entries::well_known::StringMethod::Concat2,
        ))
        .unwrap();

    let generated = hao::managed_body! {
        exception: exception.clone(),
        types {
            AppDomain: class(app_domain.clone()),
            Assembly: class(assembly.clone()),
            Assemblies: array(Assembly),
        },
        methods {
            get_current_domain: static &get_current_domain,
            get_assemblies: virtual &get_assemblies,
            string_equals: static &string_equals,
            get_length: virtual &get_length,
            concat: static &concat,
        },
        params(k: i32),
        result: i32 = 0,
        body {
            let acc: string = str "";
            let needle: string = str "x";
            let domain: AppDomain = get_current_domain();
            let asms: Assemblies = domain.get_assemblies();
            let n: i32 = len(asms);
            for a: Assembly in asms {
                if n == k {
                    result = const 1;
                    return;
                }
                if acc.get_length() {
                    acc = concat(acc, str "\n");
                } else {
                    acc = concat(acc, needle);
                }
                if string_equals(acc, needle) {
                    return;
                }
            }
        },
    };

    let expected = Emitter::build(|e| {
        let k = e.declare_local(TypeSig::INT32);
        e.ldarg(0).stloc(k);
        let result = e.declare_local(TypeSig::INT32);
        e.ldc_i4(0).stloc(result);
        let done = e.label();
        e.try_catch(
            exception,
            |e| {
                let acc = e.declare_local(TypeSig::STRING);
                e.ldstr("").stloc(acc);
                let needle = e.declare_local(TypeSig::STRING);
                e.ldstr("x").stloc(needle);
                let domain = e.declare_local(TypeSig::class(app_domain.clone()));
                e.call(&get_current_domain).stloc(domain);
                let asms =
                    e.declare_local(TypeSig::sz_array(TypeSig::class(assembly.clone())));
                e.ldloc(domain).callvirt(&get_assemblies).stloc(asms);
                let n = e.declare_local(TypeSig::INT32);
                e.ldloc(asms).op(OpCode::Ldlen).op(OpCode::Conv_I4).stloc(n);
                // for a: Assembly in asms
                let idx = e.declare_local(TypeSig::INT32);
                e.ldc_i4(0).stloc(idx);
                let a = e.declare_local(TypeSig::class(assembly.clone()));
                e.while_loop(
                    |e| {
                        e.ldloc(idx).ldloc(asms);
                        e.op(OpCode::Ldlen).op(OpCode::Conv_I4).op(OpCode::Clt);
                    },
                    |e| {
                        e.ldloc(asms).ldloc(idx).op(OpCode::Ldelem_Ref).stloc(a);
                        // if n == k { result = 1; return; }
                        e.ldloc(n).ldloc(k).op(OpCode::Ceq);
                        e.if_then(|e| {
                            e.ldc_i4(1).stloc(result);
                            e.branch(OpCode::Leave, done);
                        });
                        // if acc.get_length() { .. } else { .. }
                        e.ldloc(acc).callvirt(&get_length);
                        e.if_else(
                            |e| {
                                e.ldloc(acc).ldstr("\n").call(&concat).stloc(acc);
                            },
                            |e| {
                                e.ldloc(acc).ldloc(needle).call(&concat).stloc(acc);
                            },
                        );
                        // if string_equals(acc, needle) { return; }
                        e.ldloc(acc).ldloc(needle).call(&string_equals);
                        e.if_then(|e| {
                            e.branch(OpCode::Leave, done);
                        });
                        e.ldloc(idx).ldc_i4(1).op(OpCode::Add).stloc(idx);
                    },
                );
            },
            |e| {
                e.pop();
            },
        );
        e.mark(done);
        e.ldloc(result).ret();
    });

    assert_eq!(generated.locals, expected.locals);
    assert_eq!(generated.instructions, expected.instructions);
    assert_eq!(generated.exception_handlers, expected.exception_handlers);
}

#[test]
fn void_body_and_custom_catch() {
    let mut module = test_module();
    let exception = module.corlib_type(SystemType::Exception).unwrap();
    let free = module
        .import_well_known_method(WellKnownMethod::Marshal(MarshalMethod::FreeCoTaskMem))
        .unwrap();

    let generated = hao::managed_body! {
        exception: exception.clone(),
        methods { free: static &free },
        params(ptr: nint),
        result: (),
        body {
            free(ptr);
        },
    };

    let expected = Emitter::build(|e| {
        let ptr = e.declare_local(TypeSig::NATIVE_INT);
        e.ldarg(0).stloc(ptr);
        e.try_catch(
            exception,
            |e| {
                e.ldloc(ptr).call(&free);
            },
            |e| {
                e.pop();
            },
        );
        e.ret();
    });

    assert_eq!(generated.locals, expected.locals);
    assert_eq!(generated.instructions, expected.instructions);
}

#[test]
fn integrates_with_dotnet_class() {
    let mut module = test_module();
    let exception = module.corlib_type(SystemType::Exception).unwrap();
    let to_string = module
        .import_well_known_method(WellKnownMethod::Marshal(MarshalMethod::PtrToStringAuto))
        .unwrap();

    hao::dotnet_class! { &mut module =>
        static class "Managed.Test".Entry {
            #[unmanaged_callers_only]
            fn Convert(text: nint) -> nint = managed {
                exception: exception.clone(),
                methods { to_string: static &to_string },
                result: nint = 0,
                body {
                    let s: string = to_string(text);
                    raw |e, r| {
                        e.ldloc(s.index);
                        r.store(e);
                    }
                },
            };
        }
    }
    .unwrap();

    let bytes = module.write().unwrap();
    let reread = Module::from_bytes(&bytes).unwrap();
    let method = reread.find_method("Managed.Test.Entry", "Convert").unwrap();
    let method = method.get();
    assert_eq!(method.signature.params, vec![TypeSig::NATIVE_INT]);
    let body = method.body.as_ref().unwrap();
    assert!(body
        .instructions
        .iter()
        .any(|i| matches!(i.opcode, OpCode::Call)));
    // The string local survived the round trip.
    assert!(body.locals.contains(&TypeSig::STRING));
}
