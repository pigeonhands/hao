//! Runs mod code once the game's Il2Cpp runtime has started.
//!
//! Unity loads the game binary with `dlopen` and then calls `il2cpp_init`. A
//! mod loaded earlier (with `LD_PRELOAD`) hooks `dlopen` to see the binary
//! arrive, then hooks its `il2cpp_init`.

use std::{
    ffi::{c_char, c_int, c_void, CStr},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};

use crate::{original, Error, Il2Cpp, Result, GAME_ASSEMBLY};

type Callback = Box<dyn FnOnce(&'static Il2Cpp) -> Result<()> + Send>;

static CALLBACKS: Mutex<Vec<Callback>> = Mutex::new(Vec::new());
static DLOPEN_HOOKED: AtomicBool = AtomicBool::new(false);
static INIT_HOOKED: AtomicBool = AtomicBool::new(false);
static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Runs `callback` once `il2cpp_init` has finished, or straight away if it already has.
///
/// The mod has to be loaded before the game binary initialises Il2Cpp, which
/// `LD_PRELOAD` guarantees. Errors are printed to stderr.
pub fn on_init(callback: impl FnOnce(&'static Il2Cpp) -> Result<()> + Send + 'static) {
    if INITIALIZED.load(Ordering::SeqCst) {
        run(Box::new(callback));
        return;
    }
    CALLBACKS.lock().unwrap_or_else(|e| e.into_inner()).push(Box::new(callback));
    if let Err(e) = install() {
        eprintln!("[hao] could not watch for {}: {}", GAME_ASSEMBLY, e);
    }
}

fn run(callback: Callback) {
    match Il2Cpp::loaded() {
        Ok(il2cpp) => {
            if let Err(e) = callback(il2cpp) {
                eprintln!("[hao] mod failed: {}", e);
            }
        }
        Err(e) => eprintln!("[hao] {}", e),
    }
}

fn install() -> Result<()> {
    // The binary may already be loaded but not initialised.
    let name = std::ffi::CString::new(GAME_ASSEMBLY).unwrap();
    let handle = unsafe { libc::dlopen(name.as_ptr(), libc::RTLD_NOW | libc::RTLD_NOLOAD) };
    if !handle.is_null() {
        return hook_init(handle);
    }
    if DLOPEN_HOOKED.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    let mut hook = unsafe { hooking::Hook::by_name(None, c"dlopen", dlopen_detour as *mut u8) }.map_err(Error::Hook)?;
    unsafe { hook.apply_hook() }.map_err(Error::Hook)
}

fn hook_init(handle: *mut c_void) -> Result<()> {
    if INIT_HOOKED.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    let init = unsafe { libc::dlsym(handle, c"il2cpp_init".as_ptr()) };
    if init.is_null() {
        return Err(Error::NotFound(format!("il2cpp_init in {}", GAME_ASSEMBLY)));
    }
    let mut hook = unsafe { hooking::Hook::create(init as *mut u8, init_detour as *mut u8) }.map_err(Error::Hook)?;
    unsafe { hook.apply_hook() }.map_err(Error::Hook)
}

extern "C" fn dlopen_detour(filename: *const c_char, flags: c_int) -> *mut c_void {
    let dlopen = original!(extern "C" fn(*const c_char, c_int) -> *mut c_void);
    let handle = dlopen(filename, flags);
    if !handle.is_null() && !filename.is_null() && !INIT_HOOKED.load(Ordering::SeqCst) {
        let name = unsafe { CStr::from_ptr(filename) }.to_string_lossy();
        if name.ends_with(GAME_ASSEMBLY) {
            if let Err(e) = hook_init(handle) {
                eprintln!("[hao] could not hook il2cpp_init: {}", e);
            }
        }
    }
    handle
}

extern "C" fn init_detour(domain_name: *const c_char) -> i32 {
    let init = original!(extern "C" fn(*const c_char) -> i32);
    let result = init(domain_name);
    if result != 0 && !INITIALIZED.swap(true, Ordering::SeqCst) {
        let callbacks = std::mem::take(&mut *CALLBACKS.lock().unwrap_or_else(|e| e.into_inner()));
        for callback in callbacks {
            run(callback);
        }
    }
    result
}
