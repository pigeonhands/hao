//! `Godot.GD` — print, push error/warning.

use hao_runtime::{Result, Runtime};

crate::bindings! {
    PRINT:        "Godot.GD" "Print"       1;
    PRINT_ERR:    "Godot.GD" "PrintErr"    1;
    PUSH_ERROR:   "Godot.GD" "PushError"   1;
    PUSH_WARNING: "Godot.GD" "PushWarning" 1;
}

pub fn print(rt: &Runtime, args: usize) -> Result<()> {
    PRINT.call1(rt, args as u64)?;
    Ok(())
}

pub fn print_err(rt: &Runtime, args: usize) -> Result<()> {
    PRINT_ERR.call1(rt, args as u64)?;
    Ok(())
}

pub fn push_error(rt: &Runtime, msg: usize) -> Result<()> {
    PUSH_ERROR.call1(rt, msg as u64)?;
    Ok(())
}

pub fn push_warning(rt: &Runtime, msg: usize) -> Result<()> {
    PUSH_WARNING.call1(rt, msg as u64)?;
    Ok(())
}
