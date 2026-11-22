//! Per-frame lifecycle callbacks on Godot's main thread.
//!
//! Three setup paths, in order of preference:
//!
//! **Path A — SceneTree hook** ([`hook_scene_tree`]): hooks
//! `SceneTree._Process` from GodotSharp — works for any Godot game with
//! no game-specific knowledge. Call after `host.attach()`.
//!
//! **Path B — IL rewrite** ([`intercept_process`]): splices a P/Invoke
//! into a game type's `_Process` before the assembly loads. Must be called
//! during [`on_clr_init`](crate::on_clr_init), before `host.attach()`.
//!
//! **Path C — JIT hook** ([`start_pump`]): hooks an already-JIT'd native
//! function pointer. Use when you have a resolved method pointer that runs
//! on the game thread each frame.
//!
//! Once any path is set up, [`on_process`] registers per-frame
//! callbacks, and [`crate::dispatch`] auto-drains its queue each frame.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

type Callback = Arc<dyn Fn() + Send + Sync>;

static CALLBACKS: Mutex<Vec<Callback>> = Mutex::new(Vec::new());
static PUMP_ACTIVE: AtomicBool = AtomicBool::new(false);
static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

/// Returns `true` after [`shutdown`] has been called.
pub fn is_shutting_down() -> bool {
    SHUTTING_DOWN.load(Ordering::Relaxed)
}

/// Signals all lifecycle hooks to stop firing. Call from a cleanup handler
/// to prevent crashes during engine teardown.
pub fn shutdown() {
    SHUTTING_DOWN.store(true, Ordering::Relaxed);
}

/// Returns `true` once either [`start_pump`] or [`intercept_process`] has
/// been called. Used by [`crate::dispatch`] to reject early queue calls.
pub fn is_pump_active() -> bool {
    PUMP_ACTIVE.load(Ordering::Relaxed)
}

/// Runs `callback` once per frame on the game's main thread.
pub fn on_process(callback: impl Fn() + Send + Sync + 'static) {
    CALLBACKS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(Arc::new(callback));
}

// ── pump core ──────────────────────────────────────────────────────────

fn tick() {
    if SHUTTING_DOWN.load(Ordering::Relaxed) {
        return;
    }
    let callbacks = CALLBACKS.lock().unwrap_or_else(|e| e.into_inner()).clone();
    for callback in callbacks {
        if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback())) {
            tracing::error!("lifecycle callback panicked: {e:?}");
        }
    }
    crate::dispatch::drain_pending();
}

// ── Path A: SceneTree hook ─────────────────────────────────────────────

/// Hooks `SceneTree._Process` from the engine's GodotSharp assembly.
///
/// This is the simplest lifecycle setup — it works for any Godot game
/// without needing to know any game-specific types or methods. The
/// engine calls `SceneTree._Process` every frame on the main thread.
///
/// Call after `host.attach()` with the [`Runtime`](hao_runtime::Runtime)
/// returned by attach.
///
/// # Example
///
/// ```no_run
/// hao_godot::on_clr_init(|host| {
///     let runtime = host.attach(9).unwrap();
///     hao_godot::lifecycle::hook_scene_tree(&runtime).unwrap();
///     // lifecycle pump is now active
/// });
/// ```
#[cfg(target_arch = "x86_64")]
pub fn hook_scene_tree(runtime: &hao_runtime::Runtime) -> hao_runtime::Result<()> {
    let ptr = runtime.find_method_from_loaded(
        "GodotSharp",
        "Godot.SceneTree",
        "_Process",
        1, // (double delta)
    )?;
    tracing::info!("lifecycle: resolved SceneTree._Process at {:#x}", ptr as usize);
    start_pump(ptr as usize)
}

// ── Path B: IL rewrite (game-specific) ────────────────────────────────

/// Intercepts the game assembly's load and rewrites the given method to
/// call [`hao_godot_tick`] via P/Invoke at the start of every invocation.
///
/// Must be called **before** the assembly is loaded (i.e., during
/// [`on_clr_init`](crate::on_clr_init), before `host.attach()`).
///
/// # Arguments
///
/// * `assembly_basename` — file name of the assembly, e.g. `"sts2.dll"`
/// * `type_name` — full type name, e.g. `"Namespace.MainNode"`
/// * `method_name` — the method to patch, e.g. `"_Process"`
/// * `lib_name` — the native library exporting `hao_godot_tick`, e.g.
///   the `LD_PRELOAD` path
#[cfg(all(unix, target_arch = "x86_64"))]
pub fn intercept_process(
    assembly_basename: &str,
    type_name: &str,
    method_name: &str,
    lib_name: &str,
) {
    let ty = type_name.to_string();
    let method = method_name.to_string();
    let lib = lib_name.to_string();

    hao_runtime::rewrite::intercept_assembly(assembly_basename, move |bytes| {
        hao_runtime::rewrite::patch_call(bytes, &ty, &method, &lib, "hao_godot_tick")
    });

    PUMP_ACTIVE.store(true, Ordering::SeqCst);
    tracing::info!(
        "lifecycle: will patch {type_name}::{method_name} in {assembly_basename} \
         to call hao_godot_tick"
    );
}

/// The native entry point called from the IL-rewritten managed method.
/// Exported so the P/Invoke can resolve it.
#[no_mangle]
pub extern "C" fn hao_godot_tick() {
    tick();
}

// ── Path C: raw JIT hook ──────────────────────────────────────────────

/// Hooks a JIT'd native function at `addr` and uses it as the per-frame
/// pump. The function must run on the game's main thread every frame.
///
/// Prefer [`hook_scene_tree`] unless you need to hook a specific method.
/// Call after the CLR is attached and you have resolved a suitable method
/// via [`Runtime::method_pointer_from_loaded`](hao_runtime::Runtime::method_pointer_from_loaded).
#[cfg(target_arch = "x86_64")]
pub fn start_pump(addr: usize) -> hao_runtime::Result<()> {
    let hook = unsafe { hooking::Hook::create(addr as *mut u8, pump_detour as *mut u8) }
        .map_err(|e| hao_runtime::Error::Other(format!("lifecycle hook failed: {e:?}")))?;

    // Leak the hook — it stays active for the process lifetime.
    std::mem::forget(hook);
    PUMP_ACTIVE.store(true, Ordering::SeqCst);
    tracing::info!("lifecycle: pump hook installed at {addr:#x}");
    Ok(())
}

#[cfg(target_arch = "x86_64")]
unsafe extern "C" fn pump_detour() {
    tick();
    let original: unsafe extern "C" fn() =
        std::mem::transmute(hooking::original_function_ptr().unwrap());
    unsafe { original() };
}
