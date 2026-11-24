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
//! Chain assemblies are built at runtime, after the CLR is attached, so the
//! method pointers to call are already known when the IL is emitted.
//! [`callchain!`](crate::callchain) therefore **bakes them into the IL as
//! constants** (`ldc.i8` + `conv.i`, the [`Emitter::call_native`] technique)
//! instead of threading them through an argument buffer; per-call scalars are
//! ordinary method parameters.  The macro generates the whole method shape:
//! the baked-pointer and parameter prologue, the calli chain with null-check
//! error exits, declarative result marshalling through a `#[repr(C)]` struct,
//! and the try/catch skeleton returning `0` on success, a declared error code
//! on a failed check, and `-1` on a managed exception.
//!
//! Chain values are uniformly `i64` (the UnmanagedCallersOnly scalar
//! constraint); anything irregular goes in a `raw |e| { .. }` block, which
//! receives the real [`Emitter`] with every chain local (`u16`) and error
//! label ([`EmitLabel`](crate::dotnet::il::EmitLabel)) in scope.
//!
//! [`Emitter`]: crate::dotnet::il::Emitter
//! [`Emitter::call_native`]: crate::dotnet::il::Emitter::call_native

use crate::dotnet::entries::well_known::{SystemRuntimeCompilerServicesType, SystemType};
use crate::dotnet::entries::TypeDefOrRef;
use crate::Module;

/// The per-module type imports a [`callchain!`](crate::callchain) body needs:
/// the catch type and the two calling-convention modifier types for the
/// `SuppressGCTransition` calli signatures.
///
/// The underlying well-known types are always the same, but `TypeDefOrRef`
/// handles are imports of a specific module, so they are resolved against the
/// module being built — either with [`ChainImports::resolve`] or by filling
/// the fields from handles already imported elsewhere.
pub struct ChainImports {
    pub exception: TypeDefOrRef,
    pub callconv_cdecl: TypeDefOrRef,
    pub callconv_suppress_gc: TypeDefOrRef,
}

impl ChainImports {
    /// Imports `System.Exception`, `CallConvCdecl` and
    /// `CallConvSuppressGCTransition` from corlib into `module`.
    pub fn resolve(module: &mut Module) -> crate::error::Result<Self> {
        Ok(Self {
            exception: module.corlib_type(SystemType::Exception)?,
            callconv_cdecl: module
                .corlib_type(SystemRuntimeCompilerServicesType::CallConvCdecl)?,
            callconv_suppress_gc: module
                .corlib_type(SystemRuntimeCompilerServicesType::CallConvSuppressGCTransition)?,
        })
    }
}

/// Emits a [`MethodBody`](crate::dotnet::il::MethodBody) for an
/// `[UnmanagedCallersOnly]` native-call chain — see the
/// [module docs](crate::dotnet::il::callchain) for the pattern.
///
/// # Grammar
///
/// ```text
/// callchain! {
///     imports: <expr: &ChainImports>,         // per-module type imports
///     methods { name: <expr: usize>, ... },   // resolved entry points, baked into the IL
///     consts { name: <expr: i64>, ... },      // emit-time constants, baked into the IL
///     params(name: i64 | nint, ...),          // the method's own parameters, in order
///     errors { name = code, ... },            // error exits
///     chain {
///         let x = f(a, b) else err;   // calli, bind result, brfalse -> err
///         let x = f(a);               // calli, bind result
///         f(a, b);                    // calli, discard result
///         let x = *(base + off);      // 8-byte read (ldind.i8)
///         let x: i32 = *(base + off); // 4-byte read, widened (ldind.i4 + conv.i8)
///         raw |e| { ... }             // escape hatch: &mut Emitter
///     },
///     returns ptr_param: StructTy {   // optional; emitted on the success path
///         field: local, ...,          // stores at offset_of!(StructTy, field)
///         ..copy(src + off, len) within cap else err,  // optional variable tail
///     },
/// }
/// ```
///
/// The chain computes, `returns` marshals: each `field: local` stores by name
/// at `core::mem::offset_of!(StructTy, field)`, and the `..copy` tail writes a
/// variable-length payload at `ptr + size_of::<StructTy>()` after a bounds
/// check (`size_of + len <= cap`, branching to the error on overflow).  The
/// struct must be `#[repr(C)]` with only `i64` fields, and the `MethodSig`
/// registered for the method must match the `params` list in order.  Offsets
/// are expressions evaluated at emit time.  The `i32` ascription on a pointer
/// read names the *load width* — the local itself is still an `i64`.
///
/// Every declared name (`methods`, `consts`, `params`, chain `let`s, `errors`)
/// is an ordinary Rust binding (`u16` local index or `EmitLabel`), so `raw`
/// blocks can keep using them with the full `Emitter` API.
///
/// # Example
///
/// ```
/// use hao::dotnet::il::ChainImports;
/// use hao::dotnet::module::{ModuleKind, TargetRuntime};
/// use hao::Module;
///
/// #[repr(C)]
/// struct NameResult {
///     name: i64,
/// }
///
/// let mut module = Module::new(
///     "Example",
///     ModuleKind::Dll,
///     TargetRuntime::NetCoreImplementation(9),
/// );
/// let imports = ChainImports::resolve(&mut module).unwrap();
///
/// // long NodeParentName(long node, IntPtr resultPtr)
/// let body = hao::callchain! {
///     imports: &imports,
///     methods {
///         // in real code: BINDING.resolve(rt)? — a doctest just needs exprs
///         get_parent: 0x1000usize,
///         get_name: 0x2000usize,
///     },
///     params(node: i64, result_ptr: nint),
///     errors { err_no_parent = 1 },
///     chain {
///         let parent = get_parent(node) else err_no_parent;
///         let name = get_name(parent);
///     },
///     returns result_ptr: NameResult { name: name },
/// };
/// assert!(!body.instructions.is_empty());
/// ```
#[macro_export]
macro_rules! callchain {
    // ── entry point ──────────────────────────────────────────────────
    (
        imports: $imports:expr,
        $( methods { $($mname:ident : $mexpr:expr),+ $(,)? }, )?
        $( consts { $($cname:ident : $cexpr:expr),+ $(,)? }, )?
        $( params( $($pname:ident : $pkind:ident),+ $(,)? ), )?
        errors { $($ename:ident = $ecode:literal),* $(,)? },
        chain { $($stmts:tt)* }
        $( , returns $rptr:ident : $rty:ty {
            $($rfield:ident : $rlocal:ident),+
            $(, ..copy( $tsrc:ident + $tsoff:expr , $tlen:ident )
                within $tcap:ident else $terr:ident )?
            $(,)?
        } )?
        $(,)?
    ) => {{
        let __imports = $imports;
        $( $( let $mname: usize = $mexpr; )+ )?
        $( $( let $cname: i64 = $cexpr; )+ )?
        $crate::dotnet::il::Emitter::build(|__e| {
            // Baked method pointers → NATIVE_INT locals.
            $( $( let $mname: u16 = {
                let __ptr = $mname;
                let __local =
                    __e.declare_local($crate::dotnet::signature::TypeSig::NATIVE_INT);
                __e.ldc_i8(__ptr as i64)
                    .op($crate::dotnet::il::OpCode::Conv_I)
                    .stloc(__local);
                __local
            }; )+ )?
            // Baked constants → INT64 locals.
            $( $( let $cname: u16 = {
                let __value = $cname;
                let __local =
                    __e.declare_local($crate::dotnet::signature::TypeSig::INT64);
                __e.ldc_i8(__value).stloc(__local);
                __local
            }; )+ )?
            // Parameters → locals, by position.
            $( $crate::callchain!(@params __e, 0u16, $($pname : $pkind,)+); )?
            let __result: u16 =
                __e.declare_local($crate::dotnet::signature::TypeSig::INT64);
            // Default return value = -1 (managed exception).
            __e.ldc_i4(-1)
                .op($crate::dotnet::il::OpCode::Conv_I8)
                .stloc(__result);
            $( let $ename = __e.label(); )*
            let __done = __e.label();
            __e.try_catch(
                __imports.exception.clone(),
                |__e| {
                    $crate::callchain!(@chain __e, __imports, $($stmts)*);
                    // `returns` marshalling, on the success path.
                    $(
                        $($crate::callchain!(@tail_check __e, $rty,
                            $tlen, $tcap, $terr);)?
                        $(
                            __e.ldloc($rptr);
                            {
                                let __off =
                                    ::core::mem::offset_of!($rty, $rfield) as i32;
                                if __off != 0 {
                                    __e.ldc_i4(__off)
                                        .op($crate::dotnet::il::OpCode::Add);
                                }
                            }
                            __e.ldloc($rlocal)
                                .op($crate::dotnet::il::OpCode::Stind_I8);
                        )+
                        $($crate::callchain!(@tail_copy __e, $rty, $rptr,
                            $tsrc, $tsoff, $tlen);)?
                    )?
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

    // ── params: positional-index muncher ─────────────────────────────
    (@params $e:ident, $idx:expr,) => {};
    (@params $e:ident, $idx:expr, $pname:ident : $pkind:ident, $($rest:tt)*) => {
        let $pname: u16 = {
            let __local = $e.declare_local($crate::callchain!(@local_ty $pkind));
            $e.ldarg($idx).stloc(__local);
            __local
        };
        $crate::callchain!(@params $e, $idx + 1u16, $($rest)*);
    };

    // ── returns tail: bounds check + cpblk ───────────────────────────
    (@tail_check $e:ident, $st:ty, $len:ident, $cap:ident, $err:ident) => {
        $e.ldc_i4(::core::mem::size_of::<$st>() as i32)
            .op($crate::dotnet::il::OpCode::Conv_I8)
            .ldloc($len)
            .op($crate::dotnet::il::OpCode::Add)
            .ldloc($cap)
            .branch($crate::dotnet::il::OpCode::Bgt, $err);
    };
    (@tail_copy $e:ident, $st:ty, $ptr:ident, $src:ident, $soff:expr, $len:ident) => {
        $e.ldloc($ptr)
            .ldc_i4(::core::mem::size_of::<$st>() as i32)
            .op($crate::dotnet::il::OpCode::Add);
        $e.ldloc($src);
        {
            let __off = ($soff) as i32;
            if __off != 0 {
                $e.ldc_i4(__off).op($crate::dotnet::il::OpCode::Add);
            }
        }
        $e.ldloc($len).op($crate::dotnet::il::OpCode::Conv_I4);
        $e.op($crate::dotnet::il::OpCode::Cpblk);
    };

    // ── chain statements (muncher; arm order matters) ────────────────
    (@chain $e:ident, $im:ident,) => {};
    (@chain $e:ident, $im:ident,
        let $dst:ident : i32 = * ( $base:ident + $off:expr ) ; $($rest:tt)*
    ) => {
        let $dst: u16 = $e.declare_local($crate::dotnet::signature::TypeSig::INT64);
        $crate::callchain!(@addr $e, $base, $off);
        $e.op($crate::dotnet::il::OpCode::Ldind_I4)
            .op($crate::dotnet::il::OpCode::Conv_I8)
            .stloc($dst);
        $crate::callchain!(@chain $e, $im, $($rest)*);
    };
    (@chain $e:ident, $im:ident,
        let $dst:ident = * ( $base:ident + $off:expr ) ; $($rest:tt)*
    ) => {
        let $dst: u16 = $e.declare_local($crate::dotnet::signature::TypeSig::INT64);
        $crate::callchain!(@addr $e, $base, $off);
        $e.op($crate::dotnet::il::OpCode::Ldind_I8).stloc($dst);
        $crate::callchain!(@chain $e, $im, $($rest)*);
    };
    (@chain $e:ident, $im:ident,
        let $dst:ident = $f:ident ( $($a:ident),* ) else $err:ident ; $($rest:tt)*
    ) => {
        let $dst: u16 = $e.declare_local($crate::dotnet::signature::TypeSig::INT64);
        $crate::callchain!(@call $e, $im, $f, $($a),*);
        $e.stloc($dst);
        $e.ldloc($dst).branch($crate::dotnet::il::OpCode::Brfalse, $err);
        $crate::callchain!(@chain $e, $im, $($rest)*);
    };
    (@chain $e:ident, $im:ident,
        let $dst:ident = $f:ident ( $($a:ident),* ) ; $($rest:tt)*
    ) => {
        let $dst: u16 = $e.declare_local($crate::dotnet::signature::TypeSig::INT64);
        $crate::callchain!(@call $e, $im, $f, $($a),*);
        $e.stloc($dst);
        $crate::callchain!(@chain $e, $im, $($rest)*);
    };
    (@chain $e:ident, $im:ident,
        raw | $re:ident | $body:block $($rest:tt)*
    ) => {
        {
            let $re: &mut $crate::dotnet::il::Emitter = &mut *$e;
            $body
        }
        $crate::callchain!(@chain $e, $im, $($rest)*);
    };
    (@chain $e:ident, $im:ident,
        $f:ident ( $($a:ident),* ) ; $($rest:tt)*
    ) => {
        $crate::callchain!(@call $e, $im, $f, $($a),*);
        $e.pop();
        $crate::callchain!(@chain $e, $im, $($rest)*);
    };

    // ── address of base + offset (offset skipped when zero) ──────────
    (@addr $e:ident, $base:ident, $off:expr) => {
        $e.ldloc($base);
        {
            let __off = ($off) as i32;
            if __off != 0 {
                $e.ldc_i4(__off).op($crate::dotnet::il::OpCode::Add);
            }
        }
    };

    // ── shared calli emission ────────────────────────────────────────
    (@call $e:ident, $im:ident, $f:ident, $($a:ident),*) => {
        $( $e.ldloc($a); )*
        $e.ldloc($f);
        $e.calli($crate::dotnet::signature::MethodSig::unmanaged_suppress_gc(
            $crate::dotnet::signature::TypeSig::INT64,
            ::std::vec![
                $crate::dotnet::signature::TypeSig::INT64;
                0usize $(+ $crate::callchain!(@one $a))*
            ],
            &$im.callconv_cdecl,
            &$im.callconv_suppress_gc,
        ));
    };
    (@one $x:ident) => { 1usize };

    // ── parameter-kind lookup ────────────────────────────────────────
    (@local_ty i64) => { $crate::dotnet::signature::TypeSig::INT64 };
    (@local_ty nint) => { $crate::dotnet::signature::TypeSig::NATIVE_INT };
}
