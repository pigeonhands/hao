//! The `Hao.Godot.Interop` helper assembly: a compound managed method that
//! performs a full texture → RGBA8 extraction in one cooperative-GC scope.
//!
//! Calling Godot Image methods one `invoke` at a time holds raw managed
//! object pointers across calls while the thread sits in preemptive GC mode
//! between them — the GC can compact the heap and move the Image, and the
//! stale pointer then corrupts the next call.  `ExtractImageRgba` instead
//! wraps the whole sequence (Texture2DGet → Decompress → Convert → GetWidth
//! → GetHeight → GetData → copy-out) inside a single `[UnmanagedCallersOnly]`
//! method whose internal `calli`s all use `SuppressGCTransition`: the thread
//! stays cooperative from entry to exit, so the GC cannot relocate anything
//! while the managed pointers are live.

use std::sync::OnceLock;

use hao_runtime::hao::{
    dotnet::{
        entries::{
            well_known::{
                SystemRuntimeCompilerServicesType, SystemRuntimeInteropServicesType,
                SystemType,
            },
            Method, TypeDef, TypeDefOrRef,
        },
        il::{Emitter, MethodBody, OpCode},
        module::{ModuleKind, TargetRuntime},
        signature::{MethodSig, TypeSig},
    },
    Module,
};
use hao_runtime::{Error, Result, Runtime};

const ASSEMBLY: &str = "Hao.Godot.Interop";

type ExtractFn = unsafe extern "system" fn(*const u64, *mut u8, i64) -> i64;

static EXTRACT_PTR: OnceLock<usize> = OnceLock::new();

/// Calls the compound extraction method.
///
/// `args` layout: `[fnTexture2DGet, fnDecompress, fnConvert, fnGetData,
/// fnGetWidth, fnGetHeight, rid, rgba8Format]`.
///
/// On success `result` is filled with `[0i64, width, height, dataLen,
/// pixel_data...]`.
pub(crate) fn extract_image_rgba(
    rt: &Runtime,
    args: &[u64; 8],
    result: &mut [u8],
) -> Result<()> {
    let ptr = resolve(rt)?;
    let f: ExtractFn = unsafe { std::mem::transmute(ptr) };
    let rc = unsafe { f(args.as_ptr(), result.as_mut_ptr(), result.len() as i64) };
    match rc {
        0 => Ok(()),
        1 => Err(Error::Other("Texture2DGet returned null".into())),
        2 => Err(Error::Other("Image.GetData returned null".into())),
        3 => Err(Error::Other("result buffer too small for pixel data".into())),
        _ => Err(Error::Other(format!("ExtractImageRgba error {rc}"))),
    }
}

/// Builds and loads the interop assembly on first use, then resolves the
/// `[UnmanagedCallersOnly]` method pointer.
fn resolve(rt: &Runtime) -> Result<usize> {
    if let Some(&ptr) = EXTRACT_PTR.get() {
        return Ok(ptr);
    }
    let major = rt
        .dotnet_major()
        .ok_or_else(|| Error::Other("compound image extraction is CoreCLR-only".into()))?;
    let module = build_assembly(major)
        .map_err(|e| Error::Other(format!("could not build {ASSEMBLY}: {e}")))?;
    rt.load_module(&module)?;
    let ptr = rt.function_pointer(ASSEMBLY, ASSEMBLY, "ExtractImageRgba")? as usize;
    Ok(*EXTRACT_PTR.get_or_init(|| ptr))
}

fn build_assembly(major: u16) -> hao_runtime::hao::error::Result<Module> {
    let mut module = Module::new(
        ASSEMBLY,
        ModuleKind::Dll,
        TargetRuntime::NetCoreImplementation(major),
    );

    let uco = module
        .corlib_attribute(SystemRuntimeInteropServicesType::UnmanagedCallersOnlyAttribute)?;
    let exception = module.corlib_type(SystemType::Exception)?;
    let callconv_cdecl =
        module.corlib_type(SystemRuntimeCompilerServicesType::CallConvCdecl)?;
    let callconv_suppress =
        module.corlib_type(SystemRuntimeCompilerServicesType::CallConvSuppressGCTransition)?;

    let body = extract_image_rgba_body(&exception, &callconv_cdecl, &callconv_suppress);

    let interop = module.add_static_class("Hao.Godot", "Interop")?;
    TypeDef::add_method(
        &interop,
        Method::public_static(
            "ExtractImageRgba",
            MethodSig::new_static(
                TypeSig::INT64,
                vec![TypeSig::NATIVE_INT, TypeSig::NATIVE_INT, TypeSig::INT64],
            ),
        )
        .with_attribute(uco)
        .with_body(body),
    );
    Ok(module)
}

/// `long ExtractImageRgba(IntPtr argsPtr, IntPtr resultPtr, long resultBufSize)`
///
/// ## Args buffer layout (8 × i64)
///
/// | offset | field            |
/// |--------|------------------|
/// | 0      | fnTexture2DGet   |
/// | 8      | fnDecompress     |
/// | 16     | fnConvert        |
/// | 24     | fnGetData        |
/// | 32     | fnGetWidth       |
/// | 40     | fnGetHeight      |
/// | 48     | rid (u64)        |
/// | 56     | rgba8Format (i64)|
///
/// ## Result buffer layout
///
/// | offset | field        |
/// |--------|--------------|
/// | 0      | error (i64)  |
/// | 8      | width (i64)  |
/// | 16     | height (i64) |
/// | 24     | dataLen (i64)|
/// | 32..   | pixel data   |
///
/// Returns 0 on success, 1 on null image, 2 on null data, 3 on buffer
/// overflow, -1 on managed exception.
fn extract_image_rgba_body(
    exception: &TypeDefOrRef,
    callconv_cdecl: &TypeDefOrRef,
    callconv_suppress: &TypeDefOrRef,
) -> MethodBody {
    let calli_1_arg = MethodSig::unmanaged_suppress_gc(
        TypeSig::INT64,
        vec![TypeSig::INT64],
        callconv_cdecl,
        callconv_suppress,
    );
    let calli_2_arg = MethodSig::unmanaged_suppress_gc(
        TypeSig::INT64,
        vec![TypeSig::INT64, TypeSig::INT64],
        callconv_cdecl,
        callconv_suppress,
    );

    Emitter::build(|e| {
        let fn_tex2d_get = e.declare_local(TypeSig::NATIVE_INT);
        let fn_decompress = e.declare_local(TypeSig::NATIVE_INT);
        let fn_convert = e.declare_local(TypeSig::NATIVE_INT);
        let fn_get_data = e.declare_local(TypeSig::NATIVE_INT);
        let fn_get_width = e.declare_local(TypeSig::NATIVE_INT);
        let fn_get_height = e.declare_local(TypeSig::NATIVE_INT);
        let rid = e.declare_local(TypeSig::INT64);
        let rgba8 = e.declare_local(TypeSig::INT64);
        let img = e.declare_local(TypeSig::INT64);
        let w = e.declare_local(TypeSig::INT64);
        let h = e.declare_local(TypeSig::INT64);
        let arr = e.declare_local(TypeSig::INT64);
        let len = e.declare_local(TypeSig::INT64);
        let data_ptr = e.declare_local(TypeSig::NATIVE_INT);
        let result_local = e.declare_local(TypeSig::INT64);

        // Default return value = -1 (exception)
        e.ldc_i4(-1).op(OpCode::Conv_I8).stloc(result_local);

        let err_null_image = e.label();
        let err_null_data = e.label();
        let err_overflow = e.label();
        let done = e.label();

        e.try_catch(
            exception.clone(),
            |e| {
                // ── Read args from the native buffer (argsPtr = arg 0) ─
                e.ldarg(0).op(OpCode::Ldind_I).stloc(fn_tex2d_get);
                e.ldarg(0).ldc_i4(8).op(OpCode::Add).op(OpCode::Ldind_I).stloc(fn_decompress);
                e.ldarg(0).ldc_i4(16).op(OpCode::Add).op(OpCode::Ldind_I).stloc(fn_convert);
                e.ldarg(0).ldc_i4(24).op(OpCode::Add).op(OpCode::Ldind_I).stloc(fn_get_data);
                e.ldarg(0).ldc_i4(32).op(OpCode::Add).op(OpCode::Ldind_I).stloc(fn_get_width);
                e.ldarg(0).ldc_i4(40).op(OpCode::Add).op(OpCode::Ldind_I).stloc(fn_get_height);
                e.ldarg(0).ldc_i4(48).op(OpCode::Add).op(OpCode::Ldind_I8).stloc(rid);
                e.ldarg(0).ldc_i4(56).op(OpCode::Add).op(OpCode::Ldind_I8).stloc(rgba8);

                // ── Texture2DGet(rid) → managed Image ─────────────────
                e.ldloc(rid);
                e.ldloc(fn_tex2d_get);
                e.calli(calli_1_arg.clone());
                e.stloc(img);
                e.ldloc(img).branch(OpCode::Brfalse, err_null_image);

                // ── Decompress(img) ───────────────────────────────────
                e.ldloc(img);
                e.ldloc(fn_decompress);
                e.calli(calli_1_arg.clone());
                e.op(OpCode::Pop);

                // ── Convert(img, rgba8) ───────────────────────────────
                e.ldloc(img);
                e.ldloc(rgba8);
                e.ldloc(fn_convert);
                e.calli(calli_2_arg.clone());
                e.op(OpCode::Pop);

                // ── GetWidth(img), GetHeight(img) ─────────────────────
                e.ldloc(img);
                e.ldloc(fn_get_width);
                e.calli(calli_1_arg.clone());
                e.stloc(w);

                e.ldloc(img);
                e.ldloc(fn_get_height);
                e.calli(calli_1_arg.clone());
                e.stloc(h);

                // ── GetData(img) → managed byte[] ─────────────────────
                e.ldloc(img);
                e.ldloc(fn_get_data);
                e.calli(calli_1_arg.clone());
                e.stloc(arr);
                e.ldloc(arr).branch(OpCode::Brfalse, err_null_data);

                // ── .NET byte[]: [MethodTable*:8][Length:4][pad:4][data…]
                e.ldloc(arr).ldc_i4(8).op(OpCode::Add).op(OpCode::Ldind_I4);
                e.op(OpCode::Conv_I8).stloc(len);
                e.ldloc(arr).ldc_i4(16).op(OpCode::Add).stloc(data_ptr);

                // ── Bounds check: 32 + len ≤ resultBufSize ────────────
                e.ldc_i4(32).op(OpCode::Conv_I8).ldloc(len).op(OpCode::Add);
                e.ldarg(2);
                e.branch(OpCode::Bgt, err_overflow);

                // ── Write result header (resultPtr = arg 1) ───────────
                e.ldarg(1).ldc_i4(0).op(OpCode::Conv_I8).op(OpCode::Stind_I8);
                e.ldarg(1).ldc_i4(8).op(OpCode::Add).ldloc(w).op(OpCode::Stind_I8);
                e.ldarg(1).ldc_i4(16).op(OpCode::Add).ldloc(h).op(OpCode::Stind_I8);
                e.ldarg(1).ldc_i4(24).op(OpCode::Add).ldloc(len).op(OpCode::Stind_I8);

                // ── cpblk(resultPtr + 32, dataPtr, len) ───────────────
                e.ldarg(1).ldc_i4(32).op(OpCode::Add);
                e.ldloc(data_ptr);
                e.ldloc(len).op(OpCode::Conv_I4);
                e.op(OpCode::Cpblk);

                e.ldc_i4(0).op(OpCode::Conv_I8).stloc(result_local);
                e.branch(OpCode::Leave, done);

                e.mark(err_null_image);
                e.ldc_i4(1).op(OpCode::Conv_I8).stloc(result_local);
                e.branch(OpCode::Leave, done);

                e.mark(err_null_data);
                e.ldc_i4(2).op(OpCode::Conv_I8).stloc(result_local);
                e.branch(OpCode::Leave, done);

                e.mark(err_overflow);
                e.ldc_i4(3).op(OpCode::Conv_I8).stloc(result_local);
                e.branch(OpCode::Leave, done);
            },
            |e| {
                e.pop();
            },
        );

        e.mark(done);
        e.ldloc(result_local).ret();
    })
}
