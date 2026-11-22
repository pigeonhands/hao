//! Inline-hooks `dlsym` so that games which load Vulkan/GL via
//! `dlopen` + `dlsym` (bypassing LD_PRELOAD symbol interposition) still
//! get our overlay hooks.

use std::ffi::{c_char, c_void, CStr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

static INSTALLED: AtomicBool = AtomicBool::new(false);
#[cfg(feature = "vulkan")]
static VK_HOOKED: AtomicBool = AtomicBool::new(false);
#[cfg(feature = "vulkan")]
static VK_LIB_HANDLE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
#[cfg(feature = "opengl")]
static GL_HOOKED: AtomicBool = AtomicBool::new(false);

type DlsymFn = unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void;
static REAL_DLSYM: OnceLock<DlsymFn> = OnceLock::new();

/// Call the real (unhooked) `dlsym`, bypassing our interception of
/// vkGetInstanceProcAddr / glXSwapBuffers. Falls back to `libc::dlsym`
/// if the hook hasn't fired yet.
pub(crate) unsafe fn real_dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void {
    match REAL_DLSYM.get() {
        Some(f) => f(handle, symbol),
        None => libc::dlsym(handle, symbol),
    }
}

pub(crate) fn install() {
    if INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }
    tracing::info!("hao-egui-overlay: hooking dlsym for Vulkan/GL interception");
    match unsafe { hooking::Hook::by_name(None, c"dlsym", hooked_dlsym as *mut u8) } {
        Ok(mut hook) => {
            // Store the trampoline to the original dlsym BEFORE the hook is
            // live so real_dlsym() never falls through to the hooked libc::dlsym.
            let orig: DlsymFn = unsafe { std::mem::transmute(hook.original_function().as_ptr()) };
            REAL_DLSYM.get_or_init(|| orig);

            if let Err(e) = unsafe { hook.apply_hook() } {
                tracing::error!("hao-egui-overlay: applying dlsym hook failed: {e:?}");
            }
            std::mem::forget(hook);
        }
        Err(e) => tracing::error!("hao-egui-overlay: hooking dlsym failed: {e:?}"),
    }
}

unsafe extern "C" fn hooked_dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void {
    let original = hooking::original_function_ptr().expect("original in dlsym hook");
    let original: DlsymFn = std::mem::transmute(original.as_ptr());
    REAL_DLSYM.get_or_init(|| original);

    // RTLD_NEXT (-1) and RTLD_DEFAULT (0) are our own code looking up real
    // functions — intercepting those would cause infinite recursion.
    let is_special = handle.is_null() || handle == (-1_isize) as *mut c_void;

    if !is_special && !symbol.is_null() {
        let name = CStr::from_ptr(symbol);
        match name.to_bytes() {
            #[cfg(feature = "vulkan")]
            b"vkGetInstanceProcAddr" => {
                if !VK_HOOKED.swap(true, Ordering::Relaxed) {
                    VK_LIB_HANDLE.store(handle as usize, Ordering::Relaxed);
                    tracing::info!("hao-egui-overlay: intercepted dlsym(vkGetInstanceProcAddr)");
                    return super::vk_backend::vkGetInstanceProcAddr as *mut c_void;
                }
            }
            #[cfg(feature = "vulkan")]
            b"vkGetDeviceProcAddr" => {
                let saved = VK_LIB_HANDLE.load(Ordering::Relaxed);
                if saved != 0 && handle as usize == saved {
                    return super::vk_backend::vk_get_device_proc_addr_raw() as *mut c_void;
                }
            }
            #[cfg(feature = "opengl")]
            b"glXSwapBuffers" => {
                if !GL_HOOKED.swap(true, Ordering::Relaxed) {
                    tracing::info!("hao-egui-overlay: intercepted dlsym(glXSwapBuffers)");
                    return super::gl_backend::glXSwapBuffers as *mut c_void;
                }
            }
            _ => {}
        }
    }
    original(handle, symbol)
}
