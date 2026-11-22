//! `Godot.Timer` — start/stop, wait time, one-shot.

use hao_runtime::{Result, Runtime};

crate::bindings! {
    START:         "Godot.Timer" "Start"         0;
    STOP:          "Godot.Timer" "Stop"          0;
    IS_STOPPED:    "Godot.Timer" "IsStopped"     0;
    GET_WAIT_TIME: "Godot.Timer" "get_WaitTime"  0;
    SET_WAIT_TIME: "Godot.Timer" "set_WaitTime"  1;
    GET_TIME_LEFT: "Godot.Timer" "get_TimeLeft"  0;
    GET_ONE_SHOT:  "Godot.Timer" "get_OneShot"   0;
    SET_ONE_SHOT:  "Godot.Timer" "set_OneShot"   1;
    GET_AUTOSTART: "Godot.Timer" "get_Autostart" 0;
    SET_AUTOSTART: "Godot.Timer" "set_Autostart" 1;
}

pub fn start(rt: &Runtime, instance: usize) -> Result<()> {
    START.call1(rt, instance as u64)?;
    Ok(())
}

pub fn stop(rt: &Runtime, instance: usize) -> Result<()> {
    STOP.call1(rt, instance as u64)?;
    Ok(())
}

pub fn is_stopped(rt: &Runtime, instance: usize) -> Result<bool> {
    Ok(IS_STOPPED.call1(rt, instance as u64)? != 0)
}

pub fn get_time_left(rt: &Runtime, instance: usize) -> Result<f64> {
    Ok(f64::from_bits(GET_TIME_LEFT.call1(rt, instance as u64)?))
}
