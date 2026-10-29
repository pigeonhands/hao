//! Attach to a Godot (CoreCLR / .NET) game from a native mod loaded before the
//! runtime starts (`LD_PRELOAD` on Linux).
//!
//! The attach machinery — hooking the loader and `coreclr_initialize` to capture
//! the running host — lives in [`hao_runtime::attach`]; this crate re-exports it
//! and adds [`init_logging`], a one-call `tracing` setup for an injected mod.
//!
//! A mod is a `cdylib` that runs on load (via `.init_array`), registers a
//! callback for when the CLR starts, and from there attaches hao's runtime to
//! call into the game's managed code:
//!
//! ```no_run
//! #[used]
//! #[link_section = ".init_array"]
//! static LOAD: extern "C" fn() = load;
//!
//! extern "C" fn load() {
//!     hao_godot::init_logging();
//!     hao_godot::on_clr_init(|host| {
//!         // Attach to the game's CLR (9 = its .NET major version), then call
//!         // managed methods through the returned `Runtime`.
//!         let _runtime = host.attach(9).unwrap();
//!     });
//! }
//! ```
//!
//! See the crate's examples for capturing the host, calling managed code, the
//! CoreCLR profiler, and rewriting the game assembly's IL.

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
