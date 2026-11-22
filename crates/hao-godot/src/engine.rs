//! Lazy-resolved bindings to GodotSharp engine types.
//!
//! Each binding resolves `GodotSharp` managed methods at first call via
//! [`crate::binding::Binding`] and caches the native pointer in a
//! [`OnceLock`]. This means:
//!
//! - **No build-time dependency** on `GodotSharp.dll` — the crate builds
//!   anywhere, and resolution happens at runtime when the game is running.
//! - **Thread-safe one-shot init** — the pointer is resolved once and
//!   reused for the lifetime of the process.
//! - **Overload-safe** — bindings specify a parameter count so methods
//!   like `Node.GetChild(int)` resolve correctly even when overloaded.

use std::ffi::c_void;

use hao_runtime::{Result, Runtime};

// ── Godot.Engine ───────────────────────────────────────────────────────

pub mod engine {
    use super::*;

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
}

// ── Godot.Time ─────────────────────────────────────────────────────────

pub mod time {
    use super::*;

    crate::bindings! {
        GET_TICKS_MSEC: "Godot.Time" "GetTicksMsec" 0;
        GET_TICKS_USEC: "Godot.Time" "GetTicksUsec" 0;
    }

    pub fn get_ticks_msec(rt: &Runtime) -> Result<u64> {
        GET_TICKS_MSEC.call0(rt)
    }

    pub fn get_ticks_usec(rt: &Runtime) -> Result<u64> {
        GET_TICKS_USEC.call0(rt)
    }
}

// ── Godot.OS ───────────────────────────────────────────────────────────

pub mod os {
    use super::*;

    crate::bindings! {
        GET_NAME:       "Godot.OS" "GetName"      0;
        GET_PROCESS_ID: "Godot.OS" "GetProcessId" 0;
    }

    pub fn get_name(rt: &Runtime) -> Result<*const c_void> {
        GET_NAME.call_ptr(rt, &[])
    }

    pub fn get_process_id(rt: &Runtime) -> Result<i64> {
        Ok(GET_PROCESS_ID.call0(rt)? as i64)
    }
}

// ── Godot.SceneTree ────────────────────────────────────────────────────

pub mod scene_tree {
    use super::*;

    crate::bindings! {
        GET_ROOT: "Godot.SceneTree" "GetRoot" 0;
        QUIT:     "Godot.SceneTree" "Quit"    0;
    }

    pub fn get_root(rt: &Runtime, instance: usize) -> Result<*const c_void> {
        GET_ROOT.call_ptr(rt, &[instance as u64])
    }

    pub fn quit(rt: &Runtime, instance: usize) -> Result<()> {
        QUIT.call1(rt, instance as u64)?;
        Ok(())
    }
}

// ── Godot.Node ─────────────────────────────────────────────────────────

pub mod node {
    use super::*;

    crate::bindings! {
        GET_NAME:        "Godot.Node" "get_Name"       0;
        GET_PARENT:      "Godot.Node" "GetParent"      0;
        GET_CHILD:       "Godot.Node" "GetChild"       1;
        GET_CHILD_COUNT: "Godot.Node" "GetChildCount"  0;
        ADD_CHILD:       "Godot.Node" "AddChild"       1;
        REMOVE_CHILD:    "Godot.Node" "RemoveChild"    1;
        QUEUE_FREE:      "Godot.Node" "QueueFree"      0;
    }

    pub fn get_name(rt: &Runtime, instance: usize) -> Result<*const c_void> {
        GET_NAME.call_ptr(rt, &[instance as u64])
    }

    pub fn get_parent(rt: &Runtime, instance: usize) -> Result<*const c_void> {
        GET_PARENT.call_ptr(rt, &[instance as u64])
    }

    pub fn get_child(rt: &Runtime, instance: usize, idx: i32) -> Result<*const c_void> {
        GET_CHILD.call_ptr(rt, &[instance as u64, idx as u64])
    }

    pub fn get_child_count(rt: &Runtime, instance: usize) -> Result<i32> {
        Ok(GET_CHILD_COUNT.call1(rt, instance as u64)? as i32)
    }

    pub fn add_child(rt: &Runtime, instance: usize, child: usize) -> Result<()> {
        ADD_CHILD.call2(rt, instance as u64, child as u64)?;
        Ok(())
    }

    pub fn remove_child(rt: &Runtime, instance: usize, child: usize) -> Result<()> {
        REMOVE_CHILD.call2(rt, instance as u64, child as u64)?;
        Ok(())
    }

    pub fn queue_free(rt: &Runtime, instance: usize) -> Result<()> {
        QUEUE_FREE.call1(rt, instance as u64)?;
        Ok(())
    }
}

// ── Godot.GodotObject ──────────────────────────────────────────────────

pub mod godot_object {
    use super::*;

    crate::bindings! {
        TO_STRING: "Godot.GodotObject" "ToString" 0;
    }

    pub fn to_string(rt: &Runtime, instance: usize) -> Result<*const c_void> {
        TO_STRING.call_ptr(rt, &[instance as u64])
    }
}
