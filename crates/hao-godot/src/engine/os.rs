//! `Godot.OS` — platform name, process ID.

use std::ffi::c_void;

use hao_runtime::{Result, Runtime};

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
