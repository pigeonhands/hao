//! `Godot.Performance` — monitor values (FPS, memory, draw calls, …).

use hao_runtime::{Result, Runtime};

crate::bindings! {
    GET_MONITOR: "Godot.Performance" "GetMonitor" 1;
}

pub fn get_monitor(rt: &Runtime, monitor: u64) -> Result<f64> {
    Ok(f64::from_bits(GET_MONITOR.call1(rt, monitor)?))
}
