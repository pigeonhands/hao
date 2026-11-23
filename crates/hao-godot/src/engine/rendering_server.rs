//! `Godot.RenderingServer` — GPU readback and texture queries.

use std::ffi::c_void;

use hao_runtime::{Result, Runtime};

crate::bindings! {
    TEXTURE_2D_GET: "Godot.RenderingServer" "Texture2DGet" 1;
}

pub fn resolve_all(rt: &Runtime) -> Result<()> {
    TEXTURE_2D_GET.resolve(rt)?;
    Ok(())
}

/// GPU readback: returns a managed `Image` from the texture's RID.
///
/// `Texture2DGet` is a static method that internally fetches the
/// RenderingServer singleton pointer, so only the RID is passed.
pub fn texture_2d_get(rt: &Runtime, rid: u64) -> Result<*const c_void> {
    TEXTURE_2D_GET.call_ptr(rt, &[rid])
}
