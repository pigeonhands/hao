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
//!
//! # Modules
//!
//! | Module | GodotSharp type | Highlights |
//! |--------|----------------|------------|
//! | [`engine`] | `Godot.Engine` | Frame counts, main loop, editor hint |
//! | [`time`] | `Godot.Time` | Tick counters |
//! | [`os`] | `Godot.OS` | Platform name, process ID |
//! | [`scene_tree`] | `Godot.SceneTree` | Root, quit |
//! | [`scene_tree_ext`] | `Godot.SceneTree` | Current scene, pause, node count |
//! | [`node`] | `Godot.Node` | Name, parent, children, add/remove |
//! | [`node_ext`] | `Godot.Node` | Path, tree, find, process toggle |
//! | [`node2d`] | `Godot.Node2D` | Position, rotation, scale (local + global) |
//! | [`node3d`] | `Godot.Node3D` | Position, rotation, scale, visibility |
//! | [`control`] | `Godot.Control` | UI position, size, visibility, focus |
//! | [`canvas_item`] | `Godot.CanvasItem` | 2D visibility, show/hide, modulate |
//! | [`camera2d`] | `Godot.Camera2D` | Zoom, offset, make current |
//! | [`camera3d`] | `Godot.Camera3D` | FOV, near/far, make current |
//! | [`viewport`] | `Godot.Viewport` | Mouse position, cameras, worlds |
//! | [`input`] | `Godot.Input` | Actions, keys, mouse, mouse mode |
//! | [`godot_object`] | `Godot.GodotObject` | ToString, class, instance ID, meta |
//! | [`gd`] | `Godot.GD` | Print, push error/warning |
//! | [`display_server`] | `Godot.DisplayServer` | Screen/window size and position |
//! | [`performance`] | `Godot.Performance` | Monitor values (FPS, memory, …) |
//! | [`resource_loader`] | `Godot.ResourceLoader` | Load/exists |
//! | [`window`] | `Godot.Window` | Title, size |
//! | [`timer`] | `Godot.Timer` | Start/stop, wait time, one-shot |
//! | [`animation_player`] | `Godot.AnimationPlayer` | Play/stop, speed scale |

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

// ── Godot.SceneTree (extras) ───────────────────────────────────────────

pub mod scene_tree_ext {
    use super::*;

    crate::bindings! {
        GET_CURRENT_SCENE: "Godot.SceneTree" "get_CurrentScene" 0;
        SET_CURRENT_SCENE: "Godot.SceneTree" "set_CurrentScene" 1;
        GET_PAUSED:        "Godot.SceneTree" "get_Paused"       0;
        SET_PAUSED:        "Godot.SceneTree" "set_Paused"       1;
        GET_NODE_COUNT:    "Godot.SceneTree" "GetNodeCount"     0;
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

// ── Godot.Node (extras) ───────────────────────────────────────────────

pub mod node_ext {
    use super::*;

    crate::bindings! {
        GET_PATH:         "Godot.Node" "GetPath"        0;
        GET_TREE:         "Godot.Node" "GetTree"        0;
        IS_INSIDE_TREE:   "Godot.Node" "IsInsideTree"   0;
        GET_INDEX:        "Godot.Node" "GetIndex"       0;
        FIND_CHILD:       "Godot.Node" "FindChild"      1;
        GET_NODE_OR_NULL: "Godot.Node" "GetNodeOrNull"  1;
        SET_PROCESS:      "Godot.Node" "SetProcess"     1;
        SET_NAME:         "Godot.Node" "set_Name"       1;
        GET_OWNER:        "Godot.Node" "get_Owner"      0;
        IS_PROCESSING:    "Godot.Node" "IsProcessing"   0;
        PRINT_TREE:       "Godot.Node" "PrintTree"      0;
        PRINT_TREE_PRETTY: "Godot.Node" "PrintTreePretty" 0;
    }

    pub fn get_path(rt: &Runtime, instance: usize) -> Result<*const c_void> {
        GET_PATH.call_ptr(rt, &[instance as u64])
    }

    pub fn get_tree(rt: &Runtime, instance: usize) -> Result<*const c_void> {
        GET_TREE.call_ptr(rt, &[instance as u64])
    }

    pub fn is_inside_tree(rt: &Runtime, instance: usize) -> Result<bool> {
        Ok(IS_INSIDE_TREE.call1(rt, instance as u64)? != 0)
    }

    pub fn get_index(rt: &Runtime, instance: usize) -> Result<i32> {
        Ok(GET_INDEX.call1(rt, instance as u64)? as i32)
    }

    pub fn find_child(rt: &Runtime, instance: usize, pattern: usize) -> Result<*const c_void> {
        FIND_CHILD.call_ptr(rt, &[instance as u64, pattern as u64])
    }

    pub fn get_node_or_null(rt: &Runtime, instance: usize, path: usize) -> Result<*const c_void> {
        GET_NODE_OR_NULL.call_ptr(rt, &[instance as u64, path as u64])
    }

    pub fn set_process(rt: &Runtime, instance: usize, enable: bool) -> Result<()> {
        SET_PROCESS.call2(rt, instance as u64, enable as u64)?;
        Ok(())
    }

    pub fn print_tree_pretty(rt: &Runtime, instance: usize) -> Result<()> {
        PRINT_TREE_PRETTY.call1(rt, instance as u64)?;
        Ok(())
    }
}

// ── Godot.GodotObject ──────────────────────────────────────────────────

pub mod godot_object {
    use super::*;

    crate::bindings! {
        TO_STRING:      "Godot.GodotObject" "ToString"      0;
        GET_CLASS:      "Godot.GodotObject" "GetClass"      0;
        GET_INSTANCE_ID: "Godot.GodotObject" "GetInstanceId" 0;
        IS_CLASS:       "Godot.GodotObject" "IsClass"       1;
        SET_META:       "Godot.GodotObject" "SetMeta"       2;
        GET_META:       "Godot.GodotObject" "GetMeta"       1;
        HAS_META:       "Godot.GodotObject" "HasMeta"       1;
    }

    pub fn to_string(rt: &Runtime, instance: usize) -> Result<*const c_void> {
        TO_STRING.call_ptr(rt, &[instance as u64])
    }

    pub fn get_class(rt: &Runtime, instance: usize) -> Result<*const c_void> {
        GET_CLASS.call_ptr(rt, &[instance as u64])
    }

    pub fn get_instance_id(rt: &Runtime, instance: usize) -> Result<u64> {
        GET_INSTANCE_ID.call1(rt, instance as u64)
    }

    pub fn is_class(rt: &Runtime, instance: usize, class_name: usize) -> Result<bool> {
        Ok(IS_CLASS.call2(rt, instance as u64, class_name as u64)? != 0)
    }
}

// ── Godot.Node2D ──────────────────────────────────────────────────────

pub mod node2d {
    use super::*;

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
}

// ── Godot.Node3D ──────────────────────────────────────────────────────

pub mod node3d {
    use super::*;

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
}

// ── Godot.Control ─────────────────────────────────────────────────────

pub mod control {
    use super::*;

    crate::bindings! {
        GET_POSITION:   "Godot.Control" "get_Position"   0;
        SET_POSITION:   "Godot.Control" "set_Position"   1;
        GET_SIZE:       "Godot.Control" "get_Size"       0;
        SET_SIZE:       "Godot.Control" "set_Size"       1;
        GET_VISIBLE:    "Godot.Control" "get_Visible"    0;
        SET_VISIBLE:    "Godot.Control" "set_Visible"    1;
        GET_CUSTOM_MINIMUM_SIZE: "Godot.Control" "get_CustomMinimumSize" 0;
        SET_CUSTOM_MINIMUM_SIZE: "Godot.Control" "set_CustomMinimumSize" 1;
        GET_TOOLTIP_TEXT: "Godot.Control" "get_TooltipText" 0;
        SET_TOOLTIP_TEXT: "Godot.Control" "set_TooltipText" 1;
        IS_VISIBLE_IN_TREE: "Godot.Control" "IsVisibleInTree" 0;
        GRAB_FOCUS:     "Godot.Control" "GrabFocus"      0;
        HAS_FOCUS:      "Godot.Control" "HasFocus"       0;
    }

    pub fn get_visible(rt: &Runtime, instance: usize) -> Result<bool> {
        Ok(GET_VISIBLE.call1(rt, instance as u64)? != 0)
    }

    pub fn set_visible(rt: &Runtime, instance: usize, visible: bool) -> Result<()> {
        SET_VISIBLE.call2(rt, instance as u64, visible as u64)?;
        Ok(())
    }

    pub fn is_visible_in_tree(rt: &Runtime, instance: usize) -> Result<bool> {
        Ok(IS_VISIBLE_IN_TREE.call1(rt, instance as u64)? != 0)
    }

    pub fn grab_focus(rt: &Runtime, instance: usize) -> Result<()> {
        GRAB_FOCUS.call1(rt, instance as u64)?;
        Ok(())
    }

    pub fn has_focus(rt: &Runtime, instance: usize) -> Result<bool> {
        Ok(HAS_FOCUS.call1(rt, instance as u64)? != 0)
    }
}

// ── Godot.CanvasItem ─────────────────────────────────────────────────

pub mod canvas_item {
    use super::*;

    crate::bindings! {
        GET_VISIBLE:  "Godot.CanvasItem" "get_Visible"  0;
        SET_VISIBLE:  "Godot.CanvasItem" "set_Visible"  1;
        SHOW:         "Godot.CanvasItem" "Show"          0;
        HIDE:         "Godot.CanvasItem" "Hide"          0;
        GET_MODULATE: "Godot.CanvasItem" "get_Modulate"  0;
        SET_MODULATE: "Godot.CanvasItem" "set_Modulate"  1;
        IS_VISIBLE_IN_TREE: "Godot.CanvasItem" "IsVisibleInTree" 0;
    }

    pub fn get_visible(rt: &Runtime, instance: usize) -> Result<bool> {
        Ok(GET_VISIBLE.call1(rt, instance as u64)? != 0)
    }

    pub fn set_visible(rt: &Runtime, instance: usize, visible: bool) -> Result<()> {
        SET_VISIBLE.call2(rt, instance as u64, visible as u64)?;
        Ok(())
    }

    pub fn show(rt: &Runtime, instance: usize) -> Result<()> {
        SHOW.call1(rt, instance as u64)?;
        Ok(())
    }

    pub fn hide(rt: &Runtime, instance: usize) -> Result<()> {
        HIDE.call1(rt, instance as u64)?;
        Ok(())
    }

    pub fn is_visible_in_tree(rt: &Runtime, instance: usize) -> Result<bool> {
        Ok(IS_VISIBLE_IN_TREE.call1(rt, instance as u64)? != 0)
    }
}

// ── Godot.Camera2D ───────────────────────────────────────────────────

pub mod camera2d {
    use super::*;

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
}

// ── Godot.Camera3D ───────────────────────────────────────────────────

pub mod camera3d {
    use super::*;

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
}

// ── Godot.Viewport ───────────────────────────────────────────────────

pub mod viewport {
    use super::*;

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
}

// ── Godot.Input ───────────────────────────────────────────────────────

pub mod input {
    use super::*;

    crate::bindings! {
        IS_ACTION_PRESSED:      "Godot.Input" "IsActionPressed"      1;
        IS_ACTION_JUST_PRESSED: "Godot.Input" "IsActionJustPressed"  1;
        IS_ACTION_JUST_RELEASED: "Godot.Input" "IsActionJustReleased" 1;
        IS_KEY_PRESSED:         "Godot.Input" "IsKeyPressed"         1;
        IS_MOUSE_BUTTON_PRESSED: "Godot.Input" "IsMouseButtonPressed" 1;
        GET_MOUSE_POSITION:     "Godot.Input" "GetGlobalMousePosition" 0;
        SET_MOUSE_MODE:         "Godot.Input" "set_MouseMode"        1;
        GET_MOUSE_MODE:         "Godot.Input" "get_MouseMode"        0;
    }

    pub fn is_action_pressed(rt: &Runtime, action: usize) -> Result<bool> {
        Ok(IS_ACTION_PRESSED.call1(rt, action as u64)? != 0)
    }

    pub fn is_action_just_pressed(rt: &Runtime, action: usize) -> Result<bool> {
        Ok(IS_ACTION_JUST_PRESSED.call1(rt, action as u64)? != 0)
    }

    pub fn is_key_pressed(rt: &Runtime, keycode: u64) -> Result<bool> {
        Ok(IS_KEY_PRESSED.call1(rt, keycode)? != 0)
    }

    pub fn is_mouse_button_pressed(rt: &Runtime, button: u64) -> Result<bool> {
        Ok(IS_MOUSE_BUTTON_PRESSED.call1(rt, button)? != 0)
    }
}

// ── Godot.GD ─────────────────────────────────────────────────────────

pub mod gd {
    use super::*;

    crate::bindings! {
        PRINT:        "Godot.GD" "Print"       1;
        PRINT_ERR:    "Godot.GD" "PrintErr"    1;
        PUSH_ERROR:   "Godot.GD" "PushError"   1;
        PUSH_WARNING: "Godot.GD" "PushWarning" 1;
    }

    pub fn print(rt: &Runtime, args: usize) -> Result<()> {
        PRINT.call1(rt, args as u64)?;
        Ok(())
    }

    pub fn print_err(rt: &Runtime, args: usize) -> Result<()> {
        PRINT_ERR.call1(rt, args as u64)?;
        Ok(())
    }

    pub fn push_error(rt: &Runtime, msg: usize) -> Result<()> {
        PUSH_ERROR.call1(rt, msg as u64)?;
        Ok(())
    }

    pub fn push_warning(rt: &Runtime, msg: usize) -> Result<()> {
        PUSH_WARNING.call1(rt, msg as u64)?;
        Ok(())
    }
}

// ── Godot.DisplayServer ──────────────────────────────────────────────

pub mod display_server {
    use super::*;

    crate::bindings! {
        SCREEN_GET_SIZE:     "Godot.DisplayServer" "ScreenGetSize"     0;
        WINDOW_GET_SIZE:     "Godot.DisplayServer" "WindowGetSize"     0;
        WINDOW_SET_SIZE:     "Godot.DisplayServer" "WindowSetSize"     1;
        WINDOW_GET_POSITION: "Godot.DisplayServer" "WindowGetPosition" 0;
        WINDOW_SET_POSITION: "Godot.DisplayServer" "WindowSetPosition" 1;
        GET_SCREEN_COUNT:    "Godot.DisplayServer" "GetScreenCount"    0;
    }

    pub fn get_screen_count(rt: &Runtime) -> Result<i32> {
        Ok(GET_SCREEN_COUNT.call0(rt)? as i32)
    }
}

// ── Godot.Performance ────────────────────────────────────────────────

pub mod performance {
    use super::*;

    crate::bindings! {
        GET_MONITOR: "Godot.Performance" "GetMonitor" 1;
    }

    pub fn get_monitor(rt: &Runtime, monitor: u64) -> Result<f64> {
        Ok(f64::from_bits(GET_MONITOR.call1(rt, monitor)?))
    }
}

// ── Godot.ResourceLoader ──────────────────────────────────────────────

pub mod resource_loader {
    use super::*;

    crate::bindings! {
        LOAD:   "Godot.ResourceLoader" "Load"   1;
        EXISTS: "Godot.ResourceLoader" "Exists" 1;
    }

    pub fn load(rt: &Runtime, path: usize) -> Result<*const c_void> {
        LOAD.call_ptr(rt, &[path as u64])
    }

    pub fn exists(rt: &Runtime, path: usize) -> Result<bool> {
        Ok(EXISTS.call1(rt, path as u64)? != 0)
    }
}

// ── Godot.Window ──────────────────────────────────────────────────────

pub mod window {
    use super::*;

    crate::bindings! {
        GET_TITLE:  "Godot.Window" "get_Title" 0;
        SET_TITLE:  "Godot.Window" "set_Title" 1;
        GET_SIZE:   "Godot.Window" "get_Size"  0;
        SET_SIZE:   "Godot.Window" "set_Size"  1;
    }

    pub fn get_title(rt: &Runtime, instance: usize) -> Result<*const c_void> {
        GET_TITLE.call_ptr(rt, &[instance as u64])
    }

    pub fn set_title(rt: &Runtime, instance: usize, title: usize) -> Result<()> {
        SET_TITLE.call2(rt, instance as u64, title as u64)?;
        Ok(())
    }
}

// ── Godot.Timer ──────────────────────────────────────────────────────

pub mod timer {
    use super::*;

    crate::bindings! {
        START:          "Godot.Timer" "Start"          0;
        STOP:           "Godot.Timer" "Stop"           0;
        IS_STOPPED:     "Godot.Timer" "IsStopped"      0;
        GET_WAIT_TIME:  "Godot.Timer" "get_WaitTime"   0;
        SET_WAIT_TIME:  "Godot.Timer" "set_WaitTime"   1;
        GET_TIME_LEFT:  "Godot.Timer" "get_TimeLeft"   0;
        GET_ONE_SHOT:   "Godot.Timer" "get_OneShot"    0;
        SET_ONE_SHOT:   "Godot.Timer" "set_OneShot"    1;
        GET_AUTOSTART:  "Godot.Timer" "get_Autostart"  0;
        SET_AUTOSTART:  "Godot.Timer" "set_Autostart"  1;
    }

    pub fn start(rt: &Runtime, instance: usize) -> Result<()> {
        START.call1(rt, instance as u64)?;
        Ok(())
    }

    pub fn stop(rt: &Runtime, instance: usize) -> Result<()> {
        STOP.call1(rt, instance as u64)?;
        Ok(())
    }

    pub fn is_stopped(rt: &Runtime, instance: usize) -> Result<bool> {
        Ok(IS_STOPPED.call1(rt, instance as u64)? != 0)
    }

    pub fn get_time_left(rt: &Runtime, instance: usize) -> Result<f64> {
        Ok(f64::from_bits(GET_TIME_LEFT.call1(rt, instance as u64)?))
    }
}

// ── Godot.AnimationPlayer ────────────────────────────────────────────

pub mod animation_player {
    use super::*;

    crate::bindings! {
        PLAY:           "Godot.AnimationPlayer" "Play"           0;
        STOP:           "Godot.AnimationPlayer" "Stop"           0;
        IS_PLAYING:     "Godot.AnimationPlayer" "IsPlaying"      0;
        GET_CURRENT:    "Godot.AnimationPlayer" "get_CurrentAnimation" 0;
        SET_CURRENT:    "Godot.AnimationPlayer" "set_CurrentAnimation" 1;
        GET_SPEED_SCALE: "Godot.AnimationPlayer" "get_SpeedScale" 0;
        SET_SPEED_SCALE: "Godot.AnimationPlayer" "set_SpeedScale" 1;
    }

    pub fn play(rt: &Runtime, instance: usize) -> Result<()> {
        PLAY.call1(rt, instance as u64)?;
        Ok(())
    }

    pub fn stop(rt: &Runtime, instance: usize) -> Result<()> {
        STOP.call1(rt, instance as u64)?;
        Ok(())
    }

    pub fn is_playing(rt: &Runtime, instance: usize) -> Result<bool> {
        Ok(IS_PLAYING.call1(rt, instance as u64)? != 0)
    }

    pub fn get_current_animation(rt: &Runtime, instance: usize) -> Result<*const c_void> {
        GET_CURRENT.call_ptr(rt, &[instance as u64])
    }

    pub fn get_speed_scale(rt: &Runtime, instance: usize) -> Result<f32> {
        Ok(f32::from_bits(GET_SPEED_SCALE.call1(rt, instance as u64)? as u32))
    }

    pub fn set_speed_scale(rt: &Runtime, instance: usize, scale: f32) -> Result<()> {
        SET_SPEED_SCALE.call2(rt, instance as u64, scale.to_bits() as u64)?;
        Ok(())
    }
}
