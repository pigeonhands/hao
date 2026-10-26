//! Experimental egui overlay for Megabonk's Linux build (or any Unity Il2Cpp
//! game). Draws an egui window over the game. **Rendering is unverified** — the
//! shader/blend/colour handling may need tuning per game.
//!
//! ```sh
//! cargo build --release -p hao-il2cpp-unity --features egui --example egui_demo
//! # Steam > Megabonk > Properties > Launch options:
//! LD_PRELOAD=/path/to/target/release/examples/libegui_demo.so %command%
//! ```

use std::sync::atomic::{AtomicU64, Ordering};

use hao_il2cpp_unity::egui;

#[used]
#[link_section = ".init_array"]
static LOAD: extern "C" fn() = load;

static FRAMES: AtomicU64 = AtomicU64::new(0);

extern "C" fn load() {
    init_logging();
    hao_il2cpp_unity::on_init(|_| {
        egui::on_egui(|ctx| {
            let frames = FRAMES.fetch_add(1, Ordering::Relaxed);
            egui::Window::new("hao + egui")
                .default_pos([24.0, 24.0])
                .show(ctx, |ui| {
                    ui.label("Drawn over the game with egui.");
                    ui.label(format!("frames rendered: {}", frames));
                    ui.separator();
                    if ui.button("Click me").clicked() {
                        tracing::info!("egui button clicked");
                    }
                    ui.horizontal(|ui| {
                        ui.label("time scale:");
                        if ui.button("pause").clicked() {
                            let _ = hao_il2cpp_unity::time::set_scale(0.0);
                        }
                        if ui.button("resume").clicked() {
                            let _ = hao_il2cpp_unity::time::set_scale(1.0);
                        }
                    });
                });
        })?;
        tracing::info!("egui overlay running");
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
