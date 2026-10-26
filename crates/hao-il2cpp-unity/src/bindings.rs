//! Every Unity method the wrappers call.

use std::ffi::c_void;

use hao_il2cpp_runtime::{Il2CppString, Object};

use crate::{
    binding::bindings, Camera, Canvas, Color, Component, GameObject, KeyCode, LoadSceneMode,
    Quaternion, RectTransform, RenderMode, Scene, TextMeshPro, Transform, UnityObject, Vector2,
    Vector3,
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
    GAME_OBJECT_ACTIVE_IN_HIERARCHY: "UnityEngine.CoreModule" "UnityEngine" "GameObject" "get_activeInHierarchy" (GameObject) -> bool;
    GAME_OBJECT_GET_TAG: "UnityEngine.CoreModule" "UnityEngine" "GameObject" "get_tag" (GameObject) -> Il2CppString;
    GAME_OBJECT_SET_TAG: "UnityEngine.CoreModule" "UnityEngine" "GameObject" "set_tag" (GameObject, Il2CppString) -> ();
    GAME_OBJECT_COMPARE_TAG: "UnityEngine.CoreModule" "UnityEngine" "GameObject" "CompareTag" (GameObject, Il2CppString) -> bool;
    GAME_OBJECT_GET_LAYER: "UnityEngine.CoreModule" "UnityEngine" "GameObject" "get_layer" (GameObject) -> i32;
    GAME_OBJECT_SET_LAYER: "UnityEngine.CoreModule" "UnityEngine" "GameObject" "set_layer" (GameObject, i32) -> ();

    BEHAVIOUR_GET_ENABLED: "UnityEngine.CoreModule" "UnityEngine" "Behaviour" "get_enabled" (Component) -> bool;
    BEHAVIOUR_SET_ENABLED: "UnityEngine.CoreModule" "UnityEngine" "Behaviour" "set_enabled" (Component, bool) -> ();

    COMPONENT_GAME_OBJECT: "UnityEngine.CoreModule" "UnityEngine" "Component" "get_gameObject" (Component) -> GameObject;
    COMPONENT_TRANSFORM: "UnityEngine.CoreModule" "UnityEngine" "Component" "get_transform" (Component) -> Transform;
    COMPONENT_GET_COMPONENT: "UnityEngine.CoreModule" "UnityEngine" "Component" "GetComponent" (Component, Object) -> Component;

    OBJECT_FIND_OBJECTS_OF_TYPE: "UnityEngine.CoreModule" "UnityEngine" "Object" "FindObjectsOfType" (Object) -> Object;

    TRANSFORM_SET_PARENT: "UnityEngine.CoreModule" "UnityEngine" "Transform" "SetParent" (Transform, Transform, bool) -> ();
    TRANSFORM_SET_LOCAL_SCALE: "UnityEngine.CoreModule" "UnityEngine" "Transform" "set_localScale" (Transform, Vector3) -> ();
    TRANSFORM_GET_POSITION: "UnityEngine.CoreModule" "UnityEngine" "Transform" "get_position" (Transform) -> Vector3;
    TRANSFORM_SET_POSITION: "UnityEngine.CoreModule" "UnityEngine" "Transform" "set_position" (Transform, Vector3) -> ();
    TRANSFORM_GET_LOCAL_POSITION: "UnityEngine.CoreModule" "UnityEngine" "Transform" "get_localPosition" (Transform) -> Vector3;
    TRANSFORM_SET_LOCAL_POSITION: "UnityEngine.CoreModule" "UnityEngine" "Transform" "set_localPosition" (Transform, Vector3) -> ();
    TRANSFORM_GET_EULER_ANGLES: "UnityEngine.CoreModule" "UnityEngine" "Transform" "get_eulerAngles" (Transform) -> Vector3;
    TRANSFORM_GET_ROTATION: "UnityEngine.CoreModule" "UnityEngine" "Transform" "get_rotation" (Transform) -> Quaternion;
    TRANSFORM_SET_ROTATION: "UnityEngine.CoreModule" "UnityEngine" "Transform" "set_rotation" (Transform, Quaternion) -> ();
    TRANSFORM_GET_PARENT: "UnityEngine.CoreModule" "UnityEngine" "Transform" "get_parent" (Transform) -> Transform;
    TRANSFORM_GET_CHILD_COUNT: "UnityEngine.CoreModule" "UnityEngine" "Transform" "get_childCount" (Transform) -> i32;
    TRANSFORM_GET_CHILD: "UnityEngine.CoreModule" "UnityEngine" "Transform" "GetChild" (Transform, i32) -> Transform;
    TRANSFORM_FIND: "UnityEngine.CoreModule" "UnityEngine" "Transform" "Find" (Transform, Il2CppString) -> Transform;

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

    // Unity calls FireOnPostRender from native after each camera renders.
    CAMERA_FIRE_ON_POST_RENDER: "UnityEngine.CoreModule" "UnityEngine" "Camera" "FireOnPostRender" (Camera) -> ();

    // GL immediate-mode drawing, valid inside a render callback.
    GL_BEGIN: "UnityEngine.CoreModule" "UnityEngine" "GL" "Begin" (i32) -> ();
    GL_END: "UnityEngine.CoreModule" "UnityEngine" "GL" "End" () -> ();
    GL_COLOR: "UnityEngine.CoreModule" "UnityEngine" "GL" "Color" (Color) -> ();
    GL_VERTEX3: "UnityEngine.CoreModule" "UnityEngine" "GL" "Vertex3" (f32, f32, f32) -> ();
    GL_LOAD_PIXEL_MATRIX: "UnityEngine.CoreModule" "UnityEngine" "GL" "LoadPixelMatrix" (f32, f32, f32, f32) -> ();
    GL_LOAD_ORTHO: "UnityEngine.CoreModule" "UnityEngine" "GL" "LoadOrtho" () -> ();

    SCREEN_WIDTH: "UnityEngine.CoreModule" "UnityEngine" "Screen" "get_width" () -> i32;
    SCREEN_HEIGHT: "UnityEngine.CoreModule" "UnityEngine" "Screen" "get_height" () -> i32;

    INPUT_GET_KEY: "UnityEngine.InputLegacyModule" "UnityEngine" "Input" "GetKey" (KeyCode) -> bool;
    INPUT_GET_KEY_DOWN: "UnityEngine.InputLegacyModule" "UnityEngine" "Input" "GetKeyDown" (KeyCode) -> bool;
    INPUT_GET_KEY_UP: "UnityEngine.InputLegacyModule" "UnityEngine" "Input" "GetKeyUp" (KeyCode) -> bool;
    INPUT_GET_MOUSE_BUTTON: "UnityEngine.InputLegacyModule" "UnityEngine" "Input" "GetMouseButton" (i32) -> bool;
    INPUT_MOUSE_POSITION: "UnityEngine.InputLegacyModule" "UnityEngine" "Input" "get_mousePosition" () -> Vector3;

    APPLICATION_IS_PLAYING: "UnityEngine.CoreModule" "UnityEngine" "Application" "get_isPlaying" () -> bool;
    APPLICATION_QUIT: "UnityEngine.CoreModule" "UnityEngine" "Application" "Quit" (i32) -> ();

    TIME_GET_TIME: "UnityEngine.CoreModule" "UnityEngine" "Time" "get_time" () -> f32;
    TIME_REALTIME: "UnityEngine.CoreModule" "UnityEngine" "Time" "get_realtimeSinceStartup" () -> f32;

    QUATERNION_EULER: "UnityEngine.CoreModule" "UnityEngine" "Quaternion" "Euler" (f32, f32, f32) -> Quaternion;
}
