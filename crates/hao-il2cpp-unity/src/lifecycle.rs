//! Running mod code every frame.

use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicI32, Ordering},
        Arc, Mutex,
    },
};

use hao_il2cpp_runtime::{Original, Result};

use crate::{bindings::CAMERA_FIRE_ON_POST_RENDER, Camera};

type Callback = Arc<dyn Fn() + Send + Sync>;
type RenderCallback = Arc<dyn Fn(Camera) + Send + Sync>;

static CALLBACKS: Mutex<Vec<Callback>> = Mutex::new(Vec::new());
static HOOKED: Mutex<bool> = Mutex::new(false);
static LAST_FRAME: AtomicI32 = AtomicI32::new(-1);

static RENDER_CALLBACKS: Mutex<Vec<RenderCallback>> = Mutex::new(Vec::new());
static RENDER_HOOKED: Mutex<bool> = Mutex::new(false);

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

/// Runs `callback` on the render thread after each camera finishes rendering,
/// passing the [`Camera`]. This is where to draw an overlay on top of the game
/// with [`gl`](crate::gl), or to drive an immediate-mode UI such as egui.
///
/// It hooks `Camera.FireOnPostRender`, which Unity invokes from native code per
/// camera render, so the callback runs in the render pipeline whether or not
/// anything subscribed to `Camera.onPostRender`. Games with several cameras call
/// it more than once per frame; compare the passed camera's `name()` if you only
/// want one. Panics are caught and printed.
pub fn on_render(callback: impl Fn(Camera) + Send + Sync + 'static) -> Result<()> {
    RENDER_CALLBACKS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(Arc::new(callback));
    let mut hooked = RENDER_HOOKED.lock().unwrap_or_else(|e| e.into_inner());
    if !*hooked {
        CAMERA_FIRE_ON_POST_RENDER.method()?.hook(
            |camera: Camera, original: Original<_, ()>| {
                original.call((camera,));
                let callbacks = RENDER_CALLBACKS
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone();
                for callback in callbacks {
                    if catch_unwind(AssertUnwindSafe(|| callback(camera))).is_err() {
                        tracing::error!("an on_render callback panicked");
                    }
                }
            },
        )?;
        *hooked = true;
        tracing::debug!("watching for camera renders");
    }
    Ok(())
}
