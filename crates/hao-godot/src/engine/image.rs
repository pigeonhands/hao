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
