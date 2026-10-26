//! Typed Unity wrappers for mods of Unity Il2Cpp games, on top of
//! [`hao_il2cpp_runtime`].
//!
//! The runtime crate finds and hooks methods by name. This crate wraps common
//! Unity functionality the same way for any Unity game: game objects and
//! components, UI, scenes, logging and time. Each Unity method is looked up the
//! first time it's used and checked against the game's real signature, so a
//! game whose Unity version differs gives an error rather than a crash.
//!
//! ```no_run
//! use std::sync::OnceLock;
//!
//! use hao_il2cpp_unity::{on_scene_loaded, TextOverlay};
//!
//! static OVERLAY: OnceLock<TextOverlay> = OnceLock::new();
//!
//! hao_il2cpp_unity::on_init(|_| {
//!     on_scene_loaded(|scene, _| {
//!         let overlay = OVERLAY.get_or_init(|| TextOverlay::new("my mod").unwrap());
//!         overlay.set_text(&format!("scene: {}", scene.name().unwrap())).unwrap();
//!     })
//! });
//! ```
//!
//! Unity's API may only be used from its main thread: inside hooks of methods
//! the game calls on it, such as [`on_scene_loaded`]'s callbacks.

mod binding;
mod bindings;
#[cfg(feature = "egui")]
pub mod egui {
    //! egui integration (requires the `egui` feature). See [`on_egui`].
    pub use crate::egui_backend::on_egui;
    pub use ::egui::*;
}
#[cfg(feature = "egui")]
mod egui_backend;
pub mod gl;
mod lifecycle;
mod objects;
mod overlay;
mod scene;
mod values;

pub use hao_il2cpp_runtime as runtime;
#[cfg(unix)]
pub use hao_il2cpp_runtime::on_init;
use hao_il2cpp_runtime::{Error, Il2Cpp, Result};
pub use lifecycle::{on_render, on_update};
pub use objects::{
    find_object_of_type, find_objects_of_type, Camera, Canvas, Component, ComponentType,
    GameObject, Material, RectTransform, Shader, TextMeshPro, Texture2D, Transform, UnityObject,
};
pub use overlay::TextOverlay;
pub use scene::{on_scene_loaded, Scene};
pub use values::{
    Color, KeyCode, LoadSceneMode, Quaternion, RenderMode, TextureFormat, Vector2, Vector3,
};

impl Quaternion {
    /// A rotation from Euler angles in degrees (`Quaternion.Euler`).
    pub fn euler(x: f32, y: f32, z: f32) -> Result<Self> {
        bindings::QUATERNION_EULER.call((x, y, z))
    }
}

use bindings::*;

fn il2cpp() -> Result<&'static Il2Cpp> {
    Il2Cpp::get().ok_or_else(|| Error::NotFound("the Il2Cpp runtime".into()))
}

/// `Debug.Log`: writes to the game's log (`Player.log`).
pub fn log(message: &str) -> Result<()> {
    DEBUG_LOG.call((il2cpp()?.string(message)?.0,))
}

/// `Debug.LogWarning`.
pub fn log_warning(message: &str) -> Result<()> {
    DEBUG_LOG_WARNING.call((il2cpp()?.string(message)?.0,))
}

/// `Debug.LogError`.
pub fn log_error(message: &str) -> Result<()> {
    DEBUG_LOG_ERROR.call((il2cpp()?.string(message)?.0,))
}

/// `Time`.
pub mod time {
    use super::*;

    /// Seconds since the last frame.
    pub fn delta() -> Result<f32> {
        TIME_DELTA.call(())
    }

    /// How fast game time passes; 0 pauses most games.
    pub fn scale() -> Result<f32> {
        TIME_GET_SCALE.call(())
    }

    pub fn set_scale(scale: f32) -> Result<()> {
        TIME_SET_SCALE.call((scale,))
    }

    /// Seconds since the game started (scaled by [`scale`]).
    pub fn time() -> Result<f32> {
        TIME_GET_TIME.call(())
    }

    /// Real seconds since startup, unaffected by [`scale`].
    pub fn realtime_since_startup() -> Result<f32> {
        TIME_REALTIME.call(())
    }
}

/// `UnityEngine.Screen`.
pub mod screen {
    use super::*;

    pub fn width() -> Result<i32> {
        SCREEN_WIDTH.call(())
    }

    pub fn height() -> Result<i32> {
        SCREEN_HEIGHT.call(())
    }

    /// `(width, height)` in pixels.
    pub fn size() -> Result<(i32, i32)> {
        Ok((width()?, height()?))
    }
}

/// `UnityEngine.Input` (the legacy input manager).
pub mod input {
    use super::*;

    /// Whether `key` is held this frame.
    pub fn key(key: KeyCode) -> Result<bool> {
        INPUT_GET_KEY.call((key,))
    }

    /// Whether `key` was pressed this frame.
    pub fn key_down(key: KeyCode) -> Result<bool> {
        INPUT_GET_KEY_DOWN.call((key,))
    }

    /// Whether `key` was released this frame.
    pub fn key_up(key: KeyCode) -> Result<bool> {
        INPUT_GET_KEY_UP.call((key,))
    }

    /// Whether mouse `button` (0 left, 1 right, 2 middle) is held.
    pub fn mouse_button(button: i32) -> Result<bool> {
        INPUT_GET_MOUSE_BUTTON.call((button,))
    }

    /// The cursor position in pixels, origin at the bottom left.
    pub fn mouse_position() -> Result<Vector3> {
        INPUT_MOUSE_POSITION.call(())
    }
}

/// `UnityEngine.Application`.
pub mod application {
    use super::*;

    pub fn is_playing() -> Result<bool> {
        APPLICATION_IS_PLAYING.call(())
    }

    /// Quits the game with an exit code.
    pub fn quit(exit_code: i32) -> Result<()> {
        APPLICATION_QUIT.call((exit_code,))
    }
}

/// Resolves every wrapped Unity method against the running game, returning
/// the ones that are missing or have a different signature.
pub fn check_bindings() -> Vec<(String, Error)> {
    bindings::all()
        .into_iter()
        .filter_map(|b| b.check().err().map(|e| (b.describe(), e)))
        .collect()
}
