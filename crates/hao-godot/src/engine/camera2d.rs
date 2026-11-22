//! `Godot.Camera2D` — zoom, offset, make current.

use hao_runtime::{Result, Runtime};

crate::bindings! {
    MAKE_CURRENT: "Godot.Camera2D" "MakeCurrent" 0;
    IS_CURRENT:   "Godot.Camera2D" "IsCurrent"   0;
    GET_ZOOM:     "Godot.Camera2D" "get_Zoom"    0;
    SET_ZOOM:     "Godot.Camera2D" "set_Zoom"    1;
    GET_OFFSET:   "Godot.Camera2D" "get_Offset"  0;
    SET_OFFSET:   "Godot.Camera2D" "set_Offset"  1;
}

pub fn make_current(rt: &Runtime, instance: usize) -> Result<()> {
    MAKE_CURRENT.call1(rt, instance as u64)?;
    Ok(())
}

pub fn is_current(rt: &Runtime, instance: usize) -> Result<bool> {
    Ok(IS_CURRENT.call1(rt, instance as u64)? != 0)
}

pub fn get_zoom(rt: &Runtime, instance: usize) -> Result<u64> {
    GET_ZOOM.call1(rt, instance as u64)
}

pub fn set_zoom(rt: &Runtime, instance: usize, zoom: u64) -> Result<()> {
    SET_ZOOM.call2(rt, instance as u64, zoom)?;
    Ok(())
}

pub fn get_offset(rt: &Runtime, instance: usize) -> Result<u64> {
    GET_OFFSET.call1(rt, instance as u64)
}

pub fn set_offset(rt: &Runtime, instance: usize, offset: u64) -> Result<()> {
    SET_OFFSET.call2(rt, instance as u64, offset)?;
    Ok(())
}
