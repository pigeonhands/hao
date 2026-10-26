//! An example mod for Megabonk's Linux build (and most other Unity Il2Cpp games
//! with TextMeshPro): draws a live overlay over the game showing the current
//! scene, the frame count and a smoothed FPS, updated every frame.
//!
//! ```sh
//! cargo build --release -p hao-il2cpp-unity --example scene_overlay
//! # Steam > Megabonk > Properties > Launch options:
//! LD_PRELOAD=/path/to/target/release/examples/libscene_overlay.so %command%
//! ```

use std::sync::Mutex;

use hao_il2cpp_unity::{on_scene_loaded, on_update, time, Color, TextOverlay};

#[used]
#[link_section = ".init_array"]
static LOAD: extern "C" fn() = load;

/// Created on the first frame, then reused. Only touched on Unity's main thread.
struct State {
    overlay: TextOverlay,
    scene: String,
    scenes_loaded: u32,
    fps: f32,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

/// Runs every frame: refresh the readout.
fn update() -> hao_il2cpp_unity::runtime::Result<()> {
    let mut guard = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let state = match guard.as_mut() {
        Some(state) => state,
        None => {
            let overlay = TextOverlay::new("hao overlay")?;
            overlay.set_color(Color::rgb(1.0, 0.85, 0.2))?;
            guard.insert(State {
                overlay,
                scene: String::new(),
                scenes_loaded: 0,
                fps: 0.0,
            })
        }
    };
    // Exponential moving average of the instantaneous FPS.
    let delta = time::delta()?.max(1e-4);
    state.fps += (1.0 / delta - state.fps) * 0.1;
    state.overlay.set_text(&format!(
        "hao | scene: {} | loaded: {} | {:.0} fps",
        state.scene, state.scenes_loaded, state.fps
    ))
}

extern "C" fn load() {
    init_logging();
    hao_il2cpp_unity::on_init(|_| {
        on_scene_loaded(|scene, _| {
            let mut guard = STATE.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(state) = guard.as_mut() {
                state.scene = scene.name().unwrap_or_default();
                state.scenes_loaded += 1;
            }
        })?;
        on_update(|| {
            if let Err(e) = update() {
                tracing::error!("could not update the overlay: {}", e);
            }
        })?;
        tracing::info!("overlay running");
        Ok(())
    });
}

/// Sends this mod's and hao's logs to the game's stderr. `RUST_LOG` picks what's
/// shown, e.g. `RUST_LOG=debug` for hao's hooks and lookups.
fn init_logging() {
    let filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}
