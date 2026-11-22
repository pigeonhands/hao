//! Scene tree access — convenience wrappers over [`crate::engine`]
//! bindings for navigating Godot's node hierarchy.
//!
//! All functions require [`crate::set_runtime`] to have been called.

use std::ffi::c_void;

use hao_runtime::Result;

use crate::engine;

/// Returns a pointer to the game's `SceneTree` (via `Engine.GetMainLoop`).
pub fn scene_tree() -> Result<*const c_void> {
    let rt = crate::runtime()?;
    engine::engine::get_main_loop(rt)
}

/// Returns a pointer to the root `Window` node of the scene tree.
pub fn root() -> Result<*const c_void> {
    let rt = crate::runtime()?;
    let tree = scene_tree()? as usize;
    engine::scene_tree::get_root(rt, tree)
}

/// Returns the number of children of a node.
pub fn child_count(node: usize) -> Result<i32> {
    let rt = crate::runtime()?;
    engine::node::get_child_count(rt, node)
}

/// Returns the i-th child of a node.
pub fn child(node: usize, index: i32) -> Result<*const c_void> {
    let rt = crate::runtime()?;
    engine::node::get_child(rt, node, index)
}

/// Returns a pointer to the managed `StringName` of a node.
pub fn node_name(node: usize) -> Result<*const c_void> {
    let rt = crate::runtime()?;
    engine::node::get_name(rt, node)
}

/// Returns the parent of a node, or null if at the root.
pub fn parent(node: usize) -> Result<*const c_void> {
    let rt = crate::runtime()?;
    engine::node::get_parent(rt, node)
}
