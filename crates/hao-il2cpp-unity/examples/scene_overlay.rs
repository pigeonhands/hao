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

extern "C" fn load() {
    hao_il2cpp_unity::on_init(|_| {
        on_scene_loaded(|scene, mode| {
            if let Err(e) = scene_loaded(scene, mode) {
                eprintln!("[scene_overlay] {}", e);
            }
        })?;
        eprintln!("[scene_overlay] waiting for a scene to load");
        Ok(())
    });
}
