//! The table macros for this module:
//! - `well_known_namespaces!` generates the `WellKnown` enum and, per sub-enum,
//!   the `from_full_name`/`full_name` arms and the `from_type_name`/`name`/
//!   `TypeName` impls.
//! - `well_known_methods!` (with its `wk_type!`/`wk_is_instance!`/`wk_new_sig!`
//!   helpers) generates `WellKnownMethod` and the per-type method sub-enums.

macro_rules! well_known_namespaces {
    ( $( $ns:literal => $variant:ident($sub:ident) ),+ $(,)? ) => {
        /// A well-known .NET type, grouped by namespace. `from_full_name`,
        /// `full_name` and `name` strip the generic arity suffix (e.g.
        /// `Nullable\`1` -> `Nullable`).
        #[derive(Debug, Copy, Clone, PartialEq)]
        pub enum WellKnown {
            $( $variant($sub), )+
        }

        impl WellKnown {
            pub fn from_full_name(namespace: &str, name: &str) -> Option<Self> {
                Some(match namespace {
                    $( $ns => Self::$variant($sub::from_type_name(name)?), )+
                    _ => return None,
                })
            }

            pub fn full_name(&self) -> (&'static str, &'static str) {
                match self {
                    $( Self::$variant(ty) => ($ns, ty.name()), )+
                }
            }

            pub fn namespace(&self) -> &'static str {
                self.full_name().0
            }

            pub fn type_name(&self) -> &'static str {
                self.full_name().1
            }
        }

        $(
            impl $sub {
                /// Parses a type name (with or without generic arity) into this
                /// well-known type.
                pub fn from_type_name(name: &str) -> Option<Self> {
                    name.parse().ok()
                }

                /// The type name without the generic arity suffix.
                pub fn name(&self) -> &'static str {
                    strip_arity(self.into())
                }
            }

            impl TypeName for $sub {
                fn type_name(&self) -> (Cow<'_, str>, Cow<'_, str>) {
                    let (namespace, name) = WellKnown::$variant(*self).full_name();
                    (Cow::Borrowed(namespace), Cow::Borrowed(name))
                }
            }
        )+
    };
}

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
    ($m:ident, i64) => {
        TypeSig::INT64
    };
    ($m:ident, f64) => {
        TypeSig::FLOAT64
    };
    ($m:ident, nint) => {
        TypeSig::NATIVE_INT
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
    // valuetype wrapped in parens: (valuetype { WellKnown::... })
    ($m:ident, (valuetype { $wk:expr })) => {
        value_type($m, $wk)?
    };
    // array wrapped in brackets: [{ WellKnown::... }] or [i32] etc.
    ($m:ident, [$($inner:tt)+]) => {
        TypeSig::sz_array(wk_type!($m, $($inner)+))
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
///
/// Each parameter and the return type is a single token tree consumed by
/// `wk_type!`. Primitives (`void`, `bool`, `i32`, `nint`, …), `string` and
/// `object` are bare keywords.  Class types are `{ WellKnown::… }`, value
/// types `(valuetype { WellKnown::… })`, and arrays `[inner]` where `inner`
/// is any of the above forms.
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
