use std::borrow::Cow;
use strum::{EnumString, IntoStaticStr};

#[macro_use]
mod macros;

mod method;
pub use method::*;

well_known_namespaces! {
    "System" => System(SystemType),
    "System.Runtime.CompilerServices" => SystemRuntimeCompilerServices(SystemRuntimeCompilerServicesType),
    "System.Collections.Generic" => SystemCollectionsGeneric(SystemCollectionsGenericType),
    "System.Collections" => SystemCollections(SystemCollectionsType),
    "System.Collections.ObjectModel" => SystemCollectionsObjectModel(SystemCollectionsObjectModelType),
    "System.Collections.Specialized" => SystemCollectionsSpecialized(SystemCollectionsSpecializedType),
    "System.Reflection" => SystemReflection(SystemReflectionType),
    "System.Diagnostics" => SystemDiagnostics(SystemDiagnosticsType),
    "System.Runtime.ExceptionServices" => SystemRuntimeExceptionServices(SystemRuntimeExceptionServicesType),
    "System.Runtime.InteropServices" => SystemRuntimeInteropServices(SystemRuntimeInteropServicesType),
    "System.Threading.Tasks" => SystemThreadingTasks(SystemThreadingTasksType),
    "System.Threading" => SystemThreading(SystemThreadingType),
    "Microsoft.CSharp.RuntimeBinder" => MicrosoftCSharpRuntimeBinder(MicrosoftCSharpRuntimeBinderType),
    "Microsoft.VisualBasic" => MicrosoftVisualBasic(MicrosoftVisualBasicType),
    "Microsoft.VisualBasic.CompilerServices" => MicrosoftVisualBasicCompilerServices(MicrosoftVisualBasicCompilerServicesType),
    "Microsoft.VisualBasic.ApplicationServices" => MicrosoftVisualBasicApplicationServices(MicrosoftVisualBasicApplicationServicesType),
    "System.Runtime.InteropServices.WindowsRuntime" => SystemRuntimeInteropServicesWindowsRuntime(SystemRuntimeInteropServicesWindowsRuntimeType),
    "Windows.Foundation" => WindowsFoundation(WindowsFoundationType),
    "System.ComponentModel" => SystemComponentModel(SystemComponentModelType),
    "System.Linq" => SystemLinq(SystemLinqType),
    "System.Linq.Expressions" => SystemLinqExpressions(SystemLinqExpressionsType),
    "System.Xml.Linq" => SystemXmlLinq(SystemXmlLinqType),
    "System.Security" => SystemSecurity(SystemSecurityType),
    "System.Security.Permissions" => SystemSecurityPermissions(SystemSecurityPermissionsType),
    "System.Windows.Forms" => SystemWindowsForms(SystemWindowsFormsType),
    "System.Runtime" => SystemRuntime(SystemRuntimeType),
    "System.Text" => SystemText(SystemTextType),
    "System.Threading.Tasks.Sources" => SystemThreadingTasksSources(SystemThreadingTasksSourcesType),
    "System.IO" => SystemIo(SystemIoType),
    "System.Globalization" => SystemGlobalization(SystemGlobalizationType),
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
    AppDomain,
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

    // Common additions (absent from Roslyn's well-known list).
    Console,
    GC,
    GCCollectionMode,
    TimeSpan,
    DateTimeOffset,
    Random,
    Version,
    Uri,
    BitConverter,
    Buffer,
    MathF,
    HashCode,
    EventArgs,
    EventHandler,
    #[strum(serialize = "EventHandler`1")]
    EventHandler_1,
    IComparable,
    #[strum(serialize = "IComparable`1")]
    IComparable_1,
    ICloneable,
    WeakReference,
    #[strum(serialize = "WeakReference`1")]
    WeakReference_1,
    #[strum(serialize = "Lazy`1")]
    Lazy_1,
    #[strum(serialize = "Progress`1")]
    Progress_1,
    #[strum(serialize = "IProgress`1")]
    IProgress_1,
    #[strum(serialize = "Comparison`1")]
    Comparison_1,
    #[strum(serialize = "Predicate`1")]
    Predicate_1,
    #[strum(serialize = "Converter`2")]
    Converter_2,
    StringComparison,
    StringSplitOptions,
    AttributeTargets,
    DateTimeKind,
    MidpointRounding,
    ArgumentException,
    ArgumentNullException,
    ArgumentOutOfRangeException,
    InvalidOperationException,
    NullReferenceException,
    IndexOutOfRangeException,
    NotImplementedException,
    FormatException,
    OverflowException,
    InvalidCastException,
    SystemException,
    AggregateException,
    ObjectDisposedException,
    TimeoutException,
    DivideByZeroException,
    ArithmeticException,
}


#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemRuntimeCompilerServicesType {
    MethodImplAttribute,
    MethodImplOptions,
    Unsafe,
    CallerMemberNameAttribute,
    CallerLineNumberAttribute,
    CallerFilePathAttribute,
    CallerArgumentExpressionAttribute,
    ModuleInitializerAttribute,
    SkipLocalsInitAttribute,
    #[strum(serialize = "StrongBox`1")]
    StrongBox_1,
    #[strum(serialize = "ConditionalWeakTable`2")]
    ConditionalWeakTable_2,
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
    CallConvCdecl,
    CallConvStdcall,
    CallConvThiscall,
    CallConvFastcall,
    CallConvSuppressGCTransition,
    CallConvMemberFunction,
}


#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemCollectionsGenericType {
    #[strum(serialize = "HashSet`1")]
    HashSet_1,
    #[strum(serialize = "Queue`1")]
    Queue_1,
    #[strum(serialize = "Stack`1")]
    Stack_1,
    #[strum(serialize = "LinkedList`1")]
    LinkedList_1,
    #[strum(serialize = "SortedDictionary`2")]
    SortedDictionary_2,
    #[strum(serialize = "SortedList`2")]
    SortedList_2,
    #[strum(serialize = "SortedSet`1")]
    SortedSet_1,
    #[strum(serialize = "ISet`1")]
    ISet_1,
    #[strum(serialize = "IReadOnlySet`1")]
    IReadOnlySet_1,
    #[strum(serialize = "IComparer`1")]
    IComparer_1,
    #[strum(serialize = "IEqualityComparer`1")]
    IEqualityComparer_1,
    #[strum(serialize = "Comparer`1")]
    Comparer_1,
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


#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemCollectionsType {
    Hashtable,
    ArrayList,
    IDictionary,
    IComparer,
    IEqualityComparer,
    Queue,
    Stack,
    BitArray,
    DictionaryEntry,
    IEnumerator,
    IEnumerable,
    IList,
    ICollection,
}


#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemCollectionsObjectModelType {
    #[strum(serialize = "Collection`1")]
    Collection_1,
    #[strum(serialize = "ReadOnlyCollection`1")]
    ReadOnlyCollection_1,
}


#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemCollectionsSpecializedType {
    INotifyCollectionChanged,
}


#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemReflectionType {
    PropertyInfo,
    EventInfo,
    ParameterInfo,
    Module,
    AssemblyName,
    TypeInfo,
    CustomAttributeData,
    Pointer,
    BindingFlags,
    MethodAttributes,
    FieldAttributes,
    TypeAttributes,
    PropertyAttributes,
    ParameterAttributes,
    MethodImplAttributes,
    CallingConventions,
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


#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemDiagnosticsType {
    Stopwatch,
    Debug,
    Trace,
    ConditionalAttribute,
    StackTrace,
    StackFrame,
    Process,
    ProcessStartInfo,
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


#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemRuntimeExceptionServicesType {
    ExceptionDispatchInfo,
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
    GCHandleType,
    UnmanagedCallersOnlyAttribute,
    LayoutKind,
    SafeHandle,
    HandleRef,
    NativeLibrary,
    MemoryMarshal,
    CollectionsMarshal,
    NativeMemory,
    RuntimeInformation,
    OSPlatform,
    Architecture,
    ComWrappers,
}


#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemThreadingTasksType {
    TaskCompletionSource,
    #[strum(serialize = "TaskCompletionSource`1")]
    TaskCompletionSource_1,
    TaskScheduler,
    Parallel,
    TaskFactory,
    TaskStatus,
    TaskCreationOptions,
    TaskContinuationOptions,
    Task,
    #[strum(serialize = "Task`1")]
    Task_1,
    #[strum(serialize = "ValueTask`1")]
    ValueTask_1,
    ValueTask,
}


#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemThreadingType {
    Interlocked,
    Monitor,
    Thread,
    CancellationToken,
    CancellationTokenSource,
    SemaphoreSlim,
    ManualResetEventSlim,
    ManualResetEvent,
    AutoResetEvent,
    Mutex,
    Volatile,
    WaitHandle,
    ThreadPool,
    SynchronizationContext,
    ThreadStart,
    ParameterizedThreadStart,
    ReaderWriterLockSlim,
    EventWaitHandle,
}


#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum MicrosoftCSharpRuntimeBinderType {
    Binder,
    CSharpArgumentInfo,
    CSharpArgumentInfoFlags,
    CSharpBinderFlags,
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


#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum MicrosoftVisualBasicApplicationServicesType {
    ApplicationBase,
    WindowsFormsApplicationBase,
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


#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemComponentModelType {
    DesignerSerializationVisibilityAttribute,
    INotifyPropertyChanged,
    EditorBrowsableAttribute,
    EditorBrowsableState,
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


#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemSecurityType {
    UnverifiableCodeAttribute,
}


#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemSecurityPermissionsType {
    SecurityAction,
    SecurityAttribute,
    SecurityPermissionAttribute,
    PermissionSetAttribute,
}


#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemWindowsFormsType {
    Form,
    Application,
}


#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemRuntimeType {
    GCLatencyMode,
}


#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemTextType {
    Encoding,
    StringBuilder,
    Rune,
    UTF8Encoding,
    UnicodeEncoding,
    ASCIIEncoding,
    NormalizationForm,
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


#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemIoType {
    Path,
    File,
    Directory,
    FileInfo,
    DirectoryInfo,
    Stream,
    MemoryStream,
    FileStream,
    StreamReader,
    StreamWriter,
    BinaryReader,
    BinaryWriter,
    TextReader,
    TextWriter,
    FileMode,
    FileAccess,
    FileShare,
    SeekOrigin,
    IOException,
    FileNotFoundException,
    DirectoryNotFoundException,
}


#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
pub enum SystemGlobalizationType {
    CultureInfo,
    NumberStyles,
    DateTimeStyles,
    NumberFormatInfo,
    DateTimeFormatInfo,
    CompareInfo,
    RegionInfo,
    TextInfo,
    Calendar,
}


/// A type named either by a full-name string (`"System.Text.StringBuilder"`) or
/// by a compile-checked [`WellKnown`] value or one of its namespace sub-enums
/// ([`SystemType`], [`SystemRuntimeInteropServicesType`], ...).
///
/// This is what the by-name importers on [`Module`](crate::Module) accept —
/// [`import_type`](crate::Module::import_type),
/// [`import_type_from`](crate::Module::import_type_from),
/// [`corlib_type`](crate::Module::corlib_type),
/// [`import_corlib_method`](crate::Module::import_corlib_method) and friends — so
/// a magic string and a checked `SystemType::String` are interchangeable.
///
/// ```
/// # use hao::dotnet::entries::well_known::{TypeName, SystemType};
/// assert_eq!(SystemType::String.type_name(), ("System".into(), "String".into()));
/// assert_eq!("System.Text.StringBuilder".type_name(), ("System.Text".into(), "StringBuilder".into()));
/// ```
pub trait TypeName {
    /// The `(namespace, type name)` pair, with any generic arity suffix stripped.
    fn type_name(&self) -> (Cow<'_, str>, Cow<'_, str>);
}

impl TypeName for &str {
    fn type_name(&self) -> (Cow<'_, str>, Cow<'_, str>) {
        let (namespace, name) = self.rsplit_once('.').unwrap_or(("", self));
        (Cow::Borrowed(namespace), Cow::Borrowed(name))
    }
}

impl TypeName for String {
    fn type_name(&self) -> (Cow<'_, str>, Cow<'_, str>) {
        let (namespace, name) = self.rsplit_once('.').unwrap_or(("", self.as_str()));
        (Cow::Borrowed(namespace), Cow::Borrowed(name))
    }
}

impl TypeName for WellKnown {
    fn type_name(&self) -> (Cow<'_, str>, Cow<'_, str>) {
        let (namespace, name) = self.full_name();
        (Cow::Borrowed(namespace), Cow::Borrowed(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn well_known_round_trips() {
        // Including the new namespaces and additions to existing sub-enums.
        for (namespace, name) in [
            ("System", "Console"),
            ("System", "ArgumentNullException"),
            ("System.Text", "StringBuilder"),
            ("System.Runtime.InteropServices", "UnmanagedCallersOnlyAttribute"),
            ("System.Reflection", "BindingFlags"),
            ("System.Diagnostics", "Stopwatch"),
            ("System.IO", "Path"),
            ("System.Globalization", "CultureInfo"),
            ("System.Collections.Generic", "HashSet`1"),
            ("System", "AppDomain"),
            (
                "System.Runtime.CompilerServices",
                "CallConvSuppressGCTransition",
            ),
            ("System.Runtime.CompilerServices", "CallConvCdecl"),
        ] {
            let wk = WellKnown::from_full_name(namespace, name)
                .unwrap_or_else(|| panic!("{namespace}.{name} should be well known"));
            let stripped = name.split('`').next().unwrap();
            assert_eq!(wk.full_name(), (namespace, stripped));
        }
    }

    #[test]
    fn type_name_impls() {
        assert_eq!(
            SystemType::String.type_name(),
            ("System".into(), "String".into())
        );
        assert_eq!(
            SystemIoType::Path.type_name(),
            ("System.IO".into(), "Path".into())
        );
        assert_eq!(
            SystemRuntimeInteropServicesType::GCHandle.type_name(),
            ("System.Runtime.InteropServices".into(), "GCHandle".into())
        );
        // Generic arity is stripped by `name()`.
        assert_eq!(
            SystemCollectionsGenericType::HashSet_1.type_name(),
            ("System.Collections.Generic".into(), "HashSet".into())
        );
        // A free-form string splits on the last dot.
        assert_eq!(
            "System.Text.StringBuilder".type_name(),
            ("System.Text".into(), "StringBuilder".into())
        );
    }
}
