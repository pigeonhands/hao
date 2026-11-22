//! `Godot.Control` — UI position, size, visibility, focus.

use hao_runtime::{Result, Runtime};

crate::bindings! {
    GET_POSITION:           "Godot.Control" "get_Position"          0;
    SET_POSITION:           "Godot.Control" "set_Position"          1;
    GET_SIZE:               "Godot.Control" "get_Size"              0;
    SET_SIZE:               "Godot.Control" "set_Size"              1;
    GET_VISIBLE:            "Godot.Control" "get_Visible"           0;
    SET_VISIBLE:            "Godot.Control" "set_Visible"           1;
    GET_CUSTOM_MINIMUM_SIZE: "Godot.Control" "get_CustomMinimumSize" 0;
    SET_CUSTOM_MINIMUM_SIZE: "Godot.Control" "set_CustomMinimumSize" 1;
    GET_TOOLTIP_TEXT:       "Godot.Control" "get_TooltipText"       0;
    SET_TOOLTIP_TEXT:       "Godot.Control" "set_TooltipText"       1;
    IS_VISIBLE_IN_TREE:    "Godot.Control" "IsVisibleInTree"       0;
    GRAB_FOCUS:            "Godot.Control" "GrabFocus"             0;
    HAS_FOCUS:             "Godot.Control" "HasFocus"              0;
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
