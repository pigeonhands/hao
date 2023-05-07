use phf::phf_map;

/// Check if a name is a well known one.
/// the `full_name` and `name` methods will return the name
/// without the generic tags.
/// 
/// e.g
/// ```
/// # use hao::dotnet::entries::well_known::{WellKnown, SystemType};
/// 
/// let nullable = WellKnown::from_full_name("System", "Nullable`1");
/// assert_eq!(nullable, Some(WellKnown::System(SystemType::Nullable_1)));
/// assert_eq!(nullable.unwrap().full_name(), ("System", "Nullable"));
/// ```
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum WellKnown {
    System(SystemType),
    SystemRuntimeCompilerServices(SystemRuntimeCompilerServicesType),
    SystemCollectionsGeneric(SystemCollectionsGenericType),
    SystemCollections(SystemCollectionsType),
    SystemCollectionsObjectModel(SystemCollectionsObjectModelType),
    SystemCollectionsSpecialized(SystemCollectionsSpecializedType),
    SystemReflection(SystemReflectionType),
    SystemDiagnostics(SystemDiagnosticsType),
    SystemRuntimeExceptionServices(SystemRuntimeExceptionServicesType),
    SystemRuntimeInteropServices(SystemRuntimeInteropServicesType),
    SystemThreadingTasks(SystemThreadingTasksType),
    SystemThreading(SystemThreadingType),
    MicrosoftCSharpRuntimeBinder(MicrosoftCSharpRuntimeBinderType),
    MicrosoftVisualBasic(MicrosoftVisualBasicType),
    MicrosoftVisualBasicCompilerServices(MicrosoftVisualBasicCompilerServicesType),
    MicrosoftVisualBasicApplicationServices(MicrosoftVisualBasicApplicationServicesType),
    SystemRuntimeInteropServicesWindowsRuntime(SystemRuntimeInteropServicesWindowsRuntimeType),
    WindowsFoundation(WindowsFoundationType),
    SystemComponentModel(SystemComponentModelType),
    SystemLinq(SystemLinqType),
    SystemLinqExpressions(SystemLinqExpressionsType),
    SystemXmlLinq(SystemXmlLinqType),
    SystemSecurity(SystemSecurityType),
    SystemSecurityPermissions(SystemSecurityPermissionsType),
    SystemWindowsForms(SystemWindowsFormsType),
    SystemRuntime(SystemRuntimeType),
    SystemText(SystemTextType),
    SystemThreadingTasksSources(SystemThreadingTasksSourcesType),
}

impl WellKnown {
    pub fn from_full_name(namespace: &str, name: &str) -> Option<Self> {
        let ty = match namespace {
            "System" => Self::System(SystemType::from_type_name(name)?),
            "System.Runtime.CompilerServices" => Self::SystemRuntimeCompilerServices(
                SystemRuntimeCompilerServicesType::from_type_name(name)?,
            ),
            "System.Collections.Generic" => {
                Self::SystemCollectionsGeneric(SystemCollectionsGenericType::from_type_name(name)?)
            }
            "System.Collections" => {
                Self::SystemCollections(SystemCollectionsType::from_type_name(name)?)
            }
            "System.Collections.ObjectModel" => Self::SystemCollectionsObjectModel(
                SystemCollectionsObjectModelType::from_type_name(name)?,
            ),
            "System.Collections.Specialized" => Self::SystemCollectionsSpecialized(
                SystemCollectionsSpecializedType::from_type_name(name)?,
            ),
            "System.Reflection" => {
                Self::SystemReflection(SystemReflectionType::from_type_name(name)?)
            }
            "System.Diagnostics" => {
                Self::SystemDiagnostics(SystemDiagnosticsType::from_type_name(name)?)
            }
            "System.Runtime.ExceptionServices" => Self::SystemRuntimeExceptionServices(
                SystemRuntimeExceptionServicesType::from_type_name(name)?,
            ),
            "System.Runtime.InteropServices" => Self::SystemRuntimeInteropServices(
                SystemRuntimeInteropServicesType::from_type_name(name)?,
            ),
            "System.Threading.Tasks" => {
                Self::SystemThreadingTasks(SystemThreadingTasksType::from_type_name(name)?)
            }
            "System.Threading" => Self::SystemThreading(SystemThreadingType::from_type_name(name)?),
            "Microsoft.CSharp.RuntimeBinder" => Self::MicrosoftCSharpRuntimeBinder(
                MicrosoftCSharpRuntimeBinderType::from_type_name(name)?,
            ),
            "Microsoft.VisualBasic" => {
                Self::MicrosoftVisualBasic(MicrosoftVisualBasicType::from_type_name(name)?)
            }
            "Microsoft.VisualBasic.CompilerServices" => Self::MicrosoftVisualBasicCompilerServices(
                MicrosoftVisualBasicCompilerServicesType::from_type_name(name)?,
            ),
            "Microsoft.VisualBasic.ApplicationServices" => {
                Self::MicrosoftVisualBasicApplicationServices(
                    MicrosoftVisualBasicApplicationServicesType::from_type_name(name)?,
                )
            }
            "System.Runtime.InteropServices.WindowsRuntime" => {
                Self::SystemRuntimeInteropServicesWindowsRuntime(
                    SystemRuntimeInteropServicesWindowsRuntimeType::from_type_name(name)?,
                )
            }
            "Windows.Foundation" => {
                Self::WindowsFoundation(WindowsFoundationType::from_type_name(name)?)
            }
            "System.ComponentModel" => {
                Self::SystemComponentModel(SystemComponentModelType::from_type_name(name)?)
            }
            "System.Linq" => Self::SystemLinq(SystemLinqType::from_type_name(name)?),
            "System.Linq.Expressions" => {
                Self::SystemLinqExpressions(SystemLinqExpressionsType::from_type_name(name)?)
            }
            "System.Xml.Linq" => Self::SystemXmlLinq(SystemXmlLinqType::from_type_name(name)?),
            "System.Security" => Self::SystemSecurity(SystemSecurityType::from_type_name(name)?),
            "System.Security.Permissions" => Self::SystemSecurityPermissions(
                SystemSecurityPermissionsType::from_type_name(name)?,
            ),
            "System.Windows.Forms" => {
                Self::SystemWindowsForms(SystemWindowsFormsType::from_type_name(name)?)
            }
            "System.Runtime" => Self::SystemRuntime(SystemRuntimeType::from_type_name(name)?),
            "System.Text" => Self::SystemText(SystemTextType::from_type_name(name)?),
            "System.Threading.Tasks.Sources" => Self::SystemThreadingTasksSources(
                SystemThreadingTasksSourcesType::from_type_name(name)?,
            ),

            _ => return None,
        };
        Some(ty)
    }
    
    pub fn full_name(&self) -> (&'static str, &'static str) {
        match self {
            Self::System(ty) => ("System", ty.name()),
            Self::SystemRuntimeCompilerServices(ty) => {
                ("System.Runtime.CompilerServices", ty.name())
            }
            Self::SystemCollectionsGeneric(ty) => ("System.Collections.Generic", ty.name()),
            Self::SystemCollections(ty) => ("System.Collections", ty.name()),
            Self::SystemCollectionsObjectModel(ty) => ("System.Collections.ObjectModel", ty.name()),
            Self::SystemCollectionsSpecialized(ty) => ("System.Collections.Specialized", ty.name()),
            Self::SystemReflection(ty) => ("System.Reflection", ty.name()),
            Self::SystemDiagnostics(ty) => ("System.Diagnostics", ty.name()),
            Self::SystemRuntimeExceptionServices(ty) => {
                ("System.Runtime.ExceptionServices", ty.name())
            }
            Self::SystemRuntimeInteropServices(ty) => ("System.Runtime.InteropServices", ty.name()),
            Self::SystemThreadingTasks(ty) => ("System.Threading.Tasks", ty.name()),
            Self::SystemThreading(ty) => ("System.Threading", ty.name()),
            Self::MicrosoftCSharpRuntimeBinder(ty) => ("Microsoft.CSharp.RuntimeBinder", ty.name()),
            Self::MicrosoftVisualBasic(ty) => ("Microsoft.VisualBasic", ty.name()),
            Self::MicrosoftVisualBasicCompilerServices(ty) => {
                ("Microsoft.VisualBasic.CompilerServices", ty.name())
            }
            Self::MicrosoftVisualBasicApplicationServices(ty) => {
                ("Microsoft.VisualBasic.ApplicationServices", ty.name())
            }
            Self::SystemRuntimeInteropServicesWindowsRuntime(ty) => {
                ("System.Runtime.InteropServices.WindowsRuntime", ty.name())
            }
            Self::WindowsFoundation(ty) => ("Windows.Foundation", ty.name()),
            Self::SystemComponentModel(ty) => ("System.ComponentModel", ty.name()),
            Self::SystemLinq(ty) => ("System.Linq", ty.name()),
            Self::SystemLinqExpressions(ty) => ("System.Linq.Expressions", ty.name()),
            Self::SystemXmlLinq(ty) => ("System.Xml.Linq", ty.name()),
            Self::SystemSecurity(ty) => ("System.Security", ty.name()),
            Self::SystemSecurityPermissions(ty) => ("System.Security.Permissions", ty.name()),
            Self::SystemWindowsForms(ty) => ("System.Windows.Forms", ty.name()),
            Self::SystemRuntime(ty) => ("System.Runtime", ty.name()),
            Self::SystemText(ty) => ("System.Text", ty.name()),
            Self::SystemThreadingTasksSources(ty) => ("System.Threading.Tasks.Sources", ty.name()),
        }
    }
    pub fn namespace(&self) -> &'static str {
        self.full_name().0
    }
    pub fn type_name(&self) -> &'static str {
        self.full_name().1
    }
}

static SYSTEM: phf::Map<&'static str, SystemType> = phf_map! {
    "Object" => SystemType::Object,
    "Enum" => SystemType::Enum,
    "MulticastDelegate" => SystemType::MulticastDelegate,
    "Delegate" => SystemType::Delegate,
    "ValueType" => SystemType::ValueType,
    "Void" => SystemType::Void,
    "Boolean" => SystemType::Boolean,
    "Char" => SystemType::Char,
    "SByte" => SystemType::SByte,
    "Byte" => SystemType::Byte,
    "Int16" => SystemType::Int16,
    "UInt16" => SystemType::UInt16,
    "Int32" => SystemType::Int32,
    "UInt32" => SystemType::UInt32,
    "Int64" => SystemType::Int64,
    "UInt64" => SystemType::UInt64,
    "Decimal" => SystemType::Decimal,
    "Single" => SystemType::Single,
    "Double" => SystemType::Double,
    "String" => SystemType::String,
    "IntPtr" => SystemType::IntPtr,
    "UIntPtr" => SystemType::UIntPtr,
    "Array" => SystemType::Array,
    "Nullable`1" => SystemType::Nullable_1,
    "DateTime" => SystemType::DateTime,
    "IDisposable" => SystemType::IDisposable,
    "TypedReference" => SystemType::TypedReference,
    "ArgIterator" => SystemType::ArgIterator,
    "RuntimeArgumentHandle" => SystemType::RuntimeArgumentHandle,
    "RuntimeFieldHandle" => SystemType::RuntimeFieldHandle,
    "RuntimeMethodHandle" => SystemType::RuntimeMethodHandle,
    "RuntimeTypeHandle" => SystemType::RuntimeTypeHandle,
    "IAsyncResult" => SystemType::IAsyncResult,
    "AsyncCallback" => SystemType::AsyncCallback,
    "Math" => SystemType::Math,
    "Attribute" => SystemType::Attribute,
    "CLSCompliantAttribute" => SystemType::CLSCompliantAttribute,
    "Convert" => SystemType::Convert,
    "Exception" => SystemType::Exception,
    "FlagsAttribute" => SystemType::FlagsAttribute,
    "FormattableString" => SystemType::FormattableString,
    "Guid" => SystemType::Guid,
    "IFormattable" => SystemType::IFormattable,
    "MarshalByRefObject" => SystemType::MarshalByRefObject,
    "Type" => SystemType::Type,
    "Func`1" => SystemType::Func_1,
    "Func`2" => SystemType::Func_2,
    "Func`3" => SystemType::Func_3,
    "Func`4" => SystemType::Func_4,
    "Func`5" => SystemType::Func_5,
    "Func`6" => SystemType::Func_6,
    "Func`7" => SystemType::Func_7,
    "Func`8" => SystemType::Func_8,
    "Func`9" => SystemType::Func_9,
    "Func`10" => SystemType::Func_10,
    "Func`11" => SystemType::Func_11,
    "Func`12" => SystemType::Func_12,
    "Func`13" => SystemType::Func_13,
    "Func`14" => SystemType::Func_14,
    "Func`15" => SystemType::Func_15,
    "Func`16" => SystemType::Func_16,
    "Func`17" => SystemType::Func_17,
    "Action" => SystemType::Action,
    "Action`1" => SystemType::Action_1,
    "Action`2" => SystemType::Action_2,
    "Action`3" => SystemType::Action_3,
    "Action`4" => SystemType::Action_4,
    "Action`5" => SystemType::Action_5,
    "Action`6" => SystemType::Action_6,
    "Action`7" => SystemType::Action_7,
    "Action`8" => SystemType::Action_8,
    "Action`9" => SystemType::Action_9,
    "Action`10" => SystemType::Action_10,
    "Action`11" => SystemType::Action_11,
    "Action`12" => SystemType::Action_12,
    "Action`13" => SystemType::Action_13,
    "Action`14" => SystemType::Action_14,
    "Action`15" => SystemType::Action_15,
    "Action`16" => SystemType::Action_16,
    "Activator" => SystemType::Activator,

    "AttributeUsageAttribute" => SystemType::AttributeUsageAttribute,
    "ParamArrayAttribute" => SystemType::ParamArrayAttribute,
    "NonSerializedAttribute" => SystemType::NonSerializedAttribute,
    "STAThreadAttribute" => SystemType::STAThreadAttribute,
    "IEquatable`1" => SystemType::IEquatable_1,
    "NotSupportedException" => SystemType::NotSupportedException,
    "Environment" => SystemType::Environment,
    "IFormatProvider" => SystemType::IFormatProvider,
    "ValueTuple`1" => SystemType::ValueTuple_1,
    "ValueTuple`2" => SystemType::ValueTuple_2,
    "ValueTuple`3" => SystemType::ValueTuple_3,
    "ValueTuple`4" => SystemType::ValueTuple_4,
    "ValueTuple`5" => SystemType::ValueTuple_5,
    "ValueTuple`6" => SystemType::ValueTuple_6,
    "ValueTuple`7" => SystemType::ValueTuple_7,
    "ValueTuple`8" => SystemType::ValueTuple_8,
    "ContextBoundObject" => SystemType::ContextBoundObject,
    "SerializableAttribute" => SystemType::SerializableAttribute,
    "RuntimeMethodHandleInternal" => SystemType::RuntimeMethodHandleInternal,
    "ByReference`1" => SystemType::ByReference_1,
    "__ComObject" => SystemType::__ComObject,
    "DBNull" => SystemType::DBNull,
    "ObsoleteAttribute" => SystemType::ObsoleteAttribute,
    "Span`1" => SystemType::Span_1,
    "ReadOnlySpan`1" => SystemType::ReadOnlySpan_1,
    "Index" => SystemType::Index,
    "Range" => SystemType::Range,
    "IAsyncDisposable" => SystemType::IAsyncDisposable,
};

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SystemType {
    Object,
    Enum,
    MulticastDelegate,
    Delegate,
    ValueType,
    Void,
    Boolean,
    Char,
    SByte,
    Byte,
    Int16,
    UInt16,
    Int32,
    UInt32,
    Int64,
    UInt64,
    Decimal,
    Single,
    Double,
    String,
    IntPtr,
    UIntPtr,
    Array,
    Nullable_1,
    DateTime,
    IDisposable,
    TypedReference,
    ArgIterator,
    RuntimeArgumentHandle,
    RuntimeFieldHandle,
    RuntimeMethodHandle,
    RuntimeTypeHandle,
    IAsyncResult,
    AsyncCallback,
    Math,
    Attribute,
    CLSCompliantAttribute,
    Convert,
    Exception,
    FlagsAttribute,
    FormattableString,
    Guid,
    IFormattable,
    MarshalByRefObject,
    Type,
    Func_1,
    Func_2,
    Func_3,
    Func_4,
    Func_5,
    Func_6,
    Func_7,
    Func_8,
    Func_9,
    Func_10,
    Func_11,
    Func_12,
    Func_13,
    Func_14,
    Func_15,
    Func_16,
    Func_17,
    Action,
    Action_1,
    Action_2,
    Action_3,
    Action_4,
    Action_5,
    Action_6,
    Action_7,
    Action_8,
    Action_9,
    Action_10,
    Action_11,
    Action_12,
    Action_13,
    Action_14,
    Action_15,
    Action_16,
    Activator,

    AttributeUsageAttribute,
    ParamArrayAttribute,
    NonSerializedAttribute,
    STAThreadAttribute,
    IEquatable_1,
    NotSupportedException,
    Environment,
    IFormatProvider,
    ValueTuple_1,
    ValueTuple_2,
    ValueTuple_3,
    ValueTuple_4,
    ValueTuple_5,
    ValueTuple_6,
    ValueTuple_7,
    ValueTuple_8,
    ContextBoundObject,
    SerializableAttribute,
    RuntimeMethodHandleInternal,
    ByReference_1,
    __ComObject,
    DBNull,
    ObsoleteAttribute,
    Span_1,
    ReadOnlySpan_1,
    Index,
    Range,
    IAsyncDisposable,
}
impl SystemType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        SYSTEM.get(name).copied()
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Object => "Object",
            Self::Enum => "Enum",
            Self::MulticastDelegate => "MulticastDelegate",
            Self::Delegate => "Delegate",
            Self::ValueType => "ValueType",
            Self::Void => "Void",
            Self::Boolean => "Boolean",
            Self::Char => "Char",
            Self::SByte => "SByte",
            Self::Byte => "Byte",
            Self::Int16 => "Int16",
            Self::UInt16 => "UInt16",
            Self::Int32 => "Int32",
            Self::UInt32 => "UInt32",
            Self::Int64 => "Int64",
            Self::UInt64 => "UInt64",
            Self::Decimal => "Decimal",
            Self::Single => "Single",
            Self::Double => "Double",
            Self::String => "String",
            Self::IntPtr => "IntPtr",
            Self::UIntPtr => "UIntPtr",
            Self::Array => "Array",
            Self::Nullable_1 => "Nullable",
            Self::DateTime => "DateTime",
            Self::IDisposable => "IDisposable",
            Self::TypedReference => "TypedReference",
            Self::ArgIterator => "ArgIterator",
            Self::RuntimeArgumentHandle => "RuntimeArgumentHandle",
            Self::RuntimeFieldHandle => "RuntimeFieldHandle",
            Self::RuntimeMethodHandle => "RuntimeMethodHandle",
            Self::RuntimeTypeHandle => "RuntimeTypeHandle",
            Self::IAsyncResult => "IAsyncResult",
            Self::AsyncCallback => "AsyncCallback",
            Self::Math => "Math",
            Self::Attribute => "Attribute",
            Self::CLSCompliantAttribute => "CLSCompliantAttribute",
            Self::Convert => "Convert",
            Self::Exception => "Exception",
            Self::FlagsAttribute => "FlagsAttribute",
            Self::FormattableString => "FormattableString",
            Self::Guid => "Guid",
            Self::IFormattable => "IFormattable",
            Self::MarshalByRefObject => "MarshalByRefObject",
            Self::Type => "Type",
            Self::Func_1 => "Func",
            Self::Func_2 => "Func",
            Self::Func_3 => "Func",
            Self::Func_4 => "Func",
            Self::Func_5 => "Func",
            Self::Func_6 => "Func",
            Self::Func_7 => "Func",
            Self::Func_8 => "Func",
            Self::Func_9 => "Func",
            Self::Func_10 => "Func`10",
            Self::Func_11 => "Func`11",
            Self::Func_12 => "Func`12",
            Self::Func_13 => "Func`13",
            Self::Func_14 => "Func`14",
            Self::Func_15 => "Func`15",
            Self::Func_16 => "Func`16",
            Self::Func_17 => "Func`17",
            Self::Action => "Action",
            Self::Action_1 => "Action",
            Self::Action_2 => "Action",
            Self::Action_3 => "Action",
            Self::Action_4 => "Action",
            Self::Action_5 => "Action",
            Self::Action_6 => "Action",
            Self::Action_7 => "Action",
            Self::Action_8 => "Action",
            Self::Action_9 => "Action",
            Self::Action_10 => "Action`10",
            Self::Action_11 => "Action`11",
            Self::Action_12 => "Action`12",
            Self::Action_13 => "Action`13",
            Self::Action_14 => "Action`14",
            Self::Action_15 => "Action`15",
            Self::Action_16 => "Action`16",
            Self::Activator => "Activator",
            Self::AttributeUsageAttribute => "AttributeUsageAttribute",
            Self::ParamArrayAttribute => "ParamArrayAttribute",
            Self::NonSerializedAttribute => "NonSerializedAttribute",
            Self::STAThreadAttribute => "STAThreadAttribute",
            Self::IEquatable_1 => "IEquatable",
            Self::NotSupportedException => "NotSupportedException",
            Self::Environment => "Environment",
            Self::IFormatProvider => "IFormatProvider",
            Self::ValueTuple_1 => "ValueTuple",
            Self::ValueTuple_2 => "ValueTuple",
            Self::ValueTuple_3 => "ValueTuple",
            Self::ValueTuple_4 => "ValueTuple",
            Self::ValueTuple_5 => "ValueTuple",
            Self::ValueTuple_6 => "ValueTuple",
            Self::ValueTuple_7 => "ValueTuple",
            Self::ValueTuple_8 => "ValueTuple",
            Self::ContextBoundObject => "ContextBoundObject",
            Self::SerializableAttribute => "SerializableAttribute",
            Self::RuntimeMethodHandleInternal => "RuntimeMethodHandleInternal",
            Self::ByReference_1 => "ByReference",
            Self::__ComObject => "__ComObject",
            Self::DBNull => "DBNull",
            Self::ObsoleteAttribute => "ObsoleteAttribute",
            Self::Span_1 => "Span",
            Self::ReadOnlySpan_1 => "ReadOnlySpan",
            Self::Index => "Index",
            Self::Range => "Range",
            Self::IAsyncDisposable => "IAsyncDisposable",
        }
    }
}

static SYSTEM_RUNTIME_COMPILER_SERVICE: phf::Map<&'static str, SystemRuntimeCompilerServicesType> = phf_map! {
    "IsVolatile" => SystemRuntimeCompilerServicesType::IsVolatile,
    "FormattableStringFactory" => SystemRuntimeCompilerServicesType::FormattableStringFactory,
    "RuntimeHelpers" => SystemRuntimeCompilerServicesType::RuntimeHelpers,
    "DateTimeConstantAttribute" => SystemRuntimeCompilerServicesType::DateTimeConstantAttribute,
    "DecimalConstantAttribute" => SystemRuntimeCompilerServicesType::DecimalConstantAttribute,
    "IUnknownConstantAttribute" => SystemRuntimeCompilerServicesType::IUnknownConstantAttribute,
    "IDispatchConstantAttribute" => SystemRuntimeCompilerServicesType::IDispatchConstantAttribute,
    "ExtensionAttribute" => SystemRuntimeCompilerServicesType::ExtensionAttribute,
    "INotifyCompletion" => SystemRuntimeCompilerServicesType::INotifyCompletion,
    "InternalsVisibleToAttribute" => SystemRuntimeCompilerServicesType::InternalsVisibleToAttribute,
    "CompilerGeneratedAttribute" => SystemRuntimeCompilerServicesType::CompilerGeneratedAttribute,
    "AccessedThroughPropertyAttribute" => SystemRuntimeCompilerServicesType::AccessedThroughPropertyAttribute,
    "CompilationRelaxationsAttribute" => SystemRuntimeCompilerServicesType::CompilationRelaxationsAttribute,
    "RuntimeCompatibilityAttribute" => SystemRuntimeCompilerServicesType::RuntimeCompatibilityAttribute,
    "UnsafeValueTypeAttribute" => SystemRuntimeCompilerServicesType::UnsafeValueTypeAttribute,
    "FixedBufferAttribute" => SystemRuntimeCompilerServicesType::FixedBufferAttribute,
    "DynamicAttribute" => SystemRuntimeCompilerServicesType::DynamicAttribute,
    "CallSiteBinder" => SystemRuntimeCompilerServicesType::CallSiteBinder,
    "CallSite" => SystemRuntimeCompilerServicesType::CallSite,
    "CallSite`1" => SystemRuntimeCompilerServicesType::CallSite_1,
    "ICriticalNotifyCompletion" => SystemRuntimeCompilerServicesType::ICriticalNotifyCompletion,
    "IAsyncStateMachine" => SystemRuntimeCompilerServicesType::IAsyncStateMachine,
    "AsyncVoidMethodBuilder" => SystemRuntimeCompilerServicesType::AsyncVoidMethodBuilder,
    "AsyncTaskMethodBuilder" => SystemRuntimeCompilerServicesType::AsyncTaskMethodBuilder,
    "AsyncTaskMethodBuilder`1" => SystemRuntimeCompilerServicesType::AsyncTaskMethodBuilder_1,
    "AsyncStateMachineAttribute" => SystemRuntimeCompilerServicesType::AsyncStateMachineAttribute,
    "IteratorStateMachineAttribute" => SystemRuntimeCompilerServicesType::IteratorStateMachineAttribute,
    "TupleElementNamesAttribute" => SystemRuntimeCompilerServicesType::TupleElementNamesAttribute,
    "ReferenceAssemblyAttribute" => SystemRuntimeCompilerServicesType::ReferenceAssemblyAttribute,
    "TypeForwardedToAttribute" => SystemRuntimeCompilerServicesType::TypeForwardedToAttribute,
    "IsReadOnlyAttribute" => SystemRuntimeCompilerServicesType::IsReadOnlyAttribute,
    "IsByRefLikeAttribute" => SystemRuntimeCompilerServicesType::IsByRefLikeAttribute,
    "AsyncIteratorMethodBuilder" => SystemRuntimeCompilerServicesType::AsyncIteratorMethodBuilder,
    "AsyncIteratorStateMachineAttribute" => SystemRuntimeCompilerServicesType::AsyncIteratorStateMachineAttribute,
    "NullableAttribute" => SystemRuntimeCompilerServicesType::NullableAttribute,
    "IsUnmanagedAttribute" => SystemRuntimeCompilerServicesType::IsUnmanagedAttribute,

};

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SystemRuntimeCompilerServicesType {
    IsVolatile,
    FormattableStringFactory,
    RuntimeHelpers,
    DateTimeConstantAttribute,
    DecimalConstantAttribute,
    IUnknownConstantAttribute,
    IDispatchConstantAttribute,
    ExtensionAttribute,
    INotifyCompletion,
    InternalsVisibleToAttribute,
    CompilerGeneratedAttribute,
    AccessedThroughPropertyAttribute,
    CompilationRelaxationsAttribute,
    RuntimeCompatibilityAttribute,
    UnsafeValueTypeAttribute,
    FixedBufferAttribute,
    DynamicAttribute,
    CallSiteBinder,
    CallSite,
    CallSite_1,
    ICriticalNotifyCompletion,
    IAsyncStateMachine,
    AsyncVoidMethodBuilder,
    AsyncTaskMethodBuilder,
    AsyncTaskMethodBuilder_1,
    AsyncStateMachineAttribute,
    IteratorStateMachineAttribute,
    TupleElementNamesAttribute,
    ReferenceAssemblyAttribute,
    TypeForwardedToAttribute,
    IsReadOnlyAttribute,
    IsByRefLikeAttribute,
    AsyncIteratorMethodBuilder,
    AsyncIteratorStateMachineAttribute,
    NullableAttribute,
    IsUnmanagedAttribute,
}

impl SystemRuntimeCompilerServicesType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        SYSTEM_RUNTIME_COMPILER_SERVICE.get(name).copied()
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::IsVolatile => "IsVolatile",
            Self::FormattableStringFactory => "FormattableStringFactory",
            Self::RuntimeHelpers => "RuntimeHelpers",
            Self::DateTimeConstantAttribute => "DateTimeConstantAttribute",
            Self::DecimalConstantAttribute => "DecimalConstantAttribute",
            Self::IUnknownConstantAttribute => "IUnknownConstantAttribute",
            Self::IDispatchConstantAttribute => "IDispatchConstantAttribute",
            Self::ExtensionAttribute => "ExtensionAttribute",
            Self::INotifyCompletion => "INotifyCompletion",
            Self::InternalsVisibleToAttribute => "InternalsVisibleToAttribute",
            Self::CompilerGeneratedAttribute => "CompilerGeneratedAttribute",
            Self::AccessedThroughPropertyAttribute => "AccessedThroughPropertyAttribute",
            Self::CompilationRelaxationsAttribute => "CompilationRelaxationsAttribute",
            Self::RuntimeCompatibilityAttribute => "RuntimeCompatibilityAttribute",
            Self::UnsafeValueTypeAttribute => "UnsafeValueTypeAttribute",
            Self::FixedBufferAttribute => "FixedBufferAttribute",
            Self::DynamicAttribute => "DynamicAttribute",
            Self::CallSiteBinder => "CallSiteBinder",
            Self::CallSite => "CallSite",
            Self::CallSite_1 => "CallSite",
            Self::ICriticalNotifyCompletion => "ICriticalNotifyCompletion",
            Self::IAsyncStateMachine => "IAsyncStateMachine",
            Self::AsyncVoidMethodBuilder => "AsyncVoidMethodBuilder",
            Self::AsyncTaskMethodBuilder => "AsyncTaskMethodBuilder",
            Self::AsyncTaskMethodBuilder_1 => "AsyncTaskMethodBuilder",
            Self::AsyncStateMachineAttribute => "AsyncStateMachineAttribute",
            Self::IteratorStateMachineAttribute => "IteratorStateMachineAttribute",
            Self::TupleElementNamesAttribute => "TupleElementNamesAttribute",
            Self::ReferenceAssemblyAttribute => "ReferenceAssemblyAttribute",
            Self::TypeForwardedToAttribute => "TypeForwardedToAttribute",
            Self::IsReadOnlyAttribute => "IsReadOnlyAttribute",
            Self::IsByRefLikeAttribute => "IsByRefLikeAttribute",
            Self::AsyncIteratorMethodBuilder => "AsyncIteratorMethodBuilder",
            Self::AsyncIteratorStateMachineAttribute => "AsyncIteratorStateMachineAttribute",
            Self::NullableAttribute => "NullableAttribute",
            Self::IsUnmanagedAttribute => "IsUnmanagedAttribute",
        }
    }
}

static SYSTEM_COLLECTIONS_GENERIC: phf::Map<&'static str, SystemCollectionsGenericType> = phf_map! {
    "IEnumerable`1" => SystemCollectionsGenericType::IEnumerable_1,
    "IList`1" => SystemCollectionsGenericType::IList_1,
    "ICollection`1" => SystemCollectionsGenericType::ICollection_1,
    "IEnumerator`1" => SystemCollectionsGenericType::IEnumerator_1,
    "IReadOnlyList`1" => SystemCollectionsGenericType::IReadOnlyList_1,
    "IReadOnlyCollection`1" => SystemCollectionsGenericType::IReadOnlyCollection_1,
    "EqualityComparer`1" => SystemCollectionsGenericType::EqualityComparer_1,
    "List`1" => SystemCollectionsGenericType::List_1,
    "IDictionary`2" => SystemCollectionsGenericType::IDictionary_2,
    "Dictionary`2" => SystemCollectionsGenericType::Dictionary_2,
    "IReadOnlyDictionary`2" => SystemCollectionsGenericType::IReadOnlyDictionary_2,
    "KeyValuePair`2" => SystemCollectionsGenericType::KeyValuePair_2,
    "IAsyncEnumerable`1" => SystemCollectionsGenericType::IAsyncEnumerable_1,
    "IAsyncEnumerator`1" => SystemCollectionsGenericType::IAsyncEnumerator_1,
};

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SystemCollectionsGenericType {
    IEnumerable_1,
    IList_1,
    ICollection_1,
    IEnumerator_1,
    IReadOnlyList_1,
    IReadOnlyCollection_1,
    EqualityComparer_1,
    List_1,
    IDictionary_2,
    Dictionary_2,
    IReadOnlyDictionary_2,
    KeyValuePair_2,
    IAsyncEnumerable_1,
    IAsyncEnumerator_1,
}

impl SystemCollectionsGenericType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        SYSTEM_COLLECTIONS_GENERIC.get(name).copied()
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::IEnumerable_1 => "IEnumerable",
            Self::IList_1 => "IList",
            Self::ICollection_1 => "ICollection",
            Self::IEnumerator_1 => "IEnumerator",
            Self::IReadOnlyList_1 => "IReadOnlyList",
            Self::IReadOnlyCollection_1 => "IReadOnlyCollection",
            Self::EqualityComparer_1 => "EqualityComparer",
            Self::List_1 => "List",
            Self::IDictionary_2 => "IDictionary",
            Self::Dictionary_2 => "Dictionary",
            Self::IReadOnlyDictionary_2 => "IReadOnlyDictionary",
            Self::KeyValuePair_2 => "KeyValuePair",
            Self::IAsyncEnumerable_1 => "IAsyncEnumerable",
            Self::IAsyncEnumerator_1 => "IAsyncEnumerator",
        }
    }
}

static SYSTEM_COLLECTIONS: phf::Map<&'static str, SystemCollectionsType> = phf_map! {
    "DictionaryEntry" => SystemCollectionsType::DictionaryEntry,
    "IEnumerator" => SystemCollectionsType::IEnumerator,
    "IEnumerable" => SystemCollectionsType::IEnumerable,
    "IList" => SystemCollectionsType::IList,
    "ICollection" => SystemCollectionsType::ICollection,
};


#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SystemCollectionsType {
    DictionaryEntry,
    IEnumerator,
    IEnumerable,
    IList,
    ICollection,
}

impl SystemCollectionsType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        SYSTEM_COLLECTIONS.get(name).copied()
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::DictionaryEntry => "DictionaryEntry",
            Self::IEnumerator => "IEnumerator",
            Self::IEnumerable => "IEnumerable",
            Self::IList => "IList",
            Self::ICollection => "ICollection",
        }
    }
}

static SYSTEM_COLLECTIONS_OBJECTMODEL: phf::Map<&'static str, SystemCollectionsObjectModelType> = phf_map! {
    "Collection`1" => SystemCollectionsObjectModelType::Collection_1,
"ReadOnlyCollection`1" => SystemCollectionsObjectModelType::ReadOnlyCollection_1,
};

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SystemCollectionsObjectModelType {
    Collection_1,
    ReadOnlyCollection_1,
}

impl SystemCollectionsObjectModelType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        SYSTEM_COLLECTIONS_OBJECTMODEL.get(name).copied()
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Collection_1 => "Collection",
            Self::ReadOnlyCollection_1 => "ReadOnlyCollection",
        }
    }
}

static SYSTEM_COLLECTIONS_SPECIALIZED: phf::Map<&'static str, SystemCollectionsSpecializedType> = phf_map! {
    "INotifyCollectionChanged" => SystemCollectionsSpecializedType::INotifyCollectionChanged,
};

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SystemCollectionsSpecializedType {
    INotifyCollectionChanged,
}

impl SystemCollectionsSpecializedType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        SYSTEM_COLLECTIONS_SPECIALIZED.get(name).copied()
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::INotifyCollectionChanged => "INotifyCollectionChanged",
        }
    }
}

static SYSTEM_REFLECTION: phf::Map<&'static str, SystemReflectionType> = phf_map! {
    "MethodInfo" => SystemReflectionType::MethodInfo,
"ConstructorInfo" => SystemReflectionType::ConstructorInfo,
"MethodBase" => SystemReflectionType::MethodBase,
"FieldInfo" => SystemReflectionType::FieldInfo,
"MemberInfo" => SystemReflectionType::MemberInfo,
"Missing" => SystemReflectionType::Missing,
"AssemblyKeyFileAttribute" => SystemReflectionType::AssemblyKeyFileAttribute,
"AssemblyKeyNameAttribute" => SystemReflectionType::AssemblyKeyNameAttribute,
"DefaultMemberAttribute" => SystemReflectionType::DefaultMemberAttribute,
"Assembly" => SystemReflectionType::Assembly,
};

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SystemReflectionType {
    MethodInfo,
    ConstructorInfo,
    MethodBase,
    FieldInfo,
    MemberInfo,
    Missing,
    AssemblyKeyFileAttribute,
    AssemblyKeyNameAttribute,
    DefaultMemberAttribute,
    Assembly,
}

impl SystemReflectionType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        SYSTEM_REFLECTION.get(name).copied()
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::MethodInfo => "MethodInfo",
            Self::ConstructorInfo => "ConstructorInfo",
            Self::MethodBase => "MethodBase",
            Self::FieldInfo => "FieldInfo",
            Self::MemberInfo => "MemberInfo",
            Self::Missing => "Missing",
            Self::AssemblyKeyFileAttribute => "AssemblyKeyFileAttribute",
            Self::AssemblyKeyNameAttribute => "AssemblyKeyNameAttribute",
            Self::DefaultMemberAttribute => "DefaultMemberAttribute",
            Self::Assembly => "Assembly",
        }
    }
}

static SYSTEM_DIAGNOSTICS: phf::Map<&'static str, SystemDiagnosticsType> = phf_map! {
    "DebuggerTypeProxyAttribute" => SystemDiagnosticsType::DebuggerTypeProxyAttribute,
"Debugger" => SystemDiagnosticsType::Debugger,
"DebuggerDisplayAttribute" => SystemDiagnosticsType::DebuggerDisplayAttribute,
"DebuggerNonUserCodeAttribute" => SystemDiagnosticsType::DebuggerNonUserCodeAttribute,
"DebuggerHiddenAttribute" => SystemDiagnosticsType::DebuggerHiddenAttribute,
"DebuggerBrowsableAttribute" => SystemDiagnosticsType::DebuggerBrowsableAttribute,
"DebuggerStepThroughAttribute" => SystemDiagnosticsType::DebuggerStepThroughAttribute,
"DebuggerBrowsableState" => SystemDiagnosticsType::DebuggerBrowsableState,
"DebuggableAttribute" => SystemDiagnosticsType::DebuggableAttribute,
};

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SystemDiagnosticsType {
    DebuggerTypeProxyAttribute,
    Debugger,
    DebuggerDisplayAttribute,
    DebuggerNonUserCodeAttribute,
    DebuggerHiddenAttribute,
    DebuggerBrowsableAttribute,
    DebuggerStepThroughAttribute,
    DebuggerBrowsableState,
    DebuggableAttribute,
}

impl SystemDiagnosticsType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        SYSTEM_DIAGNOSTICS.get(name).copied()
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::DebuggerTypeProxyAttribute => "DebuggerTypeProxyAttribute",
            Self::Debugger => "Debugger",
            Self::DebuggerDisplayAttribute => "DebuggerDisplayAttribute",
            Self::DebuggerNonUserCodeAttribute => "DebuggerNonUserCodeAttribute",
            Self::DebuggerHiddenAttribute => "DebuggerHiddenAttribute",
            Self::DebuggerBrowsableAttribute => "DebuggerBrowsableAttribute",
            Self::DebuggerStepThroughAttribute => "DebuggerStepThroughAttribute",
            Self::DebuggerBrowsableState => "DebuggerBrowsableState",
            Self::DebuggableAttribute => "DebuggableAttribute",
        }
    }
}

static SYSTEM_RUNTIME_EXCEPTIONSERVICES: phf::Map<
    &'static str,
    SystemRuntimeExceptionServicesType,
> = phf_map! {
    "ExceptionDispatchInfo" => SystemRuntimeExceptionServicesType::ExceptionDispatchInfo,
};

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SystemRuntimeExceptionServicesType {
    ExceptionDispatchInfo,
}

impl SystemRuntimeExceptionServicesType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        SYSTEM_RUNTIME_EXCEPTIONSERVICES.get(name).copied()
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::ExceptionDispatchInfo => "ExceptionDispatchInfo",
        }
    }
}

static SYSTEM_RUNTIME_INTEROPSERVICES: phf::Map<&'static str, SystemRuntimeInteropServicesType> = phf_map! {
    "StructLayoutAttribute" => SystemRuntimeInteropServicesType::StructLayoutAttribute,
    "UnknownWrapper" => SystemRuntimeInteropServicesType::UnknownWrapper,
    "DispatchWrapper" => SystemRuntimeInteropServicesType::DispatchWrapper,
    "CallingConvention" => SystemRuntimeInteropServicesType::CallingConvention,
    "ClassInterfaceAttribute" => SystemRuntimeInteropServicesType::ClassInterfaceAttribute,
    "ClassInterfaceType" => SystemRuntimeInteropServicesType::ClassInterfaceType,
    "CoClassAttribute" => SystemRuntimeInteropServicesType::CoClassAttribute,
    "ComAwareEventInfo" => SystemRuntimeInteropServicesType::ComAwareEventInfo,
    "ComEventInterfaceAttribute" => SystemRuntimeInteropServicesType::ComEventInterfaceAttribute,
    "ComInterfaceType" => SystemRuntimeInteropServicesType::ComInterfaceType,
    "ComSourceInterfacesAttribute" => SystemRuntimeInteropServicesType::ComSourceInterfacesAttribute,
    "ComVisibleAttribute" => SystemRuntimeInteropServicesType::ComVisibleAttribute,
    "DispIdAttribute" => SystemRuntimeInteropServicesType::DispIdAttribute,
    "GuidAttribute" => SystemRuntimeInteropServicesType::GuidAttribute,
    "InterfaceTypeAttribute" => SystemRuntimeInteropServicesType::InterfaceTypeAttribute,
    "Marshal" => SystemRuntimeInteropServicesType::Marshal,
    "TypeIdentifierAttribute" => SystemRuntimeInteropServicesType::TypeIdentifierAttribute,
    "BestFitMappingAttribute" => SystemRuntimeInteropServicesType::BestFitMappingAttribute,
    "DefaultParameterValueAttribute" => SystemRuntimeInteropServicesType::DefaultParameterValueAttribute,
    "LCIDConversionAttribute" => SystemRuntimeInteropServicesType::LCIDConversionAttribute,
    "UnmanagedFunctionPointerAttribute" => SystemRuntimeInteropServicesType::UnmanagedFunctionPointerAttribute,
    "ComImportAttribute" => SystemRuntimeInteropServicesType::ComImportAttribute,
    "DllImportAttribute" => SystemRuntimeInteropServicesType::DllImportAttribute,
    "FieldOffsetAttribute" => SystemRuntimeInteropServicesType::FieldOffsetAttribute,
    "InAttribute" => SystemRuntimeInteropServicesType::InAttribute,
    "MarshalAsAttribute" => SystemRuntimeInteropServicesType::MarshalAsAttribute,
    "OptionalAttribute" => SystemRuntimeInteropServicesType::OptionalAttribute,
    "OutAttribute" => SystemRuntimeInteropServicesType::OutAttribute,
    "PreserveSigAttribute" => SystemRuntimeInteropServicesType::PreserveSigAttribute,
    "CharSet" => SystemRuntimeInteropServicesType::CharSet,
    "UnmanagedType" => SystemRuntimeInteropServicesType::UnmanagedType,
    "VarEnum" => SystemRuntimeInteropServicesType::VarEnum,
    "GCHandle" => SystemRuntimeInteropServicesType::GCHandle,
};

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SystemRuntimeInteropServicesType {
    StructLayoutAttribute,
    UnknownWrapper,
    DispatchWrapper,
    CallingConvention,
    ClassInterfaceAttribute,
    ClassInterfaceType,
    CoClassAttribute,
    ComAwareEventInfo,
    ComEventInterfaceAttribute,
    ComInterfaceType,
    ComSourceInterfacesAttribute,
    ComVisibleAttribute,
    DispIdAttribute,
    GuidAttribute,
    InterfaceTypeAttribute,
    Marshal,
    TypeIdentifierAttribute,
    BestFitMappingAttribute,
    DefaultParameterValueAttribute,
    LCIDConversionAttribute,
    UnmanagedFunctionPointerAttribute,
    ComImportAttribute,
    DllImportAttribute,
    FieldOffsetAttribute,
    InAttribute,
    MarshalAsAttribute,
    OptionalAttribute,
    OutAttribute,
    PreserveSigAttribute,
    CharSet,
    UnmanagedType,
    VarEnum,
    GCHandle,
}

impl SystemRuntimeInteropServicesType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        SYSTEM_RUNTIME_INTEROPSERVICES.get(name).copied()
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::StructLayoutAttribute => "StructLayoutAttribute",
            Self::UnknownWrapper => "UnknownWrapper",
            Self::DispatchWrapper => "DispatchWrapper",
            Self::CallingConvention => "CallingConvention",
            Self::ClassInterfaceAttribute => "ClassInterfaceAttribute",
            Self::ClassInterfaceType => "ClassInterfaceType",
            Self::CoClassAttribute => "CoClassAttribute",
            Self::ComAwareEventInfo => "ComAwareEventInfo",
            Self::ComEventInterfaceAttribute => "ComEventInterfaceAttribute",
            Self::ComInterfaceType => "ComInterfaceType",
            Self::ComSourceInterfacesAttribute => "ComSourceInterfacesAttribute",
            Self::ComVisibleAttribute => "ComVisibleAttribute",
            Self::DispIdAttribute => "DispIdAttribute",
            Self::GuidAttribute => "GuidAttribute",
            Self::InterfaceTypeAttribute => "InterfaceTypeAttribute",
            Self::Marshal => "Marshal",
            Self::TypeIdentifierAttribute => "TypeIdentifierAttribute",
            Self::BestFitMappingAttribute => "BestFitMappingAttribute",
            Self::DefaultParameterValueAttribute => "DefaultParameterValueAttribute",
            Self::LCIDConversionAttribute => "LCIDConversionAttribute",
            Self::UnmanagedFunctionPointerAttribute => "UnmanagedFunctionPointerAttribute",
            Self::ComImportAttribute => "ComImportAttribute",
            Self::DllImportAttribute => "DllImportAttribute",
            Self::FieldOffsetAttribute => "FieldOffsetAttribute",
            Self::InAttribute => "InAttribute",
            Self::MarshalAsAttribute => "MarshalAsAttribute",
            Self::OptionalAttribute => "OptionalAttribute",
            Self::OutAttribute => "OutAttribute",
            Self::PreserveSigAttribute => "PreserveSigAttribute",
            Self::CharSet => "CharSet",
            Self::UnmanagedType => "UnmanagedType",
            Self::VarEnum => "VarEnum",
            Self::GCHandle => "GCHandle",
        }
    }
}

static SYSTEM_THREADING_TASKS: phf::Map<&'static str, SystemThreadingTasksType> = phf_map! {
    "Task" => SystemThreadingTasksType::Task,
"Task`1" => SystemThreadingTasksType::Task_1,
"ValueTask`1" => SystemThreadingTasksType::ValueTask_1,
"ValueTask" => SystemThreadingTasksType::ValueTask,
};

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SystemThreadingTasksType {
    Task,
    Task_1,
    ValueTask_1,
    ValueTask,
}

impl SystemThreadingTasksType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        SYSTEM_THREADING_TASKS.get(name).copied()
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Task => "Task",
            Self::Task_1 => "Task",
            Self::ValueTask_1 => "ValueTask",
            Self::ValueTask => "ValueTask",
        }
    }
}

static SYSTEM_THREADING: phf::Map<&'static str, SystemThreadingType> = phf_map! {
    "Interlocked" => SystemThreadingType::Interlocked,
"Monitor" => SystemThreadingType::Monitor,
"Thread" => SystemThreadingType::Thread,
"CancellationToken" => SystemThreadingType::CancellationToken,
};

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SystemThreadingType {
    Interlocked,
    Monitor,
    Thread,
    CancellationToken,
}

impl SystemThreadingType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        SYSTEM_THREADING.get(name).copied()
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Interlocked => "Interlocked",
            Self::Monitor => "Monitor",
            Self::Thread => "Thread",
            Self::CancellationToken => "CancellationToken",
        }
    }
}

static MICROSOFT_CSHARP_RUNTIMEBINDER: phf::Map<&'static str, MicrosoftCSharpRuntimeBinderType> = phf_map! {
    "Binder" => MicrosoftCSharpRuntimeBinderType::Binder,
"CSharpArgumentInfo" => MicrosoftCSharpRuntimeBinderType::CSharpArgumentInfo,
"CSharpArgumentInfoFlags" => MicrosoftCSharpRuntimeBinderType::CSharpArgumentInfoFlags,
"CSharpBinderFlags" => MicrosoftCSharpRuntimeBinderType::CSharpBinderFlags,
};

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum MicrosoftCSharpRuntimeBinderType {
    Binder,
    CSharpArgumentInfo,
    CSharpArgumentInfoFlags,
    CSharpBinderFlags,
}

impl MicrosoftCSharpRuntimeBinderType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        MICROSOFT_CSHARP_RUNTIMEBINDER.get(name).copied()
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Binder => "Binder",
            Self::CSharpArgumentInfo => "CSharpArgumentInfo",
            Self::CSharpArgumentInfoFlags => "CSharpArgumentInfoFlags",
            Self::CSharpBinderFlags => "CSharpBinderFlags",
        }
    }
}

static MICROSOFT_VISUALBASIC: phf::Map<&'static str, MicrosoftVisualBasicType> = phf_map! {
    "CallType" => MicrosoftVisualBasicType::CallType,
"Embedded" => MicrosoftVisualBasicType::Embedded,
"CompareMethod" => MicrosoftVisualBasicType::CompareMethod,
"Strings" => MicrosoftVisualBasicType::Strings,
"ErrObject" => MicrosoftVisualBasicType::ErrObject,
"FileSystem" => MicrosoftVisualBasicType::FileSystem,
"Information" => MicrosoftVisualBasicType::Information,
"Interaction" => MicrosoftVisualBasicType::Interaction,
"Conversion" => MicrosoftVisualBasicType::Conversion,
};

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum MicrosoftVisualBasicType {
    CallType,
    Embedded,
    CompareMethod,
    Strings,
    ErrObject,
    FileSystem,
    Information,
    Interaction,
    Conversion,
}

impl MicrosoftVisualBasicType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        MICROSOFT_VISUALBASIC.get(name).copied()
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::CallType => "CallType",
            Self::Embedded => "Embedded",
            Self::CompareMethod => "CompareMethod",
            Self::Strings => "Strings",
            Self::ErrObject => "ErrObject",
            Self::FileSystem => "FileSystem",
            Self::Information => "Information",
            Self::Interaction => "Interaction",
            Self::Conversion => "Conversion",
        }
    }
}

static MICROSOFT_VISUALBASIC_COMPILERSERVICES: phf::Map<
    &'static str,
    MicrosoftVisualBasicCompilerServicesType,
> = phf_map! {
    "Conversions" => MicrosoftVisualBasicCompilerServicesType::Conversions,
"Operators" => MicrosoftVisualBasicCompilerServicesType::Operators,
"NewLateBinding" => MicrosoftVisualBasicCompilerServicesType::NewLateBinding,
"EmbeddedOperators" => MicrosoftVisualBasicCompilerServicesType::EmbeddedOperators,
"StandardModuleAttribute" => MicrosoftVisualBasicCompilerServicesType::StandardModuleAttribute,
"Utils" => MicrosoftVisualBasicCompilerServicesType::Utils,
"LikeOperator" => MicrosoftVisualBasicCompilerServicesType::LikeOperator,
"ProjectData" => MicrosoftVisualBasicCompilerServicesType::ProjectData,
"ObjectFlowControl" => MicrosoftVisualBasicCompilerServicesType::ObjectFlowControl,
"StaticLocalInitFlag" => MicrosoftVisualBasicCompilerServicesType::StaticLocalInitFlag,
"StringType" => MicrosoftVisualBasicCompilerServicesType::StringType,
"IncompleteInitialization" => MicrosoftVisualBasicCompilerServicesType::IncompleteInitialization,
"Versioned" => MicrosoftVisualBasicCompilerServicesType::Versioned,
};

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum MicrosoftVisualBasicCompilerServicesType {
    Conversions,
    Operators,
    NewLateBinding,
    EmbeddedOperators,
    StandardModuleAttribute,
    Utils,
    LikeOperator,
    ProjectData,
    ObjectFlowControl,
    StaticLocalInitFlag,
    StringType,
    IncompleteInitialization,
    Versioned,
}

impl MicrosoftVisualBasicCompilerServicesType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        MICROSOFT_VISUALBASIC_COMPILERSERVICES.get(name).copied()
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Conversions => "Conversions",
            Self::Operators => "Operators",
            Self::NewLateBinding => "NewLateBinding",
            Self::EmbeddedOperators => "EmbeddedOperators",
            Self::StandardModuleAttribute => "StandardModuleAttribute",
            Self::Utils => "Utils",
            Self::LikeOperator => "LikeOperator",
            Self::ProjectData => "ProjectData",
            Self::ObjectFlowControl => "ObjectFlowControl",
            Self::StaticLocalInitFlag => "StaticLocalInitFlag",
            Self::StringType => "StringType",
            Self::IncompleteInitialization => "IncompleteInitialization",
            Self::Versioned => "Versioned",
        }
    }
}

static MICROSOFT_VISUALBASIC_APPLICATIONSERVICES: phf::Map<
    &'static str,
    MicrosoftVisualBasicApplicationServicesType,
> = phf_map! {
    "ApplicationBase" => MicrosoftVisualBasicApplicationServicesType::ApplicationBase,
"WindowsFormsApplicationBase" => MicrosoftVisualBasicApplicationServicesType::WindowsFormsApplicationBase,
};

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum MicrosoftVisualBasicApplicationServicesType {
    ApplicationBase,
    WindowsFormsApplicationBase,
}

impl MicrosoftVisualBasicApplicationServicesType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        MICROSOFT_VISUALBASIC_APPLICATIONSERVICES.get(name).copied()
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::ApplicationBase => "ApplicationBase",
            Self::WindowsFormsApplicationBase => "WindowsFormsApplicationBase",
        }
    }
}

static SYSTEM_RUNTIME_INTEROPSERVICES_WINDOWSRUNTIME: phf::Map<
    &'static str,
    SystemRuntimeInteropServicesWindowsRuntimeType,
> = phf_map! {
    "EventRegistrationToken" => SystemRuntimeInteropServicesWindowsRuntimeType::EventRegistrationToken,
"EventRegistrationTokenTable`1" => SystemRuntimeInteropServicesWindowsRuntimeType::EventRegistrationTokenTable_1,
"WindowsRuntimeMarshal" => SystemRuntimeInteropServicesWindowsRuntimeType::WindowsRuntimeMarshal,
"RuntimeClass" => SystemRuntimeInteropServicesWindowsRuntimeType::RuntimeClass,
};

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SystemRuntimeInteropServicesWindowsRuntimeType {
    EventRegistrationToken,
    EventRegistrationTokenTable_1,
    WindowsRuntimeMarshal,
    RuntimeClass,
}

impl SystemRuntimeInteropServicesWindowsRuntimeType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        SYSTEM_RUNTIME_INTEROPSERVICES_WINDOWSRUNTIME
            .get(name)
            .copied()
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::EventRegistrationToken => "EventRegistrationToken",
            Self::EventRegistrationTokenTable_1 => "EventRegistrationTokenTable",
            Self::WindowsRuntimeMarshal => "WindowsRuntimeMarshal",
            Self::RuntimeClass => "RuntimeClass",
        }
    }
}

static WINDOWS_FOUNDATION: phf::Map<&'static str, WindowsFoundationType> = phf_map! {
    "IAsyncAction" => WindowsFoundationType::IAsyncAction,
"IAsyncActionWithProgress`1" => WindowsFoundationType::IAsyncActionWithProgress_1,
"IAsyncOperation`1" => WindowsFoundationType::IAsyncOperation_1,
"IAsyncOperationWithProgress`2" => WindowsFoundationType::IAsyncOperationWithProgress_2,
};

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum WindowsFoundationType {
    IAsyncAction,
    IAsyncActionWithProgress_1,
    IAsyncOperation_1,
    IAsyncOperationWithProgress_2,
}

impl WindowsFoundationType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        WINDOWS_FOUNDATION.get(name).copied()
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::IAsyncAction => "IAsyncAction",
            Self::IAsyncActionWithProgress_1 => "IAsyncActionWithProgress",
            Self::IAsyncOperation_1 => "IAsyncOperation",
            Self::IAsyncOperationWithProgress_2 => "IAsyncOperationWithProgress",
        }
    }
}

static SYSTEM_COMPONENTMODEL: phf::Map<&'static str, SystemComponentModelType> = phf_map! {
    "DesignerSerializationVisibilityAttribute" => SystemComponentModelType::DesignerSerializationVisibilityAttribute,
"INotifyPropertyChanged" => SystemComponentModelType::INotifyPropertyChanged,
"EditorBrowsableAttribute" => SystemComponentModelType::EditorBrowsableAttribute,
"EditorBrowsableState" => SystemComponentModelType::EditorBrowsableState,
};

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SystemComponentModelType {
    DesignerSerializationVisibilityAttribute,
    INotifyPropertyChanged,
    EditorBrowsableAttribute,
    EditorBrowsableState,
}

impl SystemComponentModelType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        SYSTEM_COMPONENTMODEL.get(name).copied()
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::DesignerSerializationVisibilityAttribute => {
                "DesignerSerializationVisibilityAttribute"
            }
            Self::INotifyPropertyChanged => "INotifyPropertyChanged",
            Self::EditorBrowsableAttribute => "EditorBrowsableAttribute",
            Self::EditorBrowsableState => "EditorBrowsableState",
        }
    }
}

static SYSTEM_LINQ: phf::Map<&'static str, SystemLinqType> = phf_map! {
    "Enumerable" => SystemLinqType::Enumerable,
"IQueryable" => SystemLinqType::IQueryable,
"IQueryable`1" => SystemLinqType::IQueryable_1,
"SystemCore_EnumerableDebugView" => SystemLinqType::SystemCore_EnumerableDebugView,
"SystemCore_EnumerableDebugView`1" => SystemLinqType::SystemCore_EnumerableDebugView_1,
};

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SystemLinqType {
    Enumerable,
    IQueryable,
    IQueryable_1,
    SystemCore_EnumerableDebugView,
    SystemCore_EnumerableDebugView_1,
}

impl SystemLinqType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        SYSTEM_LINQ.get(name).copied()
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Enumerable => "Enumerable",
            Self::IQueryable => "IQueryable",
            Self::IQueryable_1 => "IQueryable",
            Self::SystemCore_EnumerableDebugView => "SystemCore_EnumerableDebugView",
            Self::SystemCore_EnumerableDebugView_1 => "SystemCore_EnumerableDebugView",
        }
    }
}

static SYSTEM_LINQ_EXPRESSIONS: phf::Map<&'static str, SystemLinqExpressionsType> = phf_map! {
    "Expression" => SystemLinqExpressionsType::Expression,
"Expression`1" => SystemLinqExpressionsType::Expression_1,
"ParameterExpression" => SystemLinqExpressionsType::ParameterExpression,
"ElementInit" => SystemLinqExpressionsType::ElementInit,
"MemberBinding" => SystemLinqExpressionsType::MemberBinding,
"ExpressionType" => SystemLinqExpressionsType::ExpressionType,
};

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SystemLinqExpressionsType {
    Expression,
    Expression_1,
    ParameterExpression,
    ElementInit,
    MemberBinding,
    ExpressionType,
}

impl SystemLinqExpressionsType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        SYSTEM_LINQ_EXPRESSIONS.get(name).copied()
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Expression => "Expression",
            Self::Expression_1 => "Expression",
            Self::ParameterExpression => "ParameterExpression",
            Self::ElementInit => "ElementInit",
            Self::MemberBinding => "MemberBinding",
            Self::ExpressionType => "ExpressionType",
        }
    }
}

static SYSTEM_XML_LINQ: phf::Map<&'static str, SystemXmlLinqType> = phf_map! {
    "Extensions" => SystemXmlLinqType::Extensions,
"XAttribute" => SystemXmlLinqType::XAttribute,
"XCData" => SystemXmlLinqType::XCData,
"XComment" => SystemXmlLinqType::XComment,
"XContainer" => SystemXmlLinqType::XContainer,
"XDeclaration" => SystemXmlLinqType::XDeclaration,
"XDocument" => SystemXmlLinqType::XDocument,
"XElement" => SystemXmlLinqType::XElement,
"XName" => SystemXmlLinqType::XName,
"XNamespace" => SystemXmlLinqType::XNamespace,
"XObject" => SystemXmlLinqType::XObject,
"XProcessingInstruction" => SystemXmlLinqType::XProcessingInstruction,
};

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SystemXmlLinqType {
    Extensions,
    XAttribute,
    XCData,
    XComment,
    XContainer,
    XDeclaration,
    XDocument,
    XElement,
    XName,
    XNamespace,
    XObject,
    XProcessingInstruction,
}

impl SystemXmlLinqType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        SYSTEM_XML_LINQ.get(name).copied()
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Extensions => "Extensions",
            Self::XAttribute => "XAttribute",
            Self::XCData => "XCData",
            Self::XComment => "XComment",
            Self::XContainer => "XContainer",
            Self::XDeclaration => "XDeclaration",
            Self::XDocument => "XDocument",
            Self::XElement => "XElement",
            Self::XName => "XName",
            Self::XNamespace => "XNamespace",
            Self::XObject => "XObject",
            Self::XProcessingInstruction => "XProcessingInstruction",
        }
    }
}

static SYSTEM_SECURITY: phf::Map<&'static str, SystemSecurityType> = phf_map! {
    "UnverifiableCodeAttribute" => SystemSecurityType::UnverifiableCodeAttribute,
};

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SystemSecurityType {
    UnverifiableCodeAttribute,
}

impl SystemSecurityType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        SYSTEM_SECURITY.get(name).copied()
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::UnverifiableCodeAttribute => "UnverifiableCodeAttribute",
        }
    }
}

static SYSTEM_SECURITY_PERMISSIONS: phf::Map<&'static str, SystemSecurityPermissionsType> = phf_map! {
    "SecurityAction" => SystemSecurityPermissionsType::SecurityAction,
"SecurityAttribute" => SystemSecurityPermissionsType::SecurityAttribute,
"SecurityPermissionAttribute" => SystemSecurityPermissionsType::SecurityPermissionAttribute,
"PermissionSetAttribute" => SystemSecurityPermissionsType::PermissionSetAttribute,
};

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SystemSecurityPermissionsType {
    SecurityAction,
    SecurityAttribute,
    SecurityPermissionAttribute,
    PermissionSetAttribute,
}

impl SystemSecurityPermissionsType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        SYSTEM_SECURITY_PERMISSIONS.get(name).copied()
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::SecurityAction => "SecurityAction",
            Self::SecurityAttribute => "SecurityAttribute",
            Self::SecurityPermissionAttribute => "SecurityPermissionAttribute",
            Self::PermissionSetAttribute => "PermissionSetAttribute",
        }
    }
}

static SYSTEM_WINDOWS_FORMS: phf::Map<&'static str, SystemWindowsFormsType> = phf_map! {
    "Form" => SystemWindowsFormsType::Form,
"Application" => SystemWindowsFormsType::Application,
};

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SystemWindowsFormsType {
    Form,
    Application,
}

impl SystemWindowsFormsType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        SYSTEM_WINDOWS_FORMS.get(name).copied()
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Form => "Form",
            Self::Application => "Application",
        }
    }
}

static SYSTEM_RUNTIME: phf::Map<&'static str, SystemRuntimeType> = phf_map! {
    "GCLatencyMode" => SystemRuntimeType::GCLatencyMode,
};

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SystemRuntimeType {
    GCLatencyMode,
}

impl SystemRuntimeType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        SYSTEM_RUNTIME.get(name).copied()
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::GCLatencyMode => "GCLatencyMode",
        }
    }
}

static SYSTEM_TEXT: phf::Map<&'static str, SystemTextType> = phf_map! {
    "Encoding" => SystemTextType::Encoding,
};

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SystemTextType {
    Encoding,
}

impl SystemTextType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        SYSTEM_TEXT.get(name).copied()
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Encoding => "Encoding",
        }
    }
}

static SYSTEM_THREADING_TASKS_SOURCES: phf::Map<&'static str, SystemThreadingTasksSourcesType> = phf_map! {
    "ManualResetValueTaskSourceCore`1" => SystemThreadingTasksSourcesType::ManualResetValueTaskSourceCore_1,
"ValueTaskSourceStatus" => SystemThreadingTasksSourcesType::ValueTaskSourceStatus,
"ValueTaskSourceOnCompletedFlags" => SystemThreadingTasksSourcesType::ValueTaskSourceOnCompletedFlags,
"IValueTaskSource`1" => SystemThreadingTasksSourcesType::IValueTaskSource_1,
"IValueTaskSource" => SystemThreadingTasksSourcesType::IValueTaskSource,
};

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SystemThreadingTasksSourcesType {
    ManualResetValueTaskSourceCore_1,
    ValueTaskSourceStatus,
    ValueTaskSourceOnCompletedFlags,
    IValueTaskSource_1,
    IValueTaskSource,
}

impl SystemThreadingTasksSourcesType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        SYSTEM_THREADING_TASKS_SOURCES.get(name).copied()
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::ManualResetValueTaskSourceCore_1 => "ManualResetValueTaskSourceCore",
            Self::ValueTaskSourceStatus => "ValueTaskSourceStatus",
            Self::ValueTaskSourceOnCompletedFlags => "ValueTaskSourceOnCompletedFlags",
            Self::IValueTaskSource_1 => "IValueTaskSource",
            Self::IValueTaskSource => "IValueTaskSource",
        }
    }
}
