//! Attach to a CoreCLR game's already-running runtime from a native mod loaded
//! with `LD_PRELOAD`.
//!
//! CoreCLR has no public "attach to the running runtime" C API:
//! `coreclr_create_delegate` needs the host handle and domain id the game's host
//! created, and the runtime initialises only once per process. So we capture
//! them as the game creates them — hooking `dlopen` to see `libcoreclr` arrive
//! (loaded late and resolved through `dlsym`, so symbol interposition misses it),
//! then inline-hooking `coreclr_initialize` to read its output.
//!
//! With the captured [`ClrHost`], [`ClrHost::attach`] builds a [`Runtime`] bound
//! to that CLR, so any managed method can be called.

use std::{
    ffi::{c_char, c_int, c_void, CStr, CString},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Mutex, OnceLock,
    },
};

use crate::{log::log, Result, Runtime};

/// `coreclr_initialize(exePath, friendlyName, propCount, keys, values, &hostHandle, &domainId)`.
type CoreclrInitialize = unsafe extern "C" fn(
    *const c_char,
    *const c_char,
    c_int,
    *const *const c_char,
    *const *const c_char,
    *mut *mut c_void,
    *mut u32,
) -> c_int;

/// `coreclr_create_delegate(hostHandle, domainId, assembly, type, method, &delegate)`.
type CoreclrCreateDelegate = unsafe extern "C" fn(
    *mut c_void,
    u32,
    *const c_char,
    *const c_char,
    *const c_char,
    *mut *mut c_void,
) -> c_int;

/// A handle to the game's running CoreCLR, captured from `coreclr_initialize`.
#[derive(Clone, Copy)]
pub struct ClrHost {
    pub handle: *mut c_void,
    pub domain: u32,
    create_delegate: CoreclrCreateDelegate,
}

// The handle and domain are used across threads; the delegate is thread safe.
unsafe impl Send for ClrHost {}
unsafe impl Sync for ClrHost {}

impl ClrHost {
    /// A native-callable pointer to a static `[UnmanagedCallersOnly]` method
    /// `type_name::method` in `assembly` (CoreCLR only exposes such methods
    /// directly; arbitrary methods need a managed loader on top of this).
    pub fn create_delegate(
        &self,
        assembly: &str,
        type_name: &str,
        method: &str,
    ) -> std::result::Result<*mut c_void, String> {
        let assembly = CString::new(assembly).map_err(|_| "assembly has a nul")?;
        let type_name = CString::new(type_name).map_err(|_| "type has a nul")?;
        let method = CString::new(method).map_err(|_| "method has a nul")?;
        let mut delegate = std::ptr::null_mut();
        let code = unsafe {
            (self.create_delegate)(
                self.handle,
                self.domain,
                assembly.as_ptr(),
                type_name.as_ptr(),
                method.as_ptr(),
                &mut delegate,
            )
        };
        if code != 0 {
            return Err(format!("coreclr_create_delegate failed: 0x{code:x}"));
        }
        Ok(delegate)
    }

    /// Attaches hao's runtime to the game's CoreCLR, so any managed method can be
    /// called (via [`Runtime::method_pointer`]). `major` is the game's .NET major
    /// version (e.g. 9 for a .NET 9 game).
    pub fn attach(&self, major: u16) -> Result<Runtime> {
        Runtime::coreclr_attach(
            self.handle,
            self.domain,
            self.create_delegate as *const c_void,
            major,
        )
    }
}

type Callback = Box<dyn FnOnce(ClrHost) + Send>;

static CALLBACKS: Mutex<Vec<Callback>> = Mutex::new(Vec::new());
static HOST: OnceLock<ClrHost> = OnceLock::new();
static CREATE_DELEGATE: AtomicUsize = AtomicUsize::new(0);
static DLOPEN_HOOKED: AtomicBool = AtomicBool::new(false);
static INIT_HOOKED: AtomicBool = AtomicBool::new(false);

/// Runs `callback` once the game's CoreCLR host has been captured. Call it from a
/// mod's `.init_array` entry, which `LD_PRELOAD` runs before the game starts the
/// runtime.
///
/// The callback runs on a background thread, **not** the game's init thread:
/// running managed code during `coreclr_initialize` corrupts the runtime's
/// bootstrap and crashes the game. It is therefore also not on any engine main
/// thread, so it may call thread-agnostic (corlib) methods but must not touch a
/// main-thread-only API until it has a main-thread callback to run on.
pub fn on_clr_init(callback: impl FnOnce(ClrHost) + Send + 'static) {
    if let Some(host) = HOST.get() {
        let host = *host;
        std::thread::spawn(move || callback(host));
        return;
    }
    CALLBACKS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(Box::new(callback));
    install();
}

/// Runs the queued callbacks on a background thread, off the init thread.
fn run_callbacks(host: ClrHost) {
    std::thread::spawn(move || {
        let callbacks = std::mem::take(&mut *CALLBACKS.lock().unwrap_or_else(|e| e.into_inner()));
        for callback in callbacks {
            callback(host);
        }
    });
}

fn install() {
    if DLOPEN_HOOKED.swap(true, Ordering::SeqCst) {
        return;
    }
    // libcoreclr may already be loaded if the mod was injected late.
    let loaded =
        unsafe { libc::dlopen(c"libcoreclr.so".as_ptr(), libc::RTLD_NOW | libc::RTLD_NOLOAD) };
    if !loaded.is_null() {
        log("libcoreclr already loaded; hooking coreclr_initialize now");
        hook_coreclr_init(loaded);
        return;
    }
    log("hooking dlopen to wait for libcoreclr");
    match unsafe { hooking::Hook::by_name(None, c"dlopen", dlopen_detour as *mut u8) } {
        Ok(mut hook) => {
            if let Err(e) = unsafe { hook.apply_hook() } {
                log(&format!("applying the dlopen hook failed: {e:?}"));
            }
            std::mem::forget(hook);
        }
        Err(e) => log(&format!("hooking dlopen failed: {e:?}")),
    }
}

fn hook_coreclr_init(handle: *mut c_void) {
    if INIT_HOOKED.swap(true, Ordering::SeqCst) {
        return;
    }
    let init = unsafe { libc::dlsym(handle, c"coreclr_initialize".as_ptr()) };
    let create = unsafe { libc::dlsym(handle, c"coreclr_create_delegate".as_ptr()) };
    if init.is_null() || create.is_null() {
        log("coreclr_initialize or coreclr_create_delegate not found in libcoreclr");
        return;
    }
    CREATE_DELEGATE.store(create as usize, Ordering::SeqCst);
    log("hooking coreclr_initialize");
    match unsafe { hooking::Hook::create(init as *mut u8, coreclr_init_detour as *mut u8) } {
        Ok(mut hook) => {
            if let Err(e) = unsafe { hook.apply_hook() } {
                log(&format!("applying the coreclr_initialize hook failed: {e:?}"));
            }
            std::mem::forget(hook);
        }
        Err(e) => log(&format!("hooking coreclr_initialize failed: {e:?}")),
    }
}

extern "C" fn dlopen_detour(filename: *const c_char, flags: c_int) -> *mut c_void {
    // Must be the first thing the detour does (it reads scratch registers).
    let original = hooking::original_function_ptr().expect("original! inside the dlopen hook");
    let original: unsafe extern "C" fn(*const c_char, c_int) -> *mut c_void =
        unsafe { std::mem::transmute(original.as_ptr()) };
    let handle = unsafe { original(filename, flags) };
    if !handle.is_null() && !filename.is_null() && !INIT_HOOKED.load(Ordering::SeqCst) {
        let name = unsafe { CStr::from_ptr(filename) }.to_string_lossy();
        if name.contains("libcoreclr.so") {
            log(&format!("libcoreclr loaded: {name}"));
            hook_coreclr_init(handle);
        }
    }
    handle
}

extern "C" fn coreclr_init_detour(
    exe: *const c_char,
    name: *const c_char,
    count: c_int,
    keys: *const *const c_char,
    values: *const *const c_char,
    host_handle: *mut *mut c_void,
    domain_id: *mut u32,
) -> c_int {
    let original =
        hooking::original_function_ptr().expect("original! inside the coreclr_initialize hook");
    let original: CoreclrInitialize = unsafe { std::mem::transmute(original.as_ptr()) };
    let result = unsafe { original(exe, name, count, keys, values, host_handle, domain_id) };
    log(&format!("coreclr_initialize returned 0x{result:x}"));
    if result == 0 && !host_handle.is_null() && !domain_id.is_null() {
        let handle = unsafe { *host_handle };
        let domain = unsafe { *domain_id };
        let create = CREATE_DELEGATE.load(Ordering::SeqCst);
        if create != 0 {
            let host = ClrHost {
                handle,
                domain,
                create_delegate: unsafe {
                    std::mem::transmute::<usize, CoreclrCreateDelegate>(create)
                },
            };
            log(&format!("captured CLR host: handle={handle:?}, domain={domain}"));
            let _ = HOST.set(host);
            // Run callbacks off this thread: it is mid-coreclr_initialize, and
            // managed work here corrupts the game's runtime bootstrap.
            run_callbacks(host);
        }
    }
    result
}
