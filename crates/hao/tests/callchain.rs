//! Pins the `callchain!` expansion contract: the generated `MethodBody`
//! must match the equivalent hand-built `Emitter` sequence exactly.

use hao::dotnet::il::{ChainImports, Emitter, OpCode};
use hao::dotnet::module::{ModuleKind, TargetRuntime};
use hao::dotnet::signature::{MethodSig, TypeSig};
use hao::Module;

#[repr(C)]
struct TestHeader {
    first: i64,
    second: i64,
}

fn test_module() -> (Module, ChainImports) {
    let mut module = Module::new(
        "CallchainTest",
        ModuleKind::Dll,
        TargetRuntime::NetCoreImplementation(9),
    );
    let imports = ChainImports::resolve(&mut module).unwrap();
    (module, imports)
}

#[test]
fn matches_hand_built_body() {
    let (_module, imports) = test_module();

    let generated = hao::callchain! {
        imports: &imports,
        methods {
            fn_first: 0x1000usize,
            fn_second: 0x2000usize,
        },
        consts { magic: 42i64 },
        params(scalar: i64, out_ptr: nint, cap: i64),
        errors { err_null = 7 },
        chain {
            let obj = fn_first(scalar, magic) else err_null;
            fn_second(obj);
            let peeked: i32 = *(obj + 8);
            raw |e| {
                e.ldloc(peeked).pop();
            }
        },
        returns out_ptr: TestHeader {
            first: obj,
            second: peeked,
            ..copy(obj + 16, peeked) within cap else err_null,
        },
    };

    let sig_1 = MethodSig::unmanaged_suppress_gc(
        TypeSig::INT64,
        vec![TypeSig::INT64],
        &imports.callconv_cdecl,
        &imports.callconv_suppress_gc,
    );
    let sig_2 = MethodSig::unmanaged_suppress_gc(
        TypeSig::INT64,
        vec![TypeSig::INT64, TypeSig::INT64],
        &imports.callconv_cdecl,
        &imports.callconv_suppress_gc,
    );

    let expected = Emitter::build(|e| {
        // methods prologue: baked pointers
        let fn_first = e.declare_local(TypeSig::NATIVE_INT);
        e.ldc_i8(0x1000).op(OpCode::Conv_I).stloc(fn_first);
        let fn_second = e.declare_local(TypeSig::NATIVE_INT);
        e.ldc_i8(0x2000).op(OpCode::Conv_I).stloc(fn_second);
        // consts prologue
        let magic = e.declare_local(TypeSig::INT64);
        e.ldc_i8(42).stloc(magic);
        // params prologue
        let scalar = e.declare_local(TypeSig::INT64);
        e.ldarg(0).stloc(scalar);
        let out_ptr = e.declare_local(TypeSig::NATIVE_INT);
        e.ldarg(1).stloc(out_ptr);
        let cap = e.declare_local(TypeSig::INT64);
        e.ldarg(2).stloc(cap);
        // skeleton
        let result = e.declare_local(TypeSig::INT64);
        e.ldc_i4(-1).op(OpCode::Conv_I8).stloc(result);
        let err_null = e.label();
        let done = e.label();
        e.try_catch(
            imports.exception.clone(),
            |e| {
                // let obj = fn_first(scalar, magic) else err_null;
                let obj = e.declare_local(TypeSig::INT64);
                e.ldloc(scalar).ldloc(magic).ldloc(fn_first).calli(sig_2);
                e.stloc(obj);
                e.ldloc(obj).branch(OpCode::Brfalse, err_null);
                // fn_second(obj);
                e.ldloc(obj).ldloc(fn_second).calli(sig_1);
                e.pop();
                // let peeked: i32 = *(obj + 8);
                let peeked = e.declare_local(TypeSig::INT64);
                e.ldloc(obj).ldc_i4(8).op(OpCode::Add);
                e.op(OpCode::Ldind_I4).op(OpCode::Conv_I8).stloc(peeked);
                // raw
                e.ldloc(peeked).pop();
                // returns: tail bounds check first
                e.ldc_i4(16).op(OpCode::Conv_I8).ldloc(peeked).op(OpCode::Add);
                e.ldloc(cap).branch(OpCode::Bgt, err_null);
                // field stores by offset_of
                e.ldloc(out_ptr).ldloc(obj).op(OpCode::Stind_I8);
                e.ldloc(out_ptr).ldc_i4(8).op(OpCode::Add);
                e.ldloc(peeked).op(OpCode::Stind_I8);
                // tail copy at out_ptr + size_of::<TestHeader>()
                e.ldloc(out_ptr).ldc_i4(16).op(OpCode::Add);
                e.ldloc(obj).ldc_i4(16).op(OpCode::Add);
                e.ldloc(peeked).op(OpCode::Conv_I4);
                e.op(OpCode::Cpblk);
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
    use hao::dotnet::il::Operand;
    use hao::dotnet::signature::CallingConvention;

    let (mut module, imports) = test_module();

    #[repr(C)]
    struct OutValue {
        value: i64,
    }

    let body = hao::callchain! {
        imports: &imports,
        methods { fn_only: 0x1000usize },
        params(input: i64, out_ptr: nint),
        errors {},
        chain {
            let out = fn_only(input);
        },
        returns out_ptr: OutValue { value: out },
    };

    let class = module.add_static_class("Test", "Chain").unwrap();
    TypeDef::add_method(
        &class,
        Method::public_static(
            "Run",
            MethodSig::new_static(
                TypeSig::INT64,
                vec![TypeSig::INT64, TypeSig::NATIVE_INT],
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
            (OpCode::Calli, Operand::Sig(sig)) => Some(sig),
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
