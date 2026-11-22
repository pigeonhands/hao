//! `Godot.DisplayServer` — screen/window size and position.

use hao_runtime::{Result, Runtime};

crate::bindings! {
    SCREEN_GET_SIZE:     "Godot.DisplayServer" "ScreenGetSize"     0;
    WINDOW_GET_SIZE:     "Godot.DisplayServer" "WindowGetSize"     0;
    WINDOW_SET_SIZE:     "Godot.DisplayServer" "WindowSetSize"     1;
    WINDOW_GET_POSITION: "Godot.DisplayServer" "WindowGetPosition" 0;
    WINDOW_SET_POSITION: "Godot.DisplayServer" "WindowSetPosition" 1;
    GET_SCREEN_COUNT:    "Godot.DisplayServer" "GetScreenCount"    0;
}

pub fn get_screen_count(rt: &Runtime) -> Result<i32> {
    Ok(GET_SCREEN_COUNT.call0(rt)? as i32)
}
