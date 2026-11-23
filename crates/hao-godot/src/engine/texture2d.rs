//! `Godot.Texture2D` — image extraction.

use std::ffi::c_void;

use hao_runtime::{Result, Runtime};

crate::bindings! {
    GET_IMAGE: "Godot.Texture2D" "GetImage" 0;
    GET_WIDTH: "Godot.Texture2D" "GetWidth" 0;
    GET_HEIGHT: "Godot.Texture2D" "GetHeight" 0;
    GET_RID: "Godot.Resource" "GetRid" 0;
}

pub fn resolve_all(rt: &Runtime) -> Result<()> {
    GET_IMAGE.resolve(rt)?;
    GET_WIDTH.resolve(rt)?;
    GET_HEIGHT.resolve(rt)?;
    GET_RID.resolve(rt)?;
    Ok(())
}

pub fn get_image(rt: &Runtime, instance: usize) -> Result<*const c_void> {
    GET_IMAGE.call_ptr(rt, &[instance as u64])
}

pub fn get_width(rt: &Runtime, instance: usize) -> Result<u64> {
    GET_WIDTH.call1(rt, instance as u64)
}

pub fn get_height(rt: &Runtime, instance: usize) -> Result<u64> {
    GET_HEIGHT.call1(rt, instance as u64)
}

/// Returns the texture's RID (a Godot resource identifier, as a raw u64).
pub fn get_rid(rt: &Runtime, instance: usize) -> Result<u64> {
    GET_RID.call1(rt, instance as u64)
}
