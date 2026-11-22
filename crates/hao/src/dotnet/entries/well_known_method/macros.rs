//! The `well_known_methods!` table macro and its `wk_type!` / `wk_is_instance!`
//! / `wk_new_sig!` helpers: generate `WellKnownMethod` and the per-type method
//! sub-enums with their `method_name`/`is_instance`/`signature` impls.

/// Expands a type token to a `TypeSig`. A class type (`{ <WellKnown> }`) resolves
/// against the module and may early-return via `?`, so it is only valid inside a
/// function returning `Result`.
macro_rules! wk_type {
    ($m:ident, void) => {
        TypeSig::VOID
    };
    ($m:ident, bool) => {
        TypeSig::BOOL
    };
    ($m:ident, char) => {
        TypeSig::CHAR
    };
    ($m:ident, i32) => {
        TypeSig::INT32
    };
    ($m:ident, string) => {
        TypeSig::STRING
    };
    ($m:ident, object) => {
        TypeSig::OBJECT
    };
    ($m:ident, { $wk:expr }) => {
        class($m, $wk)?
    };
}

macro_rules! wk_is_instance {
    (Static) => {
        false
    };
    (Instance) => {
        true
    };
}

macro_rules! wk_new_sig {
    (Static, $ret:expr, $params:expr) => {
        MethodSig::new_static($ret, $params)
    };
    (Instance, $ret:expr, $params:expr) => {
        MethodSig::new_instance($ret, $params)
    };
}

/// Generates [`WellKnownMethod`], every per-type method sub-enum, and their
/// `method_name`/`is_instance`/`signature` impls from a declarative table.
macro_rules! well_known_methods {
    ( $(
        $group:ident($sub:ident) = $declaring:expr => {
            $( $variant:ident = $kind:ident $name:literal ( $($param:tt),* $(,)? ) -> $ret:tt ; )+
        }
    )+ ) => {
        /// A common BCL method, keyed by declaring type. Each variant wraps a
        /// per-type method sub-enum, e.g.
        /// `WellKnownMethod::Console(ConsoleMethod::WriteLineStr)`.
        #[derive(Debug, Copy, Clone, PartialEq, Eq)]
        pub enum WellKnownMethod {
            $( $group($sub), )+
        }

        impl WellKnownMethod {
            /// The declaring type, as a [`WellKnown`] (which is itself a `TypeName`).
            pub fn declaring(self) -> WellKnown {
                match self { $( Self::$group(_) => $declaring, )+ }
            }

            /// The .NET method name, e.g. `"WriteLine"`, `".ctor"`, or a property
            /// getter like `"get_NewLine"`.
            pub fn method_name(self) -> &'static str {
                match self { $( Self::$group(m) => m.method_name(), )+ }
            }

            /// Whether the method is an instance method (vs. static).
            /// Constructors are instance methods.
            pub fn is_instance(self) -> bool {
                match self { $( Self::$group(m) => m.is_instance(), )+ }
            }

            fn signature(self, module: &mut Module) -> Result<MethodSig> {
                match self { $( Self::$group(m) => m.signature(module), )+ }
            }
        }

        $(
            #[derive(Debug, Copy, Clone, PartialEq, Eq)]
            pub enum $sub {
                $( $variant, )+
            }

            impl $sub {
                fn method_name(self) -> &'static str {
                    match self { $( Self::$variant => $name, )+ }
                }
                fn is_instance(self) -> bool {
                    match self { $( Self::$variant => wk_is_instance!($kind), )+ }
                }
                #[allow(unused_variables)]
                fn signature(self, module: &mut Module) -> Result<MethodSig> {
                    Ok(match self {
                        $(
                            Self::$variant => wk_new_sig!(
                                $kind,
                                wk_type!(module, $ret),
                                vec![ $( wk_type!(module, $param) ),* ]
                            ),
                        )+
                    })
                }
            }
        )+
    };
}
