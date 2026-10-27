//! Attach to a Godot (CoreCLR / .NET) game from a native mod loaded with
//! `LD_PRELOAD`.
//!
//! The attach machinery — hooking `coreclr_initialize` to capture the running
//! host — lives in [`hao_runtime::attach`]. This crate adds the Godot-facing
//! conveniences: a log file that survives Steam's sandbox, and re-exports of the
//! pieces a mod uses.

pub use hao_runtime::attach::ClrHost;
pub use hao_runtime::Runtime;

/// Runs `callback` once the game's CoreCLR host is captured (see
/// [`hao_runtime::attach::on_clr_init`]). Also points hao-runtime's logger at
/// this crate's [`log`], so the attach machinery logs where we can read it.
pub fn on_clr_init(callback: impl FnOnce(ClrHost) + Send + 'static) {
    hao_runtime::set_logger(log);
    hao_runtime::attach::on_clr_init(callback);
}

/// Appends `msg` to `~/.local/share/hao-godot/hao-godot.log` and stderr. The log
/// file is reliable even inside Steam's sandbox, where the game's stderr and
/// `/tmp` may not reach the host.
pub fn log(msg: &str) {
    use std::io::Write;
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    eprintln!("[hao-godot] {msg}");
    let Some(home) = std::env::var_os("HOME") else {
        return;
    };
    let path = std::path::Path::new(&home).join(".local/share/hao-godot/hao-godot.log");
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(file, "{secs} {msg}");
    }
}
