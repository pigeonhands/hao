//! Pins the `callchain!` expansion contract: the generated `MethodBody`
//! must match the equivalent hand-built `Emitter` sequence exactly.

use hao::dotnet::entries::well_known::{SystemRuntimeCompilerServicesType, SystemType};
use hao::dotnet::entries::TypeDefOrRef;
use hao::dotnet::il::{Emitter, OpCode};
use hao::dotnet::module::{ModuleKind, TargetRuntime};
use hao::dotnet::signature::{MethodSig, TypeSig};
use hao::Module;

fn modifier_types() -> (TypeDefOrRef, TypeDefOrRef, TypeDefOrRef) {
    let mut module = Module::new(
        "CallchainTest",
        ModuleKind::Dll,
        TargetRuntime::NetCoreImplementation(9),
    );
    let exception = module.corlib_type(SystemType::Exception).unwrap();
    let cdecl = module
        .corlib_type(SystemRuntimeCompilerServicesType::CallConvCdecl)
        .unwrap();
    let suppress = module
        .corlib_type(SystemRuntimeCompilerServicesType::CallConvSuppressGCTransition)
        .unwrap();
    (exception, cdecl, suppress)
}

#[test]
fn matches_hand_built_body() {
    let (exception, cdecl, suppress) = modifier_types();

    let generated = hao::callchain! {
        exception: exception.clone(),
        suppress_gc: (&cdecl, &suppress),
        args_from: arg 0 {
            fn_first: fnptr,
            fn_second: fnptr,
            scalar: i64,
        },
        errors { err_null = 7 },
        chain {
            let obj = fn_first(scalar) else err_null;
            fn_second(obj, scalar);
            raw |e| {
                e.ldarg(1).ldloc(obj).op(OpCode::Stind_I8);
            }
        }
    };

    let sig_1 = MethodSig::unmanaged_suppress_gc(
        TypeSig::INT64,
        vec![TypeSig::INT64],
        &cdecl,
        &suppress,
    );
    let sig_2 = MethodSig::unmanaged_suppress_gc(
        TypeSig::INT64,
        vec![TypeSig::INT64, TypeSig::INT64],
        &cdecl,
        &suppress,
    );

    let expected = Emitter::build(|e| {
        let fn_first = e.declare_local(TypeSig::NATIVE_INT);
        let fn_second = e.declare_local(TypeSig::NATIVE_INT);
        let scalar = e.declare_local(TypeSig::INT64);
        let result = e.declare_local(TypeSig::INT64);
        e.ldc_i4(-1).op(OpCode::Conv_I8).stloc(result);
        let err_null = e.label();
        let done = e.label();
        e.try_catch(
            exception,
            |e| {
                // unpack
                e.ldarg(0).op(OpCode::Ldind_I).stloc(fn_first);
                e.ldarg(0).ldc_i4(8).op(OpCode::Add).op(OpCode::Ldind_I).stloc(fn_second);
                e.ldarg(0).ldc_i4(16).op(OpCode::Add).op(OpCode::Ldind_I8).stloc(scalar);
                // let obj = fn_first(scalar) else err_null;
                let obj = e.declare_local(TypeSig::INT64);
                e.ldloc(scalar).ldloc(fn_first).calli(sig_1);
                e.stloc(obj);
                e.ldloc(obj).branch(OpCode::Brfalse, err_null);
                // fn_second(obj, scalar);
                e.ldloc(obj).ldloc(scalar).ldloc(fn_second).calli(sig_2);
                e.pop();
                // raw
                e.ldarg(1).ldloc(obj).op(OpCode::Stind_I8);
                // success + error exits
                e.ldc_i4(0).op(OpCode::Conv_I8).stloc(result).branch(OpCode::Leave, done);
                e.mark(err_null);
                e.ldc_i4(7).op(OpCode::Conv_I8).stloc(result).branch(OpCode::Leave, done);
            },
            |e| {
                e.pop();
            },
        );
        e.mark(done);
        e.ldloc(result).ret();
    });

    assert_eq!(generated.locals, expected.locals);
    assert_eq!(generated.init_locals, expected.init_locals);
    assert_eq!(generated.instructions, expected.instructions);
    assert_eq!(generated.exception_handlers, expected.exception_handlers);
}

#[test]
fn survives_a_module_round_trip() {
    use hao::dotnet::entries::{Method, TypeDef};
    use hao::dotnet::signature::CallingConvention;

    let mut module = Module::new(
        "CallchainRoundTrip",
        ModuleKind::Dll,
        TargetRuntime::NetCoreImplementation(9),
    );
    let exception = module.corlib_type(SystemType::Exception).unwrap();
    let cdecl = module
        .corlib_type(SystemRuntimeCompilerServicesType::CallConvCdecl)
        .unwrap();
    let suppress = module
        .corlib_type(SystemRuntimeCompilerServicesType::CallConvSuppressGCTransition)
        .unwrap();

    let body = hao::callchain! {
        exception: exception.clone(),
        suppress_gc: (&cdecl, &suppress),
        args_from: arg 0 { fn_only: fnptr, value: i64 },
        errors {},
        chain {
            let out = fn_only(value);
            raw |e| {
                e.ldarg(1).ldloc(out).op(OpCode::Stind_I8);
            }
        }
    };

    let class = module.add_static_class("Test", "Chain").unwrap();
    TypeDef::add_method(
        &class,
        Method::public_static(
            "Run",
            MethodSig::new_static(
                TypeSig::INT64,
                vec![TypeSig::NATIVE_INT, TypeSig::NATIVE_INT, TypeSig::INT64],
            ),
        )
        .with_body(body),
    );

    let bytes = module.write().unwrap();
    let reread = Module::from_bytes(&bytes).unwrap();
    let method = reread.find_method("Test.Chain", "Run").unwrap();
    let body = method.get().body.as_ref().unwrap().clone();

    // The calli signature must survive with Unmanaged + both modopts.
    let calli_sigs: Vec<_> = body
        .instructions
        .iter()
        .filter_map(|i| match (&i.opcode, &i.operand) {
            (OpCode::Calli, hao::dotnet::il::Operand::Sig(sig)) => Some(sig),
            _ => None,
        })
        .collect();
    assert_eq!(calli_sigs.len(), 1);
    let sig = calli_sigs[0];
    assert_eq!(sig.calling_convention, CallingConvention::Unmanaged);
    let mut modifiers = 0;
    let mut ty = &sig.ret;
    while let TypeSig::Modified { ty: inner, .. } = ty {
        modifiers += 1;
        ty = inner;
    }
    assert_eq!(modifiers, 2);
    assert_eq!(*ty, TypeSig::INT64);
}
