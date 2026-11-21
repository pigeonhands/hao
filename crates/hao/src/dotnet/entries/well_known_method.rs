//! A small vocabulary of common BCL methods that carry their signature, so a
//! caller imports e.g. `Console.WriteLine(string)` in one line without writing
//! the [`MethodSig`] by hand.
//!
//! Mirrors [`WellKnown`](super::well_known::WellKnown): a top-level
//! [`WellKnownMethod`] keyed by declaring type, each wrapping a per-type method
//! sub-enum. Resolve a member against a [`Module`] with
//! [`Module::import_well_known_method`](crate::Module::import_well_known_method).

use super::well_known::{SystemTextType, SystemType, WellKnown};
use super::TypeDefOrRef;
use crate::dotnet::module::Module;
use crate::dotnet::signature::{MethodSig, TypeSig};
use crate::error::Result;

/// Resolves a well-known type, reusing an existing reference or loaded
/// dependency before falling back to the module's corlib reference. See
/// [`WellKnownMethod::resolve`] for why both paths are needed. Used for a
/// member's declaring type and for any class-typed parameter or return type.
fn resolve_type(module: &mut Module, ty: WellKnown) -> Result<TypeDefOrRef> {
    match module.import_type(ty) {
        Ok(resolved) => Ok(resolved),
        Err(_) => module.corlib_type(ty),
    }
}

/// A common BCL method, keyed by declaring type. Each variant wraps a per-type
/// method sub-enum, e.g. `WellKnownMethod::Console(ConsoleMethod::WriteLineStr)`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WellKnownMethod {
    Console(ConsoleMethod),
    Object(ObjectMethod),
    String(StringMethod),
    StringBuilder(StringBuilderMethod),
    Math(MathMethod),
    Convert(ConvertMethod),
    Environment(EnvironmentMethod),
    Gc(GcMethod),
}

impl WellKnownMethod {
    /// The declaring type, as a [`WellKnown`] (which is itself a `TypeName`).
    pub fn declaring(self) -> WellKnown {
        match self {
            Self::Console(_) => WellKnown::System(SystemType::Console),
            Self::Object(_) => WellKnown::System(SystemType::Object),
            Self::String(_) => WellKnown::System(SystemType::String),
            Self::StringBuilder(_) => WellKnown::SystemText(SystemTextType::StringBuilder),
            Self::Math(_) => WellKnown::System(SystemType::Math),
            Self::Convert(_) => WellKnown::System(SystemType::Convert),
            Self::Environment(_) => WellKnown::System(SystemType::Environment),
            Self::Gc(_) => WellKnown::System(SystemType::GC),
        }
    }

    /// The .NET method name, e.g. `"WriteLine"`, `"Append"`, or the property
    /// getter `"get_NewLine"`.
    pub fn method_name(self) -> &'static str {
        match self {
            Self::Console(m) => m.method_name(),
            Self::Object(m) => m.method_name(),
            Self::String(m) => m.method_name(),
            Self::StringBuilder(m) => m.method_name(),
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
            Self::StringBuilder(m) => m.is_instance(),
            Self::Math(m) => m.is_instance(),
            Self::Convert(m) => m.is_instance(),
            Self::Environment(m) => m.is_instance(),
            Self::Gc(m) => m.is_instance(),
        }
    }

    /// Builds the signature. Takes `&mut Module` because a member with a
    /// class-typed parameter or return (e.g. `StringBuilder.Append`) must
    /// resolve those types; members with only primitive/string/object
    /// signatures ignore the module.
    fn signature(self, module: &mut Module) -> Result<MethodSig> {
        match self {
            Self::Console(m) => m.signature(),
            Self::Object(m) => m.signature(module),
            Self::String(m) => m.signature(),
            Self::StringBuilder(m) => m.signature(module),
            Self::Math(m) => m.signature(),
            Self::Convert(m) => m.signature(),
            Self::Environment(m) => m.signature(),
            Self::Gc(m) => m.signature(),
        }
    }

    /// Resolves the declaring type in `module` and returns the pieces needed to
    /// import the member.
    ///
    /// The declaring type (and any class-typed parameter) is resolved via
    /// [`resolve_type`]: prefer an existing reference or loaded dependency, so a
    /// type that lives outside corlib (e.g. `System.Console`, in the
    /// `System.Console` assembly) resolves to the right assembly rather than
    /// being forced into the corlib reference; fall back to corlib for
    /// freshly-generated modules whose dependencies aren't resolved.
    pub fn resolve(
        self,
        module: &mut Module,
    ) -> Result<(TypeDefOrRef, &'static str, MethodSig)> {
        let ty = resolve_type(module, self.declaring())?;
        let signature = self.signature(module)?;
        Ok((ty, self.method_name(), signature))
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
    fn signature(self) -> Result<MethodSig> {
        let params = match self {
            Self::WriteLine => vec![],
            Self::WriteLineStr | Self::WriteStr => vec![TypeSig::STRING],
            Self::WriteLineObj => vec![TypeSig::OBJECT],
            Self::WriteLineI32 => vec![TypeSig::INT32],
            Self::WriteLineBool => vec![TypeSig::BOOL],
            Self::WriteLineChar => vec![TypeSig::CHAR],
        };
        Ok(MethodSig::new_static(TypeSig::VOID, params))
    }
}

/// `System.Object` members.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ObjectMethod {
    /// `ToString()` (instance)
    ToString,
    /// `GetHashCode()` (instance)
    GetHashCode,
    /// `GetType()` (instance) → `System.Type`
    GetType,
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
            Self::GetType => "GetType",
            Self::EqualsObj => "Equals",
            Self::ReferenceEquals => "ReferenceEquals",
        }
    }
    fn is_instance(self) -> bool {
        !matches!(self, Self::ReferenceEquals)
    }
    fn signature(self, module: &mut Module) -> Result<MethodSig> {
        Ok(match self {
            Self::ToString => MethodSig::new_instance(TypeSig::STRING, vec![]),
            Self::GetHashCode => MethodSig::new_instance(TypeSig::INT32, vec![]),
            Self::GetType => {
                let ty = TypeSig::Class(resolve_type(module, WellKnown::System(SystemType::Type))?);
                MethodSig::new_instance(ty, vec![])
            }
            Self::EqualsObj => MethodSig::new_instance(TypeSig::BOOL, vec![TypeSig::OBJECT]),
            Self::ReferenceEquals => {
                MethodSig::new_static(TypeSig::BOOL, vec![TypeSig::OBJECT, TypeSig::OBJECT])
            }
        })
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
    fn signature(self) -> Result<MethodSig> {
        Ok(match self {
            Self::Concat2 => {
                MethodSig::new_static(TypeSig::STRING, vec![TypeSig::STRING, TypeSig::STRING])
            }
            Self::IsNullOrEmpty => MethodSig::new_static(TypeSig::BOOL, vec![TypeSig::STRING]),
            Self::EqualsStr => MethodSig::new_instance(TypeSig::BOOL, vec![TypeSig::STRING]),
        })
    }
}

/// `System.Text.StringBuilder` members (all instance). The fluent members
/// return `StringBuilder`, so their signatures resolve that class type.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum StringBuilderMethod {
    /// `Append(string)` → StringBuilder
    AppendStr,
    /// `Append(char)` → StringBuilder
    AppendChar,
    /// `Append(int)` → StringBuilder
    AppendI32,
    /// `Append(object)` → StringBuilder
    AppendObj,
    /// `Clear()` → StringBuilder
    Clear,
    /// `ToString()` → string
    ToString,
}

impl StringBuilderMethod {
    fn method_name(self) -> &'static str {
        match self {
            Self::AppendStr | Self::AppendChar | Self::AppendI32 | Self::AppendObj => "Append",
            Self::Clear => "Clear",
            Self::ToString => "ToString",
        }
    }
    fn is_instance(self) -> bool {
        true
    }
    fn signature(self, module: &mut Module) -> Result<MethodSig> {
        if let Self::ToString = self {
            return Ok(MethodSig::new_instance(TypeSig::STRING, vec![]));
        }
        let builder = TypeSig::Class(resolve_type(
            module,
            WellKnown::SystemText(SystemTextType::StringBuilder),
        )?);
        let params = match self {
            Self::AppendStr => vec![TypeSig::STRING],
            Self::AppendChar => vec![TypeSig::CHAR],
            Self::AppendI32 => vec![TypeSig::INT32],
            Self::AppendObj => vec![TypeSig::OBJECT],
            Self::Clear => vec![],
            Self::ToString => unreachable!("handled above"),
        };
        Ok(MethodSig::new_instance(builder, params))
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
    fn signature(self) -> Result<MethodSig> {
        Ok(MethodSig::new_static(
            TypeSig::INT32,
            vec![TypeSig::INT32, TypeSig::INT32],
        ))
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
    fn signature(self) -> Result<MethodSig> {
        Ok(MethodSig::new_static(TypeSig::STRING, vec![TypeSig::OBJECT]))
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
    fn signature(self) -> Result<MethodSig> {
        Ok(MethodSig::new_static(TypeSig::STRING, vec![]))
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
    fn signature(self) -> Result<MethodSig> {
        Ok(MethodSig::new_static(TypeSig::VOID, vec![]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dotnet::il::MemberToken;
    use crate::dotnet::module::{ModuleKind, TargetRuntime};

    #[test]
    fn resolves_common_methods() {
        let mut module =
            Module::new("Test", ModuleKind::Dll, TargetRuntime::NetCoreImplementation(8));
        for m in [
            WellKnownMethod::Console(ConsoleMethod::WriteLineStr),
            WellKnownMethod::Object(ObjectMethod::ToString),
            // Class-typed return (System.Type), built with the module.
            WellKnownMethod::Object(ObjectMethod::GetType),
            WellKnownMethod::String(StringMethod::Concat2),
            // Class-typed parameter and return (StringBuilder).
            WellKnownMethod::StringBuilder(StringBuilderMethod::AppendStr),
            WellKnownMethod::StringBuilder(StringBuilderMethod::ToString),
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
            WellKnownMethod::StringBuilder(StringBuilderMethod::AppendChar).method_name(),
            "Append"
        );
        assert_eq!(
            WellKnownMethod::Environment(EnvironmentMethod::GetNewLine).method_name(),
            "get_NewLine"
        );
    }
}
