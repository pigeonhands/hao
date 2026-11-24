//! Builds the `Hao.Loader` helper assembly at runtime.
//!
//! The assembly is a small .NET DLL with `[UnmanagedCallersOnly]` methods
//! that give Rust access to reflection, `Assembly.LoadFrom`, and method
//! resolution — including overload-safe lookup by parameter count.
//!
//! Each method body is emitted in its own function so the IL generation
//! is easier to follow than a single monolithic builder.

use hao::{
    dotnet::{
        entries::{
            well_known::{
                AppDomainMethod, ArrayListMethod, AssemblyMethod, AssemblyNameMethod, BindingFlags,
                ExceptionMethod, GcMethod, IEnumerableMethod, IEnumeratorMethod, MarshalMethod,
                MemberInfoMethod, MethodBaseMethod, RuntimeHelpersMethod, RuntimeMethodHandleMethod,
                StringExtMethod, StringMethod, SystemCollectionsType, SystemReflectionType,
                SystemType, TypeExtMethod, TypeMethod, WellKnownMethod,
            },
            TypeDefOrRef,
        },
        il::{ChainImports, Emitter, MemberToken},
        module::{ModuleKind, TargetRuntime},
        signature::{MethodSig, TypeSig},
    },
    Module,
};

use crate::error::{Error, Result};

const LOADER: &str = "Hao.Loader";

const BINDING_FLAGS: i32 = BindingFlags::INSTANCE
    .union(BindingFlags::STATIC)
    .union(BindingFlags::PUBLIC)
    .union(BindingFlags::NON_PUBLIC)
    .bits();

/// Shared tokens imported once and referenced by every body builder.
struct Tokens {
    to_string: MemberToken,
    exception: TypeDefOrRef,
    type_ty: TypeDefOrRef,
    method_info: TypeDefOrRef,
    handle_ty: TypeDefOrRef,
    assembly: TypeDefOrRef,
    assembly_name: TypeDefOrRef,
    app_domain: TypeDefOrRef,
    parameter_info: TypeDefOrRef,
    get_type: MemberToken,
    get_method: MemberToken,
    method_handle: MemberToken,
    prepare: MemberToken,
    function_pointer: MemberToken,
    get_current_domain: MemberToken,
    get_assemblies: MemberToken,
    get_asm_name: MemberToken,
    get_name_prop: MemberToken,
    asm_get_type: MemberToken,
    asm_get_types: MemberToken,
    string_equals: MemberToken,
    string_concat: MemberToken,
    get_methods: MemberToken,
    get_member_name: MemberToken,
    get_parameters: MemberToken,
    type_get_full_name: MemberToken,
    marshal_to_cotask: MemberToken,
    get_length: MemberToken,
    asm_load_from: MemberToken,
    hresult: MemberToken,
    gc_try_start: MemberToken,
    gc_end: MemberToken,
    // IEnumerable materialization (`Enumerate`).
    arraylist_type: TypeDefOrRef,
    ienumerator_type: TypeDefOrRef,
    arraylist_ctor: MemberToken,
    arraylist_add: MemberToken,
    arraylist_to_array: MemberToken,
    get_enumerator: MemberToken,
    move_next: MemberToken,
    get_current: MemberToken,
}

impl Tokens {
    /// Imports every type and method the Hao.Loader bodies reference.
    fn resolve(module: &mut Module) -> hao::error::Result<Self> {
        let arraylist_type = module.corlib_type(SystemCollectionsType::ArrayList)?;
        let ienumerator_type = module.corlib_type(SystemCollectionsType::IEnumerator)?;
        Ok(Self {
            exception: module.corlib_type(SystemType::Exception)?,
            type_ty: module.corlib_type(SystemType::Type)?,
            handle_ty: module.corlib_type(SystemType::RuntimeMethodHandle)?,
            method_info: module.corlib_type(SystemReflectionType::MethodInfo)?,
            assembly: module.corlib_type(SystemReflectionType::Assembly)?,
            assembly_name: module.corlib_type(SystemReflectionType::AssemblyName)?,
            app_domain: module.corlib_type(SystemType::AppDomain)?,
            parameter_info: module.corlib_type(SystemReflectionType::ParameterInfo)?,
            to_string: module
                .import_well_known_method(WellKnownMethod::Marshal(MarshalMethod::PtrToStringAuto))?,
            hresult: module
                .import_well_known_method(WellKnownMethod::Exception(ExceptionMethod::GetHResult))?,
            asm_load_from: module
                .import_well_known_method(WellKnownMethod::Assembly(AssemblyMethod::LoadFrom))?,
            get_type: module
                .import_well_known_method(WellKnownMethod::TypeExt(TypeExtMethod::GetTypeStatic))?,
            get_method: module
                .import_well_known_method(WellKnownMethod::TypeExt(TypeExtMethod::GetMethodBF))?,
            method_handle: module.import_well_known_method(WellKnownMethod::MethodBase(
                MethodBaseMethod::GetMethodHandle,
            ))?,
            prepare: module.import_well_known_method(WellKnownMethod::RuntimeHelpers(
                RuntimeHelpersMethod::PrepareMethod,
            ))?,
            function_pointer: module.import_well_known_method(
                WellKnownMethod::RuntimeMethodHandle(RuntimeMethodHandleMethod::GetFunctionPointer),
            )?,
            get_current_domain: module.import_well_known_method(WellKnownMethod::AppDomain(
                AppDomainMethod::GetCurrentDomain,
            ))?,
            get_assemblies: module
                .import_well_known_method(WellKnownMethod::AppDomain(AppDomainMethod::GetAssemblies))?,
            get_asm_name: module
                .import_well_known_method(WellKnownMethod::Assembly(AssemblyMethod::GetName))?,
            get_name_prop: module.import_well_known_method(WellKnownMethod::AssemblyName(
                AssemblyNameMethod::GetName,
            ))?,
            asm_get_type: module
                .import_well_known_method(WellKnownMethod::Assembly(AssemblyMethod::GetTypeStr))?,
            string_equals: module
                .import_well_known_method(WellKnownMethod::StringExt(StringExtMethod::OpEquality))?,
            get_methods: module
                .import_well_known_method(WellKnownMethod::TypeExt(TypeExtMethod::GetMethodsBF))?,
            get_member_name: module.import_well_known_method(WellKnownMethod::MemberInfo(
                MemberInfoMethod::GetName,
            ))?,
            get_parameters: module.import_well_known_method(WellKnownMethod::MethodBase(
                MethodBaseMethod::GetParameters,
            ))?,
            asm_get_types: module
                .import_well_known_method(WellKnownMethod::Assembly(AssemblyMethod::GetTypes))?,
            string_concat: module
                .import_well_known_method(WellKnownMethod::String(StringMethod::Concat2))?,
            type_get_full_name: module
                .import_well_known_method(WellKnownMethod::Type(TypeMethod::GetFullName))?,
            marshal_to_cotask: module.import_well_known_method(WellKnownMethod::Marshal(
                MarshalMethod::StringToCoTaskMemUni,
            ))?,
            get_length: module
                .import_well_known_method(WellKnownMethod::String(StringMethod::GetLength))?,
            gc_try_start: module
                .import_well_known_method(WellKnownMethod::Gc(GcMethod::TryStartNoGCRegion))?,
            gc_end: module.import_well_known_method(WellKnownMethod::Gc(GcMethod::EndNoGCRegion))?,
            arraylist_ctor: module
                .import_well_known_method(WellKnownMethod::ArrayList(ArrayListMethod::New))?,
            arraylist_add: module
                .import_well_known_method(WellKnownMethod::ArrayList(ArrayListMethod::Add))?,
            arraylist_to_array: module
                .import_well_known_method(WellKnownMethod::ArrayList(ArrayListMethod::ToArray))?,
            get_enumerator: module.import_well_known_method(WellKnownMethod::IEnumerable(
                IEnumerableMethod::GetEnumerator,
            ))?,
            move_next: module
                .import_well_known_method(WellKnownMethod::IEnumerator(IEnumeratorMethod::MoveNext))?,
            get_current: module.import_well_known_method(WellKnownMethod::IEnumerator(
                IEnumeratorMethod::GetCurrent,
            ))?,
            arraylist_type,
            ienumerator_type,
        })
    }
}

// ---------------------------------------------------------------------------
// Individual method bodies
// ---------------------------------------------------------------------------

/// `int LoadFrom(IntPtr path)`
///
/// ```csharp
/// try { Assembly.LoadFrom(Marshal.PtrToStringAuto(path)); return 0; }
/// catch (Exception e) { return e.HResult; }
/// ```
fn load_from_body(t: &Tokens) -> hao::dotnet::il::MethodBody {
    hao::managed_body! {
        exception: t.exception.clone(),
        methods {
            to_string: static &t.to_string,
            load_from: static &t.asm_load_from,
            hresult: virtual &t.hresult,
        },
        params(path: nint),
        result: i32 = 0,
        body {
            let path_str: string = to_string(path);
            _ = load_from(path_str);
        },
        catch |e, r| {
            hresult.emit_call(e); // the exception object is the receiver
            r.store(e);
        },
    }
}

/// `IntPtr GetMethod(IntPtr fullName, IntPtr name)`
///
/// Resolves via `Type.GetType(fullName).GetMethod(name, flags)`, then
/// `PrepareMethod` + `GetFunctionPointer`. Returns 0 on any exception.
fn get_method_body(t: &Tokens) -> hao::dotnet::il::MethodBody {
    hao::managed_body! {
        exception: t.exception.clone(),
        types {
            Type: class(t.type_ty.clone()),
            MethodInfo: class(t.method_info.clone()),
            MethodHandle: valuetype(t.handle_ty.clone()),
        },
        methods {
            to_string: static &t.to_string,
            get_type: static &t.get_type,
            get_method: virtual &t.get_method,
            method_handle: virtual &t.method_handle,
            prepare: static &t.prepare,
            function_pointer: instance &t.function_pointer,
        },
        params(full_name: nint, name: nint),
        result: nint = 0,
        body {
            let type_str: string = to_string(full_name);
            let found: Type = get_type(type_str);
            let name_str: string = to_string(name);
            let method: MethodInfo = found.get_method(name_str, const BINDING_FLAGS);
            let handle: MethodHandle = method.method_handle();
            prepare(handle);
            result = handle.function_pointer();
        },
    }
}

/// `IntPtr GetMethodFromLoaded(IntPtr typeName, IntPtr methodName, IntPtr asmName)`
///
/// Iterates `AppDomain.CurrentDomain.GetAssemblies()`, matches by assembly
/// name, then uses `Type.GetMethod(name, flags)`. Returns 0 on failure.
fn get_method_from_loaded_body(t: &Tokens) -> hao::dotnet::il::MethodBody {
    hao::managed_body! {
        exception: t.exception.clone(),
        types {
            AppDomain: class(t.app_domain.clone()),
            Assembly: class(t.assembly.clone()),
            Assemblies: array(Assembly),
            AssemblyName: class(t.assembly_name.clone()),
            Type: class(t.type_ty.clone()),
            MethodInfo: class(t.method_info.clone()),
            MethodHandle: valuetype(t.handle_ty.clone()),
        },
        methods {
            to_string: static &t.to_string,
            get_current_domain: static &t.get_current_domain,
            get_assemblies: virtual &t.get_assemblies,
            get_asm_name: virtual &t.get_asm_name,
            get_name_prop: virtual &t.get_name_prop,
            string_equals: static &t.string_equals,
            asm_get_type: virtual &t.asm_get_type,
            get_method: virtual &t.get_method,
            method_handle: virtual &t.method_handle,
            prepare: static &t.prepare,
            function_pointer: instance &t.function_pointer,
        },
        params(type_name_ptr: nint, method_name_ptr: nint, asm_name_ptr: nint),
        result: nint = 0,
        body {
            let type_name: string = to_string(type_name_ptr);
            let method_name: string = to_string(method_name_ptr);
            let asm_name: string = to_string(asm_name_ptr);
            let domain: AppDomain = get_current_domain();
            let assemblies: Assemblies = domain.get_assemblies();
            for current: Assembly in assemblies {
                let an: AssemblyName = current.get_asm_name();
                let current_name: string = an.get_name_prop();
                if string_equals(current_name, asm_name) {
                    let found: Type = current.asm_get_type(type_name);
                    if found {
                        let method: MethodInfo = found.get_method(method_name, const BINDING_FLAGS);
                        if method {
                            let handle: MethodHandle = method.method_handle();
                            prepare(handle);
                            result = handle.function_pointer();
                            return;
                        }
                    }
                }
            }
        },
    }
}

/// `IntPtr FindMethodFromLoaded(IntPtr typeName, IntPtr methodName, IntPtr asmName, int paramCount)`
///
/// Like `GetMethodFromLoaded` but uses `GetMethods()` + name/param-count
/// filter, so overloaded methods resolve correctly.
fn find_method_from_loaded_body(t: &Tokens) -> hao::dotnet::il::MethodBody {
    hao::managed_body! {
        exception: t.exception.clone(),
        types {
            AppDomain: class(t.app_domain.clone()),
            Assembly: class(t.assembly.clone()),
            Assemblies: array(Assembly),
            AssemblyName: class(t.assembly_name.clone()),
            Type: class(t.type_ty.clone()),
            MethodInfo: class(t.method_info.clone()),
            Methods: array(MethodInfo),
            ParameterInfo: class(t.parameter_info.clone()),
            Parameters: array(ParameterInfo),
            MethodHandle: valuetype(t.handle_ty.clone()),
        },
        methods {
            to_string: static &t.to_string,
            get_current_domain: static &t.get_current_domain,
            get_assemblies: virtual &t.get_assemblies,
            get_asm_name: virtual &t.get_asm_name,
            get_name_prop: virtual &t.get_name_prop,
            string_equals: static &t.string_equals,
            asm_get_type: virtual &t.asm_get_type,
            get_methods: virtual &t.get_methods,
            get_member_name: virtual &t.get_member_name,
            get_parameters: virtual &t.get_parameters,
            method_handle: virtual &t.method_handle,
            prepare: static &t.prepare,
            function_pointer: instance &t.function_pointer,
        },
        params(type_name_ptr: nint, method_name_ptr: nint, asm_name_ptr: nint, param_count: i32),
        result: nint = 0,
        body {
            let type_name: string = to_string(type_name_ptr);
            let method_name: string = to_string(method_name_ptr);
            let asm_name: string = to_string(asm_name_ptr);
            let domain: AppDomain = get_current_domain();
            let assemblies: Assemblies = domain.get_assemblies();
            for current: Assembly in assemblies {
                let an: AssemblyName = current.get_asm_name();
                let current_name: string = an.get_name_prop();
                if string_equals(current_name, asm_name) {
                    let found: Type = current.asm_get_type(type_name);
                    if found {
                        let methods: Methods = found.get_methods(const BINDING_FLAGS);
                        for candidate: MethodInfo in methods {
                            let candidate_name: string = candidate.get_member_name();
                            if string_equals(candidate_name, method_name) {
                                let parameters: Parameters = candidate.get_parameters();
                                let arity: i32 = len(parameters);
                                if arity == param_count {
                                    let handle: MethodHandle = candidate.method_handle();
                                    prepare(handle);
                                    result = handle.function_pointer();
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        },
    }
}

/// `IntPtr GetTypeNames(IntPtr asmName)`
///
/// Iterates `AppDomain.CurrentDomain.GetAssemblies()`, matches by assembly
/// name, then returns a CoTaskMem-allocated UTF-16 string of `\n`-separated
/// type full names.  Returns 0 on failure or if the assembly has no types.
fn get_type_names_body(t: &Tokens) -> hao::dotnet::il::MethodBody {
    hao::managed_body! {
        exception: t.exception.clone(),
        types {
            AppDomain: class(t.app_domain.clone()),
            Assembly: class(t.assembly.clone()),
            Assemblies: array(Assembly),
            AssemblyName: class(t.assembly_name.clone()),
            Type: class(t.type_ty.clone()),
            Types: array(Type),
        },
        methods {
            to_string: static &t.to_string,
            get_current_domain: static &t.get_current_domain,
            get_assemblies: virtual &t.get_assemblies,
            get_asm_name: virtual &t.get_asm_name,
            get_name_prop: virtual &t.get_name_prop,
            string_equals: static &t.string_equals,
            asm_get_types: virtual &t.asm_get_types,
            type_get_full_name: virtual &t.type_get_full_name,
            get_length: virtual &t.get_length,
            string_concat: static &t.string_concat,
            marshal_to_cotask: static &t.marshal_to_cotask,
        },
        params(asm_name_ptr: nint),
        result: nint = 0,
        body {
            let asm_name: string = to_string(asm_name_ptr);
            let acc: string = str "";
            let domain: AppDomain = get_current_domain();
            let assemblies: Assemblies = domain.get_assemblies();
            for current: Assembly in assemblies {
                let an: AssemblyName = current.get_asm_name();
                let current_name: string = an.get_name_prop();
                if string_equals(current_name, asm_name) {
                    // Found the assembly — collect its type full names.
                    let types: Types = current.asm_get_types();
                    for ty: Type in types {
                        let name: string = ty.type_get_full_name();
                        if name {
                            if acc.get_length() {
                                acc = string_concat(acc, str "\n");
                            }
                            acc = string_concat(acc, name);
                        }
                    }
                    if acc.get_length() {
                        result = marshal_to_cotask(acc);
                    }
                    return;
                }
            }
        },
    }
}

/// `IntPtr GetMethodNames(IntPtr typeName, IntPtr asmName)`
///
/// Finds the named type in the specified loaded assembly and returns a
/// CoTaskMem-allocated UTF-16 string of `\n`-separated method names.
/// Returns 0 on failure.
fn get_method_names_body(t: &Tokens) -> hao::dotnet::il::MethodBody {
    hao::managed_body! {
        exception: t.exception.clone(),
        types {
            AppDomain: class(t.app_domain.clone()),
            Assembly: class(t.assembly.clone()),
            Assemblies: array(Assembly),
            AssemblyName: class(t.assembly_name.clone()),
            Type: class(t.type_ty.clone()),
            MethodInfo: class(t.method_info.clone()),
            Methods: array(MethodInfo),
        },
        methods {
            to_string: static &t.to_string,
            get_current_domain: static &t.get_current_domain,
            get_assemblies: virtual &t.get_assemblies,
            get_asm_name: virtual &t.get_asm_name,
            get_name_prop: virtual &t.get_name_prop,
            string_equals: static &t.string_equals,
            asm_get_type: virtual &t.asm_get_type,
            get_methods: virtual &t.get_methods,
            get_member_name: virtual &t.get_member_name,
            get_length: virtual &t.get_length,
            string_concat: static &t.string_concat,
            marshal_to_cotask: static &t.marshal_to_cotask,
        },
        params(type_name_ptr: nint, asm_name_ptr: nint),
        result: nint = 0,
        body {
            let type_name: string = to_string(type_name_ptr);
            let asm_name: string = to_string(asm_name_ptr);
            let acc: string = str "";
            let domain: AppDomain = get_current_domain();
            let assemblies: Assemblies = domain.get_assemblies();
            for current: Assembly in assemblies {
                let an: AssemblyName = current.get_asm_name();
                let current_name: string = an.get_name_prop();
                if string_equals(current_name, asm_name) {
                    let found: Type = current.asm_get_type(type_name);
                    if found {
                        let methods: Methods = found.get_methods(const BINDING_FLAGS);
                        for candidate: MethodInfo in methods {
                            let name: string = candidate.get_member_name();
                            if acc.get_length() {
                                acc = string_concat(acc, str "\n");
                            }
                            acc = string_concat(acc, name);
                        }
                        if acc.get_length() {
                            result = marshal_to_cotask(acc);
                        }
                        return;
                    }
                }
            }
        },
    }
}

/// `long Invoke(IntPtr methodPtr, long a0, long a1, long a2, long a3, long a4, long a5)`
///
/// Calls the JIT'd method at `methodPtr` with up to 6 int64 arguments.
/// Being `[UnmanagedCallersOnly]`, the reverse P/Invoke stub registers the
/// calling thread with the CLR.  The `calli` uses `SuppressGCTransition` so
/// the thread stays in cooperative mode — the GC cannot compact while the
/// target method is running.
fn invoke_body(imports: &ChainImports) -> hao::dotnet::il::MethodBody {
    Emitter::build(|e| {
        for i in 1..=6u16 {
            e.ldarg(i);
        }
        e.ldarg(0);
        e.calli(MethodSig::unmanaged_suppress_gc(
            TypeSig::INT64,
            vec![TypeSig::INT64; 6],
            &imports.callconv_cdecl,
            &imports.callconv_suppress_gc,
        ));
        e.ret();
    })
}

/// `double InvokeFloat(IntPtr methodPtr, long a0, long a1, long a2, long a3, long a4, long a5)`
///
/// Same as `Invoke` but returns `double`.
fn invoke_float_body(imports: &ChainImports) -> hao::dotnet::il::MethodBody {
    Emitter::build(|e| {
        for i in 1..=6u16 {
            e.ldarg(i);
        }
        e.ldarg(0);
        e.calli(MethodSig::unmanaged_suppress_gc(
            TypeSig::FLOAT64,
            vec![TypeSig::INT64; 6],
            &imports.callconv_cdecl,
            &imports.callconv_suppress_gc,
        ));
        e.ret();
    })
}

/// `long SuppressGC(long budget)`
///
/// Calls `GC.TryStartNoGCRegion(budget)` inside a try/catch.  Returns 1 on
/// success, 0 on failure (not enough memory, or already in a no-GC region).
fn suppress_gc_body(t: &Tokens) -> hao::dotnet::il::MethodBody {
    hao::managed_body! {
        exception: t.exception.clone(),
        methods { try_start: static &t.gc_try_start },
        params(budget: i64),
        result: i64 = 0,
        body {
            // TryStartNoGCRegion returns bool; the result store converts to 1/0 i64.
            result = try_start(budget);
        },
    }
}

/// `void ResumeGC()`
///
/// Calls `GC.EndNoGCRegion()` inside a try/catch.  Silently ignores
/// `InvalidOperationException` when no region is active (the GC already
/// broke out because allocations exceeded the budget).
fn resume_gc_body(t: &Tokens) -> hao::dotnet::il::MethodBody {
    hao::managed_body! {
        exception: t.exception.clone(),
        methods { end_no_gc: static &t.gc_end },
        result: (),
        body {
            end_no_gc();
        },
    }
}

/// `object[] Enumerate(object enumerable)`
///
/// Materializes any `System.Collections.IEnumerable` (including compiler
/// generated lazy iterators and LINQ chains, whose concrete layout native code
/// cannot read) into a plain `object[]` the caller can walk:
///
/// ```csharp
/// try {
///     var list = new ArrayList();
///     foreach (var x in (IEnumerable)enumerable) list.Add(x);
///     return list.ToArray();
/// } catch (Exception) { return null; }
/// ```
///
/// A hand-built [`Emitter`] body rather than `managed_body!` because the
/// `foreach` is a loop (the macro's `for` is array-index only). This method is
/// deliberately NOT `[UnmanagedCallersOnly]` so it stays callable from a
/// cooperative-mode (game) thread via its JIT entry.
fn enumerate_body(t: &Tokens) -> hao::dotnet::il::MethodBody {
    use hao::dotnet::signature::TypeSig;
    Emitter::build(|e| {
        let list = e.declare_local(TypeSig::class(t.arraylist_type.clone()));
        let en = e.declare_local(TypeSig::class(t.ienumerator_type.clone()));
        let result = e.declare_local(TypeSig::sz_array(TypeSig::OBJECT));
        e.try_catch(
            t.exception.clone(),
            |e| {
                e.newobj(&t.arraylist_ctor).stloc(list);
                e.ldarg(0).callvirt(&t.get_enumerator).stloc(en);
                e.while_loop(
                    |e| {
                        e.ldloc(en).callvirt(&t.move_next);
                    },
                    |e| {
                        e.ldloc(list)
                            .ldloc(en)
                            .callvirt(&t.get_current)
                            .callvirt(&t.arraylist_add)
                            .pop(); // ArrayList.Add returns the insert index
                    },
                );
                e.ldloc(list).callvirt(&t.arraylist_to_array).stloc(result);
            },
            |e| {
                e.pop(); // discard the exception; result stays null
            },
        );
        e.ldloc(result).ret();
    })
}

// ---------------------------------------------------------------------------
// Assembly builder
// ---------------------------------------------------------------------------

/// Builds the `Hao.Loader` assembly with `[UnmanagedCallersOnly]` helpers:
///
/// - `int LoadFrom(IntPtr path)` — `Assembly.LoadFrom`, HResult on failure.
/// - `IntPtr GetMethod(IntPtr fullName, IntPtr name)` — resolve via
///   `Type.GetType`, JIT-compile, return native pointer (0 on failure).
/// - `IntPtr GetMethodFromLoaded(IntPtr typeName, IntPtr methodName,
///   IntPtr asmName)` — like `GetMethod` but iterates loaded assemblies.
/// - `IntPtr FindMethodFromLoaded(IntPtr typeName, IntPtr methodName,
///   IntPtr asmName, int paramCount)` — overload-safe: matches by name
///   and parameter count via `GetMethods()`.
/// - `IntPtr GetTypeNames(IntPtr asmName)` — returns CoTaskMem UTF-16
///   string of `\n`-separated type full names from a loaded assembly.
/// - `IntPtr GetMethodNames(IntPtr typeName, IntPtr asmName)` — returns
///   CoTaskMem UTF-16 string of `\n`-separated method names.
pub fn loader_assembly(major: u16) -> Result<Vec<u8>> {
    let build = || -> hao::error::Result<Vec<u8>> {
        let mut module = Module::new(
            LOADER,
            ModuleKind::Dll,
            TargetRuntime::NetCoreImplementation(major),
        );

        let tokens = Tokens::resolve(&mut module)?;
        let imports = ChainImports::resolve(&mut module)?;

        // ── Assemble the Hao.Loader class ─────────────────────────────────
        hao::dotnet_class! { &mut module =>
            static class "Hao".Loader {
                #[unmanaged_callers_only]
                fn LoadFrom(path: nint) -> i32 =
                    load_from_body(&tokens);
                #[unmanaged_callers_only]
                fn GetMethod(full_name: nint, name: nint) -> nint =
                    get_method_body(&tokens);
                #[unmanaged_callers_only]
                fn GetMethodFromLoaded(type_name: nint, method_name: nint, asm_name: nint) -> nint =
                    get_method_from_loaded_body(&tokens);
                #[unmanaged_callers_only]
                fn FindMethodFromLoaded(type_name: nint, method_name: nint, asm_name: nint, param_count: i32) -> nint =
                    find_method_from_loaded_body(&tokens);
                #[unmanaged_callers_only]
                fn Invoke(method_ptr: nint, a0: i64, a1: i64, a2: i64, a3: i64, a4: i64, a5: i64) -> i64 =
                    invoke_body(&imports);
                #[unmanaged_callers_only]
                fn InvokeFloat(method_ptr: nint, a0: i64, a1: i64, a2: i64, a3: i64, a4: i64, a5: i64) -> f64 =
                    invoke_float_body(&imports);
                #[unmanaged_callers_only]
                fn GetTypeNames(asm_name: nint) -> nint =
                    get_type_names_body(&tokens);
                #[unmanaged_callers_only]
                fn GetMethodNames(type_name: nint, asm_name: nint) -> nint =
                    get_method_names_body(&tokens);
                #[unmanaged_callers_only]
                fn SuppressGC(budget: i64) -> i64 =
                    suppress_gc_body(&tokens);
                #[unmanaged_callers_only]
                fn ResumeGC() -> () =
                    resume_gc_body(&tokens);
                // NOT unmanaged_callers_only: called from a cooperative-mode
                // thread via its JIT entry, so it must not carry the UCO prolog.
                fn Enumerate(enumerable: object) -> [object] =
                    enumerate_body(&tokens);
            }
        }?;
        module.write()
    };
    build().map_err(|e| Error::Other(format!("could not build the loader assembly: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::loader_assembly;

    /// The loader (including the hand-emitted `Enumerate` IL body) builds to a
    /// valid PE image for every supported target runtime. Catches malformed IL
    /// or bad member tokens at test time, without a live CLR.
    #[test]
    fn loader_assembly_builds() {
        for major in [8u16, 9, 10] {
            let bytes = loader_assembly(major)
                .unwrap_or_else(|e| panic!(".NET {major}: loader build failed: {e}"));
            assert!(bytes.len() > 512, ".NET {major}: suspiciously small image");
            assert_eq!(&bytes[..2], b"MZ", ".NET {major}: not a PE image");
        }
    }
}
