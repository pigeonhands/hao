//! Attach to a Godot (CoreCLR / .NET) game from a native mod loaded before the
//! runtime starts (`LD_PRELOAD` on Linux).
//!
//! The attach machinery — hooking the loader and `coreclr_initialize` to capture
//! the running host — lives in [`hao_runtime::attach`]; this crate re-exports it
//! and adds [`init_logging`], a one-call `tracing` setup for an injected mod.

#[cfg(feature = "reflect")]
pub mod reflect;

pub use hao_runtime::attach::{on_clr_init, ClrHost};
pub use hao_runtime::Runtime;

/// Allocates a console (on platforms that need one) and installs a `tracing`
/// subscriber writing to stderr, so a mod's logs are visible. The level comes
/// from `RUST_LOG`, defaulting to `info`. Safe to call more than once.
pub fn init_logging() {
    hao_runtime::console::alloc_console();
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info".into());
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}
