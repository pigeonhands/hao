//! A small vocabulary of common BCL methods that carry their signature, so a
//! caller imports e.g. `Console.WriteLine(string)` in one line without writing
//! the [`MethodSig`] by hand.
//!
//! Mirrors [`WellKnown`](super::well_known::WellKnown): a top-level
//! [`WellKnownMethod`] keyed by declaring type, each wrapping a per-type method
//! sub-enum. Resolve a member against a [`Module`] with
//! [`Module::import_well_known_method`](crate::Module::import_well_known_method).

use super::well_known::{SystemType, WellKnown};
use super::TypeDefOrRef;
use crate::dotnet::module::Module;
use crate::dotnet::signature::{MethodSig, TypeSig};
use crate::error::Result;

/// A common BCL method, keyed by declaring type. Each variant wraps a per-type
/// method sub-enum, e.g. `WellKnownMethod::Console(ConsoleMethod::WriteLineStr)`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WellKnownMethod {
    Console(ConsoleMethod),
    Object(ObjectMethod),
    String(StringMethod),
    Math(MathMethod),
    Convert(ConvertMethod),
    Environment(EnvironmentMethod),
    Gc(GcMethod),
}

impl WellKnownMethod {
    /// The declaring type, as a [`WellKnown`] (which is itself a `TypeName`).
    pub fn declaring(self) -> WellKnown {
        let ty = match self {
            Self::Console(_) => SystemType::Console,
            Self::Object(_) => SystemType::Object,
            Self::String(_) => SystemType::String,
            Self::Math(_) => SystemType::Math,
            Self::Convert(_) => SystemType::Convert,
            Self::Environment(_) => SystemType::Environment,
            Self::Gc(_) => SystemType::GC,
        };
        WellKnown::System(ty)
    }

    /// The .NET method name, e.g. `"WriteLine"` or the property getter
    /// `"get_NewLine"`.
    pub fn method_name(self) -> &'static str {
        match self {
            Self::Console(m) => m.method_name(),
            Self::Object(m) => m.method_name(),
            Self::String(m) => m.method_name(),
            Self::Math(m) => m.method_name(),
            Self::Convert(m) => m.method_name(),
            Self::Environment(m) => m.method_name(),
            Self::Gc(m) => m.method_name(),
        }
    }

    /// Whether the method is an instance method (vs. static).
    pub fn is_instance(self) -> bool {
        match self {
            Self::Console(m) => m.is_instance(),
            Self::Object(m) => m.is_instance(),
            Self::String(m) => m.is_instance(),
            Self::Math(m) => m.is_instance(),
            Self::Convert(m) => m.is_instance(),
            Self::Environment(m) => m.is_instance(),
            Self::Gc(m) => m.is_instance(),
        }
    }

    fn signature(self) -> MethodSig {
        match self {
            Self::Console(m) => m.signature(),
            Self::Object(m) => m.signature(),
            Self::String(m) => m.signature(),
            Self::Math(m) => m.signature(),
            Self::Convert(m) => m.signature(),
            Self::Environment(m) => m.signature(),
            Self::Gc(m) => m.signature(),
        }
    }

    /// Resolves the declaring type in `module` and returns the pieces needed to
    /// import the member. The single `&mut Module` entry point means members
    /// with class-typed parameters can be added later without a second API; the
    /// starter set's signatures are built from primitives and ignore the module.
    pub fn resolve(
        self,
        module: &mut Module,
    ) -> Result<(TypeDefOrRef, &'static str, MethodSig)> {
        let declaring = self.declaring();
        // Prefer an existing reference or loaded dependency, so a type that
        // lives outside corlib (e.g. `System.Console`, in the `System.Console`
        // assembly) resolves to the right assembly rather than being forced into
        // the corlib reference. Fall back to corlib for freshly-generated modules
        // whose dependencies aren't resolved (where the only reference is corlib).
        let ty = match module.import_type(declaring) {
            Ok(ty) => ty,
            Err(_) => module.corlib_type(declaring)?,
        };
        Ok((ty, self.method_name(), self.signature()))
    }
}

/// `System.Console` members (all static).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ConsoleMethod {
    /// `WriteLine()`
    WriteLine,
    /// `WriteLine(string)`
    WriteLineStr,
    /// `WriteLine(object)`
    WriteLineObj,
    /// `WriteLine(int)`
    WriteLineI32,
    /// `WriteLine(bool)`
    WriteLineBool,
    /// `WriteLine(char)`
    WriteLineChar,
    /// `Write(string)`
    WriteStr,
}

impl ConsoleMethod {
    fn method_name(self) -> &'static str {
        match self {
            Self::WriteStr => "Write",
            _ => "WriteLine",
        }
    }
    fn is_instance(self) -> bool {
        false
    }
    fn signature(self) -> MethodSig {
        let params = match self {
            Self::WriteLine => vec![],
            Self::WriteLineStr | Self::WriteStr => vec![TypeSig::STRING],
            Self::WriteLineObj => vec![TypeSig::OBJECT],
            Self::WriteLineI32 => vec![TypeSig::INT32],
            Self::WriteLineBool => vec![TypeSig::BOOL],
            Self::WriteLineChar => vec![TypeSig::CHAR],
        };
        MethodSig::new_static(TypeSig::VOID, params)
    }
}

/// `System.Object` members.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ObjectMethod {
    /// `ToString()` (instance)
    ToString,
    /// `GetHashCode()` (instance)
    GetHashCode,
    /// `Equals(object)` (instance)
    EqualsObj,
    /// `ReferenceEquals(object, object)` (static)
    ReferenceEquals,
}

impl ObjectMethod {
    fn method_name(self) -> &'static str {
        match self {
            Self::ToString => "ToString",
            Self::GetHashCode => "GetHashCode",
            Self::EqualsObj => "Equals",
            Self::ReferenceEquals => "ReferenceEquals",
        }
    }
    fn is_instance(self) -> bool {
        !matches!(self, Self::ReferenceEquals)
    }
    fn signature(self) -> MethodSig {
        match self {
            Self::ToString => MethodSig::new_instance(TypeSig::STRING, vec![]),
            Self::GetHashCode => MethodSig::new_instance(TypeSig::INT32, vec![]),
            Self::EqualsObj => MethodSig::new_instance(TypeSig::BOOL, vec![TypeSig::OBJECT]),
            Self::ReferenceEquals => {
                MethodSig::new_static(TypeSig::BOOL, vec![TypeSig::OBJECT, TypeSig::OBJECT])
            }
        }
    }
}

/// `System.String` members.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum StringMethod {
    /// `Concat(string, string)` (static)
    Concat2,
    /// `IsNullOrEmpty(string)` (static)
    IsNullOrEmpty,
    /// `Equals(string)` (instance)
    EqualsStr,
}

impl StringMethod {
    fn method_name(self) -> &'static str {
        match self {
            Self::Concat2 => "Concat",
            Self::IsNullOrEmpty => "IsNullOrEmpty",
            Self::EqualsStr => "Equals",
        }
    }
    fn is_instance(self) -> bool {
        matches!(self, Self::EqualsStr)
    }
    fn signature(self) -> MethodSig {
        match self {
            Self::Concat2 => {
                MethodSig::new_static(TypeSig::STRING, vec![TypeSig::STRING, TypeSig::STRING])
            }
            Self::IsNullOrEmpty => MethodSig::new_static(TypeSig::BOOL, vec![TypeSig::STRING]),
            Self::EqualsStr => MethodSig::new_instance(TypeSig::BOOL, vec![TypeSig::STRING]),
        }
    }
}

/// `System.Math` members (all static).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MathMethod {
    /// `Max(int, int)`
    MaxI32,
    /// `Min(int, int)`
    MinI32,
}

impl MathMethod {
    fn method_name(self) -> &'static str {
        match self {
            Self::MaxI32 => "Max",
            Self::MinI32 => "Min",
        }
    }
    fn is_instance(self) -> bool {
        false
    }
    fn signature(self) -> MethodSig {
        MethodSig::new_static(TypeSig::INT32, vec![TypeSig::INT32, TypeSig::INT32])
    }
}

/// `System.Convert` members (static).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ConvertMethod {
    /// `ToString(object)`
    ToStringObj,
}

impl ConvertMethod {
    fn method_name(self) -> &'static str {
        "ToString"
    }
    fn is_instance(self) -> bool {
        false
    }
    fn signature(self) -> MethodSig {
        MethodSig::new_static(TypeSig::STRING, vec![TypeSig::OBJECT])
    }
}

/// `System.Environment` members (static).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum EnvironmentMethod {
    /// `Environment.NewLine` getter
    GetNewLine,
}

impl EnvironmentMethod {
    fn method_name(self) -> &'static str {
        "get_NewLine"
    }
    fn is_instance(self) -> bool {
        false
    }
    fn signature(self) -> MethodSig {
        MethodSig::new_static(TypeSig::STRING, vec![])
    }
}

/// `System.GC` members (static).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum GcMethod {
    /// `Collect()`
    Collect,
}

impl GcMethod {
    fn method_name(self) -> &'static str {
        "Collect"
    }
    fn is_instance(self) -> bool {
        false
    }
    fn signature(self) -> MethodSig {
        MethodSig::new_static(TypeSig::VOID, vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dotnet::il::MemberToken;
    use crate::dotnet::module::{ModuleKind, TargetRuntime};

    #[test]
    fn resolves_common_methods() {
        let mut module = Module::new("Test", ModuleKind::Dll, TargetRuntime::NetCoreImplementation(8));
        for m in [
            WellKnownMethod::Console(ConsoleMethod::WriteLineStr),
            WellKnownMethod::Object(ObjectMethod::ToString),
            WellKnownMethod::String(StringMethod::Concat2),
            WellKnownMethod::Math(MathMethod::MaxI32),
            WellKnownMethod::Convert(ConvertMethod::ToStringObj),
            WellKnownMethod::Environment(EnvironmentMethod::GetNewLine),
            WellKnownMethod::Gc(GcMethod::Collect),
        ] {
            let token = module
                .import_well_known_method(m)
                .expect("well-known method resolves");
            assert!(matches!(
                token,
                MemberToken::MemberRef(_) | MemberToken::Method(_)
            ));
        }
    }

    #[test]
    fn accessors() {
        let write = WellKnownMethod::Console(ConsoleMethod::WriteLineStr);
        assert_eq!(write.method_name(), "WriteLine");
        assert!(!write.is_instance());
        assert!(WellKnownMethod::Object(ObjectMethod::ToString).is_instance());
        assert!(!WellKnownMethod::Object(ObjectMethod::ReferenceEquals).is_instance());
        assert_eq!(
            WellKnownMethod::Environment(EnvironmentMethod::GetNewLine).method_name(),
            "get_NewLine"
        );
    }
}
