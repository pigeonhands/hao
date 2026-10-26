//! Scenes, and running code when one loads.

use std::{
    ffi::c_void,
    sync::{Arc, Mutex},
};

use hao_il2cpp_runtime::{Il2CppValue, Original, Result, TypeInfo};

use crate::{bindings::*, values::LoadSceneMode};

/// `UnityEngine.SceneManagement.Scene`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Scene {
    pub handle: i32,
}

unsafe impl Il2CppValue for Scene {
    fn matches(ty: TypeInfo) -> bool {
        ty.is_value_type("UnityEngine.SceneManagement", "Scene")
    }
}

impl Scene {
    /// The scene being played.
    pub fn active() -> Result<Self> {
        SCENE_MANAGER_ACTIVE.call(())
    }

    pub fn name(&self) -> Result<String> {
        // Instance methods of structs take a pointer to the struct.
        let mut scene = *self;
        let name = SCENE_GET_NAME.call((&mut scene as *mut Scene as *mut c_void,))?;
        Ok(name.to_rust().unwrap_or_default())
    }
}

type Callback = Arc<dyn Fn(Scene, LoadSceneMode) + Send + Sync>;

static CALLBACKS: Mutex<Vec<Callback>> = Mutex::new(Vec::new());
static HOOKED: Mutex<bool> = Mutex::new(false);

/// Runs `callback` on Unity's main thread after each scene loads, which is a
/// good time to add objects to the game. Panics are caught and printed.
pub fn on_scene_loaded(
    callback: impl Fn(Scene, LoadSceneMode) + Send + Sync + 'static,
) -> Result<()> {
    CALLBACKS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(Arc::new(callback));
    let mut hooked = HOOKED.lock().unwrap_or_else(|e| e.into_inner());
    if !*hooked {
        // Unity calls this to raise `SceneManager.sceneLoaded`.
        SCENE_LOADED.method()?.hook(
            |scene: Scene, mode: LoadSceneMode, original: Original<_, ()>| {
                original.call((scene, mode));
                tracing::debug!(scene = scene.handle, mode = mode.0, "scene loaded");
                let callbacks = CALLBACKS.lock().unwrap_or_else(|e| e.into_inner()).clone();
                // Managed exceptions must flow back to Unity, not be caught here
                // (Rust can't catch foreign exceptions; catching would abort).
                for callback in callbacks {
                    callback(scene, mode);
                }
            },
        )?;
        *hooked = true;
        tracing::debug!("watching for scene loads");
    }
    Ok(())
}
