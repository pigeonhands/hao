//! Engine-agnostic egui overlay via OpenGL/Vulkan swapchain hooks.
//!
//! Both backends are hooked at init; whichever the game activates first is
//! used. The draw callback runs on the render thread each frame.
//!
//! ```no_run
//! hao_egui_overlay::on_frame(|ctx| {
//!     egui::Window::new("My Overlay").show(ctx, |ui| {
//!         ui.label("Hello from the overlay!");
//!     });
//! });
//! hao_egui_overlay::init();
//! ```

pub use egui;

#[cfg(feature = "hooking")]
mod hook;
pub mod input;
#[cfg(feature = "opengl")]
pub mod gl_backend;
#[cfg(feature = "vulkan")]
pub mod vk_backend;

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

static UI_SCALE: AtomicU32 = AtomicU32::new(0);

/// Scale the overlay UI. Values below 1.0 shrink it; above 1.0 enlarge it.
/// Pass 0.0 to reset to the default (1.0).
pub fn set_scale(scale: f32) {
    if scale <= 0.0 {
        UI_SCALE.store(0, Ordering::Relaxed);
    } else {
        UI_SCALE.store(scale.to_bits(), Ordering::Relaxed);
    }
}

pub fn ui_scale() -> f32 {
    let bits = UI_SCALE.load(Ordering::Relaxed);
    if bits == 0 { 1.0 } else { f32::from_bits(bits) }
}

type DrawFn = dyn Fn(&egui::Context) + Send + Sync + 'static;

struct Bridge {
    draw: Arc<DrawFn>,
    visible: AtomicBool,
}

static BRIDGE: Mutex<Option<Bridge>> = Mutex::new(None);

/// Register a draw callback that runs on the render thread each frame.
pub fn on_frame(draw: impl Fn(&egui::Context) + Send + Sync + 'static) {
    *BRIDGE.lock().unwrap() = Some(Bridge {
        draw: Arc::new(draw),
        visible: AtomicBool::new(true),
    });
}

pub fn set_visible(visible: bool) {
    if let Some(b) = BRIDGE.lock().unwrap().as_ref() {
        b.visible.store(visible, Ordering::Relaxed);
    }
}

pub fn is_visible() -> bool {
    BRIDGE
        .lock()
        .unwrap()
        .as_ref()
        .is_some_and(|b| b.visible.load(Ordering::Relaxed))
}

/// Install OpenGL and Vulkan hooks. The first renderer the game activates wins.
pub fn init() {
    tracing::info!("hao-egui-overlay: installing hooks");
    #[cfg(feature = "hooking")]
    hook::install();
    #[cfg(feature = "opengl")]
    gl_backend::install();
    #[cfg(feature = "vulkan")]
    vk_backend::install();
}

pub(crate) fn get_draw() -> Option<Arc<DrawFn>> {
    let guard = BRIDGE.lock().unwrap();
    let bridge = guard.as_ref()?;
    if !bridge.visible.load(Ordering::Relaxed) {
        return None;
    }
    Some(Arc::clone(&bridge.draw))
}
