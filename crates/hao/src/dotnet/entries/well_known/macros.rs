//! The `well_known_namespaces!` macro: generates the `WellKnown` enum and,
//! per sub-enum, the `from_full_name`/`full_name` arms and the
//! `from_type_name`/`name`/`TypeName` impls from a declarative table.

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
