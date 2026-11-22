//! `Godot.Engine` — frame counts, main loop, editor hint.

use std::ffi::c_void;

use hao_runtime::{Result, Runtime};

crate::bindings! {
    GET_PROCESS_FRAMES:  "Godot.Engine" "GetProcessFrames"  0;
    GET_PHYSICS_FRAMES:  "Godot.Engine" "GetPhysicsFrames"  0;
    GET_MAIN_LOOP:       "Godot.Engine" "GetMainLoop"       0;
    IS_EDITOR_HINT:      "Godot.Engine" "IsEditorHint"      0;
}

pub fn get_process_frames(rt: &Runtime) -> Result<u64> {
    GET_PROCESS_FRAMES.call0(rt)
}

pub fn get_physics_frames(rt: &Runtime) -> Result<u64> {
    GET_PHYSICS_FRAMES.call0(rt)
}

pub fn get_main_loop(rt: &Runtime) -> Result<*const c_void> {
    GET_MAIN_LOOP.call_ptr(rt, &[])
}

pub fn is_editor_hint(rt: &Runtime) -> Result<bool> {
    Ok(IS_EDITOR_HINT.call0(rt)? != 0)
}
