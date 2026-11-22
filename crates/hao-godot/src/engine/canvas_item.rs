//! `Godot.CanvasItem` — 2D visibility, show/hide, modulate.

use hao_runtime::{Result, Runtime};

crate::bindings! {
    GET_VISIBLE:        "Godot.CanvasItem" "get_Visible"     0;
    SET_VISIBLE:        "Godot.CanvasItem" "set_Visible"     1;
    SHOW:               "Godot.CanvasItem" "Show"            0;
    HIDE:               "Godot.CanvasItem" "Hide"            0;
    GET_MODULATE:       "Godot.CanvasItem" "get_Modulate"    0;
    SET_MODULATE:       "Godot.CanvasItem" "set_Modulate"    1;
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
