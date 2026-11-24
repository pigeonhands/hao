//! The [`dotnet_class!`](crate::dotnet_class) macro: declare a static .NET
//! class and its methods with Rust-shaped signatures, instead of hand-building
//! [`MethodSig`](crate::dotnet::signature::MethodSig)s out of
//! [`TypeSig`](crate::dotnet::signature::TypeSig) vectors.
//!
//! Bodies are either any expression yielding a
//! [`MethodBody`](crate::dotnet::il::MethodBody) — prebuilt bodies welcome —
//! or an inline `chain { ... }`, which forwards to
//! [`callchain!`](crate::callchain) with the method's own parameter list
//! injected, so the registered signature can never drift out of sync with the
//! chain's parameters.

/// Declares a static class with methods in `module` and returns
/// `hao::error::Result<Entry<TypeDef>>`.
///
/// # Grammar
///
/// ```text
/// dotnet_class! { <expr: &mut Module> =>
///     static class "Name.Space".TypeName {
///         #[unmanaged_callers_only]              // optional, per method
///         fn MethodName(p: i64, q: nint) -> i64
///             = chain { <callchain! sections, without params> };
///         fn Other(x: i32) -> () = some_prebuilt_body;   // any MethodBody expr
///     }
/// }
/// ```
///
/// Parameter and return types are Rust tokens mapped to
/// [`TypeSig`](crate::dotnet::signature::TypeSig): `i64`, `i32`, `nint`,
/// `f64`, `bool`, `object`, `string`, a single-dimension array `[T]`
/// (e.g. `[object]`), and `()` (return only).  `#[unmanaged_callers_only]`
/// resolves and attaches the corlib `UnmanagedCallersOnlyAttribute`.
///
/// Body expressions are evaluated first, in the caller's context — so `?`
/// inside them propagates to the enclosing function with the caller's error
/// type — and the metadata registration then runs in its own
/// `hao::error::Result` scope, returned from the macro.
///
/// # Example
///
/// ```
/// use hao::dotnet::il::{ChainImports, Emitter};
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
/// let prebuilt = Emitter::build(|e| {
///     e.ldc_i4(0).ret();
/// });
///
/// let class = hao::dotnet_class! { &mut module =>
///     static class "Example.Native".Entry {
///         #[unmanaged_callers_only]
///         fn NodeParentName(node: i64, result_ptr: nint) -> i64 = chain {
///             imports: &imports,
///             methods { get_parent: 0x1000usize, get_name: 0x2000usize },
///             errors { err_no_parent = 1 },
///             chain {
///                 let parent = get_parent(node) else err_no_parent;
///                 let name = get_name(parent);
///             },
///             returns result_ptr: NameResult { name: name },
///         };
///         fn Zero() -> i32 = prebuilt;
///     }
/// }
/// .unwrap();
/// assert_eq!(class.get().methods.len(), 2);
/// ```
#[macro_export]
macro_rules! dotnet_class {
    (
        $module:expr =>
        static class $ns:literal . $tyname:ident {
            $($methods:tt)+
        }
    ) => {{
        // Phase 1: evaluate every body in the caller's context (caller's
        // error type for any `?` inside), bound to the method's own ident.
        $crate::dotnet_class!(@bind $($methods)+);
        // Phase 2: register the metadata in a `hao::error::Result` scope.
        let __module = $module;
        (move || -> $crate::error::Result<
            $crate::dotnet::entries::Entry<$crate::dotnet::entries::TypeDef>,
        > {
            let __class = __module.add_static_class($ns, stringify!($tyname))?;
            $crate::dotnet_class!(@register __module, __class, $($methods)+);
            Ok(__class)
        })()
    }};

    // ── phase 1: `let <MethodName> = <body>;` per method ─────────────
    (@bind) => {};
    (@bind
        $( #[$attr:ident] )?
        fn $mname:ident ( $($p:ident : $pk:tt),* $(,)? ) -> $ret:tt
            = chain { $($cc:tt)* } ; $($rest:tt)*
    ) => {
        #[allow(non_snake_case)]
        let $mname = $crate::callchain!(@from_class params( $($p : $pk),* ), $($cc)*);
        $crate::dotnet_class!(@bind $($rest)*);
    };
    (@bind
        $( #[$attr:ident] )?
        fn $mname:ident ( $($p:ident : $pk:tt),* $(,)? ) -> $ret:tt
            = managed { $($mb:tt)* } ; $($rest:tt)*
    ) => {
        #[allow(non_snake_case)]
        let $mname = $crate::managed_body!(@from_class params( $($p : $pk),* ), $($mb)*);
        $crate::dotnet_class!(@bind $($rest)*);
    };
    (@bind
        $( #[$attr:ident] )?
        fn $mname:ident ( $($p:ident : $pk:tt),* $(,)? ) -> $ret:tt
            = $body:expr ; $($rest:tt)*
    ) => {
        #[allow(non_snake_case)]
        let $mname = $body;
        $crate::dotnet_class!(@bind $($rest)*);
    };

    // ── phase 2: build the sig and register each method ──────────────
    (@register $module:ident, $class:ident,) => {};
    (@register $module:ident, $class:ident,
        $( #[$attr:ident] )?
        fn $mname:ident ( $($p:ident : $pk:tt),* $(,)? ) -> $ret:tt
            = chain { $($cc:tt)* } ; $($rest:tt)*
    ) => {
        $crate::dotnet_class!(@add $module, $class,
            $( #[$attr] )? fn $mname ( $($p : $pk),* ) -> $ret);
        $crate::dotnet_class!(@register $module, $class, $($rest)*);
    };
    (@register $module:ident, $class:ident,
        $( #[$attr:ident] )?
        fn $mname:ident ( $($p:ident : $pk:tt),* $(,)? ) -> $ret:tt
            = managed { $($mb:tt)* } ; $($rest:tt)*
    ) => {
        $crate::dotnet_class!(@add $module, $class,
            $( #[$attr] )? fn $mname ( $($p : $pk),* ) -> $ret);
        $crate::dotnet_class!(@register $module, $class, $($rest)*);
    };
    (@register $module:ident, $class:ident,
        $( #[$attr:ident] )?
        fn $mname:ident ( $($p:ident : $pk:tt),* $(,)? ) -> $ret:tt
            = $body:expr ; $($rest:tt)*
    ) => {
        $crate::dotnet_class!(@add $module, $class,
            $( #[$attr] )? fn $mname ( $($p : $pk),* ) -> $ret);
        $crate::dotnet_class!(@register $module, $class, $($rest)*);
    };
    (@add $module:ident, $class:ident,
        $( #[$attr:ident] )?
        fn $mname:ident ( $($p:ident : $pk:tt),* ) -> $ret:tt
    ) => {
        #[allow(unused_mut)]
        let mut __method = $crate::dotnet::entries::Method::public_static(
            stringify!($mname),
            $crate::dotnet::signature::MethodSig::new_static(
                $crate::dotnet_class!(@ty $ret),
                ::std::vec![ $( $crate::dotnet_class!(@ty $pk) ),* ],
            ),
        )
        .with_body($mname);
        $( __method = __method.with_attribute($crate::dotnet_class!(@attr $module, $attr)); )?
        $crate::dotnet::entries::TypeDef::add_method(&$class, __method);
    };

    // ── attributes ───────────────────────────────────────────────────
    (@attr $module:ident, unmanaged_callers_only) => {
        $module.corlib_attribute(
            $crate::dotnet::entries::well_known::SystemRuntimeInteropServicesType::UnmanagedCallersOnlyAttribute,
        )?
    };

    // ── Rust type tokens → TypeSig ───────────────────────────────────
    (@ty i64) => { $crate::dotnet::signature::TypeSig::INT64 };
    (@ty i32) => { $crate::dotnet::signature::TypeSig::INT32 };
    (@ty nint) => { $crate::dotnet::signature::TypeSig::NATIVE_INT };
    (@ty f64) => { $crate::dotnet::signature::TypeSig::FLOAT64 };
    (@ty bool) => { $crate::dotnet::signature::TypeSig::BOOL };
    (@ty object) => { $crate::dotnet::signature::TypeSig::OBJECT };
    (@ty string) => { $crate::dotnet::signature::TypeSig::STRING };
    // A single-element bracket group is one token tree, so `[object]` /
    // `[i32]` map to the corresponding `T[]` single-dimension array.
    (@ty [ $elem:tt ]) => {
        $crate::dotnet::signature::TypeSig::sz_array($crate::dotnet_class!(@ty $elem))
    };
    (@ty ()) => { $crate::dotnet::signature::TypeSig::VOID };
}
