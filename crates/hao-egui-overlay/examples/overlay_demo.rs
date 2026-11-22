//! A minimal egui overlay that draws a window on top of any OpenGL or Vulkan
//! game. F1 toggles visibility.
//!
//! ```sh
//! cargo build --release -p hao-egui-overlay --example overlay_demo
//! # Inject into any game:
//! LD_PRELOAD=target/release/examples/liboverlay_demo.so ./my_game
//! # Or via Steam launch options:
//! LD_PRELOAD=/path/to/target/release/examples/liboverlay_demo.so %command%
//! ```

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

#[used]
#[link_section = ".init_array"]
static LOAD: extern "C" fn() = load;

static VISIBLE: AtomicBool = AtomicBool::new(true);
static FRAMES: AtomicU64 = AtomicU64::new(0);
static NOTE: Mutex<String> = Mutex::new(String::new());

extern "C" fn load() {
    hao_egui_overlay::on_frame(|ctx| {
        let frame = FRAMES.fetch_add(1, Ordering::Relaxed);

        ctx.input(|i| {
            if i.key_pressed(egui::Key::F1) {
                let was = VISIBLE.load(Ordering::Relaxed);
                VISIBLE.store(!was, Ordering::Relaxed);
            }
        });

        if !VISIBLE.load(Ordering::Relaxed) {
            return;
        }

        egui::Window::new("hao-egui-overlay demo")
            .default_size([320.0, 200.0])
            .show(ctx, |ui| {
                ui.label(format!("Frame: {frame}"));
                ui.label(format!("Scale: {:.2}", hao_egui_overlay::ui_scale()));

                ui.separator();
                ui.label("Type something:");
                let mut note = NOTE.lock().unwrap();
                ui.text_edit_singleline(&mut *note);

                ui.separator();
                if ui.button("Scale 0.5x").clicked() {
                    hao_egui_overlay::set_scale(0.5);
                }
                if ui.button("Scale 1.0x").clicked() {
                    hao_egui_overlay::set_scale(1.0);
                }
                if ui.button("Scale 1.5x").clicked() {
                    hao_egui_overlay::set_scale(1.5);
                }

                ui.separator();
                ui.label("Press F1 to toggle overlay");
            });
    });

    hao_egui_overlay::set_scale(0.75);
    hao_egui_overlay::init();
}
