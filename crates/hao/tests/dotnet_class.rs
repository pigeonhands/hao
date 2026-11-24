//! `dotnet_class!` end-to-end: declared Rust-typed signatures must survive a
//! module write/read round trip, including the UCO attribute and an
//! integrated `chain { .. }` body's calli signature.

use hao::dotnet::il::{ChainImports, Emitter, OpCode, Operand};
use hao::dotnet::module::{ModuleKind, TargetRuntime};
use hao::dotnet::signature::{CallingConvention, TypeSig};
use hao::Module;

#[repr(C)]
struct OutValue {
    value: i64,
}

#[test]
fn declares_and_round_trips() {
    let mut module = Module::new(
        "ClassTest",
        ModuleKind::Dll,
        TargetRuntime::NetCoreImplementation(9),
    );
    let imports = ChainImports::resolve(&mut module).unwrap();
    let prebuilt = Emitter::build(|e| {
        e.ldc_i4(7).ret();
    });

    let class = hao::dotnet_class! { &mut module =>
        static class "Class.Test".Native {
            #[unmanaged_callers_only]
            fn RunChain(input: i64, out_ptr: nint, cap: i64) -> i64 = chain {
                imports: &imports,
                methods { fn_only: 0x1000usize },
                errors { err_overflow = 2 },
                chain {
                    let out = fn_only(input);
                },
                returns out_ptr: OutValue {
                    value: out,
                    ..copy(out + 0, out) within cap else err_overflow,
                },
            };
            fn Seven() -> i32 = prebuilt;
        }
    }
    .unwrap();
    assert_eq!(class.get().methods.len(), 2);

    let bytes = module.write().unwrap();
    let reread = Module::from_bytes(&bytes).unwrap();

    let run = reread.find_method("Class.Test.Native", "RunChain").unwrap();
    let run = run.get();
    assert_eq!(
        run.signature.params,
        vec![TypeSig::INT64, TypeSig::NATIVE_INT, TypeSig::INT64]
    );
    assert_eq!(run.signature.ret, TypeSig::INT64);
    // The UCO attribute survived.
    assert!(!run.custom_attributes.is_empty());
    // The chain body's calli signature survived with Unmanaged + both modopts.
    let body = run.body.as_ref().unwrap();
    let sig = body
        .instructions
        .iter()
        .find_map(|i| match (&i.opcode, &i.operand) {
            (OpCode::Calli, Operand::Sig(sig)) => Some(sig),
            _ => None,
        })
        .expect("a calli with a signature operand");
    assert_eq!(sig.calling_convention, CallingConvention::Unmanaged);

    let seven = reread.find_method("Class.Test.Native", "Seven").unwrap();
    let seven = seven.get();
    assert!(seven.signature.params.is_empty());
    assert_eq!(seven.signature.ret, TypeSig::INT32);
    assert!(seven.custom_attributes.is_empty());
}
