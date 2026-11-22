//! `Godot.Input` — actions, keys, mouse buttons, mouse mode.

use hao_runtime::{Result, Runtime};

crate::bindings! {
    IS_ACTION_PRESSED:       "Godot.Input" "IsActionPressed"       1;
    IS_ACTION_JUST_PRESSED:  "Godot.Input" "IsActionJustPressed"   1;
    IS_ACTION_JUST_RELEASED: "Godot.Input" "IsActionJustReleased"  1;
    IS_KEY_PRESSED:          "Godot.Input" "IsKeyPressed"          1;
    IS_MOUSE_BUTTON_PRESSED: "Godot.Input" "IsMouseButtonPressed"  1;
    GET_MOUSE_POSITION:      "Godot.Input" "GetGlobalMousePosition" 0;
    SET_MOUSE_MODE:          "Godot.Input" "set_MouseMode"         1;
    GET_MOUSE_MODE:          "Godot.Input" "get_MouseMode"         0;
}

pub fn is_action_pressed(rt: &Runtime, action: usize) -> Result<bool> {
    Ok(IS_ACTION_PRESSED.call1(rt, action as u64)? != 0)
}

pub fn is_action_just_pressed(rt: &Runtime, action: usize) -> Result<bool> {
    Ok(IS_ACTION_JUST_PRESSED.call1(rt, action as u64)? != 0)
}

pub fn is_key_pressed(rt: &Runtime, keycode: u64) -> Result<bool> {
    Ok(IS_KEY_PRESSED.call1(rt, keycode)? != 0)
}

pub fn is_mouse_button_pressed(rt: &Runtime, button: u64) -> Result<bool> {
    Ok(IS_MOUSE_BUTTON_PRESSED.call1(rt, button)? != 0)
}
