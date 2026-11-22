//! `Godot.Viewport` — mouse position, cameras, worlds.

use std::ffi::c_void;

use hao_runtime::{Result, Runtime};

crate::bindings! {
    GET_MOUSE_POSITION: "Godot.Viewport" "GetMousePosition" 0;
    GET_VISIBLE_RECT:   "Godot.Viewport" "GetVisibleRect"   0;
    GET_CAMERA_2D:      "Godot.Viewport" "GetCamera2D"      0;
    GET_CAMERA_3D:      "Godot.Viewport" "GetCamera3D"      0;
    GET_WORLD_2D:       "Godot.Viewport" "get_World2D"      0;
    GET_WORLD_3D:       "Godot.Viewport" "get_World3D"      0;
}

pub fn get_camera_2d(rt: &Runtime, instance: usize) -> Result<*const c_void> {
    GET_CAMERA_2D.call_ptr(rt, &[instance as u64])
}

pub fn get_camera_3d(rt: &Runtime, instance: usize) -> Result<*const c_void> {
    GET_CAMERA_3D.call_ptr(rt, &[instance as u64])
}

pub fn get_world_2d(rt: &Runtime, instance: usize) -> Result<*const c_void> {
    GET_WORLD_2D.call_ptr(rt, &[instance as u64])
}

pub fn get_world_3d(rt: &Runtime, instance: usize) -> Result<*const c_void> {
    GET_WORLD_3D.call_ptr(rt, &[instance as u64])
}
