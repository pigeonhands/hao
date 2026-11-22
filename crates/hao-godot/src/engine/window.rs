//! `Godot.Window` — title, size.

use std::ffi::c_void;

use hao_runtime::{Result, Runtime};

crate::bindings! {
    GET_TITLE: "Godot.Window" "get_Title" 0;
    SET_TITLE: "Godot.Window" "set_Title" 1;
    GET_SIZE:  "Godot.Window" "get_Size"  0;
    SET_SIZE:  "Godot.Window" "set_Size"  1;
}

pub fn get_title(rt: &Runtime, instance: usize) -> Result<*const c_void> {
    GET_TITLE.call_ptr(rt, &[instance as u64])
}

pub fn set_title(rt: &Runtime, instance: usize, title: usize) -> Result<()> {
    SET_TITLE.call2(rt, instance as u64, title as u64)?;
    Ok(())
}
