//! An egui overlay for Megabonk's Linux build (or any Unity Il2Cpp game). Draws
//! an egui window over the game, on top of its UI, with F1 to toggle it.
//!
//! ```sh
//! cargo build --release -p hao-il2cpp-unity --features egui --example egui_demo
//! # Steam > Megabonk > Properties > Launch options:
//! LD_PRELOAD=/path/to/target/release/examples/libegui_demo.so %command%
//! ```

use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Mutex,
};

use hao_il2cpp_unity::{egui, input, time, KeyCode};

#[used]
#[link_section = ".init_array"]
static LOAD: extern "C" fn() = load;

static FRAMES: AtomicU64 = AtomicU64::new(0);
/// Shown by default; F1 toggles it.
static VISIBLE: AtomicBool = AtomicBool::new(true);
/// Previous held state of F1, so we toggle once per press (its own edge), not
/// via GetKeyDown, whose per-frame edge is unreliable from the render callback.
static F1_DOWN: AtomicBool = AtomicBool::new(false);
/// Contents of the demo text field (keyboard input test).
static NOTE: Mutex<String> = Mutex::new(String::new());

extern "C" fn load() {
    init_logging();
    hao_il2cpp_unity::on_init(|_| {
        egui::on_egui(|ctx| {
            let f1 = input::key(KeyCode::F1).unwrap_or(false);
            let f1_was_down = F1_DOWN.swap(f1, Ordering::Relaxed);
            if f1 && !f1_was_down {
                VISIBLE.fetch_xor(true, Ordering::Relaxed);
            }
            if !VISIBLE.load(Ordering::Relaxed) {
                return;
            }
            let frames = FRAMES.fetch_add(1, Ordering::Relaxed);
            // egui's default window frame is translucent; force an opaque fill so
            // the overlay reads clearly over the game.
            let frame = egui::Frame::window(&ctx.style()).fill(egui::Color32::from_rgb(24, 24, 28));
            egui::Window::new("hao + egui")
                .frame(frame)
                .default_pos([24.0, 24.0])
                .show(ctx, |ui| {
                    ui.label("Drawn over the game with egui. F1 to toggle.");
                    ui.label(format!("frames rendered: {}", frames));
                    ui.separator();

                    // Read the game's live Time.timeScale, let the slider drive it,
                    // and show the value that the game reports back — so you can see
                    // the UI is actually talking to the game both ways.
                    let reported = time::scale().unwrap_or(f32::NAN);
                    let mut scale = reported;
                    ui.label(format!("Time.timeScale (from game): {reported:.2}"));
                    if ui.add(egui::Slider::new(&mut scale, 0.0..=3.0)).changed() {
                        let _ = time::set_scale(scale);
                    }
                    ui.horizontal(|ui| {
                        if ui.button("pause (0x)").clicked() {
                            let _ = time::set_scale(0.0);
                        }
                        if ui.button("normal (1x)").clicked() {
                            let _ = time::set_scale(1.0);
                        }
                        if ui.button("fast (2x)").clicked() {
                            let _ = time::set_scale(2.0);
                        }
                    });

                    ui.separator();
                    // Keyboard test: click the field and type (editing keys and
                    // ctrl+A/C/V/X work).
                    ui.horizontal(|ui| {
                        ui.label("note:");
                        let mut note = NOTE.lock().unwrap_or_else(|e| e.into_inner());
                        ui.text_edit_singleline(&mut *note);
                    });

                    // Scroll test.
                    egui::ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                        for i in 1..=40 {
                            ui.label(format!("scrollable line {i}"));
                        }
                    });
                });
        })?;
        tracing::info!("egui overlay running (F1 to toggle)");
        Ok(())
    });
}

fn init_logging() {
    let filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}
