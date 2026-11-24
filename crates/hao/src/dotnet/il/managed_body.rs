//! The [`managed_body!`](crate::managed_body) macro: declarative method
//! bodies for linear sequences of managed `call`/`callvirt` with typed
//! locals — the reflection/interop glue shape, e.g. "resolve a type, fetch a
//! method, prepare it, take its function pointer".
//!
//! Where [`callchain!`](crate::callchain) calls native function pointers with
//! monomorphic `i64` values, `managed_body!` calls managed methods through
//! [`MemberToken`]s, with class- and value-typed locals.  The macro does no
//! type reasoning of its own: the `types` and `methods` sections bind names
//! to small emit-time values ([`TypeSig`], [`ManagedMethod`]), and locals
//! bind to [`ManagedLocal`] instead of a bare `u16` — receiver loading
//! (`ldloc` for classes, `ldloca` for value types) and `call` vs `callvirt`
//! are decided from those values when the IL is emitted.
//!
//! The DSL is deliberately **linear**: straight-line calls plus the `raw`
//! escape hatch.  Bodies dominated by loops or branching belong on
//! [`Emitter`] directly (its `while_loop`/`if_else` combinators), or inside
//! `raw` blocks.

use crate::dotnet::il::{Emitter, MemberToken};
use crate::dotnet::signature::TypeSig;

/// A typed IL local declared by [`managed_body!`](crate::managed_body):
/// its slot index plus whether it holds a value type, which decides how it
/// is loaded as a call receiver.
#[derive(Debug, Clone, Copy)]
pub struct ManagedLocal {
    pub index: u16,
    pub is_value_type: bool,
}

impl ManagedLocal {
    pub fn declare(e: &mut Emitter, sig: TypeSig) -> Self {
        let is_value_type = matches!(sig, TypeSig::ValueType(_));
        Self {
            index: e.declare_local(sig),
            is_value_type,
        }
    }

    /// Loads the local's value (`ldloc`).
    pub fn load(&self, e: &mut Emitter) {
        e.ldloc(self.index);
    }

    /// Loads the local as an instance-call receiver: `ldloca` for a value
    /// type (the receiver is passed by reference), `ldloc` otherwise.
    pub fn load_receiver(&self, e: &mut Emitter) {
        if self.is_value_type {
            e.ldloca(self.index);
        } else {
            e.ldloc(self.index);
        }
    }

    /// Stores the value on top of the stack into the local (`stloc`).
    pub fn store(&self, e: &mut Emitter) {
        e.stloc(self.index);
    }
}

/// How a managed method is invoked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallKind {
    /// `call`, no receiver.
    Static,
    /// `call`, receiver first (non-virtual instance call; `ldloca` receiver
    /// for value types).
    Instance,
    /// `callvirt`, receiver first.
    Virtual,
}

/// A managed method a [`managed_body!`](crate::managed_body) can call: its
/// metadata token plus how to invoke it.
#[derive(Debug, Clone)]
pub struct ManagedMethod {
    pub token: MemberToken,
    pub kind: CallKind,
}

impl ManagedMethod {
    pub fn new(token: impl Into<MemberToken>, kind: CallKind) -> Self {
        Self {
            token: token.into(),
            kind,
        }
    }

    /// Whether calls take a receiver before the arguments.
    pub fn has_receiver(&self) -> bool {
        self.kind != CallKind::Static
    }

    /// Emits the call instruction (`call` or `callvirt`).
    pub fn emit_call(&self, e: &mut Emitter) {
        match self.kind {
            CallKind::Virtual => e.callvirt(self.token.clone()),
            CallKind::Static | CallKind::Instance => e.call(self.token.clone()),
        };
    }
}

/// Emits a [`MethodBody`](crate::dotnet::il::MethodBody) of straight-line
/// managed calls — see the [module docs](crate::dotnet::il::managed_body).
///
/// # Grammar
///
/// ```text
/// managed_body! {
///     exception: <expr: TypeDefOrRef>,                  // catch type
///     types {                                           // named local types
///         Name: class(<expr: TypeDefOrRef>),
///         Name: valuetype(<expr: TypeDefOrRef>),
///     },
///     methods {                                         // named methods
///         name: static <expr: impl Into<MemberToken>>,  // call, no receiver
///         name: instance <expr>,                        // call, receiver first
///         name: virtual <expr>,                         // callvirt, receiver first
///     },
///     params(name: i64 | i32 | nint, ...),              // args -> typed locals
///     result: i32 | i64 | nint = <int literal>,         // or `result: ()`
///     body {
///         let x: Ty = m(a, b);      // Ty: a `types` name, or a primitive
///         let x: Ty = recv.m(a);    //   (string/object/i64/i32/nint/f64/bool)
///         m(a); | recv.m(a);        // void call
///         _ = m(a);                 // call, discard result
///         result = [recv.]m(a);     // call -> conv to result type -> store
///         raw |e| { ... }           // &mut Emitter; add `, r` for the result
///         raw |e, r| { ... }        //   local as a ManagedLocal
///     },
///     catch |e| { ... },            // optional; default handler is `pop`.
///     catch |e, r| { ... },         //   The exception object is on the stack.
/// }
/// ```
///
/// Argument positions take a local/param name or `const <expr>`
/// (an emit-time `i32`, loaded with `ldc_i4`).  A non-`()` result requires a
/// default literal; it is returned when the body throws.
///
/// # Example
///
/// ```
/// use hao::dotnet::entries::well_known::{
///     ExceptionMethod, MarshalMethod, SystemType, WellKnownMethod,
/// };
/// use hao::dotnet::module::{ModuleKind, TargetRuntime};
/// use hao::Module;
///
/// let mut module = Module::new(
///     "Example",
///     ModuleKind::Dll,
///     TargetRuntime::NetCoreImplementation(9),
/// );
/// let exception = module.corlib_type(SystemType::Exception).unwrap();
/// let to_string = module
///     .import_well_known_method(WellKnownMethod::Marshal(MarshalMethod::PtrToStringAuto))
///     .unwrap();
/// let hresult = module
///     .import_well_known_method(WellKnownMethod::Exception(ExceptionMethod::GetHResult))
///     .unwrap();
///
/// // int StringLength(IntPtr text) — returns the HResult on failure.
/// let body = hao::managed_body! {
///     exception: exception.clone(),
///     methods {
///         to_string: static &to_string,
///         hresult: virtual &hresult,
///     },
///     params(text: nint),
///     result: i32 = 0,
///     body {
///         let s: string = to_string(text);
///         raw |e, r| {
///             e.ldloc(s.index);
///             // ... use the string ...
///             let _ = r;
///         }
///     },
///     catch |e, r| {
///         hresult.emit_call(e);   // the exception object is the receiver
///         r.store(e);
///     },
/// };
/// assert!(!body.instructions.is_empty());
/// ```
#[macro_export]
macro_rules! managed_body {
    // ── public entry ─────────────────────────────────────────────────
    (
        exception: $exc:expr,
        $( types { $($tname:ident : $tkind:ident ( $texpr:expr )),+ $(,)? }, )?
        $( methods { $($mname:ident : $mkind:tt $mexpr:expr),+ $(,)? }, )?
        $( params( $($pname:ident : $pkind:ident),+ $(,)? ), )?
        result: $rty:tt $( = $rdefault:literal )?,
        body { $($stmts:tt)* }
        $( , catch | $ce:ident $(, $cr:ident)? | $cbody:block )?
        $(,)?
    ) => {
        $crate::managed_body!(@build ($exc),
            types( $($( $tname : $tkind ( $texpr ), )+)? ),
            methods( $($( $mname : $mkind $mexpr, )+)? ),
            params( $($( $pname : $pkind, )+)? ),
            result( $rty $( = $rdefault )? ),
            body( $($stmts)* ),
            catch( $( | $ce $(, $cr)? | $cbody )? )
        )
    };

    // ── entry used by `dotnet_class!`: params come from the declaration ──
    (@from_class params( $($pname:ident : $pkind:ident),* $(,)? ),
        exception: $exc:expr,
        $( types { $($tname:ident : $tkind:ident ( $texpr:expr )),+ $(,)? }, )?
        $( methods { $($mname:ident : $mkind:tt $mexpr:expr),+ $(,)? }, )?
        result: $rty:tt $( = $rdefault:literal )?,
        body { $($stmts:tt)* }
        $( , catch | $ce:ident $(, $cr:ident)? | $cbody:block )?
        $(,)?
    ) => {
        $crate::managed_body!(@build ($exc),
            types( $($( $tname : $tkind ( $texpr ), )+)? ),
            methods( $($( $mname : $mkind $mexpr, )+)? ),
            params( $( $pname : $pkind, )* ),
            result( $rty $( = $rdefault )? ),
            body( $($stmts)* ),
            catch( $( | $ce $(, $cr)? | $cbody )? )
        )
    };

    // ── the single expansion ─────────────────────────────────────────
    (@build ($exc:expr),
        types( $($tname:ident : $tkind:ident ( $texpr:expr ),)* ),
        methods( $($mname:ident : $mkind:tt $mexpr:expr,)* ),
        params( $($pname:ident : $pkind:ident,)* ),
        result( $rty:tt $( = $rdefault:literal )? ),
        body( $($stmts:tt)* ),
        catch( $( | $ce:ident $(, $cr:ident)? | $cbody:block )? )
    ) => {{
        let __exc = $exc;
        $(
            #[allow(non_snake_case)]
            let $tname: $crate::dotnet::signature::TypeSig =
                $crate::managed_body!(@type_sig $tkind, $texpr);
        )*
        $(
            #[allow(non_snake_case)]
            let $mname: $crate::dotnet::il::ManagedMethod =
                $crate::dotnet::il::ManagedMethod::new(
                    $mexpr,
                    $crate::managed_body!(@call_kind $mkind),
                );
        )*
        $crate::dotnet::il::Emitter::build(|__e| {
            // Parameters → typed locals, by position.
            $crate::managed_body!(@params __e, 0u16, $($pname : $pkind,)*);
            // Result local + default, unless the body is void.
            $crate::managed_body!(@result_decl __e, __result, $rty $( = $rdefault )?);
            __e.try_catch(
                __exc,
                |__e| {
                    $crate::managed_body!(@stmts __e, __result, $rty, $($stmts)*);
                },
                |__e| {
                    $crate::managed_body!(@catch __e, __result,
                        $( | $ce $(, $cr)? | $cbody )?);
                },
            );
            $crate::managed_body!(@result_ret __e, __result, $rty);
        })
    }};

    // ── sections ─────────────────────────────────────────────────────
    (@type_sig class, $e:expr) => { $crate::dotnet::signature::TypeSig::class($e) };
    (@type_sig valuetype, $e:expr) => { $crate::dotnet::signature::TypeSig::value_type($e) };
    (@call_kind static) => { $crate::dotnet::il::CallKind::Static };
    (@call_kind instance) => { $crate::dotnet::il::CallKind::Instance };
    (@call_kind virtual) => { $crate::dotnet::il::CallKind::Virtual };

    (@params $e:ident, $idx:expr,) => {};
    (@params $e:ident, $idx:expr, $pname:ident : $pkind:ident, $($rest:tt)*) => {
        let $pname: $crate::dotnet::il::ManagedLocal = {
            let __local = $crate::dotnet::il::ManagedLocal::declare(
                $e,
                $crate::managed_body!(@prim_sig $pkind),
            );
            $e.ldarg($idx);
            __local.store($e);
            __local
        };
        $crate::managed_body!(@params $e, $idx + 1u16, $($rest)*);
    };

    // ── result handling ──────────────────────────────────────────────
    (@result_decl $e:ident, $r:ident, ()) => {
        let $r: $crate::dotnet::il::ManagedLocal =
            $crate::dotnet::il::ManagedLocal { index: 0, is_value_type: false };
        let _ = $r; // void body: no result local
    };
    (@result_decl $e:ident, $r:ident, $rty:tt = $default:literal) => {
        let $r: $crate::dotnet::il::ManagedLocal =
            $crate::dotnet::il::ManagedLocal::declare(
                $e,
                $crate::managed_body!(@prim_sig $rty),
            );
        $e.ldc_i4($default);
        $crate::managed_body!(@result_conv $e, $rty);
        $r.store($e);
    };
    (@result_ret $e:ident, $r:ident, ()) => {
        $e.ret();
    };
    (@result_ret $e:ident, $r:ident, $rty:tt) => {
        $r.load($e);
        $e.ret();
    };
    (@result_conv $e:ident, i32) => {};
    (@result_conv $e:ident, i64) => {
        $e.op($crate::dotnet::il::OpCode::Conv_I8);
    };
    (@result_conv $e:ident, nint) => {
        $e.op($crate::dotnet::il::OpCode::Conv_I);
    };

    (@catch $e:ident, $r:ident,) => {
        $e.pop(); // discard the exception
    };
    (@catch $e:ident, $r:ident, | $ce:ident | $cbody:block) => {
        {
            let $ce: &mut $crate::dotnet::il::Emitter = &mut *$e;
            $cbody
        }
    };
    (@catch $e:ident, $r:ident, | $ce:ident , $cr:ident | $cbody:block) => {
        {
            let $cr: $crate::dotnet::il::ManagedLocal = $r;
            let $ce: &mut $crate::dotnet::il::Emitter = &mut *$e;
            $cbody
        }
    };

    // ── body statements (muncher; arm order matters) ─────────────────
    (@stmts $e:ident, $r:ident, $rty:tt,) => {};
    (@stmts $e:ident, $r:ident, $rty:tt,
        let $dst:ident : $ty:tt = $recv:ident . $m:ident ( $($args:tt)* ) ; $($rest:tt)*
    ) => {
        let $dst: $crate::dotnet::il::ManagedLocal =
            $crate::dotnet::il::ManagedLocal::declare(
                $e,
                $crate::managed_body!(@local_sig $ty),
            );
        $recv.load_receiver($e);
        $crate::managed_body!(@args $e, $($args)*);
        $m.emit_call($e);
        $dst.store($e);
        $crate::managed_body!(@stmts $e, $r, $rty, $($rest)*);
    };
    (@stmts $e:ident, $r:ident, $rty:tt,
        let $dst:ident : $ty:tt = $m:ident ( $($args:tt)* ) ; $($rest:tt)*
    ) => {
        let $dst: $crate::dotnet::il::ManagedLocal =
            $crate::dotnet::il::ManagedLocal::declare(
                $e,
                $crate::managed_body!(@local_sig $ty),
            );
        $crate::managed_body!(@args $e, $($args)*);
        $m.emit_call($e);
        $dst.store($e);
        $crate::managed_body!(@stmts $e, $r, $rty, $($rest)*);
    };
    (@stmts $e:ident, $r:ident, $rty:tt,
        result = $recv:ident . $m:ident ( $($args:tt)* ) ; $($rest:tt)*
    ) => {
        $recv.load_receiver($e);
        $crate::managed_body!(@args $e, $($args)*);
        $m.emit_call($e);
        $crate::managed_body!(@result_conv $e, $rty);
        $r.store($e);
        $crate::managed_body!(@stmts $e, $r, $rty, $($rest)*);
    };
    (@stmts $e:ident, $r:ident, $rty:tt,
        result = $m:ident ( $($args:tt)* ) ; $($rest:tt)*
    ) => {
        $crate::managed_body!(@args $e, $($args)*);
        $m.emit_call($e);
        $crate::managed_body!(@result_conv $e, $rty);
        $r.store($e);
        $crate::managed_body!(@stmts $e, $r, $rty, $($rest)*);
    };
    (@stmts $e:ident, $r:ident, $rty:tt,
        _ = $recv:ident . $m:ident ( $($args:tt)* ) ; $($rest:tt)*
    ) => {
        $recv.load_receiver($e);
        $crate::managed_body!(@args $e, $($args)*);
        $m.emit_call($e);
        $e.pop();
        $crate::managed_body!(@stmts $e, $r, $rty, $($rest)*);
    };
    (@stmts $e:ident, $r:ident, $rty:tt,
        _ = $m:ident ( $($args:tt)* ) ; $($rest:tt)*
    ) => {
        $crate::managed_body!(@args $e, $($args)*);
        $m.emit_call($e);
        $e.pop();
        $crate::managed_body!(@stmts $e, $r, $rty, $($rest)*);
    };
    (@stmts $e:ident, $r:ident, $rty:tt,
        raw | $re:ident | $body:block $($rest:tt)*
    ) => {
        {
            let $re: &mut $crate::dotnet::il::Emitter = &mut *$e;
            $body
        }
        $crate::managed_body!(@stmts $e, $r, $rty, $($rest)*);
    };
    (@stmts $e:ident, $r:ident, $rty:tt,
        raw | $re:ident , $rr:ident | $body:block $($rest:tt)*
    ) => {
        {
            let $rr: $crate::dotnet::il::ManagedLocal = $r;
            let $re: &mut $crate::dotnet::il::Emitter = &mut *$e;
            $body
        }
        $crate::managed_body!(@stmts $e, $r, $rty, $($rest)*);
    };
    (@stmts $e:ident, $r:ident, $rty:tt,
        $recv:ident . $m:ident ( $($args:tt)* ) ; $($rest:tt)*
    ) => {
        $recv.load_receiver($e);
        $crate::managed_body!(@args $e, $($args)*);
        $m.emit_call($e);
        $crate::managed_body!(@stmts $e, $r, $rty, $($rest)*);
    };
    (@stmts $e:ident, $r:ident, $rty:tt,
        $m:ident ( $($args:tt)* ) ; $($rest:tt)*
    ) => {
        $crate::managed_body!(@args $e, $($args)*);
        $m.emit_call($e);
        $crate::managed_body!(@stmts $e, $r, $rty, $($rest)*);
    };

    // ── call arguments: local names or `const <i32 expr>` ────────────
    (@args $e:ident,) => {};
    (@args $e:ident, const $c:expr) => {
        $e.ldc_i4($c);
    };
    (@args $e:ident, const $c:expr, $($rest:tt)*) => {
        $e.ldc_i4($c);
        $crate::managed_body!(@args $e, $($rest)*);
    };
    (@args $e:ident, $a:ident) => {
        $a.load($e);
    };
    (@args $e:ident, $a:ident, $($rest:tt)*) => {
        $a.load($e);
        $crate::managed_body!(@args $e, $($rest)*);
    };

    // ── primitive type tokens ────────────────────────────────────────
    (@prim_sig i32) => { $crate::dotnet::signature::TypeSig::INT32 };
    (@prim_sig i64) => { $crate::dotnet::signature::TypeSig::INT64 };
    (@prim_sig nint) => { $crate::dotnet::signature::TypeSig::NATIVE_INT };
    // Local types: primitives first, then any `types`-declared name.
    (@local_sig string) => { $crate::dotnet::signature::TypeSig::STRING };
    (@local_sig object) => { $crate::dotnet::signature::TypeSig::OBJECT };
    (@local_sig i32) => { $crate::dotnet::signature::TypeSig::INT32 };
    (@local_sig i64) => { $crate::dotnet::signature::TypeSig::INT64 };
    (@local_sig nint) => { $crate::dotnet::signature::TypeSig::NATIVE_INT };
    (@local_sig f64) => { $crate::dotnet::signature::TypeSig::FLOAT64 };
    (@local_sig bool) => { $crate::dotnet::signature::TypeSig::BOOL };
    (@local_sig $t:ident) => { $t.clone() };
}
