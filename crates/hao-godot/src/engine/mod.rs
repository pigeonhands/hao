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
//! # Usage
//!
//! ```ignore
//! use hao_godot::engine::{engine, scene_tree, node};
//!
//! // Get the scene tree (Engine.GetMainLoop returns the SceneTree).
//! let tree = engine::get_main_loop(rt)?;
//! let root = scene_tree::get_root(rt, tree as usize)?;
//!
//! // Walk children of the root node.
//! let count = node::get_child_count(rt, root as usize)?;
//! for i in 0..count {
//!     let child = node::get_child(rt, root as usize, i)?;
//!     // child is a *const c_void pointing to the managed Node.
//! }
//! ```
//!
//! # Modules
//!
//! | Module | GodotSharp type | Highlights |
//! |--------|----------------|------------|
//! | [`engine`] | `Godot.Engine` | Frame counts, main loop, editor hint |
//! | [`time`] | `Godot.Time` | Tick counters |
//! | [`os`] | `Godot.OS` | Platform name, process ID |
//! | [`gd`] | `Godot.GD` | Print, push error/warning |
//! | [`performance`] | `Godot.Performance` | Monitor values (FPS, memory, …) |
//! | [`display_server`] | `Godot.DisplayServer` | Screen/window size and position |
//! | [`scene_tree`] | `Godot.SceneTree` | Root, quit, current scene, pause |
//! | [`node`] | `Godot.Node` | Name, parent, children, path, find |
//! | [`godot_object`] | `Godot.GodotObject` | ToString, class, instance ID, meta |
//! | [`node2d`] | `Godot.Node2D` | Position, rotation, scale (local + global) |
//! | [`node3d`] | `Godot.Node3D` | Position, rotation, scale, visibility |
//! | [`canvas_item`] | `Godot.CanvasItem` | 2D visibility, show/hide, modulate |
//! | [`control`] | `Godot.Control` | UI position, size, visibility, focus |
//! | [`window`] | `Godot.Window` | Title, size |
//! | [`camera2d`] | `Godot.Camera2D` | Zoom, offset, make current |
//! | [`camera3d`] | `Godot.Camera3D` | FOV, near/far, make current |
//! | [`viewport`] | `Godot.Viewport` | Mouse position, cameras, worlds |
//! | [`input`] | `Godot.Input` | Actions, keys, mouse, mouse mode |
//! | [`resource_loader`] | `Godot.ResourceLoader` | Load/exists |
//! | [`timer`] | `Godot.Timer` | Start/stop, wait time, one-shot |
//! | [`animation_player`] | `Godot.AnimationPlayer` | Play/stop, speed scale |

pub mod animation_player;
pub mod camera2d;
pub mod camera3d;
pub mod canvas_item;
pub mod control;
pub mod display_server;
pub mod engine;
pub mod gd;
pub mod godot_object;
pub mod input;
pub mod node;
pub mod node2d;
pub mod node3d;
pub mod os;
pub mod performance;
pub mod resource_loader;
pub mod scene_tree;
pub mod time;
pub mod timer;
pub mod viewport;
pub mod window;
