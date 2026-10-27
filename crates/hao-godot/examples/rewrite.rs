//! LD_PRELOAD mod that rewrites the game assembly's IL before CoreCLR JITs it,
//! so one of the game's own methods calls back into Rust — no injected assembly,
//! no C#.
//!
//! It intercepts the game assembly's load and hands the runtime a hao-rewritten
//! copy in which `hao_tick` (exported below) is called at the start of a chosen
//! method. Point it at a method the engine runs each frame on the main thread
//! (e.g. the main `Node`'s `_Process`) and `hao_tick` runs there every frame.
//!
//! The target is set by env var, since it is game-specific:
//!
//! ```sh
//! cargo build --release -p hao-godot --example rewrite
//! # Launch options (run from a terminal to see logs):
//! #   HAO_REWRITE_TYPE=Namespace.MainNode HAO_REWRITE_METHOD=_Process \
//! #   LD_PRELOAD=.../target/release/examples/librewrite.so %command%
//! ```

use std::sync::atomic::{AtomicU64, Ordering};

static FRAMES: AtomicU64 = AtomicU64::new(0);

#[used]
#[link_section = ".init_array"]
static LOAD: extern "C" fn() = load;

extern "C" fn load() {
    hao_godot::init_logging();

    let (Ok(ty), Ok(method)) = (
        std::env::var("HAO_REWRITE_TYPE"),
        std::env::var("HAO_REWRITE_METHOD"),
    ) else {
        tracing::error!(
            "set HAO_REWRITE_TYPE (full type name) and HAO_REWRITE_METHOD to the lifecycle method to patch"
        );
        return;
    };
    let assembly = std::env::var("HAO_REWRITE_ASSEMBLY").unwrap_or_else(|_| "sts2.dll".into());
    let lib = lib_path();
    tracing::info!("rewriting {assembly}: call hao_tick at the start of {ty}::{method} (via {lib})");

    hao_runtime::rewrite::intercept_assembly(&assembly, move |bytes| {
        hao_runtime::rewrite::patch_call(bytes, &ty, &method, &lib, "hao_tick")
    });
}

/// The library name the rewritten P/Invoke imports — this `.so`. The CLR resolves
/// it by `dlopen`, and we are already loaded, so the absolute preload path works.
/// `HAO_REWRITE_LIB` overrides it.
fn lib_path() -> String {
    if let Ok(lib) = std::env::var("HAO_REWRITE_LIB") {
        return lib;
    }
    std::env::var("LD_PRELOAD")
        .ok()
        .and_then(|preload| {
            preload
                .split(':')
                .find(|p| p.contains("rewrite"))
                .map(str::to_string)
        })
        .unwrap_or_else(|| "librewrite.so".into())
}

/// Called from the patched managed method, on whatever thread the engine runs it
/// — the main thread for a lifecycle method.
#[no_mangle]
pub extern "C" fn hao_tick() {
    let frame = FRAMES.fetch_add(1, Ordering::Relaxed);
    if frame == 0 || frame % 300 == 0 {
        tracing::info!("hao_tick: the game's patched method called us (frame {frame})");
    }
}
