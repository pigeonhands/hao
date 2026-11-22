//! `Godot.ResourceLoader` — load/exists.

use std::ffi::c_void;

use hao_runtime::{Result, Runtime};

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
