//! `Godot.Node3D` — position, rotation, scale, visibility.

use hao_runtime::{Result, Runtime};

crate::bindings! {
    GET_POSITION:        "Godot.Node3D" "get_Position"       0;
    SET_POSITION:        "Godot.Node3D" "set_Position"       1;
    GET_ROTATION:        "Godot.Node3D" "get_Rotation"       0;
    SET_ROTATION:        "Godot.Node3D" "set_Rotation"       1;
    GET_SCALE:           "Godot.Node3D" "get_Scale"          0;
    SET_SCALE:           "Godot.Node3D" "set_Scale"          1;
    GET_GLOBAL_POSITION: "Godot.Node3D" "get_GlobalPosition" 0;
    SET_GLOBAL_POSITION: "Godot.Node3D" "set_GlobalPosition" 1;
    GET_VISIBLE:         "Godot.Node3D" "get_Visible"        0;
    SET_VISIBLE:         "Godot.Node3D" "set_Visible"        1;
}

pub fn get_position(rt: &Runtime, instance: usize) -> Result<u64> {
    GET_POSITION.call1(rt, instance as u64)
}

pub fn set_position(rt: &Runtime, instance: usize, pos: u64) -> Result<()> {
    SET_POSITION.call2(rt, instance as u64, pos)?;
    Ok(())
}

pub fn get_visible(rt: &Runtime, instance: usize) -> Result<bool> {
    Ok(GET_VISIBLE.call1(rt, instance as u64)? != 0)
}

pub fn set_visible(rt: &Runtime, instance: usize, visible: bool) -> Result<()> {
    SET_VISIBLE.call2(rt, instance as u64, visible as u64)?;
    Ok(())
}
