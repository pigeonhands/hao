//! `Godot.Image` — format conversion, decompression, pixel data.

use std::ffi::c_void;

use hao_runtime::{Result, Runtime};

crate::bindings! {
    DECOMPRESS:  "Godot.Image" "Decompress" 0;
    CONVERT:     "Godot.Image" "Convert"    1;
    GET_DATA:    "Godot.Image" "GetData"    0;
    GET_FORMAT:  "Godot.Image" "GetFormat"  0;
    GET_WIDTH:   "Godot.Image" "GetWidth"   0;
    GET_HEIGHT:  "Godot.Image" "GetHeight"  0;
    IS_EMPTY:    "Godot.Image" "IsEmpty"    0;
}

/// Godot Image.Format enum values.
pub mod format {
    pub const RGBA8: i64 = 5;
}

pub fn decompress(rt: &Runtime, instance: usize) -> Result<()> {
    let err = DECOMPRESS.call1(rt, instance as u64)?;
    if err != 0 {
        return Err(hao_runtime::Error::Other(
            format!("Image.Decompress failed (godot error {err})")
        ));
    }
    Ok(())
}

pub fn convert(rt: &Runtime, instance: usize, format: i64) -> Result<()> {
    let err = CONVERT.call2(rt, instance as u64, format as u64)?;
    if err != 0 {
        return Err(hao_runtime::Error::Other(
            format!("Image.Convert failed (godot error {err})")
        ));
    }
    Ok(())
}

pub fn get_format(rt: &Runtime, instance: usize) -> Result<u64> {
    GET_FORMAT.call1(rt, instance as u64)
}

pub fn get_width(rt: &Runtime, instance: usize) -> Result<u64> {
    GET_WIDTH.call1(rt, instance as u64)
}

pub fn get_height(rt: &Runtime, instance: usize) -> Result<u64> {
    GET_HEIGHT.call1(rt, instance as u64)
}

pub fn is_empty(rt: &Runtime, instance: usize) -> Result<u64> {
    IS_EMPTY.call1(rt, instance as u64)
}

/// Returns a pointer to the managed `byte[]` from `Image.GetData()`.
pub fn get_data(rt: &Runtime, instance: usize) -> Result<*const c_void> {
    GET_DATA.call_ptr(rt, &[instance as u64])
}

/// Pre-resolve all bindings so they can be called from any thread.
pub fn resolve_all(rt: &Runtime) -> Result<()> {
    DECOMPRESS.resolve(rt)?;
    CONVERT.resolve(rt)?;
    GET_DATA.resolve(rt)?;
    GET_FORMAT.resolve(rt)?;
    GET_WIDTH.resolve(rt)?;
    GET_HEIGHT.resolve(rt)?;
    IS_EMPTY.resolve(rt)?;
    Ok(())
}

/// Read the managed `byte[]` returned by `Image.GetData()` into a `Vec<u8>`.
///
/// # Safety
/// `arr` must be a valid pointer to a .NET `byte[]` on the managed heap.
unsafe fn read_managed_byte_array(arr: *const c_void) -> Result<Vec<u8>> {
    // .NET byte[] layout: [MethodTable*:8][Length:4][pad:4][data...]
    let len = *(arr as *const u8).add(8).cast::<i32>() as usize;
    let data_ptr = (arr as *const u8).add(16);
    Ok(std::slice::from_raw_parts(data_ptr, len).to_vec())
}

/// Extract RGBA8 pixel data from a Godot texture by RID, using a compound
/// managed call that keeps the thread in cooperative GC mode throughout.
///
/// This avoids the GC compaction bug where managed Image pointers are
/// invalidated between individual `rt.invoke()` calls.
pub fn extract_rgba_compound(rt: &Runtime, rid: u64) -> Result<(u32, u32, Vec<u8>)> {
    let fn_tex2d_get = super::rendering_server::TEXTURE_2D_GET.resolve(rt)?;
    let fn_decompress = DECOMPRESS.resolve(rt)?;
    let fn_convert = CONVERT.resolve(rt)?;
    let fn_get_data = GET_DATA.resolve(rt)?;
    let fn_get_width = GET_WIDTH.resolve(rt)?;
    let fn_get_height = GET_HEIGHT.resolve(rt)?;

    let args: [u64; 8] = [
        fn_tex2d_get as u64,
        fn_decompress as u64,
        fn_convert as u64,
        fn_get_data as u64,
        fn_get_width as u64,
        fn_get_height as u64,
        rid,
        format::RGBA8 as u64,
    ];

    // 32-byte header + up to 16 MiB of pixel data.
    let buf_size = 32 + 16 * 1024 * 1024;
    let mut buf = vec![0u8; buf_size];

    super::image_interop::extract_image_rgba(rt, &args, &mut buf)?;

    let error = i64::from_le_bytes(buf[0..8].try_into().unwrap());
    if error != 0 {
        return Err(hao_runtime::Error::Other(format!(
            "ExtractImageRgba returned error {error}"
        )));
    }
    let w = i64::from_le_bytes(buf[8..16].try_into().unwrap()) as u32;
    let h = i64::from_le_bytes(buf[16..24].try_into().unwrap()) as u32;
    let data_len = i64::from_le_bytes(buf[24..32].try_into().unwrap()) as usize;

    if w == 0 || h == 0 {
        return Err(hao_runtime::Error::Other("image has zero dimensions".into()));
    }

    let data = buf[32..32 + data_len].to_vec();
    Ok((w, h, data))
}

/// Extract RGBA8 pixel data from a Godot Image.
///
/// Decompresses (if compressed), converts to RGBA8, and reads the raw bytes.
/// Returns `(width, height, rgba_pixels)` or an error.
pub fn extract_rgba(rt: &Runtime, instance: usize) -> Result<(u32, u32, Vec<u8>)> {
    let fmt = get_format(rt, instance)?;
    let w = get_width(rt, instance)? as u32;
    let h = get_height(rt, instance)? as u32;
    if fmt != format::RGBA8 as u64 {
        decompress(rt, instance)?;
        let fmt_after = get_format(rt, instance)?;
        if fmt_after != format::RGBA8 as u64 {
            convert(rt, instance, format::RGBA8)?;
        }
    }
    if w == 0 || h == 0 {
        return Err(hao_runtime::Error::Other("image has zero dimensions".into()));
    }

    let arr = get_data(rt, instance)?;
    if arr.is_null() {
        return Err(hao_runtime::Error::Other("GetData returned null".into()));
    }

    let expected = (w as usize) * (h as usize) * 4;
    let data = unsafe { read_managed_byte_array(arr)? };
    if data.len() != expected {
        return Err(hao_runtime::Error::Other(format!(
            "data length mismatch: got {} expected {}",
            data.len(),
            expected
        )));
    }

    Ok((w, h, data))
}
