//! `Godot.SceneTree` — root window, quit, current scene, pause.

use std::ffi::c_void;

use hao_runtime::{Result, Runtime};

crate::bindings! {
    GET_ROOT:          "Godot.SceneTree" "GetRoot"          0;
    QUIT:              "Godot.SceneTree" "Quit"             0;
    GET_CURRENT_SCENE: "Godot.SceneTree" "get_CurrentScene" 0;
    SET_CURRENT_SCENE: "Godot.SceneTree" "set_CurrentScene" 1;
    GET_PAUSED:        "Godot.SceneTree" "get_Paused"       0;
    SET_PAUSED:        "Godot.SceneTree" "set_Paused"       1;
    GET_NODE_COUNT:    "Godot.SceneTree" "GetNodeCount"     0;
}

pub fn get_root(rt: &Runtime, instance: usize) -> Result<*const c_void> {
    GET_ROOT.call_ptr(rt, &[instance as u64])
}

pub fn quit(rt: &Runtime, instance: usize) -> Result<()> {
    QUIT.call1(rt, instance as u64)?;
    Ok(())
}

pub fn get_current_scene(rt: &Runtime, instance: usize) -> Result<*const c_void> {
    GET_CURRENT_SCENE.call_ptr(rt, &[instance as u64])
}

pub fn set_current_scene(rt: &Runtime, instance: usize, scene: usize) -> Result<()> {
    SET_CURRENT_SCENE.call2(rt, instance as u64, scene as u64)?;
    Ok(())
}

pub fn get_paused(rt: &Runtime, instance: usize) -> Result<bool> {
    Ok(GET_PAUSED.call1(rt, instance as u64)? != 0)
}

pub fn set_paused(rt: &Runtime, instance: usize, paused: bool) -> Result<()> {
    SET_PAUSED.call2(rt, instance as u64, paused as u64)?;
    Ok(())
}

pub fn get_node_count(rt: &Runtime, instance: usize) -> Result<i32> {
    Ok(GET_NODE_COUNT.call1(rt, instance as u64)? as i32)
}
