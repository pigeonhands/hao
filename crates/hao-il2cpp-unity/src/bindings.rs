//! Every Unity method the wrappers call.

use std::ffi::c_void;

use hao_il2cpp_runtime::{Il2CppString, Object};

use crate::{
    binding::bindings, Canvas, Color, Component, GameObject, LoadSceneMode, RectTransform,
    RenderMode, Scene, TextMeshPro, Transform, UnityObject, Vector2, Vector3,
};

bindings! {
    OBJECT_GET_NAME: "UnityEngine.CoreModule" "UnityEngine" "Object" "get_name" (UnityObject) -> Il2CppString;
    OBJECT_SET_NAME: "UnityEngine.CoreModule" "UnityEngine" "Object" "set_name" (UnityObject, Il2CppString) -> ();
    OBJECT_DONT_DESTROY_ON_LOAD: "UnityEngine.CoreModule" "UnityEngine" "Object" "DontDestroyOnLoad" (UnityObject) -> ();
    OBJECT_DESTROY: "UnityEngine.CoreModule" "UnityEngine" "Object" "Destroy" (UnityObject) -> ();

    GAME_OBJECT_CTOR: "UnityEngine.CoreModule" "UnityEngine" "GameObject" ".ctor" (GameObject, Il2CppString) -> ();
    GAME_OBJECT_FIND: "UnityEngine.CoreModule" "UnityEngine" "GameObject" "Find" (Il2CppString) -> GameObject;
    GAME_OBJECT_ADD_COMPONENT: "UnityEngine.CoreModule" "UnityEngine" "GameObject" "AddComponent" (GameObject, Object) -> Component;
    GAME_OBJECT_GET_COMPONENT: "UnityEngine.CoreModule" "UnityEngine" "GameObject" "GetComponent" (GameObject, Object) -> Component;
    GAME_OBJECT_TRANSFORM: "UnityEngine.CoreModule" "UnityEngine" "GameObject" "get_transform" (GameObject) -> Transform;
    GAME_OBJECT_SET_ACTIVE: "UnityEngine.CoreModule" "UnityEngine" "GameObject" "SetActive" (GameObject, bool) -> ();
    GAME_OBJECT_ACTIVE_SELF: "UnityEngine.CoreModule" "UnityEngine" "GameObject" "get_activeSelf" (GameObject) -> bool;

    COMPONENT_GAME_OBJECT: "UnityEngine.CoreModule" "UnityEngine" "Component" "get_gameObject" (Component) -> GameObject;
    COMPONENT_TRANSFORM: "UnityEngine.CoreModule" "UnityEngine" "Component" "get_transform" (Component) -> Transform;
    COMPONENT_GET_COMPONENT: "UnityEngine.CoreModule" "UnityEngine" "Component" "GetComponent" (Component, Object) -> Component;

    OBJECT_FIND_OBJECTS_OF_TYPE: "UnityEngine.CoreModule" "UnityEngine" "Object" "FindObjectsOfType" (Object) -> Object;

    TRANSFORM_SET_PARENT: "UnityEngine.CoreModule" "UnityEngine" "Transform" "SetParent" (Transform, Transform, bool) -> ();
    TRANSFORM_SET_LOCAL_SCALE: "UnityEngine.CoreModule" "UnityEngine" "Transform" "set_localScale" (Transform, Vector3) -> ();

    RECT_SET_ANCHOR_MIN: "UnityEngine.CoreModule" "UnityEngine" "RectTransform" "set_anchorMin" (RectTransform, Vector2) -> ();
    RECT_SET_ANCHOR_MAX: "UnityEngine.CoreModule" "UnityEngine" "RectTransform" "set_anchorMax" (RectTransform, Vector2) -> ();
    RECT_SET_PIVOT: "UnityEngine.CoreModule" "UnityEngine" "RectTransform" "set_pivot" (RectTransform, Vector2) -> ();
    RECT_SET_ANCHORED_POSITION: "UnityEngine.CoreModule" "UnityEngine" "RectTransform" "set_anchoredPosition" (RectTransform, Vector2) -> ();
    RECT_SET_SIZE_DELTA: "UnityEngine.CoreModule" "UnityEngine" "RectTransform" "set_sizeDelta" (RectTransform, Vector2) -> ();

    CANVAS_SET_RENDER_MODE: "UnityEngine.UIModule" "UnityEngine" "Canvas" "set_renderMode" (Canvas, RenderMode) -> ();
    CANVAS_SET_SORTING_ORDER: "UnityEngine.UIModule" "UnityEngine" "Canvas" "set_sortingOrder" (Canvas, i32) -> ();

    TMP_SET_TEXT: "Unity.TextMeshPro" "TMPro" "TMP_Text" "set_text" (TextMeshPro, Il2CppString) -> ();
    TMP_SET_FONT_SIZE: "Unity.TextMeshPro" "TMPro" "TMP_Text" "set_fontSize" (TextMeshPro, f32) -> ();
    TMP_SET_COLOR: "Unity.TextMeshPro" "TMPro" "TMP_Text" "set_color" (TextMeshPro, Color) -> ();

    SCENE_GET_NAME: "UnityEngine.CoreModule" "UnityEngine.SceneManagement" "Scene" "get_name" (*mut c_void) -> Il2CppString;
    SCENE_MANAGER_ACTIVE: "UnityEngine.CoreModule" "UnityEngine.SceneManagement" "SceneManager" "GetActiveScene" () -> Scene;
    SCENE_LOADED: "UnityEngine.CoreModule" "UnityEngine.SceneManagement" "SceneManager" "Internal_SceneLoaded" (Scene, LoadSceneMode) -> ();

    DEBUG_LOG: "UnityEngine.CoreModule" "UnityEngine" "Debug" "Log" (Object) -> ();
    DEBUG_LOG_WARNING: "UnityEngine.CoreModule" "UnityEngine" "Debug" "LogWarning" (Object) -> ();
    DEBUG_LOG_ERROR: "UnityEngine.CoreModule" "UnityEngine" "Debug" "LogError" (Object) -> ();

    TIME_DELTA: "UnityEngine.CoreModule" "UnityEngine" "Time" "get_deltaTime" () -> f32;
    TIME_GET_SCALE: "UnityEngine.CoreModule" "UnityEngine" "Time" "get_timeScale" () -> f32;
    TIME_SET_SCALE: "UnityEngine.CoreModule" "UnityEngine" "Time" "set_timeScale" (f32) -> ();
}
