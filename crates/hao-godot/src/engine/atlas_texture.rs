//! `Godot.AtlasTexture` — sub-region of a larger atlas texture.

use std::ffi::c_void;

use hao_runtime::{Result, Runtime};

crate::bindings! {
    GET_ATLAS:  "Godot.AtlasTexture" "get_Atlas"  0;
    GET_REGION: "Godot.AtlasTexture" "get_Region" 0;
}

pub fn resolve_all(rt: &Runtime) -> Result<()> {
    GET_ATLAS.resolve(rt)?;
    GET_REGION.resolve(rt)?;
    Ok(())
}

/// Returns the managed reference to the underlying atlas `Texture2D`.
pub fn get_atlas(rt: &Runtime, instance: usize) -> Result<*const c_void> {
    GET_ATLAS.call_ptr(rt, &[instance as u64])
}

/// Returns the atlas region as `[x, y, width, height]` floats.
pub fn get_region(rt: &Runtime, instance: usize) -> Result<[f32; 4]> {
    GET_REGION.call_f32x4(rt, &[instance as u64])
}
