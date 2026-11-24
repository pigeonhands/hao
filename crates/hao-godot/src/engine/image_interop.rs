//! The `Hao.Godot.Interop` helper assembly: a compound managed method that
//! performs a full texture → RGBA8 extraction in one cooperative-GC scope.
//!
//! Calling Godot Image methods one `invoke` at a time holds raw managed
//! object pointers across calls while the thread sits in preemptive GC mode
//! between them — the GC can compact the heap and move the Image, and the
//! stale pointer then corrupts the next call.  `ExtractImageRgba` instead
//! wraps the whole sequence (Texture2DGet → Decompress → Convert → GetWidth
//! → GetHeight → GetData → copy-out) inside a single `[UnmanagedCallersOnly]`
//! method emitted with [`callchain!`](hao_runtime::hao::callchain): the
//! resolved GodotSharp method pointers are baked into the IL, every internal
//! `calli` uses `SuppressGCTransition`, and the thread stays cooperative from
//! entry to exit, so the GC cannot relocate anything while the managed
//! pointers are live.

use std::sync::OnceLock;

use hao_runtime::hao::{
    dotnet::{
        entries::{well_known::SystemRuntimeInteropServicesType, Method, TypeDef},
        il::ChainImports,
        module::{ModuleKind, TargetRuntime},
        signature::{MethodSig, TypeSig},
    },
    Module,
};
use hao_runtime::{Error, Result, Runtime};

const ASSEMBLY: &str = "Hao.Godot.Interop";

/// The fixed result-buffer header, written by the chain's `returns` section
/// and read back by [`extract_rgba_compound`](super::image::extract_rgba_compound).
/// Pixel data follows immediately after.
#[repr(C)]
pub(crate) struct ExtractHeader {
    pub(crate) width: i64,
    pub(crate) height: i64,
    pub(crate) data_len: i64,
}

/// `long ExtractImageRgba(long rid, IntPtr resultPtr, long bufSize)`
type ExtractFn = unsafe extern "system" fn(u64, *mut u8, i64) -> i64;

static EXTRACT_PTR: OnceLock<usize> = OnceLock::new();

/// Calls the compound extraction method for the texture `rid`.
///
/// On success `result` starts with an [`ExtractHeader`] followed by
/// `data_len` bytes of RGBA8 pixels.
pub(crate) fn extract_image_rgba(rt: &Runtime, rid: u64, result: &mut [u8]) -> Result<()> {
    let ptr = resolve(rt)?;
    let f: ExtractFn = unsafe { std::mem::transmute(ptr) };
    let rc = unsafe { f(rid, result.as_mut_ptr(), result.len() as i64) };
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
    let module = build_assembly(rt, major)?;
    rt.load_module(&module)?;
    let ptr = rt.function_pointer(ASSEMBLY, ASSEMBLY, "ExtractImageRgba")? as usize;
    Ok(*EXTRACT_PTR.get_or_init(|| ptr))
}

fn hao_err(e: impl std::fmt::Display) -> Error {
    Error::Other(format!("could not build {ASSEMBLY}: {e}"))
}

fn build_assembly(rt: &Runtime, major: u16) -> Result<Module> {
    let mut module = Module::new(
        ASSEMBLY,
        ModuleKind::Dll,
        TargetRuntime::NetCoreImplementation(major),
    );

    let uco = module
        .corlib_attribute(SystemRuntimeInteropServicesType::UnmanagedCallersOnlyAttribute)
        .map_err(hao_err)?;
    let imports = ChainImports::resolve(&mut module).map_err(hao_err)?;

    let body = hao_runtime::hao::callchain! {
        imports: &imports,
        methods {
            tex2d_get: super::rendering_server::TEXTURE_2D_GET.resolve(rt)?,
            decompress: super::image::DECOMPRESS.resolve(rt)?,
            convert: super::image::CONVERT.resolve(rt)?,
            get_data: super::image::GET_DATA.resolve(rt)?,
            get_width: super::image::GET_WIDTH.resolve(rt)?,
            get_height: super::image::GET_HEIGHT.resolve(rt)?,
        },
        consts { rgba8: super::image::format::RGBA8 },
        params(rid: i64, result_ptr: nint, buf_size: i64),
        errors { err_null_image = 1, err_null_data = 2, err_overflow = 3 },
        chain {
            let img = tex2d_get(rid) else err_null_image;
            decompress(img);
            convert(img, rgba8);
            let w = get_width(img);
            let h = get_height(img);
            let arr = get_data(img) else err_null_data;
            // .NET byte[]: [MethodTable*:8][Length:4][pad:4][data…]
            let len: i32 = *(arr + 8);
        },
        returns result_ptr: ExtractHeader {
            width: w,
            height: h,
            data_len: len,
            ..copy(arr + 16, len) within buf_size else err_overflow,
        },
    };

    let interop = module.add_static_class("Hao.Godot", "Interop").map_err(hao_err)?;
    TypeDef::add_method(
        &interop,
        Method::public_static(
            "ExtractImageRgba",
            MethodSig::new_static(
                TypeSig::INT64,
                vec![TypeSig::INT64, TypeSig::NATIVE_INT, TypeSig::INT64],
            ),
        )
        .with_attribute(uco)
        .with_body(body),
    );
    Ok(module)
}
