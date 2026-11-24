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
        il::{MethodBody, OpCode},
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
    hao_runtime::hao::callchain! {
        exception: exception.clone(),
        suppress_gc: (callconv_cdecl, callconv_suppress),
        args_from: arg 0 {
            fn_tex2d_get: fnptr,
            fn_decompress: fnptr,
            fn_convert: fnptr,
            fn_get_data: fnptr,
            fn_get_width: fnptr,
            fn_get_height: fnptr,
            rid: i64,
            rgba8: i64,
        },
        errors { err_null_image = 1, err_null_data = 2, err_overflow = 3 },
        chain {
            let img = fn_tex2d_get(rid) else err_null_image;
            fn_decompress(img);
            fn_convert(img, rgba8);
            let w = fn_get_width(img);
            let h = fn_get_height(img);
            let arr = fn_get_data(img) else err_null_data;
            raw |e| {
                let len = e.declare_local(TypeSig::INT64);
                let data_ptr = e.declare_local(TypeSig::NATIVE_INT);
                // .NET byte[]: [MethodTable*:8][Length:4][pad:4][data…]
                e.ldloc(arr).ldc_i4(8).op(OpCode::Add).op(OpCode::Ldind_I4);
                e.op(OpCode::Conv_I8).stloc(len);
                e.ldloc(arr).ldc_i4(16).op(OpCode::Add).stloc(data_ptr);

                // Bounds check: 32 + len ≤ resultBufSize (arg 2)
                e.ldc_i4(32).op(OpCode::Conv_I8).ldloc(len).op(OpCode::Add);
                e.ldarg(2).branch(OpCode::Bgt, err_overflow);

                // Result header (resultPtr = arg 1)
                e.ldarg(1).ldc_i4(0).op(OpCode::Conv_I8).op(OpCode::Stind_I8);
                e.ldarg(1).ldc_i4(8).op(OpCode::Add).ldloc(w).op(OpCode::Stind_I8);
                e.ldarg(1).ldc_i4(16).op(OpCode::Add).ldloc(h).op(OpCode::Stind_I8);
                e.ldarg(1).ldc_i4(24).op(OpCode::Add).ldloc(len).op(OpCode::Stind_I8);

                // cpblk(resultPtr + 32, dataPtr, len)
                e.ldarg(1).ldc_i4(32).op(OpCode::Add);
                e.ldloc(data_ptr).ldloc(len).op(OpCode::Conv_I4);
                e.op(OpCode::Cpblk);
            }
        }
    }
}
