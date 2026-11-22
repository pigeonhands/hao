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
        entries::{Method, TypeDef, TypeDefOrRef},
        il::{Emitter, MemberToken, OpCode},
        module::{ModuleKind, TargetRuntime},
        signature::{MethodSig, TypeSig},
    },
    Module,
};

use crate::error::{Error, Result};

const LOADER: &str = "Hao.Loader";

/// Instance | Static | Public | NonPublic
const BINDING_FLAGS: i32 = 4 | 8 | 16 | 32;

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
    string_equals: MemberToken,
    get_methods: MemberToken,
    get_member_name: MemberToken,
    get_parameters: MemberToken,
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
fn load_from_body(
    to_string: &MemberToken,
    exception: &TypeDefOrRef,
    load_from: MemberToken,
    hresult: MemberToken,
) -> hao::dotnet::il::MethodBody {
    Emitter::build(|e| {
        let result = e.declare_local(TypeSig::INT32);
        e.try_catch(
            exception.clone(),
            |e| {
                e.ldarg(0).call(to_string).call(load_from).pop();
            },
            |e| {
                e.callvirt(hresult).stloc(result);
            },
        );
        e.ldloc(result).ret();
    })
}

/// `IntPtr GetMethod(IntPtr fullName, IntPtr name)`
///
/// Resolves via `Type.GetType(fullName).GetMethod(name, flags)`, then
/// `PrepareMethod` + `GetFunctionPointer`. Returns 0 on any exception.
fn get_method_body(t: &Tokens) -> hao::dotnet::il::MethodBody {
    Emitter::build(|e| {
        let found_type = e.declare_local(TypeSig::class(t.type_ty.clone()));
        let method = e.declare_local(TypeSig::class(t.method_info.clone()));
        let handle = e.declare_local(TypeSig::value_type(t.handle_ty.clone()));
        let result = e.declare_local(TypeSig::NATIVE_INT);
        e.ldc_i4(0).op(OpCode::Conv_I).stloc(result);
        e.try_catch(
            t.exception.clone(),
            |e| {
                e.ldarg(0)
                    .call(&t.to_string)
                    .call(&t.get_type)
                    .stloc(found_type);
                e.ldloc(found_type)
                    .ldarg(1)
                    .call(&t.to_string)
                    .ldc_i4(BINDING_FLAGS)
                    .callvirt(&t.get_method)
                    .stloc(method);
                e.ldloc(method).callvirt(&t.method_handle).stloc(handle);
                e.ldloc(handle).call(&t.prepare);
                e.ldloca(handle).call(&t.function_pointer).stloc(result);
            },
            |e| {
                e.pop();
            },
        );
        e.ldloc(result).ret();
    })
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

// ---------------------------------------------------------------------------
// Assembly builder
// ---------------------------------------------------------------------------

/// Builds a small `Hao.Loader` assembly with four `[UnmanagedCallersOnly]`
/// helpers:
///
/// - `int LoadFrom(IntPtr path)` — `Assembly.LoadFrom`, HResult on failure.
/// - `IntPtr GetMethod(IntPtr fullName, IntPtr name)` — resolve via
///   `Type.GetType`, JIT-compile, return native pointer (0 on failure).
/// - `IntPtr GetMethodFromLoaded(IntPtr typeName, IntPtr methodName,
///   IntPtr asmName)` — like `GetMethod` but iterates loaded assemblies.
/// - `IntPtr FindMethodFromLoaded(IntPtr typeName, IntPtr methodName,
///   IntPtr asmName, int paramCount)` — overload-safe: matches by name
///   and parameter count via `GetMethods()`.
pub fn loader_assembly(major: u16) -> Result<Vec<u8>> {
    let build = || -> hao::error::Result<Vec<u8>> {
        let mut module = Module::new(
            LOADER,
            ModuleKind::Dll,
            TargetRuntime::NetCoreImplementation(major),
        );

        let uco = module
            .corlib_attribute("System.Runtime.InteropServices.UnmanagedCallersOnlyAttribute")?;

        let to_string = module.import_corlib_method(
            "System.Runtime.InteropServices.Marshal",
            "PtrToStringAuto",
            MethodSig::new_static(TypeSig::STRING, vec![TypeSig::NATIVE_INT]),
        )?;
        let exception = module.corlib_type("System.Exception")?;

        // ── LoadFrom-specific imports ─────────────────────────────────────
        let assembly = module.corlib_type("System.Reflection.Assembly")?;
        let asm_load_from = module.import_method_on(
            &assembly,
            "LoadFrom",
            MethodSig::new_static(TypeSig::class(assembly.clone()), vec![TypeSig::STRING]),
        )?;
        let hresult = module.import_method_on(
            &exception,
            "get_HResult",
            MethodSig::new_instance(TypeSig::INT32, vec![]),
        )?;

        // ── Shared reflection imports ─────────────────────────────────────
        let type_ty = module.corlib_type("System.Type")?;
        let method_info = module.corlib_type("System.Reflection.MethodInfo")?;
        let method_base = module.corlib_type("System.Reflection.MethodBase")?;
        let handle_ty = module.corlib_type("System.RuntimeMethodHandle")?;
        let binding_flags = module.corlib_type("System.Reflection.BindingFlags")?;

        let get_type = module.import_method_on(
            &type_ty,
            "GetType",
            MethodSig::new_static(TypeSig::class(type_ty.clone()), vec![TypeSig::STRING]),
        )?;
        let get_method = module.import_method_on(
            &type_ty,
            "GetMethod",
            MethodSig::new_instance(
                TypeSig::class(method_info.clone()),
                vec![TypeSig::STRING, TypeSig::value_type(binding_flags.clone())],
            ),
        )?;
        let method_handle = module.import_method_on(
            &method_base,
            "get_MethodHandle",
            MethodSig::new_instance(TypeSig::value_type(handle_ty.clone()), vec![]),
        )?;
        let prepare = module.import_corlib_method(
            "System.Runtime.CompilerServices.RuntimeHelpers",
            "PrepareMethod",
            MethodSig::new_static(TypeSig::VOID, vec![TypeSig::value_type(handle_ty.clone())]),
        )?;
        let function_pointer = module.import_method_on(
            &handle_ty,
            "GetFunctionPointer",
            MethodSig::new_instance(TypeSig::NATIVE_INT, vec![]),
        )?;

        // ── Assembly-iteration imports ────────────────────────────────────
        let app_domain = module.corlib_type("System.AppDomain")?;
        let assembly_name_ty = module.corlib_type("System.Reflection.AssemblyName")?;
        let get_current_domain = module.import_method_on(
            &app_domain,
            "get_CurrentDomain",
            MethodSig::new_static(TypeSig::class(app_domain.clone()), vec![]),
        )?;
        let get_assemblies = module.import_method_on(
            &app_domain,
            "GetAssemblies",
            MethodSig::new_instance(
                TypeSig::sz_array(TypeSig::class(assembly.clone())),
                vec![],
            ),
        )?;
        let get_asm_name = module.import_method_on(
            &assembly,
            "GetName",
            MethodSig::new_instance(TypeSig::class(assembly_name_ty.clone()), vec![]),
        )?;
        let get_name_prop = module.import_method_on(
            &assembly_name_ty,
            "get_Name",
            MethodSig::new_instance(TypeSig::STRING, vec![]),
        )?;
        let asm_get_type = module.import_method_on(
            &assembly,
            "GetType",
            MethodSig::new_instance(TypeSig::class(type_ty.clone()), vec![TypeSig::STRING]),
        )?;
        let string_equals = module.import_corlib_method(
            "System.String",
            "op_Equality",
            MethodSig::new_static(TypeSig::BOOL, vec![TypeSig::STRING, TypeSig::STRING]),
        )?;

        // ── Overload-resolution imports ───────────────────────────────────
        let get_methods = module.import_method_on(
            &type_ty,
            "GetMethods",
            MethodSig::new_instance(
                TypeSig::sz_array(TypeSig::class(method_info.clone())),
                vec![TypeSig::value_type(binding_flags)],
            ),
        )?;
        let member_info = module.corlib_type("System.Reflection.MemberInfo")?;
        let get_member_name = module.import_method_on(
            &member_info,
            "get_Name",
            MethodSig::new_instance(TypeSig::STRING, vec![]),
        )?;
        let param_info = module.corlib_type("System.Reflection.ParameterInfo")?;
        let get_parameters = module.import_method_on(
            &method_base,
            "GetParameters",
            MethodSig::new_instance(
                TypeSig::sz_array(TypeSig::class(param_info)),
                vec![],
            ),
        )?;

        // ── Token bundle ──────────────────────────────────────────────────
        let tokens = Tokens {
            to_string,
            exception,
            type_ty,
            method_info,
            handle_ty,
            assembly,
            get_type,
            get_method,
            method_handle,
            prepare,
            function_pointer,
            get_current_domain,
            get_assemblies,
            get_asm_name,
            get_name_prop,
            asm_get_type,
            string_equals,
            get_methods,
            get_member_name,
            get_parameters,
        };

        // ── Build method bodies ───────────────────────────────────────────
        let load_from =
            load_from_body(&tokens.to_string, &tokens.exception, asm_load_from, hresult);
        let get_method = get_method_body(&tokens);
        let get_method_from_loaded = get_method_from_loaded_body(&tokens);
        let find_method_from_loaded = find_method_from_loaded_body(&tokens);

        // ── Assemble the Hao.Loader class ─────────────────────────────────
        let loader = module.add_static_class("Hao", "Loader")?;
        TypeDef::add_method(
            &loader,
            Method::public_static(
                "LoadFrom",
                MethodSig::new_static(TypeSig::INT32, vec![TypeSig::NATIVE_INT]),
            )
            .with_attribute(uco.clone())
            .with_body(load_from),
        );
        TypeDef::add_method(
            &loader,
            Method::public_static(
                "GetMethod",
                MethodSig::new_static(
                    TypeSig::NATIVE_INT,
                    vec![TypeSig::NATIVE_INT, TypeSig::NATIVE_INT],
                ),
            )
            .with_attribute(uco.clone())
            .with_body(get_method),
        );
        TypeDef::add_method(
            &loader,
            Method::public_static(
                "GetMethodFromLoaded",
                MethodSig::new_static(
                    TypeSig::NATIVE_INT,
                    vec![
                        TypeSig::NATIVE_INT,
                        TypeSig::NATIVE_INT,
                        TypeSig::NATIVE_INT,
                    ],
                ),
            )
            .with_attribute(uco.clone())
            .with_body(get_method_from_loaded),
        );
        TypeDef::add_method(
            &loader,
            Method::public_static(
                "FindMethodFromLoaded",
                MethodSig::new_static(
                    TypeSig::NATIVE_INT,
                    vec![
                        TypeSig::NATIVE_INT,
                        TypeSig::NATIVE_INT,
                        TypeSig::NATIVE_INT,
                        TypeSig::INT32,
                    ],
                ),
            )
            .with_attribute(uco)
            .with_body(find_method_from_loaded),
        );
        module.write()
    };
    build().map_err(|e| Error::Other(format!("could not build the loader assembly: {}", e)))
}
