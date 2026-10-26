//! Experimental egui overlay for Megabonk's Linux build (or any Unity Il2Cpp
//! game). Draws an egui window over the game. **Rendering is unverified** — the
//! shader/blend/colour handling may need tuning per game.
//!
//! ```sh
//! cargo build --release -p hao-il2cpp-unity --features egui --example egui_demo
//! # Steam > Megabonk > Properties > Launch options:
//! LD_PRELOAD=/path/to/target/release/examples/libegui_demo.so %command%
//! ```

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

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
            // egui's default window fill is slightly see-through; make it opaque so
            // it reads clearly over the game.
            ctx.style_mut(|style| {
                style.visuals.window_fill = egui::Color32::from_rgb(27, 27, 27);
                style.visuals.panel_fill = egui::Color32::from_rgb(27, 27, 27);
            });
            let frames = FRAMES.fetch_add(1, Ordering::Relaxed);
            egui::Window::new("hao + egui")
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
