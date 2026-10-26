//! Running mod code every frame.

use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicI32, Ordering},
        Arc, Mutex,
    },
};

use hao_il2cpp_runtime::{Original, Result};

type Callback = Arc<dyn Fn() + Send + Sync>;

static CALLBACKS: Mutex<Vec<Callback>> = Mutex::new(Vec::new());
static HOOKED: Mutex<bool> = Mutex::new(false);
static LAST_FRAME: AtomicI32 = AtomicI32::new(-1);

/// Runs `callback` once per frame on Unity's main thread, a good place to update
/// an overlay. Panics are caught and printed.
///
/// It works by hooking `UnityEngine.Time.frameCount`, which the engine reads
/// every frame, and firing when the frame number changes. A frame where nothing
/// reads `Time.frameCount` is skipped, which in an active game does not happen.
pub fn on_update(callback: impl Fn() + Send + Sync + 'static) -> Result<()> {
    CALLBACKS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(Arc::new(callback));
    let mut hooked = HOOKED.lock().unwrap_or_else(|e| e.into_inner());
    if !*hooked {
        let frame_count = crate::il2cpp()?
            .class("UnityEngine.CoreModule", "UnityEngine", "Time")?
            .method("get_frameCount", 0)?;
        frame_count.hook(|original: Original<_, i32>| {
            let frame = original.call(());
            // Fire once per frame. The callback calls the original code, not the
            // hook, so this never recurses.
            if LAST_FRAME.swap(frame, Ordering::Relaxed) != frame {
                let callbacks = CALLBACKS.lock().unwrap_or_else(|e| e.into_inner()).clone();
                for callback in callbacks {
                    if catch_unwind(AssertUnwindSafe(|| callback())).is_err() {
                        tracing::error!("an on_update callback panicked");
                    }
                }
            }
            frame
        })?;
        *hooked = true;
        tracing::debug!("watching for frame updates");
    }
    Ok(())
}
