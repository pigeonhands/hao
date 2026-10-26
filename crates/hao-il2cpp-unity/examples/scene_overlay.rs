//! An example mod for Megabonk's Linux build (and most other Unity Il2Cpp games
//! with TextMeshPro): draws text over the game showing the current scene.
//!
//! ```sh
//! cargo build --release -p hao-il2cpp-unity --example scene_overlay
//! # Steam > Megabonk > Properties > Launch options:
//! LD_PRELOAD=/path/to/target/release/examples/libscene_overlay.so %command%
//! ```

use std::sync::Mutex;

use hao_il2cpp_unity::{on_scene_loaded, Color, LoadSceneMode, Scene, TextOverlay};

#[used]
#[link_section = ".init_array"]
static LOAD: extern "C" fn() = load;

/// Created on the first scene load, then reused.
struct State {
    overlay: TextOverlay,
    scenes_loaded: u32,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn scene_loaded(scene: Scene, mode: LoadSceneMode) -> hao_il2cpp_unity::runtime::Result<()> {
    let mut state = STATE.lock().unwrap();
    let state = match state.as_mut() {
        Some(state) => state,
        None => {
            let overlay = TextOverlay::new("hao overlay")?;
            overlay.set_color(Color::rgb(1.0, 0.85, 0.2))?;
            state.insert(State {
                overlay,
                scenes_loaded: 0,
            })
        }
    };
    state.scenes_loaded += 1;
    let how = if mode == LoadSceneMode::ADDITIVE {
        " (additive)"
    } else {
        ""
    };
    state.overlay.set_text(&format!(
        "hao | scene: {}{} | scenes loaded: {}",
        scene.name()?,
        how,
        state.scenes_loaded
    ))
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

extern "C" fn load() {
    init_logging();
    hao_il2cpp_unity::on_init(|_| {
        on_scene_loaded(|scene, mode| {
            if let Err(e) = scene_loaded(scene, mode) {
                tracing::error!("could not update the overlay: {}", e);
            }
        })?;
        tracing::info!("waiting for a scene to load");
        Ok(())
    });
}
