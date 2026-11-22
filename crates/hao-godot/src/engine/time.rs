//! `Godot.Time` — tick counters.

use hao_runtime::{Result, Runtime};

crate::bindings! {
    GET_TICKS_MSEC: "Godot.Time" "GetTicksMsec" 0;
    GET_TICKS_USEC: "Godot.Time" "GetTicksUsec" 0;
}

pub fn get_ticks_msec(rt: &Runtime) -> Result<u64> {
    GET_TICKS_MSEC.call0(rt)
}

pub fn get_ticks_usec(rt: &Runtime) -> Result<u64> {
    GET_TICKS_USEC.call0(rt)
}
