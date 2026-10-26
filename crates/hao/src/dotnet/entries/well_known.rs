use strum::{EnumString, IntoStaticStr};

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

fn strip_arity(name: &'static str) -> &'static str {
    name.split('`').next().unwrap_or(name)
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
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
    #[strum(serialize = "Nullable`1")]
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
    #[strum(serialize = "Func`1")]
    Func_1,
    #[strum(serialize = "Func`2")]
    Func_2,
    #[strum(serialize = "Func`3")]
    Func_3,
    #[strum(serialize = "Func`4")]
    Func_4,
    #[strum(serialize = "Func`5")]
    Func_5,
    #[strum(serialize = "Func`6")]
    Func_6,
    #[strum(serialize = "Func`7")]
    Func_7,
    #[strum(serialize = "Func`8")]
    Func_8,
    #[strum(serialize = "Func`9")]
    Func_9,
    #[strum(serialize = "Func`10")]
    Func_10,
    #[strum(serialize = "Func`11")]
    Func_11,
    #[strum(serialize = "Func`12")]
    Func_12,
    #[strum(serialize = "Func`13")]
    Func_13,
    #[strum(serialize = "Func`14")]
    Func_14,
    #[strum(serialize = "Func`15")]
    Func_15,
    #[strum(serialize = "Func`16")]
    Func_16,
    #[strum(serialize = "Func`17")]
    Func_17,
    Action,
    #[strum(serialize = "Action`1")]
    Action_1,
    #[strum(serialize = "Action`2")]
    Action_2,
    #[strum(serialize = "Action`3")]
    Action_3,
    #[strum(serialize = "Action`4")]
    Action_4,
    #[strum(serialize = "Action`5")]
    Action_5,
    #[strum(serialize = "Action`6")]
    Action_6,
    #[strum(serialize = "Action`7")]
    Action_7,
    #[strum(serialize = "Action`8")]
    Action_8,
    #[strum(serialize = "Action`9")]
    Action_9,
    #[strum(serialize = "Action`10")]
    Action_10,
    #[strum(serialize = "Action`11")]
    Action_11,
    #[strum(serialize = "Action`12")]
    Action_12,
    #[strum(serialize = "Action`13")]
    Action_13,
    #[strum(serialize = "Action`14")]
    Action_14,
    #[strum(serialize = "Action`15")]
    Action_15,
    #[strum(serialize = "Action`16")]
    Action_16,
    Activator,
    AttributeUsageAttribute,
    ParamArrayAttribute,
    NonSerializedAttribute,
    STAThreadAttribute,
    #[strum(serialize = "IEquatable`1")]
    IEquatable_1,
    NotSupportedException,
    Environment,
    IFormatProvider,
    #[strum(serialize = "ValueTuple`1")]
    ValueTuple_1,
    #[strum(serialize = "ValueTuple`2")]
    ValueTuple_2,
    #[strum(serialize = "ValueTuple`3")]
    ValueTuple_3,
    #[strum(serialize = "ValueTuple`4")]
    ValueTuple_4,
    #[strum(serialize = "ValueTuple`5")]
    ValueTuple_5,
    #[strum(serialize = "ValueTuple`6")]
    ValueTuple_6,
    #[strum(serialize = "ValueTuple`7")]
    ValueTuple_7,
    #[strum(serialize = "ValueTuple`8")]
    ValueTuple_8,
    ContextBoundObject,
    SerializableAttribute,
    RuntimeMethodHandleInternal,
    #[strum(serialize = "ByReference`1")]
    ByReference_1,
    __ComObject,
    DBNull,
    ObsoleteAttribute,
    #[strum(serialize = "Span`1")]
    Span_1,
    #[strum(serialize = "ReadOnlySpan`1")]
    ReadOnlySpan_1,
    Index,
    Range,
    IAsyncDisposable,
}

impl SystemType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }

    pub fn from_full_name(namespace: &str, name: &str) -> Option<Self> {
        if namespace != "System" {
            return None;
        }
        Self::from_type_name(name)
    }

    /// The type name without the generic arity suffix.
    pub fn name(&self) -> &'static str {
        strip_arity(self.into())
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
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
    #[strum(serialize = "CallSite`1")]
    CallSite_1,
    ICriticalNotifyCompletion,
    IAsyncStateMachine,
    AsyncVoidMethodBuilder,
    AsyncTaskMethodBuilder,
    #[strum(serialize = "AsyncTaskMethodBuilder`1")]
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
        name.parse().ok()
    }

    /// The type name without the generic arity suffix.
    pub fn name(&self) -> &'static str {
        strip_arity(self.into())
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemCollectionsGenericType {
    #[strum(serialize = "IEnumerable`1")]
    IEnumerable_1,
    #[strum(serialize = "IList`1")]
    IList_1,
    #[strum(serialize = "ICollection`1")]
    ICollection_1,
    #[strum(serialize = "IEnumerator`1")]
    IEnumerator_1,
    #[strum(serialize = "IReadOnlyList`1")]
    IReadOnlyList_1,
    #[strum(serialize = "IReadOnlyCollection`1")]
    IReadOnlyCollection_1,
    #[strum(serialize = "EqualityComparer`1")]
    EqualityComparer_1,
    #[strum(serialize = "List`1")]
    List_1,
    #[strum(serialize = "IDictionary`2")]
    IDictionary_2,
    #[strum(serialize = "Dictionary`2")]
    Dictionary_2,
    #[strum(serialize = "IReadOnlyDictionary`2")]
    IReadOnlyDictionary_2,
    #[strum(serialize = "KeyValuePair`2")]
    KeyValuePair_2,
    #[strum(serialize = "IAsyncEnumerable`1")]
    IAsyncEnumerable_1,
    #[strum(serialize = "IAsyncEnumerator`1")]
    IAsyncEnumerator_1,
}

impl SystemCollectionsGenericType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }

    /// The type name without the generic arity suffix.
    pub fn name(&self) -> &'static str {
        strip_arity(self.into())
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemCollectionsType {
    DictionaryEntry,
    IEnumerator,
    IEnumerable,
    IList,
    ICollection,
}

impl SystemCollectionsType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }

    /// The type name without the generic arity suffix.
    pub fn name(&self) -> &'static str {
        strip_arity(self.into())
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemCollectionsObjectModelType {
    #[strum(serialize = "Collection`1")]
    Collection_1,
    #[strum(serialize = "ReadOnlyCollection`1")]
    ReadOnlyCollection_1,
}

impl SystemCollectionsObjectModelType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }

    /// The type name without the generic arity suffix.
    pub fn name(&self) -> &'static str {
        strip_arity(self.into())
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemCollectionsSpecializedType {
    INotifyCollectionChanged,
}

impl SystemCollectionsSpecializedType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }

    /// The type name without the generic arity suffix.
    pub fn name(&self) -> &'static str {
        strip_arity(self.into())
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
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
        name.parse().ok()
    }

    /// The type name without the generic arity suffix.
    pub fn name(&self) -> &'static str {
        strip_arity(self.into())
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
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
        name.parse().ok()
    }

    /// The type name without the generic arity suffix.
    pub fn name(&self) -> &'static str {
        strip_arity(self.into())
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemRuntimeExceptionServicesType {
    ExceptionDispatchInfo,
}

impl SystemRuntimeExceptionServicesType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }

    /// The type name without the generic arity suffix.
    pub fn name(&self) -> &'static str {
        strip_arity(self.into())
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
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
        name.parse().ok()
    }

    /// The type name without the generic arity suffix.
    pub fn name(&self) -> &'static str {
        strip_arity(self.into())
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemThreadingTasksType {
    Task,
    #[strum(serialize = "Task`1")]
    Task_1,
    #[strum(serialize = "ValueTask`1")]
    ValueTask_1,
    ValueTask,
}

impl SystemThreadingTasksType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }

    /// The type name without the generic arity suffix.
    pub fn name(&self) -> &'static str {
        strip_arity(self.into())
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemThreadingType {
    Interlocked,
    Monitor,
    Thread,
    CancellationToken,
}

impl SystemThreadingType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }

    /// The type name without the generic arity suffix.
    pub fn name(&self) -> &'static str {
        strip_arity(self.into())
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum MicrosoftCSharpRuntimeBinderType {
    Binder,
    CSharpArgumentInfo,
    CSharpArgumentInfoFlags,
    CSharpBinderFlags,
}

impl MicrosoftCSharpRuntimeBinderType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }

    /// The type name without the generic arity suffix.
    pub fn name(&self) -> &'static str {
        strip_arity(self.into())
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
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
        name.parse().ok()
    }

    /// The type name without the generic arity suffix.
    pub fn name(&self) -> &'static str {
        strip_arity(self.into())
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
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
        name.parse().ok()
    }

    /// The type name without the generic arity suffix.
    pub fn name(&self) -> &'static str {
        strip_arity(self.into())
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum MicrosoftVisualBasicApplicationServicesType {
    ApplicationBase,
    WindowsFormsApplicationBase,
}

impl MicrosoftVisualBasicApplicationServicesType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }

    /// The type name without the generic arity suffix.
    pub fn name(&self) -> &'static str {
        strip_arity(self.into())
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemRuntimeInteropServicesWindowsRuntimeType {
    EventRegistrationToken,
    #[strum(serialize = "EventRegistrationTokenTable`1")]
    EventRegistrationTokenTable_1,
    WindowsRuntimeMarshal,
    RuntimeClass,
}

impl SystemRuntimeInteropServicesWindowsRuntimeType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }

    /// The type name without the generic arity suffix.
    pub fn name(&self) -> &'static str {
        strip_arity(self.into())
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum WindowsFoundationType {
    IAsyncAction,
    #[strum(serialize = "IAsyncActionWithProgress`1")]
    IAsyncActionWithProgress_1,
    #[strum(serialize = "IAsyncOperation`1")]
    IAsyncOperation_1,
    #[strum(serialize = "IAsyncOperationWithProgress`2")]
    IAsyncOperationWithProgress_2,
}

impl WindowsFoundationType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }

    /// The type name without the generic arity suffix.
    pub fn name(&self) -> &'static str {
        strip_arity(self.into())
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemComponentModelType {
    DesignerSerializationVisibilityAttribute,
    INotifyPropertyChanged,
    EditorBrowsableAttribute,
    EditorBrowsableState,
}

impl SystemComponentModelType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }

    /// The type name without the generic arity suffix.
    pub fn name(&self) -> &'static str {
        strip_arity(self.into())
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemLinqType {
    Enumerable,
    IQueryable,
    #[strum(serialize = "IQueryable`1")]
    IQueryable_1,
    SystemCore_EnumerableDebugView,
    #[strum(serialize = "SystemCore_EnumerableDebugView`1")]
    SystemCore_EnumerableDebugView_1,
}

impl SystemLinqType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }

    /// The type name without the generic arity suffix.
    pub fn name(&self) -> &'static str {
        strip_arity(self.into())
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemLinqExpressionsType {
    Expression,
    #[strum(serialize = "Expression`1")]
    Expression_1,
    ParameterExpression,
    ElementInit,
    MemberBinding,
    ExpressionType,
}

impl SystemLinqExpressionsType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }

    /// The type name without the generic arity suffix.
    pub fn name(&self) -> &'static str {
        strip_arity(self.into())
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
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
        name.parse().ok()
    }

    /// The type name without the generic arity suffix.
    pub fn name(&self) -> &'static str {
        strip_arity(self.into())
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemSecurityType {
    UnverifiableCodeAttribute,
}

impl SystemSecurityType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }

    /// The type name without the generic arity suffix.
    pub fn name(&self) -> &'static str {
        strip_arity(self.into())
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemSecurityPermissionsType {
    SecurityAction,
    SecurityAttribute,
    SecurityPermissionAttribute,
    PermissionSetAttribute,
}

impl SystemSecurityPermissionsType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }

    /// The type name without the generic arity suffix.
    pub fn name(&self) -> &'static str {
        strip_arity(self.into())
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemWindowsFormsType {
    Form,
    Application,
}

impl SystemWindowsFormsType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }

    /// The type name without the generic arity suffix.
    pub fn name(&self) -> &'static str {
        strip_arity(self.into())
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemRuntimeType {
    GCLatencyMode,
}

impl SystemRuntimeType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }

    /// The type name without the generic arity suffix.
    pub fn name(&self) -> &'static str {
        strip_arity(self.into())
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemTextType {
    Encoding,
}

impl SystemTextType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }

    /// The type name without the generic arity suffix.
    pub fn name(&self) -> &'static str {
        strip_arity(self.into())
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemThreadingTasksSourcesType {
    #[strum(serialize = "ManualResetValueTaskSourceCore`1")]
    ManualResetValueTaskSourceCore_1,
    ValueTaskSourceStatus,
    ValueTaskSourceOnCompletedFlags,
    #[strum(serialize = "IValueTaskSource`1")]
    IValueTaskSource_1,
    IValueTaskSource,
}

impl SystemThreadingTasksSourcesType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }

    /// The type name without the generic arity suffix.
    pub fn name(&self) -> &'static str {
        strip_arity(self.into())
    }
}
