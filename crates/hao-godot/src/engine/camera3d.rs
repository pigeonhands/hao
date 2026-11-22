//! `Godot.Camera3D` — FOV, near/far, make current.

use hao_runtime::{Result, Runtime};

crate::bindings! {
    MAKE_CURRENT: "Godot.Camera3D" "MakeCurrent" 0;
    GET_CURRENT:  "Godot.Camera3D" "get_Current" 0;
    SET_CURRENT:  "Godot.Camera3D" "set_Current" 1;
    GET_FOV:      "Godot.Camera3D" "get_Fov"     0;
    SET_FOV:      "Godot.Camera3D" "set_Fov"     1;
    GET_NEAR:     "Godot.Camera3D" "get_Near"    0;
    SET_NEAR:     "Godot.Camera3D" "set_Near"    1;
    GET_FAR:      "Godot.Camera3D" "get_Far"     0;
    SET_FAR:      "Godot.Camera3D" "set_Far"     1;
}

pub fn make_current(rt: &Runtime, instance: usize) -> Result<()> {
    MAKE_CURRENT.call1(rt, instance as u64)?;
    Ok(())
}

pub fn get_current(rt: &Runtime, instance: usize) -> Result<bool> {
    Ok(GET_CURRENT.call1(rt, instance as u64)? != 0)
}

pub fn set_current(rt: &Runtime, instance: usize, current: bool) -> Result<()> {
    SET_CURRENT.call2(rt, instance as u64, current as u64)?;
    Ok(())
}

pub fn get_fov(rt: &Runtime, instance: usize) -> Result<f32> {
    Ok(f32::from_bits(GET_FOV.call1(rt, instance as u64)? as u32))
}

pub fn set_fov(rt: &Runtime, instance: usize, fov: f32) -> Result<()> {
    SET_FOV.call2(rt, instance as u64, fov.to_bits() as u64)?;
    Ok(())
}
