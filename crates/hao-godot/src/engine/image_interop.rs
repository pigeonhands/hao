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
        il::ChainImports,
        module::{ModuleKind, TargetRuntime},
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
static EXTRACT_PTR: OnceLock<usize> = OnceLock::new();

/// Builds and loads the interop assembly and resolves the extraction method
/// pointer, caching it. Call this off the Godot main thread (e.g. during
/// binding warm-up on the script thread): resolution uses the UCO Hao.Loader
/// methods, which cannot be invoked from the game thread's cooperative GC
/// mode. Once warmed, [`extract_image_rgba`] only makes a raw native call.
pub(crate) fn warm(rt: &Runtime) -> Result<()> {
    resolve(rt).map(|_| ())
}

/// Calls the compound extraction method for the texture `rid`.
///
/// On success `result` starts with an [`ExtractHeader`] followed by
/// `data_len` bytes of RGBA8 pixels.
pub(crate) fn extract_image_rgba(rt: &Runtime, rid: u64, result: &mut [u8]) -> Result<()> {
    let ptr = resolve(rt)?;
    // The method is an ordinary managed static (not `[UnmanagedCallersOnly]`),
    // so its JIT entry is callable raw from the cooperative-mode game thread —
    // a UCO reverse-P/Invoke entry would failfast there. Its internal
    // SuppressGCTransition calls keep the body cooperative throughout.
    let rc = unsafe {
        hao_runtime::jit::call_native(ptr, &[rid, result.as_mut_ptr() as u64, result.len() as u64])
    } as i64;
    match rc {
        0 => Ok(()),
        1 => Err(Error::Other("Texture2DGet returned null".into())),
        2 => Err(Error::Other("Image.GetData returned null".into())),
        3 => Err(Error::Other("result buffer too small for pixel data".into())),
        _ => Err(Error::Other(format!("ExtractImageRgba error {rc}"))),
    }
}

/// Builds and loads the interop assembly on first use, then resolves the
/// method's JIT entry pointer (coop-callable, no UCO prolog).
fn resolve(rt: &Runtime) -> Result<usize> {
    if let Some(&ptr) = EXTRACT_PTR.get() {
        return Ok(ptr);
    }
    let major = rt
        .dotnet_major()
        .ok_or_else(|| Error::Other("compound image extraction is CoreCLR-only".into()))?;
    let module = build_assembly(rt, major)?;
    rt.load_module(&module)?;
    let ptr = rt.method_pointer(ASSEMBLY, "Hao.Godot.Interop", "ExtractImageRgba")? as usize;
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

    let imports = ChainImports::resolve(&mut module).map_err(hao_err)?;

    hao_runtime::hao::dotnet_class! { &mut module =>
        static class "Hao.Godot".Interop {
            // Not [UnmanagedCallersOnly]: reached via method_pointer + raw
            // call_native from the cooperative-mode game thread, where a UCO
            // reverse-P/Invoke entry would failfast.
            fn ExtractImageRgba(rid: i64, result_ptr: nint, buf_size: i64) -> i64 = chain {
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
                errors { err_null_image = 1, err_null_data = 2, err_overflow = 3 },
                chain {
                    let img = tex2d_get(rid) else err_null_image;
                    decompress(img);
                    convert(img, rgba8);
                    let w = get_width(img);
                    let h = get_height(img);
                    let arr = get_data(img) else err_null_data;
                    // .NET SZ-array layout: [MethodTable*:8][Length:4][pad:4][data…].
                    // Reading length/data directly (rather than calling
                    // Array.get_Length / Marshal.UnsafeAddrOfPinnedArrayElement)
                    // is required here: those are intrinsic/FCall methods whose
                    // function pointers cannot be invoked through the chain's
                    // unmanaged SuppressGCTransition `calli`. The GC cannot move
                    // `arr` mid-chain (cooperative mode), so the data pointer at
                    // `arr + 16` stays valid.
                    let len: i32 = *(arr + 8);
                },
                returns result_ptr: ExtractHeader {
                    width: w,
                    height: h,
                    data_len: len,
                    ..copy(arr + 16, len) within buf_size else err_overflow,
                },
            };
        }
    }
    .map_err(hao_err)?;
    Ok(module)
}
