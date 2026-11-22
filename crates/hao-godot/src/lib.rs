//! Attach to a Godot (CoreCLR / .NET) game from a native mod loaded before the
//! runtime starts (`LD_PRELOAD` on Linux).
//!
//! The attach machinery — hooking the loader and `coreclr_initialize` to capture
//! the running host — lives in [`hao_runtime::attach`]; this crate re-exports it
//! and adds [`init_logging`], a one-call `tracing` setup for an injected mod.
//!
//! # Architecture
//!
//! - **[`lifecycle`]** — per-frame callbacks on the game's main thread, via IL
//!   rewrite or JIT hooking.
//! - **[`dispatch`]** — main-thread dispatch queue, drained each frame by the
//!   lifecycle pump.
//! - **[`engine`]** — lazy-resolved bindings to `GodotSharp` engine types
//!   (`Engine`, `Time`, `OS`, `SceneTree`, `Node`, `Node2D`, `Node3D`,
//!   `Control`, `CanvasItem`, `Camera2D`, `Camera3D`, `Viewport`, `Input`,
//!   `GD`, `DisplayServer`, `Performance`, `ResourceLoader`, `Window`,
//!   `Timer`, `AnimationPlayer`, `GodotObject`).
//!
//! # Quick start
//!
//! ```no_run
//! #[used]
//! #[link_section = ".init_array"]
//! static LOAD: extern "C" fn() = load;
//!
//! extern "C" fn load() {
//!     hao_godot::init_logging();
//!     // Set up the lifecycle pump before the game assembly loads.
//!     hao_godot::lifecycle::intercept_process(
//!         "game.dll", "Game.MainNode", "_Process", "libmymod.so",
//!     );
//!     hao_godot::on_clr_init(|host| {
//!         let runtime = host.attach(9).unwrap();
//!         // Now dispatch::queue/call will run on the game thread.
//!     });
//! }
//! ```
//!
//! See the crate's examples for capturing the host, calling managed code, the
//! CoreCLR profiler, and rewriting the game assembly's IL.

pub mod binding;
pub mod dispatch;
pub mod engine;
pub mod lifecycle;
pub mod scene;

#[cfg(feature = "reflect")]
pub mod reflect;

pub use hao_runtime::attach::{on_clr_init, ClrHost};
pub use hao_runtime::Runtime;
pub use lifecycle::{is_shutting_down, on_process, shutdown};

use std::sync::OnceLock;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// Stores the runtime for use by [`scene`] and [`engine`] modules.
/// Call once after `host.attach()` succeeds.
pub fn set_runtime(rt: Runtime) {
    let _ = RUNTIME.set(rt);
}

/// Returns the stored runtime, or an error if [`set_runtime`] hasn't
/// been called yet.
pub fn runtime() -> hao_runtime::Result<&'static Runtime> {
    RUNTIME
        .get()
        .ok_or_else(|| hao_runtime::Error::NotFound("runtime not set — call set_runtime first".into()))
}

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
