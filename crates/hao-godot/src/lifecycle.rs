//! Per-frame lifecycle callbacks on Godot's main thread.
//!
//! Three setup paths, in order of preference:
//!
//! **Path A — SceneTree hook** ([`hook_scene_tree`]): rewrites
//! `SceneTree._Process` in GodotSharp via IL patching — works for any
//! Godot game with no game-specific knowledge. Call early (before
//! GodotSharp loads). If the assembly is already loaded (DLL injection),
//! [`ensure_scene_tree_pump`] falls back to native code patching.
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
static SCENE_TREE_REQUESTED: AtomicBool = AtomicBool::new(false);
static SCENE_TREE_PATCHED: AtomicBool = AtomicBool::new(false);
static TICK_LOGGED: AtomicBool = AtomicBool::new(false);

crate::bindings! {
    SCENE_TREE_INVOKE: "Godot.SceneTree" "InvokeGodotClassMethod" 3;
}

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
    if !TICK_LOGGED.swap(true, Ordering::Relaxed) {
        tracing::info!("lifecycle: tick() called for the first time");
    }
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

/// Sets up a lifecycle pump on `SceneTree._Process` from GodotSharp.
///
/// On platforms with assembly interception (unix), this rewrites the
/// method's IL to include a P/Invoke call to [`hao_godot_tick`] — the
/// same mechanism as [`intercept_process`], but hardcoded for the
/// engine's own SceneTree. Call **before** GodotSharp loads (from your
/// library constructor or `on_clr_init` before `host.attach()`).
///
/// After CLR attach, call [`ensure_scene_tree_pump`] (or have your init
/// code call it) to fall back to native code patching if the IL rewrite
/// didn't fire (e.g. DLL injection where GodotSharp was already loaded,
/// or platforms without assembly interception).
///
/// `lib_name` is the native library exporting `hao_godot_tick` (usually
/// the `LD_PRELOAD` path or DLL name).
pub fn hook_scene_tree(lib_name: &str) {
    SCENE_TREE_REQUESTED.store(true, Ordering::SeqCst);

    #[cfg(all(unix, target_arch = "x86_64"))]
    {
        let ty = "Godot.SceneTree".to_string();
        let method = "InvokeGodotClassMethod".to_string();
        let lib = lib_name.to_string();

        hao_runtime::rewrite::intercept_assembly("GodotSharp.dll", move |bytes| {
            let result =
                hao_runtime::rewrite::patch_call(bytes, &ty, &method, &lib, "hao_godot_tick");
            match &result {
                Ok(_) => {
                    tracing::info!("lifecycle: GodotSharp.dll IL rewrite succeeded");
                    SCENE_TREE_PATCHED.store(true, Ordering::SeqCst);
                    PUMP_ACTIVE.store(true, Ordering::SeqCst);
                    deferred_native_hook();
                }
                Err(e) => tracing::error!("lifecycle: GodotSharp.dll IL rewrite failed: {e}"),
            }
            result
        });
        tracing::info!(
            "lifecycle: will patch Godot.SceneTree::InvokeGodotClassMethod \
             in GodotSharp.dll to call hao_godot_tick"
        );
    }

    let _ = lib_name;
}

/// Spawns a thread that waits for the runtime and GodotSharp to be fully
/// loaded, then installs the native hook. Called from the rewrite callback
/// which runs during the assembly's `open()` — the method isn't JIT'd yet.
#[cfg(all(unix, target_arch = "x86_64"))]
fn deferred_native_hook() {
    fn diag(msg: &str) {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/hao-diag.log")
        {
            let _ = writeln!(f, "[lifecycle] {msg}");
        }
    }

    std::thread::spawn(|| {
        diag("deferred_native_hook thread started");
        for i in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            match crate::runtime() {
                Ok(rt) => match SCENE_TREE_INVOKE.resolve(rt) {
                    Ok(ptr) => {
                        diag(&format!("resolved at {ptr:#x} after {i} attempts"));
                        match start_pump(ptr) {
                            Ok(()) => diag("native hook installed!"),
                            Err(e) => diag(&format!("start_pump failed: {e}")),
                        }
                        return;
                    }
                    Err(e) => {
                        if i == 0 {
                            diag(&format!("resolve attempt {i}: {e}"));
                        }
                    }
                },
                Err(e) => {
                    if i == 0 {
                        diag(&format!("runtime not available: {e}"));
                    }
                }
            }
        }
        diag("TIMED OUT after 5s — native hook NOT installed");
    });
}

/// Ensures the SceneTree lifecycle pump is running after CLR attach.
///
/// Tries to install the native hook immediately. If the assembly hasn't been
/// loaded yet (common on the LD_PRELOAD path), the rewrite callback's
/// [`deferred_native_hook`] will handle it once the assembly is available.
#[cfg(target_arch = "x86_64")]
pub fn ensure_scene_tree_pump(runtime: &hao_runtime::Runtime) -> hao_runtime::Result<()> {
    if !SCENE_TREE_REQUESTED.load(Ordering::SeqCst) {
        return Ok(());
    }
    let ptr = SCENE_TREE_INVOKE.resolve(runtime)?;
    tracing::info!("lifecycle: resolved SceneTree.InvokeGodotClassMethod at {ptr:#x}");
    start_pump(ptr)
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
        let result =
            hao_runtime::rewrite::patch_call(bytes, &ty, &method, &lib, "hao_godot_tick");
        if result.is_ok() {
            PUMP_ACTIVE.store(true, Ordering::SeqCst);
        }
        result
    });

    tracing::info!(
        "lifecycle: will patch {type_name}::{method_name} in {assembly_basename} \
         to call hao_godot_tick"
    );
}

/// The native entry point called from the IL-rewritten managed method.
/// Exported so the P/Invoke can resolve it.
#[no_mangle]
pub extern "C" fn hao_godot_tick() {
    static LOGGED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if !LOGGED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/hao-diag.log") {
            let _ = writeln!(f, "[tick] hao_godot_tick called for the first time");
        }
    }
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
    let mut hook = unsafe { hooking::Hook::create(addr as *mut u8, pump_detour as *mut u8) }
        .map_err(|e| hao_runtime::Error::Other(format!("lifecycle hook failed: {e:?}")))?;
    unsafe { hook.apply_hook() }
        .map_err(|e| hao_runtime::Error::Other(format!("applying lifecycle hook failed: {e:?}")))?;

    // Leak the hook — it stays active for the process lifetime.
    std::mem::forget(hook);
    PUMP_ACTIVE.store(true, Ordering::SeqCst);
    tracing::info!("lifecycle: pump hook installed at {addr:#x}");
    Ok(())
}

#[cfg(target_arch = "x86_64")]
unsafe extern "C" fn pump_detour(this: usize, a: usize, b: usize, c: usize) -> usize {
    let original: unsafe extern "C" fn(usize, usize, usize, usize) -> usize =
        std::mem::transmute(hooking::original_function_ptr().unwrap());
    static LOGGED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if !LOGGED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/hao-diag.log") {
            let _ = writeln!(f, "[tick] pump_detour called for the first time");
        }
    }
    tick();
    unsafe { original(this, a, b, c) }
}
