//! A process-wide log sink the host can redirect.
//!
//! When hao-runtime is injected into a game (as a profiler, or an `attach` mod)
//! its stderr may be swallowed by a sandbox, so a host sets a logger that writes
//! somewhere it can read — a file, say. Without one, lines go to stderr.

use std::sync::atomic::{AtomicPtr, Ordering};

static LOGGER: AtomicPtr<()> = AtomicPtr::new(std::ptr::null_mut());

/// Routes hao-runtime's log lines to `f`.
pub fn set_logger(f: fn(&str)) {
    LOGGER.store(f as *mut (), Ordering::SeqCst);
}

pub(crate) fn log(msg: &str) {
    let f = LOGGER.load(Ordering::SeqCst);
    if f.is_null() {
        eprintln!("[hao-runtime] {msg}");
    } else {
        let f: fn(&str) = unsafe { std::mem::transmute(f) };
        f(msg);
    }
}
