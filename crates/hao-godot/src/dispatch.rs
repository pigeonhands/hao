//! Main-thread dispatch queue.
//!
//! Godot's managed API may only be called from its main thread. Code that
//! runs on another thread (an egui overlay, a background task) can use
//! [`queue`] to fire-and-forget a closure, or [`call`] to run a closure
//! and block until the result is ready.
//!
//! The queue is drained once per frame by the lifecycle pump, and can also
//! be drained explicitly with [`drain_pending`] from any code that holds
//! the main thread (e.g. an egui bridge's wait loop).
//!
//! A waker ([`set_waker`]) lets the dispatch module notify sleeping
//! main-thread code that work has been queued, so it can wake up and call
//! [`drain_pending`] without polling.

use std::sync::Mutex;

use hao_runtime::{Error, Result};

type Job = Box<dyn FnOnce() + Send>;
type Waker = Box<dyn Fn() + Send + Sync>;

static QUEUE: Mutex<Vec<Job>> = Mutex::new(Vec::new());
static WAKER: Mutex<Option<Waker>> = Mutex::new(None);

fn drain() {
    let jobs: Vec<Job> =
        std::mem::take(&mut *QUEUE.lock().unwrap_or_else(|e| e.into_inner()));
    for job in jobs {
        job();
    }
}

fn wake() {
    if let Some(w) = &*WAKER.lock().unwrap_or_else(|e| e.into_inner()) {
        w();
    }
}

/// Installs a waker that is called (from the queuing thread) every time a
/// new job is pushed. Use this to wake a condvar or similar so the main
/// thread can call [`drain_pending`] promptly instead of polling.
pub fn set_waker(f: impl Fn() + Send + Sync + 'static) {
    *WAKER.lock().unwrap_or_else(|e| e.into_inner()) = Some(Box::new(f));
}

/// Runs all pending jobs on the current thread, then returns. Safe to call
/// from any thread, but only useful on the main thread — the jobs expect
/// to run there.
pub fn drain_pending() {
    drain();
}

/// Queues `f` to run on Godot's main thread on the next frame. Returns
/// immediately without waiting for `f` to execute.
pub fn queue(f: impl FnOnce() + Send + 'static) -> Result<()> {
    if !crate::lifecycle::is_pump_active() {
        tracing::warn!("dispatch::queue called but pump not active");
        return Err(Error::Other(
            "dispatch: lifecycle pump not started — call lifecycle::start_pump or \
             lifecycle::intercept_process first"
                .into(),
        ));
    }
    QUEUE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(Box::new(f));
    wake();
    Ok(())
}

/// Runs `f` on Godot's main thread and blocks the caller until the result
/// is ready.
pub fn call<R: Send + 'static>(
    f: impl FnOnce() -> R + Send + 'static,
) -> Result<R> {
    if !crate::lifecycle::is_pump_active() {
        return Err(Error::Other(
            "dispatch: lifecycle pump not started — call lifecycle::start_pump or \
             lifecycle::intercept_process first"
                .into(),
        ));
    }
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    QUEUE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(Box::new(move || {
            let _ = tx.send(f());
        }));
    wake();
    rx.recv()
        .map_err(|_| Error::Other("main-thread dispatch: channel closed".into()))
}
