//! Registers hao-runtime's CoreCLR profiler, the basis for patching the game's
//! managed methods in memory (ReJIT) and for looking up JITed native code
//! addresses.
//!
//! Unlike the other examples this is loaded by CoreCLR itself as a profiler, so
//! it exports `DllGetClassObject`. Point the runtime at it:
//!
//! ```sh
//! cargo build --release -p hao-godot --example profiler
//! # Launch options (one line):
//! #   CORECLR_ENABLE_PROFILING=1 \
//! #   CORECLR_PROFILER={7F9A1B2C-3D4E-4F50-9A1B-2C3D4E5F6071} \
//! #   CORECLR_PROFILER_PATH=.../target/release/examples/libprofiler.so %command%
//! # Then read ~/.local/share/hao-godot/hao-godot.log
//! ```
//!
//! If the log shows `profiler initialized: ICorProfilerInfo=...`, CoreCLR loaded
//! us as a profiler and ReJIT is enabled.

use std::ffi::c_void;

#[used]
#[link_section = ".init_array"]
static LOAD: extern "C" fn() = load;

extern "C" fn load() {
    hao_godot::log("profiler mod loaded; routing profiler logs to the hao-godot log");
    hao_runtime::set_logger(hao_godot::log);
    // Log JITed methods of the game assembly (override with HAO_PROFILE_MODULE).
    let watch = std::env::var("HAO_PROFILE_MODULE").unwrap_or_else(|_| "sts2".to_string());
    hao_runtime::profiler::watch_module(&watch);
}

/// The symbol CoreCLR looks up in a profiler library.
///
/// # Safety
/// Called by the runtime with COM out-parameters; delegates straight to
/// hao-runtime, which validates them.
#[no_mangle]
pub unsafe extern "C" fn DllGetClassObject(
    rclsid: *const hao_runtime::profiler::Guid,
    riid: *const hao_runtime::profiler::Guid,
    ppv: *mut *mut c_void,
) -> i32 {
    unsafe { hao_runtime::profiler::dll_get_class_object(rclsid, riid, ppv) }
}
