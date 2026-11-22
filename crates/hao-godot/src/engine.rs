//! Lazy-resolved bindings to GodotSharp engine types.
//!
//! Each binding resolves `GodotSharp` managed methods at first call via
//! [`Runtime::method_pointer_from_loaded`] and caches the native pointer in
//! a [`OnceLock`]. This means:
//!
//! - **No build-time dependency** on `GodotSharp.dll` — the crate builds
//!   anywhere, and resolution happens at runtime when the game is running.
//! - **Thread-safe one-shot init** — the pointer is resolved once and
//!   reused for the lifetime of the process.
//!
//! All functions are `unsafe` wrappers around raw `call_native` calls.
//! The caller must ensure the Runtime has loaded GodotSharp and that the
//! game is in a state where these calls make sense.

use std::ffi::c_void;
use std::sync::OnceLock;

use hao_runtime::jit::call_native;
use hao_runtime::{Result, Runtime};

// ── helpers ────────────────────────────────────────────────────────────

fn resolve(
    runtime: &Runtime,
    cache: &OnceLock<usize>,
    type_name: &str,
    method: &str,
) -> Result<usize> {
    if let Some(&ptr) = cache.get() {
        return Ok(ptr);
    }
    let ptr = runtime
        .method_pointer_from_loaded("GodotSharp", type_name, method)
        .map(|p| p as usize)?;
    let _ = cache.set(ptr);
    Ok(ptr)
}

// ── Godot.Engine ───────────────────────────────────────────────────────

pub mod engine {
    use super::*;

    static GET_PROCESS_FRAMES: OnceLock<usize> = OnceLock::new();
    static GET_PHYSICS_FRAMES: OnceLock<usize> = OnceLock::new();
    static GET_MAIN_LOOP: OnceLock<usize> = OnceLock::new();
    static IS_EDITOR_HINT: OnceLock<usize> = OnceLock::new();

    pub fn get_process_frames(rt: &Runtime) -> Result<u64> {
        let ptr = resolve(rt, &GET_PROCESS_FRAMES, "Godot.Engine", "GetProcessFrames")?;
        Ok(unsafe { call_native(ptr, &[]) })
    }

    pub fn get_physics_frames(rt: &Runtime) -> Result<u64> {
        let ptr = resolve(rt, &GET_PHYSICS_FRAMES, "Godot.Engine", "GetPhysicsFrames")?;
        Ok(unsafe { call_native(ptr, &[]) })
    }

    /// Returns a raw pointer to the `MainLoop` object.
    pub fn get_main_loop(rt: &Runtime) -> Result<*const c_void> {
        let ptr = resolve(rt, &GET_MAIN_LOOP, "Godot.Engine", "GetMainLoop")?;
        Ok(unsafe { call_native(ptr, &[]) } as *const c_void)
    }

    pub fn is_editor_hint(rt: &Runtime) -> Result<bool> {
        let ptr = resolve(rt, &IS_EDITOR_HINT, "Godot.Engine", "IsEditorHint")?;
        Ok(unsafe { call_native(ptr, &[]) } != 0)
    }
}

// ── Godot.Time ─────────────────────────────────────────────────────────

pub mod time {
    use super::*;

    static GET_TICKS_MSEC: OnceLock<usize> = OnceLock::new();
    static GET_TICKS_USEC: OnceLock<usize> = OnceLock::new();

    pub fn get_ticks_msec(rt: &Runtime) -> Result<u64> {
        let ptr = resolve(rt, &GET_TICKS_MSEC, "Godot.Time", "GetTicksMsec")?;
        Ok(unsafe { call_native(ptr, &[]) })
    }

    pub fn get_ticks_usec(rt: &Runtime) -> Result<u64> {
        let ptr = resolve(rt, &GET_TICKS_USEC, "Godot.Time", "GetTicksUsec")?;
        Ok(unsafe { call_native(ptr, &[]) })
    }
}

// ── Godot.OS ───────────────────────────────────────────────────────────

pub mod os {
    use super::*;

    static GET_NAME: OnceLock<usize> = OnceLock::new();
    static GET_PROCESS_ID: OnceLock<usize> = OnceLock::new();

    /// Returns a raw pointer to the managed `string` object.
    pub fn get_name(rt: &Runtime) -> Result<*const c_void> {
        let ptr = resolve(rt, &GET_NAME, "Godot.OS", "GetName")?;
        Ok(unsafe { call_native(ptr, &[]) } as *const c_void)
    }

    pub fn get_process_id(rt: &Runtime) -> Result<i64> {
        let ptr = resolve(rt, &GET_PROCESS_ID, "Godot.OS", "GetProcessId")?;
        Ok(unsafe { call_native(ptr, &[]) } as i64)
    }
}

// ── Godot.SceneTree ────────────────────────────────────────────────────

pub mod scene_tree {
    use super::*;

    static GET_ROOT: OnceLock<usize> = OnceLock::new();
    static QUIT: OnceLock<usize> = OnceLock::new();

    /// Returns a raw pointer to the root `Window` node. Requires a
    /// `SceneTree` instance pointer as the first argument.
    pub fn get_root(rt: &Runtime, instance: usize) -> Result<*const c_void> {
        let ptr = resolve(rt, &GET_ROOT, "Godot.SceneTree", "GetRoot")?;
        Ok(unsafe { call_native(ptr, &[instance as u64]) } as *const c_void)
    }

    pub fn quit(rt: &Runtime, instance: usize) -> Result<()> {
        let ptr = resolve(rt, &QUIT, "Godot.SceneTree", "Quit")?;
        unsafe { call_native(ptr, &[instance as u64]) };
        Ok(())
    }
}

// ── Godot.Node ─────────────────────────────────────────────────────────

pub mod node {
    use super::*;

    static GET_NAME: OnceLock<usize> = OnceLock::new();
    static GET_PARENT: OnceLock<usize> = OnceLock::new();
    static GET_CHILD: OnceLock<usize> = OnceLock::new();
    static GET_CHILD_COUNT: OnceLock<usize> = OnceLock::new();
    static ADD_CHILD: OnceLock<usize> = OnceLock::new();
    static REMOVE_CHILD: OnceLock<usize> = OnceLock::new();
    static QUEUE_FREE: OnceLock<usize> = OnceLock::new();

    /// Returns a raw pointer to the node's name (`StringName`).
    pub fn get_name(rt: &Runtime, instance: usize) -> Result<*const c_void> {
        let ptr = resolve(rt, &GET_NAME, "Godot.Node", "GetName")?;
        Ok(unsafe { call_native(ptr, &[instance as u64]) } as *const c_void)
    }

    pub fn get_parent(rt: &Runtime, instance: usize) -> Result<*const c_void> {
        let ptr = resolve(rt, &GET_PARENT, "Godot.Node", "GetParent")?;
        Ok(unsafe { call_native(ptr, &[instance as u64]) } as *const c_void)
    }

    pub fn get_child(rt: &Runtime, instance: usize, idx: i32) -> Result<*const c_void> {
        let ptr = resolve(rt, &GET_CHILD, "Godot.Node", "GetChild")?;
        Ok(unsafe { call_native(ptr, &[instance as u64, idx as u64]) } as *const c_void)
    }

    pub fn get_child_count(rt: &Runtime, instance: usize) -> Result<i32> {
        let ptr = resolve(rt, &GET_CHILD_COUNT, "Godot.Node", "GetChildCount")?;
        Ok(unsafe { call_native(ptr, &[instance as u64]) } as i32)
    }

    pub fn add_child(rt: &Runtime, instance: usize, child: usize) -> Result<()> {
        let ptr = resolve(rt, &ADD_CHILD, "Godot.Node", "AddChild")?;
        unsafe { call_native(ptr, &[instance as u64, child as u64]) };
        Ok(())
    }

    pub fn remove_child(rt: &Runtime, instance: usize, child: usize) -> Result<()> {
        let ptr = resolve(rt, &REMOVE_CHILD, "Godot.Node", "RemoveChild")?;
        unsafe { call_native(ptr, &[instance as u64, child as u64]) };
        Ok(())
    }

    pub fn queue_free(rt: &Runtime, instance: usize) -> Result<()> {
        let ptr = resolve(rt, &QUEUE_FREE, "Godot.Node", "QueueFree")?;
        unsafe { call_native(ptr, &[instance as u64]) };
        Ok(())
    }
}
