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
///         Name: array(Earlier),         // sz-array of a previously declared name
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
///         let x: Ty = str "lit";    // ldstr init
///         let n: i32 = len(arr);    // array length (ldlen + conv.i4)
///         m(a); | recv.m(a);        // void call
///         _ = m(a);                 // call, discard result
///         x = [recv.]m(a);          // assign an existing local
///         result = [recv.]m(a);     // call -> conv to result type -> store
///         result = const <expr>;    // set the result to an emit-time i32
///         for x: Ty in arr { ... }  // index loop over an array local
///         if <cond> { ... }         // structured if; optional else { ... }
///         return;                   // early exit to the function epilogue
///         raw |e| { ... }           // &mut Emitter; add `, r` for the result
///         raw |e, r| { ... }        //   local as a ManagedLocal
///     },
///     // <cond> ::= local            (truthy: non-null / non-zero)
///     //          | [recv.]m(a, b)   (call result, truthy)
///     //          | a == b           (i32 equality)
///     catch |e| { ... },            // optional; default handler is `pop`.
///     catch |e, r| { ... },         //   The exception object is on the stack.
/// }
/// ```
///
/// Argument positions take a local/param name, `const <expr>` (an emit-time
/// `i32`, loaded with `ldc_i4`), or `str "lit"` (`ldstr`).  A non-`()` result
/// requires a default literal; it is returned when the body throws.
/// `len`, `str`, `const`, `for`, `if`, `return` and `raw` are reserved words
/// in the body DSL.  Anything the statement forms cannot express — further
/// loop shapes, arithmetic — belongs in `raw` or directly on `Emitter`.
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
            let __done = __e.label();
            __e.try_catch(
                __exc,
                |__e| {
                    $crate::managed_body!(@stmts __e, __result, $rty, __done, $($stmts)*);
                },
                |__e| {
                    $crate::managed_body!(@catch __e, __result,
                        $( | $ce $(, $cr)? | $cbody )?);
                },
            );
            __e.mark(__done);
            $crate::managed_body!(@result_ret __e, __result, $rty);
        })
    }};

    // ── sections ─────────────────────────────────────────────────────
    (@type_sig class, $e:expr) => { $crate::dotnet::signature::TypeSig::class($e) };
    (@type_sig valuetype, $e:expr) => { $crate::dotnet::signature::TypeSig::value_type($e) };
    // array(Name) — an sz-array of a previously declared `types` name.
    (@type_sig array, $e:expr) => { $crate::dotnet::signature::TypeSig::sz_array($e.clone()) };
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
    (@stmts $e:ident, $r:ident, $rty:tt, $d:ident,) => {};
    // let x: Ty = str "lit";
    (@stmts $e:ident, $r:ident, $rty:tt, $d:ident,
        let $dst:ident : $ty:tt = str $s:literal ; $($rest:tt)*
    ) => {
        let $dst: $crate::dotnet::il::ManagedLocal =
            $crate::dotnet::il::ManagedLocal::declare(
                $e,
                $crate::managed_body!(@local_sig $ty),
            );
        $e.ldstr($s);
        $dst.store($e);
        $crate::managed_body!(@stmts $e, $r, $rty, $d, $($rest)*);
    };
    // let n: i32 = len(arr);
    (@stmts $e:ident, $r:ident, $rty:tt, $d:ident,
        let $dst:ident : i32 = len ( $arr:ident ) ; $($rest:tt)*
    ) => {
        let $dst: $crate::dotnet::il::ManagedLocal =
            $crate::dotnet::il::ManagedLocal::declare(
                $e,
                $crate::dotnet::signature::TypeSig::INT32,
            );
        $arr.load($e);
        $e.op($crate::dotnet::il::OpCode::Ldlen)
            .op($crate::dotnet::il::OpCode::Conv_I4);
        $dst.store($e);
        $crate::managed_body!(@stmts $e, $r, $rty, $d, $($rest)*);
    };
    (@stmts $e:ident, $r:ident, $rty:tt, $d:ident,
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
        $crate::managed_body!(@stmts $e, $r, $rty, $d, $($rest)*);
    };
    (@stmts $e:ident, $r:ident, $rty:tt, $d:ident,
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
        $crate::managed_body!(@stmts $e, $r, $rty, $d, $($rest)*);
    };
    // result = const <expr>;
    (@stmts $e:ident, $r:ident, $rty:tt, $d:ident,
        result = const $c:expr ; $($rest:tt)*
    ) => {
        $e.ldc_i4($c);
        $crate::managed_body!(@result_conv $e, $rty);
        $r.store($e);
        $crate::managed_body!(@stmts $e, $r, $rty, $d, $($rest)*);
    };
    (@stmts $e:ident, $r:ident, $rty:tt, $d:ident,
        result = $recv:ident . $m:ident ( $($args:tt)* ) ; $($rest:tt)*
    ) => {
        $recv.load_receiver($e);
        $crate::managed_body!(@args $e, $($args)*);
        $m.emit_call($e);
        $crate::managed_body!(@result_conv $e, $rty);
        $r.store($e);
        $crate::managed_body!(@stmts $e, $r, $rty, $d, $($rest)*);
    };
    (@stmts $e:ident, $r:ident, $rty:tt, $d:ident,
        result = $m:ident ( $($args:tt)* ) ; $($rest:tt)*
    ) => {
        $crate::managed_body!(@args $e, $($args)*);
        $m.emit_call($e);
        $crate::managed_body!(@result_conv $e, $rty);
        $r.store($e);
        $crate::managed_body!(@stmts $e, $r, $rty, $d, $($rest)*);
    };
    (@stmts $e:ident, $r:ident, $rty:tt, $d:ident,
        _ = $recv:ident . $m:ident ( $($args:tt)* ) ; $($rest:tt)*
    ) => {
        $recv.load_receiver($e);
        $crate::managed_body!(@args $e, $($args)*);
        $m.emit_call($e);
        $e.pop();
        $crate::managed_body!(@stmts $e, $r, $rty, $d, $($rest)*);
    };
    (@stmts $e:ident, $r:ident, $rty:tt, $d:ident,
        _ = $m:ident ( $($args:tt)* ) ; $($rest:tt)*
    ) => {
        $crate::managed_body!(@args $e, $($args)*);
        $m.emit_call($e);
        $e.pop();
        $crate::managed_body!(@stmts $e, $r, $rty, $d, $($rest)*);
    };
    // return; — leave the try region for the function epilogue.
    (@stmts $e:ident, $r:ident, $rty:tt, $d:ident,
        return ; $($rest:tt)*
    ) => {
        $e.branch($crate::dotnet::il::OpCode::Leave, $d);
        $crate::managed_body!(@stmts $e, $r, $rty, $d, $($rest)*);
    };
    // for x: Ty in arr { ... }
    (@stmts $e:ident, $r:ident, $rty:tt, $d:ident,
        for $x:ident : $ty:tt in $arr:ident { $($body:tt)* } $($rest:tt)*
    ) => {
        let __idx = $crate::dotnet::il::ManagedLocal::declare(
            $e,
            $crate::dotnet::signature::TypeSig::INT32,
        );
        $e.ldc_i4(0);
        __idx.store($e);
        let $x: $crate::dotnet::il::ManagedLocal =
            $crate::dotnet::il::ManagedLocal::declare(
                $e,
                $crate::managed_body!(@local_sig $ty),
            );
        $e.while_loop(
            |__e| {
                __idx.load(__e);
                $arr.load(__e);
                __e.op($crate::dotnet::il::OpCode::Ldlen)
                    .op($crate::dotnet::il::OpCode::Conv_I4)
                    .op($crate::dotnet::il::OpCode::Clt);
            },
            |__e| {
                $arr.load(__e);
                __idx.load(__e);
                __e.op($crate::dotnet::il::OpCode::Ldelem_Ref);
                $x.store(__e);
                $crate::managed_body!(@stmts __e, $r, $rty, $d, $($body)*);
                __idx.load(__e);
                __e.ldc_i4(1).op($crate::dotnet::il::OpCode::Add);
                __idx.store(__e);
            },
        );
        $crate::managed_body!(@stmts $e, $r, $rty, $d, $($rest)*);
    };
    // if <cond> { ... } [else { ... }]
    (@stmts $e:ident, $r:ident, $rty:tt, $d:ident,
        if $a:ident == $b:ident { $($then:tt)* } else { $($else:tt)* } $($rest:tt)*
    ) => {
        $a.load($e);
        $b.load($e);
        $e.op($crate::dotnet::il::OpCode::Ceq);
        $e.if_else(
            |__e| { $crate::managed_body!(@stmts __e, $r, $rty, $d, $($then)*); },
            |__e| { $crate::managed_body!(@stmts __e, $r, $rty, $d, $($else)*); },
        );
        $crate::managed_body!(@stmts $e, $r, $rty, $d, $($rest)*);
    };
    (@stmts $e:ident, $r:ident, $rty:tt, $d:ident,
        if $a:ident == $b:ident { $($then:tt)* } $($rest:tt)*
    ) => {
        $a.load($e);
        $b.load($e);
        $e.op($crate::dotnet::il::OpCode::Ceq);
        $e.if_then(|__e| { $crate::managed_body!(@stmts __e, $r, $rty, $d, $($then)*); });
        $crate::managed_body!(@stmts $e, $r, $rty, $d, $($rest)*);
    };
    (@stmts $e:ident, $r:ident, $rty:tt, $d:ident,
        if $recv:ident . $m:ident ( $($args:tt)* ) { $($then:tt)* } else { $($else:tt)* } $($rest:tt)*
    ) => {
        $recv.load_receiver($e);
        $crate::managed_body!(@args $e, $($args)*);
        $m.emit_call($e);
        $e.if_else(
            |__e| { $crate::managed_body!(@stmts __e, $r, $rty, $d, $($then)*); },
            |__e| { $crate::managed_body!(@stmts __e, $r, $rty, $d, $($else)*); },
        );
        $crate::managed_body!(@stmts $e, $r, $rty, $d, $($rest)*);
    };
    (@stmts $e:ident, $r:ident, $rty:tt, $d:ident,
        if $recv:ident . $m:ident ( $($args:tt)* ) { $($then:tt)* } $($rest:tt)*
    ) => {
        $recv.load_receiver($e);
        $crate::managed_body!(@args $e, $($args)*);
        $m.emit_call($e);
        $e.if_then(|__e| { $crate::managed_body!(@stmts __e, $r, $rty, $d, $($then)*); });
        $crate::managed_body!(@stmts $e, $r, $rty, $d, $($rest)*);
    };
    (@stmts $e:ident, $r:ident, $rty:tt, $d:ident,
        if $m:ident ( $($args:tt)* ) { $($then:tt)* } else { $($else:tt)* } $($rest:tt)*
    ) => {
        $crate::managed_body!(@args $e, $($args)*);
        $m.emit_call($e);
        $e.if_else(
            |__e| { $crate::managed_body!(@stmts __e, $r, $rty, $d, $($then)*); },
            |__e| { $crate::managed_body!(@stmts __e, $r, $rty, $d, $($else)*); },
        );
        $crate::managed_body!(@stmts $e, $r, $rty, $d, $($rest)*);
    };
    (@stmts $e:ident, $r:ident, $rty:tt, $d:ident,
        if $m:ident ( $($args:tt)* ) { $($then:tt)* } $($rest:tt)*
    ) => {
        $crate::managed_body!(@args $e, $($args)*);
        $m.emit_call($e);
        $e.if_then(|__e| { $crate::managed_body!(@stmts __e, $r, $rty, $d, $($then)*); });
        $crate::managed_body!(@stmts $e, $r, $rty, $d, $($rest)*);
    };
    (@stmts $e:ident, $r:ident, $rty:tt, $d:ident,
        if $x:ident { $($then:tt)* } else { $($else:tt)* } $($rest:tt)*
    ) => {
        $x.load($e);
        $e.if_else(
            |__e| { $crate::managed_body!(@stmts __e, $r, $rty, $d, $($then)*); },
            |__e| { $crate::managed_body!(@stmts __e, $r, $rty, $d, $($else)*); },
        );
        $crate::managed_body!(@stmts $e, $r, $rty, $d, $($rest)*);
    };
    (@stmts $e:ident, $r:ident, $rty:tt, $d:ident,
        if $x:ident { $($then:tt)* } $($rest:tt)*
    ) => {
        $x.load($e);
        $e.if_then(|__e| { $crate::managed_body!(@stmts __e, $r, $rty, $d, $($then)*); });
        $crate::managed_body!(@stmts $e, $r, $rty, $d, $($rest)*);
    };
    (@stmts $e:ident, $r:ident, $rty:tt, $d:ident,
        raw | $re:ident | $body:block $($rest:tt)*
    ) => {
        {
            let $re: &mut $crate::dotnet::il::Emitter = &mut *$e;
            $body
        }
        $crate::managed_body!(@stmts $e, $r, $rty, $d, $($rest)*);
    };
    (@stmts $e:ident, $r:ident, $rty:tt, $d:ident,
        raw | $re:ident , $rr:ident | $body:block $($rest:tt)*
    ) => {
        {
            let $rr: $crate::dotnet::il::ManagedLocal = $r;
            let $re: &mut $crate::dotnet::il::Emitter = &mut *$e;
            $body
        }
        $crate::managed_body!(@stmts $e, $r, $rty, $d, $($rest)*);
    };
    // x = [recv.]m(...); — assign to an existing local.
    (@stmts $e:ident, $r:ident, $rty:tt, $d:ident,
        $x:ident = $recv:ident . $m:ident ( $($args:tt)* ) ; $($rest:tt)*
    ) => {
        $recv.load_receiver($e);
        $crate::managed_body!(@args $e, $($args)*);
        $m.emit_call($e);
        $x.store($e);
        $crate::managed_body!(@stmts $e, $r, $rty, $d, $($rest)*);
    };
    (@stmts $e:ident, $r:ident, $rty:tt, $d:ident,
        $x:ident = $m:ident ( $($args:tt)* ) ; $($rest:tt)*
    ) => {
        $crate::managed_body!(@args $e, $($args)*);
        $m.emit_call($e);
        $x.store($e);
        $crate::managed_body!(@stmts $e, $r, $rty, $d, $($rest)*);
    };
    (@stmts $e:ident, $r:ident, $rty:tt, $d:ident,
        $recv:ident . $m:ident ( $($args:tt)* ) ; $($rest:tt)*
    ) => {
        $recv.load_receiver($e);
        $crate::managed_body!(@args $e, $($args)*);
        $m.emit_call($e);
        $crate::managed_body!(@stmts $e, $r, $rty, $d, $($rest)*);
    };
    (@stmts $e:ident, $r:ident, $rty:tt, $d:ident,
        $m:ident ( $($args:tt)* ) ; $($rest:tt)*
    ) => {
        $crate::managed_body!(@args $e, $($args)*);
        $m.emit_call($e);
        $crate::managed_body!(@stmts $e, $r, $rty, $d, $($rest)*);
    };

    // ── call arguments: locals, `const <i32 expr>`, or `str "lit"` ───
    (@args $e:ident,) => {};
    (@args $e:ident, str $s:literal) => {
        $e.ldstr($s);
    };
    (@args $e:ident, str $s:literal, $($rest:tt)*) => {
        $e.ldstr($s);
        $crate::managed_body!(@args $e, $($rest)*);
    };
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
