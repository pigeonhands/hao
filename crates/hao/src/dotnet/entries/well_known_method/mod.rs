//! A small vocabulary of common BCL methods that carry their signature, so a
//! caller imports e.g. `Console.WriteLine(string)` in one line without writing
//! the [`MethodSig`] by hand.
//!
//! Mirrors [`WellKnown`](super::well_known::WellKnown): a top-level
//! [`WellKnownMethod`] keyed by declaring type, each wrapping a per-type method
//! sub-enum. Resolve a member against a [`Module`] with
//! [`Module::import_well_known_method`](crate::Module::import_well_known_method).
//!
//! The whole vocabulary is declared by the [`well_known_methods!`] table below:
//! each line is `Variant = Static|Instance "<.NET name>"(params) -> ret`, where a
//! type is a primitive (`void`/`bool`/`char`/`i32`/`string`/`object`) or a
//! brace-wrapped `{ <WellKnown> }` class type. The macro generates the enums and
//! all the `method_name`/`is_instance`/`signature` impls.

use super::well_known::{SystemReflectionType, SystemTextType, SystemType, WellKnown};
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

/// A class-typed signature fragment, resolved against a module.
fn class(module: &mut Module, ty: WellKnown) -> Result<TypeSig> {
    Ok(TypeSig::Class(resolve_type(module, ty)?))
}

#[macro_use]
mod macros;

well_known_methods! {
    Console(ConsoleMethod) = WellKnown::System(SystemType::Console) => {
        WriteLine     = Static "WriteLine" ()        -> void;
        WriteLineStr  = Static "WriteLine" (string)  -> void;
        WriteLineObj  = Static "WriteLine" (object)  -> void;
        WriteLineI32  = Static "WriteLine" (i32)     -> void;
        WriteLineBool = Static "WriteLine" (bool)    -> void;
        WriteLineChar = Static "WriteLine" (char)    -> void;
        WriteStr      = Static "Write"     (string)  -> void;
        ReadLine      = Static "ReadLine"  ()        -> string;
    }
    Object(ObjectMethod) = WellKnown::System(SystemType::Object) => {
        ToString        = Instance "ToString"        ()               -> string;
        GetHashCode     = Instance "GetHashCode"     ()               -> i32;
        GetType         = Instance "GetType"         ()               -> { WellKnown::System(SystemType::Type) };
        EqualsObj       = Instance "Equals"          (object)         -> bool;
        ReferenceEquals = Static   "ReferenceEquals" (object, object) -> bool;
    }
    Type(TypeMethod) = WellKnown::System(SystemType::Type) => {
        GetFullName  = Instance "get_FullName" ()       -> string;
        GetName      = Instance "get_Name"     ()       -> string;
        GetMethodStr = Instance "GetMethod"    (string) -> { WellKnown::SystemReflection(SystemReflectionType::MethodInfo) };
    }
    String(StringMethod) = WellKnown::System(SystemType::String) => {
        Concat2       = Static   "Concat"        (string, string) -> string;
        FormatObj     = Static   "Format"        (string, object) -> string;
        IsNullOrEmpty = Static   "IsNullOrEmpty" (string)         -> bool;
        EqualsStr     = Instance "Equals"        (string)         -> bool;
        GetLength     = Instance "get_Length"    ()               -> i32;
        SubstringI32  = Instance "Substring"     (i32)            -> string;
        ToUpper       = Instance "ToUpper"       ()               -> string;
        ToLower       = Instance "ToLower"       ()               -> string;
        ContainsStr   = Instance "Contains"      (string)         -> bool;
        ReplaceStr    = Instance "Replace"       (string, string) -> string;
    }
    StringBuilder(StringBuilderMethod) = WellKnown::SystemText(SystemTextType::StringBuilder) => {
        New           = Instance ".ctor"      ()       -> void;
        NewStr        = Instance ".ctor"      (string) -> void;
        AppendStr     = Instance "Append"     (string) -> { WellKnown::SystemText(SystemTextType::StringBuilder) };
        AppendChar    = Instance "Append"     (char)   -> { WellKnown::SystemText(SystemTextType::StringBuilder) };
        AppendI32     = Instance "Append"     (i32)    -> { WellKnown::SystemText(SystemTextType::StringBuilder) };
        AppendObj     = Instance "Append"     (object) -> { WellKnown::SystemText(SystemTextType::StringBuilder) };
        AppendLineStr = Instance "AppendLine" (string) -> { WellKnown::SystemText(SystemTextType::StringBuilder) };
        Clear         = Instance "Clear"      ()       -> { WellKnown::SystemText(SystemTextType::StringBuilder) };
        ToString      = Instance "ToString"   ()       -> string;
    }
    Math(MathMethod) = WellKnown::System(SystemType::Math) => {
        MaxI32 = Static "Max" (i32, i32) -> i32;
        MinI32 = Static "Min" (i32, i32) -> i32;
        AbsI32 = Static "Abs" (i32)      -> i32;
    }
    Convert(ConvertMethod) = WellKnown::System(SystemType::Convert) => {
        ToStringObj  = Static "ToString"  (object) -> string;
        ToInt32Str   = Static "ToInt32"   (string) -> i32;
        ToBooleanStr = Static "ToBoolean" (string) -> bool;
    }
    Environment(EnvironmentMethod) = WellKnown::System(SystemType::Environment) => {
        GetNewLine                = Static "get_NewLine"            ()       -> string;
        GetMachineName            = Static "get_MachineName"        ()       -> string;
        GetProcessorCount         = Static "get_ProcessorCount"     ()       -> i32;
        GetEnvironmentVariableStr = Static "GetEnvironmentVariable" (string) -> string;
        Exit                      = Static "Exit"                   (i32)    -> void;
    }
    Gc(GcMethod) = WellKnown::System(SystemType::GC) => {
        Collect          = Static "Collect"          ()       -> void;
        CollectGen       = Static "Collect"          (i32)    -> void;
        SuppressFinalize = Static "SuppressFinalize" (object) -> void;
    }
}

impl WellKnownMethod {
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
            WellKnownMethod::Console(ConsoleMethod::ReadLine),
            WellKnownMethod::Object(ObjectMethod::ToString),
            // Class-typed return (System.Type), built with the module.
            WellKnownMethod::Object(ObjectMethod::GetType),
            // Class-typed return (System.Reflection.MethodInfo).
            WellKnownMethod::Type(TypeMethod::GetMethodStr),
            WellKnownMethod::Type(TypeMethod::GetFullName),
            WellKnownMethod::String(StringMethod::FormatObj),
            WellKnownMethod::String(StringMethod::ReplaceStr),
            // Constructor + class-typed parameter/return (StringBuilder).
            WellKnownMethod::StringBuilder(StringBuilderMethod::NewStr),
            WellKnownMethod::StringBuilder(StringBuilderMethod::AppendLineStr),
            WellKnownMethod::Math(MathMethod::AbsI32),
            WellKnownMethod::Convert(ConvertMethod::ToInt32Str),
            WellKnownMethod::Environment(EnvironmentMethod::GetMachineName),
            WellKnownMethod::Gc(GcMethod::SuppressFinalize),
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
        // Constructors are instance methods named `.ctor`.
        let ctor = WellKnownMethod::StringBuilder(StringBuilderMethod::NewStr);
        assert_eq!(ctor.method_name(), ".ctor");
        assert!(ctor.is_instance());
        assert_eq!(
            WellKnownMethod::Environment(EnvironmentMethod::GetNewLine).method_name(),
            "get_NewLine"
        );
    }
}
