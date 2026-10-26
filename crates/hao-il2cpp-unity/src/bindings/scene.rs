//! Scenes and scene loading.

use std::ffi::c_void;

use hao_il2cpp_runtime::Il2CppString;

use crate::{binding::bindings, LoadSceneMode, Scene};

bindings! { scene_checks;
    SCENE_GET_NAME: "UnityEngine.CoreModule" "UnityEngine.SceneManagement" "Scene" "get_name" (*mut c_void) -> Il2CppString;
    SCENE_MANAGER_ACTIVE: "UnityEngine.CoreModule" "UnityEngine.SceneManagement" "SceneManager" "GetActiveScene" () -> Scene;
    SCENE_LOADED: "UnityEngine.CoreModule" "UnityEngine.SceneManagement" "SceneManager" "Internal_SceneLoaded" (Scene, LoadSceneMode) -> ();
}
