//! The [`callchain!`](crate::callchain) macro: declarative method bodies for
//! `[UnmanagedCallersOnly]` entry points that run a chain of native calls in
//! one cooperative-GC scope.
//!
//! # The pattern
//!
//! Calling managed methods one `calli` at a time from native code leaves the
//! thread in preemptive GC mode between calls, so the GC can compact the heap
//! and relocate any managed object whose raw pointer is held across the
//! sequence.  The fix is to run the whole sequence inside a single
//! `[UnmanagedCallersOnly]` method whose internal `calli`s all carry
//! `modopt(CallConvSuppressGCTransition)`: the thread stays in cooperative
//! mode from entry to exit and the GC cannot move anything mid-chain.
//!
//! Because `[UnmanagedCallersOnly]` signatures are limited to simple scalars,
//! such a method conventionally receives `(argsPtr, resultPtr, bufSize)` and
//! unpacks its real inputs — function pointers and `i64` scalars — from the
//! args buffer.  [`callchain!`](crate::callchain) generates that whole shape:
//! the unpack prologue, the calli chain with null-check error exits, and the
//! try/catch skeleton returning `0` on success, a declared error code on a
//! failed check, and `-1` on a managed exception.
//!
//! Chain values are uniformly `i64` (the UnmanagedCallersOnly scalar
//! constraint); anything irregular goes in a `raw |e| { .. }` block, which
//! receives the real [`Emitter`](crate::dotnet::il::Emitter) with every chain
//! local (`u16`) and error label
//! ([`EmitLabel`](crate::dotnet::il::EmitLabel)) in scope.

/// Emits a [`MethodBody`](crate::dotnet::il::MethodBody) for an
/// `[UnmanagedCallersOnly]` native-call chain — see the
/// [module docs](crate::dotnet::il::callchain) for the pattern.
///
/// # Grammar
///
/// ```text
/// callchain! {
///     exception: <expr: TypeDefOrRef>,                      // catch type
///     suppress_gc: (<expr: &TypeDefOrRef>, <expr: &TypeDefOrRef>),  // (cdecl, suppress)
///     args_from: arg N { name: fnptr | i64, ... },          // 8-byte buffer fields
///     errors { name = code, ... },                          // error exits
///     chain {
///         let x = f(a, b) else err;   // calli, bind result, brfalse -> err
///         let x = f(a);               // calli, bind result
///         f(a, b);                    // calli, discard result
///         raw |e| { ... }             // escape hatch: &mut Emitter
///     }
/// }
/// ```
///
/// # Example
///
/// ```
/// use hao::dotnet::entries::well_known::{
///     SystemRuntimeCompilerServicesType, SystemType,
/// };
/// use hao::dotnet::il::OpCode;
/// use hao::dotnet::module::{ModuleKind, TargetRuntime};
/// use hao::Module;
///
/// let mut module = Module::new(
///     "Example",
///     ModuleKind::Dll,
///     TargetRuntime::NetCoreImplementation(9),
/// );
/// let exception = module.corlib_type(SystemType::Exception).unwrap();
/// let cdecl = module
///     .corlib_type(SystemRuntimeCompilerServicesType::CallConvCdecl)
///     .unwrap();
/// let suppress = module
///     .corlib_type(SystemRuntimeCompilerServicesType::CallConvSuppressGCTransition)
///     .unwrap();
///
/// // long NodeParentName(IntPtr argsPtr, IntPtr resultPtr, long bufSize)
/// // args buffer: [fn_get_parent, fn_get_name, node]
/// let body = hao::callchain! {
///     exception: exception.clone(),
///     suppress_gc: (&cdecl, &suppress),
///     args_from: arg 0 {
///         fn_get_parent: fnptr,
///         fn_get_name: fnptr,
///         node: i64,
///     },
///     errors { err_no_parent = 1 },
///     chain {
///         let parent = fn_get_parent(node) else err_no_parent;
///         let name = fn_get_name(parent);
///         raw |e| {
///             // write the result handle to the result buffer (arg 1)
///             e.ldarg(1).ldloc(name).op(OpCode::Stind_I8);
///         }
///     }
/// };
/// assert!(!body.instructions.is_empty());
/// ```
#[macro_export]
macro_rules! callchain {
    // ── entry point ──────────────────────────────────────────────────
    (
        exception: $exc:expr,
        suppress_gc: ($cdecl:expr, $suppress:expr),
        args_from: arg $argn:literal { $($fname:ident : $fkind:ident),+ $(,)? },
        errors { $($ename:ident = $ecode:literal),* $(,)? },
        chain { $($stmts:tt)* } $(,)?
    ) => {{
        let __cdecl = $cdecl;
        let __suppress = $suppress;
        $crate::dotnet::il::Emitter::build(|__e| {
            $( let $fname: u16 = __e.declare_local($crate::callchain!(@local_ty $fkind)); )+
            let __result: u16 =
                __e.declare_local($crate::dotnet::signature::TypeSig::INT64);
            // Default return value = -1 (managed exception).
            __e.ldc_i4(-1)
                .op($crate::dotnet::il::OpCode::Conv_I8)
                .stloc(__result);
            $( let $ename = __e.label(); )*
            let __done = __e.label();
            __e.try_catch(
                $exc,
                |__e| {
                    $crate::callchain!(@unpack __e, $argn, (0i32), $($fname : $fkind,)+);
                    $crate::callchain!(@chain __e, __cdecl, __suppress, $($stmts)*);
                    __e.ldc_i4(0)
                        .op($crate::dotnet::il::OpCode::Conv_I8)
                        .stloc(__result)
                        .branch($crate::dotnet::il::OpCode::Leave, __done);
                    $(
                        __e.mark($ename);
                        __e.ldc_i4($ecode)
                            .op($crate::dotnet::il::OpCode::Conv_I8)
                            .stloc(__result)
                            .branch($crate::dotnet::il::OpCode::Leave, __done);
                    )*
                },
                |__e| {
                    __e.pop(); // discard the exception
                },
            );
            __e.mark(__done);
            __e.ldloc(__result).ret();
        })
    }};

    // ── chain statements (muncher; arm order matters) ────────────────
    (@chain $e:ident, $cc:ident, $sg:ident,) => {};
    (@chain $e:ident, $cc:ident, $sg:ident,
        let $dst:ident = $f:ident ( $($a:ident),* ) else $err:ident ; $($rest:tt)*
    ) => {
        let $dst: u16 = $e.declare_local($crate::dotnet::signature::TypeSig::INT64);
        $crate::callchain!(@call $e, $cc, $sg, $f, $($a),*);
        $e.stloc($dst);
        $e.ldloc($dst).branch($crate::dotnet::il::OpCode::Brfalse, $err);
        $crate::callchain!(@chain $e, $cc, $sg, $($rest)*);
    };
    (@chain $e:ident, $cc:ident, $sg:ident,
        let $dst:ident = $f:ident ( $($a:ident),* ) ; $($rest:tt)*
    ) => {
        let $dst: u16 = $e.declare_local($crate::dotnet::signature::TypeSig::INT64);
        $crate::callchain!(@call $e, $cc, $sg, $f, $($a),*);
        $e.stloc($dst);
        $crate::callchain!(@chain $e, $cc, $sg, $($rest)*);
    };
    (@chain $e:ident, $cc:ident, $sg:ident,
        raw | $re:ident | $body:block $($rest:tt)*
    ) => {
        {
            let $re: &mut $crate::dotnet::il::Emitter = &mut *$e;
            $body
        }
        $crate::callchain!(@chain $e, $cc, $sg, $($rest)*);
    };
    (@chain $e:ident, $cc:ident, $sg:ident,
        $f:ident ( $($a:ident),* ) ; $($rest:tt)*
    ) => {
        $crate::callchain!(@call $e, $cc, $sg, $f, $($a),*);
        $e.pop();
        $crate::callchain!(@chain $e, $cc, $sg, $($rest)*);
    };

    // ── shared calli emission ────────────────────────────────────────
    (@call $e:ident, $cc:ident, $sg:ident, $f:ident, $($a:ident),*) => {
        $( $e.ldloc($a); )*
        $e.ldloc($f);
        $e.calli($crate::dotnet::signature::MethodSig::unmanaged_suppress_gc(
            $crate::dotnet::signature::TypeSig::INT64,
            ::std::vec![
                $crate::dotnet::signature::TypeSig::INT64;
                0usize $(+ $crate::callchain!(@one $a))*
            ],
            $cc,
            $sg,
        ));
    };
    (@one $x:ident) => { 1usize };

    // ── args-buffer unpack (offset-accumulating muncher) ─────────────
    (@unpack $e:ident, $argn:literal, ($off:expr),) => {};
    (@unpack $e:ident, $argn:literal, ($off:expr), $n:ident : $k:ident, $($rest:tt)*) => {
        $e.ldarg($argn);
        if $off != 0 {
            $e.ldc_i4($off).op($crate::dotnet::il::OpCode::Add);
        }
        $e.op($crate::callchain!(@ldind $k)).stloc($n);
        $crate::callchain!(@unpack $e, $argn, ($off + 8), $($rest)*);
    };

    // ── field-kind lookups ───────────────────────────────────────────
    (@local_ty fnptr) => { $crate::dotnet::signature::TypeSig::NATIVE_INT };
    (@local_ty i64) => { $crate::dotnet::signature::TypeSig::INT64 };
    (@ldind fnptr) => { $crate::dotnet::il::OpCode::Ldind_I };
    (@ldind i64) => { $crate::dotnet::il::OpCode::Ldind_I8 };
}
