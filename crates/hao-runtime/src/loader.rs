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
                AppDomainMethod, AssemblyMethod, AssemblyNameMethod, BindingFlags,
                ExceptionMethod, GcMethod, MarshalMethod, MemberInfoMethod, MethodBaseMethod,
                RuntimeHelpersMethod, RuntimeMethodHandleMethod, StringExtMethod,
                StringMethod, SystemReflectionType, SystemType, TypeExtMethod, TypeMethod,
                WellKnownMethod,
            },
            TypeDefOrRef,
        },
        il::{ChainImports, Emitter, MemberToken, OpCode},
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
}

impl Tokens {
    /// Imports every type and method the Hao.Loader bodies reference.
    fn resolve(module: &mut Module) -> hao::error::Result<Self> {
        Ok(Self {
            exception: module.corlib_type(SystemType::Exception)?,
            type_ty: module.corlib_type(SystemType::Type)?,
            handle_ty: module.corlib_type(SystemType::RuntimeMethodHandle)?,
            method_info: module.corlib_type(SystemReflectionType::MethodInfo)?,
            assembly: module.corlib_type(SystemReflectionType::Assembly)?,
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
    Emitter::build(|e| {
        let type_name = e.declare_local(TypeSig::STRING);
        let method_name = e.declare_local(TypeSig::STRING);
        let asm_name = e.declare_local(TypeSig::STRING);
        let assemblies = e.declare_local(TypeSig::sz_array(TypeSig::class(t.assembly.clone())));
        let idx = e.declare_local(TypeSig::INT32);
        let current_asm = e.declare_local(TypeSig::class(t.assembly.clone()));
        let found_type = e.declare_local(TypeSig::class(t.type_ty.clone()));
        let method = e.declare_local(TypeSig::class(t.method_info.clone()));
        let handle = e.declare_local(TypeSig::value_type(t.handle_ty.clone()));
        let result = e.declare_local(TypeSig::NATIVE_INT);

        e.ldarg(0).call(&t.to_string).stloc(type_name);
        e.ldarg(1).call(&t.to_string).stloc(method_name);
        e.ldarg(2).call(&t.to_string).stloc(asm_name);

        e.ldc_i4(0).op(OpCode::Conv_I).stloc(result);

        let loop_check = e.label();
        let loop_body = e.label();
        let done = e.label();

        e.try_catch(
            t.exception.clone(),
            |e| {
                e.call(&t.get_current_domain)
                    .callvirt(&t.get_assemblies)
                    .stloc(assemblies);
                e.ldc_i4(0).stloc(idx);
                e.branch(OpCode::Br, loop_check);

                e.mark(loop_body);
                e.ldloc(assemblies)
                    .ldloc(idx)
                    .op(OpCode::Ldelem_Ref)
                    .stloc(current_asm);
                e.ldloc(current_asm)
                    .callvirt(&t.get_asm_name)
                    .callvirt(&t.get_name_prop);
                e.ldloc(asm_name).call(&t.string_equals);
                let skip = e.label();
                e.branch(OpCode::Brfalse, skip);

                e.ldloc(current_asm)
                    .ldloc(type_name)
                    .callvirt(&t.asm_get_type)
                    .stloc(found_type);
                e.ldloc(found_type).branch(OpCode::Brfalse, skip);

                e.ldloc(found_type)
                    .ldloc(method_name)
                    .ldc_i4(BINDING_FLAGS)
                    .callvirt(&t.get_method)
                    .stloc(method);
                e.ldloc(method).branch(OpCode::Brfalse, skip);

                e.ldloc(method).callvirt(&t.method_handle).stloc(handle);
                e.ldloc(handle).call(&t.prepare);
                e.ldloca(handle)
                    .call(&t.function_pointer)
                    .stloc(result);
                e.branch(OpCode::Leave, done);

                e.mark(skip);
                e.ldloc(idx).ldc_i4(1).op(OpCode::Add).stloc(idx);

                e.mark(loop_check);
                e.ldloc(idx)
                    .ldloc(assemblies)
                    .op(OpCode::Ldlen)
                    .op(OpCode::Conv_I4);
                e.branch(OpCode::Blt, loop_body);
            },
            |e| {
                e.pop();
            },
        );

        e.mark(done);
        e.ldloc(result).ret();
    })
}

/// `IntPtr FindMethodFromLoaded(IntPtr typeName, IntPtr methodName, IntPtr asmName, int paramCount)`
///
/// Like `GetMethodFromLoaded` but uses `GetMethods()` + name/param-count
/// filter, so overloaded methods resolve correctly.
fn find_method_from_loaded_body(t: &Tokens) -> hao::dotnet::il::MethodBody {
    Emitter::build(|e| {
        let type_name = e.declare_local(TypeSig::STRING);
        let method_name = e.declare_local(TypeSig::STRING);
        let asm_name = e.declare_local(TypeSig::STRING);
        let assemblies = e.declare_local(TypeSig::sz_array(TypeSig::class(t.assembly.clone())));
        let asm_idx = e.declare_local(TypeSig::INT32);
        let current_asm = e.declare_local(TypeSig::class(t.assembly.clone()));
        let found_type = e.declare_local(TypeSig::class(t.type_ty.clone()));
        let methods_arr =
            e.declare_local(TypeSig::sz_array(TypeSig::class(t.method_info.clone())));
        let m_idx = e.declare_local(TypeSig::INT32);
        let candidate = e.declare_local(TypeSig::class(t.method_info.clone()));
        let handle = e.declare_local(TypeSig::value_type(t.handle_ty.clone()));
        let result = e.declare_local(TypeSig::NATIVE_INT);

        e.ldarg(0).call(&t.to_string).stloc(type_name);
        e.ldarg(1).call(&t.to_string).stloc(method_name);
        e.ldarg(2).call(&t.to_string).stloc(asm_name);

        e.ldc_i4(0).op(OpCode::Conv_I).stloc(result);

        let asm_loop_check = e.label();
        let asm_loop_body = e.label();
        let done = e.label();

        e.try_catch(
            t.exception.clone(),
            |e| {
                e.call(&t.get_current_domain)
                    .callvirt(&t.get_assemblies)
                    .stloc(assemblies);
                e.ldc_i4(0).stloc(asm_idx);
                e.branch(OpCode::Br, asm_loop_check);

                e.mark(asm_loop_body);
                e.ldloc(assemblies)
                    .ldloc(asm_idx)
                    .op(OpCode::Ldelem_Ref)
                    .stloc(current_asm);
                e.ldloc(current_asm)
                    .callvirt(&t.get_asm_name)
                    .callvirt(&t.get_name_prop);
                e.ldloc(asm_name).call(&t.string_equals);
                let skip_asm = e.label();
                e.branch(OpCode::Brfalse, skip_asm);

                e.ldloc(current_asm)
                    .ldloc(type_name)
                    .callvirt(&t.asm_get_type)
                    .stloc(found_type);
                e.ldloc(found_type).branch(OpCode::Brfalse, skip_asm);

                e.ldloc(found_type)
                    .ldc_i4(BINDING_FLAGS)
                    .callvirt(&t.get_methods)
                    .stloc(methods_arr);
                e.ldc_i4(0).stloc(m_idx);
                let m_loop_check = e.label();
                let m_loop_body = e.label();
                e.branch(OpCode::Br, m_loop_check);

                e.mark(m_loop_body);
                e.ldloc(methods_arr)
                    .ldloc(m_idx)
                    .op(OpCode::Ldelem_Ref)
                    .stloc(candidate);
                e.ldloc(candidate).callvirt(&t.get_member_name);
                e.ldloc(method_name).call(&t.string_equals);
                let skip_m = e.label();
                e.branch(OpCode::Brfalse, skip_m);
                e.ldloc(candidate)
                    .callvirt(&t.get_parameters)
                    .op(OpCode::Ldlen)
                    .op(OpCode::Conv_I4);
                e.ldarg(3);
                e.branch(OpCode::Bne_Un, skip_m);

                e.ldloc(candidate).callvirt(&t.method_handle).stloc(handle);
                e.ldloc(handle).call(&t.prepare);
                e.ldloca(handle)
                    .call(&t.function_pointer)
                    .stloc(result);
                e.branch(OpCode::Leave, done);

                e.mark(skip_m);
                e.ldloc(m_idx).ldc_i4(1).op(OpCode::Add).stloc(m_idx);

                e.mark(m_loop_check);
                e.ldloc(m_idx)
                    .ldloc(methods_arr)
                    .op(OpCode::Ldlen)
                    .op(OpCode::Conv_I4);
                e.branch(OpCode::Blt, m_loop_body);

                e.mark(skip_asm);
                e.ldloc(asm_idx)
                    .ldc_i4(1)
                    .op(OpCode::Add)
                    .stloc(asm_idx);

                e.mark(asm_loop_check);
                e.ldloc(asm_idx)
                    .ldloc(assemblies)
                    .op(OpCode::Ldlen)
                    .op(OpCode::Conv_I4);
                e.branch(OpCode::Blt, asm_loop_body);
            },
            |e| {
                e.pop();
            },
        );

        e.mark(done);
        e.ldloc(result).ret();
    })
}

/// `IntPtr GetTypeNames(IntPtr asmName)`
///
/// Iterates `AppDomain.CurrentDomain.GetAssemblies()`, matches by assembly
/// name, then returns a CoTaskMem-allocated UTF-16 string of `\n`-separated
/// type full names.  Returns 0 on failure or if the assembly has no types.
fn get_type_names_body(t: &Tokens) -> hao::dotnet::il::MethodBody {
    Emitter::build(|e| {
        let asm_name = e.declare_local(TypeSig::STRING);
        let result = e.declare_local(TypeSig::STRING);
        let assemblies = e.declare_local(TypeSig::sz_array(TypeSig::class(t.assembly.clone())));
        let asm_idx = e.declare_local(TypeSig::INT32);
        let current_asm = e.declare_local(TypeSig::class(t.assembly.clone()));
        let types = e.declare_local(TypeSig::sz_array(TypeSig::class(t.type_ty.clone())));
        let t_idx = e.declare_local(TypeSig::INT32);
        let current_type = e.declare_local(TypeSig::class(t.type_ty.clone()));
        let name = e.declare_local(TypeSig::STRING);
        let ret = e.declare_local(TypeSig::NATIVE_INT);

        e.ldarg(0).call(&t.to_string).stloc(asm_name);
        e.ldstr("").stloc(result);
        e.ldc_i4(0).op(OpCode::Conv_I).stloc(ret);

        let asm_loop_check = e.label();
        let asm_loop_body = e.label();
        let done = e.label();

        e.try_catch(
            t.exception.clone(),
            |e| {
                e.call(&t.get_current_domain)
                    .callvirt(&t.get_assemblies)
                    .stloc(assemblies);
                e.ldc_i4(0).stloc(asm_idx);
                e.branch(OpCode::Br, asm_loop_check);

                e.mark(asm_loop_body);
                e.ldloc(assemblies)
                    .ldloc(asm_idx)
                    .op(OpCode::Ldelem_Ref)
                    .stloc(current_asm);
                e.ldloc(current_asm)
                    .callvirt(&t.get_asm_name)
                    .callvirt(&t.get_name_prop);
                e.ldloc(asm_name).call(&t.string_equals);
                let skip_asm = e.label();
                e.branch(OpCode::Brfalse, skip_asm);

                // Found the assembly — iterate its types.
                e.ldloc(current_asm)
                    .callvirt(&t.asm_get_types)
                    .stloc(types);
                e.ldc_i4(0).stloc(t_idx);
                let t_loop_check = e.label();
                let t_loop_body = e.label();
                e.branch(OpCode::Br, t_loop_check);

                e.mark(t_loop_body);
                e.ldloc(types)
                    .ldloc(t_idx)
                    .op(OpCode::Ldelem_Ref)
                    .stloc(current_type);
                e.ldloc(current_type)
                    .callvirt(&t.type_get_full_name)
                    .stloc(name);
                e.ldloc(name);
                let skip_type = e.label();
                e.branch(OpCode::Brfalse, skip_type);

                // if (result.Length > 0) result = result + "\n";
                e.ldloc(result).callvirt(&t.get_length);
                let no_sep = e.label();
                e.branch(OpCode::Brfalse, no_sep);
                e.ldloc(result).ldstr("\n").call(&t.string_concat).stloc(result);
                e.mark(no_sep);
                // result = result + name;
                e.ldloc(result).ldloc(name).call(&t.string_concat).stloc(result);

                e.mark(skip_type);
                e.ldloc(t_idx).ldc_i4(1).op(OpCode::Add).stloc(t_idx);

                e.mark(t_loop_check);
                e.ldloc(t_idx)
                    .ldloc(types)
                    .op(OpCode::Ldlen)
                    .op(OpCode::Conv_I4);
                e.branch(OpCode::Blt, t_loop_body);

                // Convert result to CoTaskMem pointer.
                e.ldloc(result).callvirt(&t.get_length);
                let empty = e.label();
                e.branch(OpCode::Brfalse, empty);
                e.ldloc(result).call(&t.marshal_to_cotask).stloc(ret);
                e.mark(empty);
                e.branch(OpCode::Leave, done);

                e.mark(skip_asm);
                e.ldloc(asm_idx).ldc_i4(1).op(OpCode::Add).stloc(asm_idx);

                e.mark(asm_loop_check);
                e.ldloc(asm_idx)
                    .ldloc(assemblies)
                    .op(OpCode::Ldlen)
                    .op(OpCode::Conv_I4);
                e.branch(OpCode::Blt, asm_loop_body);
            },
            |e| {
                e.pop();
            },
        );

        e.mark(done);
        e.ldloc(ret).ret();
    })
}

/// `IntPtr GetMethodNames(IntPtr typeName, IntPtr asmName)`
///
/// Finds the named type in the specified loaded assembly and returns a
/// CoTaskMem-allocated UTF-16 string of `\n`-separated method names.
/// Returns 0 on failure.
fn get_method_names_body(t: &Tokens) -> hao::dotnet::il::MethodBody {
    Emitter::build(|e| {
        let type_name = e.declare_local(TypeSig::STRING);
        let asm_name = e.declare_local(TypeSig::STRING);
        let result = e.declare_local(TypeSig::STRING);
        let assemblies = e.declare_local(TypeSig::sz_array(TypeSig::class(t.assembly.clone())));
        let asm_idx = e.declare_local(TypeSig::INT32);
        let current_asm = e.declare_local(TypeSig::class(t.assembly.clone()));
        let found_type = e.declare_local(TypeSig::class(t.type_ty.clone()));
        let methods_arr =
            e.declare_local(TypeSig::sz_array(TypeSig::class(t.method_info.clone())));
        let m_idx = e.declare_local(TypeSig::INT32);
        let candidate = e.declare_local(TypeSig::class(t.method_info.clone()));
        let name = e.declare_local(TypeSig::STRING);
        let ret = e.declare_local(TypeSig::NATIVE_INT);

        e.ldarg(0).call(&t.to_string).stloc(type_name);
        e.ldarg(1).call(&t.to_string).stloc(asm_name);
        e.ldstr("").stloc(result);
        e.ldc_i4(0).op(OpCode::Conv_I).stloc(ret);

        let asm_loop_check = e.label();
        let asm_loop_body = e.label();
        let done = e.label();

        e.try_catch(
            t.exception.clone(),
            |e| {
                e.call(&t.get_current_domain)
                    .callvirt(&t.get_assemblies)
                    .stloc(assemblies);
                e.ldc_i4(0).stloc(asm_idx);
                e.branch(OpCode::Br, asm_loop_check);

                e.mark(asm_loop_body);
                e.ldloc(assemblies)
                    .ldloc(asm_idx)
                    .op(OpCode::Ldelem_Ref)
                    .stloc(current_asm);
                e.ldloc(current_asm)
                    .callvirt(&t.get_asm_name)
                    .callvirt(&t.get_name_prop);
                e.ldloc(asm_name).call(&t.string_equals);
                let skip_asm = e.label();
                e.branch(OpCode::Brfalse, skip_asm);

                // Found assembly — find the type.
                e.ldloc(current_asm)
                    .ldloc(type_name)
                    .callvirt(&t.asm_get_type)
                    .stloc(found_type);
                e.ldloc(found_type).branch(OpCode::Brfalse, skip_asm);

                // Get all methods.
                e.ldloc(found_type)
                    .ldc_i4(BINDING_FLAGS)
                    .callvirt(&t.get_methods)
                    .stloc(methods_arr);
                e.ldc_i4(0).stloc(m_idx);
                let m_loop_check = e.label();
                let m_loop_body = e.label();
                e.branch(OpCode::Br, m_loop_check);

                e.mark(m_loop_body);
                e.ldloc(methods_arr)
                    .ldloc(m_idx)
                    .op(OpCode::Ldelem_Ref)
                    .stloc(candidate);
                e.ldloc(candidate).callvirt(&t.get_member_name).stloc(name);

                // if (result.Length > 0) result = result + "\n";
                e.ldloc(result).callvirt(&t.get_length);
                let no_sep = e.label();
                e.branch(OpCode::Brfalse, no_sep);
                e.ldloc(result).ldstr("\n").call(&t.string_concat).stloc(result);
                e.mark(no_sep);
                e.ldloc(result).ldloc(name).call(&t.string_concat).stloc(result);

                e.ldloc(m_idx).ldc_i4(1).op(OpCode::Add).stloc(m_idx);

                e.mark(m_loop_check);
                e.ldloc(m_idx)
                    .ldloc(methods_arr)
                    .op(OpCode::Ldlen)
                    .op(OpCode::Conv_I4);
                e.branch(OpCode::Blt, m_loop_body);

                // Convert to CoTaskMem.
                e.ldloc(result).callvirt(&t.get_length);
                let empty = e.label();
                e.branch(OpCode::Brfalse, empty);
                e.ldloc(result).call(&t.marshal_to_cotask).stloc(ret);
                e.mark(empty);
                e.branch(OpCode::Leave, done);

                e.mark(skip_asm);
                e.ldloc(asm_idx).ldc_i4(1).op(OpCode::Add).stloc(asm_idx);

                e.mark(asm_loop_check);
                e.ldloc(asm_idx)
                    .ldloc(assemblies)
                    .op(OpCode::Ldlen)
                    .op(OpCode::Conv_I4);
                e.branch(OpCode::Blt, asm_loop_body);
            },
            |e| {
                e.pop();
            },
        );

        e.mark(done);
        e.ldloc(ret).ret();
    })
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
            }
        }?;
        module.write()
    };
    build().map_err(|e| Error::Other(format!("could not build the loader assembly: {}", e)))
}
