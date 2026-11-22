//! `Godot.Node2D` — position, rotation, scale (local + global).

use hao_runtime::{Result, Runtime};

crate::bindings! {
    GET_POSITION:        "Godot.Node2D" "get_Position"       0;
    SET_POSITION:        "Godot.Node2D" "set_Position"       1;
    GET_ROTATION:        "Godot.Node2D" "get_Rotation"       0;
    SET_ROTATION:        "Godot.Node2D" "set_Rotation"       1;
    GET_SCALE:           "Godot.Node2D" "get_Scale"          0;
    SET_SCALE:           "Godot.Node2D" "set_Scale"          1;
    GET_GLOBAL_POSITION: "Godot.Node2D" "get_GlobalPosition" 0;
    SET_GLOBAL_POSITION: "Godot.Node2D" "set_GlobalPosition" 1;
    GET_GLOBAL_ROTATION: "Godot.Node2D" "get_GlobalRotation" 0;
    SET_GLOBAL_ROTATION: "Godot.Node2D" "set_GlobalRotation" 1;
}

pub fn get_position(rt: &Runtime, instance: usize) -> Result<u64> {
    GET_POSITION.call1(rt, instance as u64)
}

pub fn set_position(rt: &Runtime, instance: usize, pos: u64) -> Result<()> {
    SET_POSITION.call2(rt, instance as u64, pos)?;
    Ok(())
}

pub fn get_rotation(rt: &Runtime, instance: usize) -> Result<f32> {
    Ok(f32::from_bits(GET_ROTATION.call1(rt, instance as u64)? as u32))
}

pub fn set_rotation(rt: &Runtime, instance: usize, radians: f32) -> Result<()> {
    SET_ROTATION.call2(rt, instance as u64, radians.to_bits() as u64)?;
    Ok(())
}

pub fn get_global_position(rt: &Runtime, instance: usize) -> Result<u64> {
    GET_GLOBAL_POSITION.call1(rt, instance as u64)
}

pub fn set_global_position(rt: &Runtime, instance: usize, pos: u64) -> Result<()> {
    SET_GLOBAL_POSITION.call2(rt, instance as u64, pos)?;
    Ok(())
}
