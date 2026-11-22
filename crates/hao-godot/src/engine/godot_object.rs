//! `Godot.GodotObject` — ToString, class, instance ID, meta.

use std::ffi::c_void;

use hao_runtime::{Result, Runtime};

crate::bindings! {
    TO_STRING:       "Godot.GodotObject" "ToString"      0;
    GET_CLASS:       "Godot.GodotObject" "GetClass"      0;
    GET_INSTANCE_ID: "Godot.GodotObject" "GetInstanceId" 0;
    IS_CLASS:        "Godot.GodotObject" "IsClass"       1;
    SET_META:        "Godot.GodotObject" "SetMeta"       2;
    GET_META:        "Godot.GodotObject" "GetMeta"       1;
    HAS_META:        "Godot.GodotObject" "HasMeta"       1;
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
