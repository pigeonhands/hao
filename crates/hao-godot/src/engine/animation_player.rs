//! `Godot.AnimationPlayer` — play/stop, speed scale.

use std::ffi::c_void;

use hao_runtime::{Result, Runtime};

crate::bindings! {
    PLAY:            "Godot.AnimationPlayer" "Play"                 0;
    STOP:            "Godot.AnimationPlayer" "Stop"                 0;
    IS_PLAYING:      "Godot.AnimationPlayer" "IsPlaying"            0;
    GET_CURRENT:     "Godot.AnimationPlayer" "get_CurrentAnimation" 0;
    SET_CURRENT:     "Godot.AnimationPlayer" "set_CurrentAnimation" 1;
    GET_SPEED_SCALE: "Godot.AnimationPlayer" "get_SpeedScale"       0;
    SET_SPEED_SCALE: "Godot.AnimationPlayer" "set_SpeedScale"       1;
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
