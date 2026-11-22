//! `Godot.Node` — name, parent, children, path, tree, find.

use std::ffi::c_void;

use hao_runtime::{Result, Runtime};

crate::bindings! {
    GET_NAME:          "Godot.Node" "get_Name"        0;
    SET_NAME:          "Godot.Node" "set_Name"        1;
    GET_PARENT:        "Godot.Node" "GetParent"       0;
    GET_CHILD:         "Godot.Node" "GetChild"        1;
    GET_CHILD_COUNT:   "Godot.Node" "GetChildCount"   0;
    ADD_CHILD:         "Godot.Node" "AddChild"        1;
    REMOVE_CHILD:      "Godot.Node" "RemoveChild"     1;
    QUEUE_FREE:        "Godot.Node" "QueueFree"       0;
    GET_PATH:          "Godot.Node" "GetPath"         0;
    GET_TREE:          "Godot.Node" "GetTree"         0;
    IS_INSIDE_TREE:    "Godot.Node" "IsInsideTree"    0;
    GET_INDEX:         "Godot.Node" "GetIndex"        0;
    FIND_CHILD:        "Godot.Node" "FindChild"       1;
    GET_NODE_OR_NULL:  "Godot.Node" "GetNodeOrNull"   1;
    SET_PROCESS:       "Godot.Node" "SetProcess"      1;
    GET_OWNER:         "Godot.Node" "get_Owner"       0;
    IS_PROCESSING:     "Godot.Node" "IsProcessing"    0;
    PRINT_TREE:        "Godot.Node" "PrintTree"       0;
    PRINT_TREE_PRETTY: "Godot.Node" "PrintTreePretty" 0;
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
