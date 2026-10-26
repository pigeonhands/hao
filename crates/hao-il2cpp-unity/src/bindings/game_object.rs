//! `UnityEngine.GameObject`.

use hao_il2cpp_runtime::{Il2CppString, Object};

use crate::{binding::bindings, Component, GameObject, Transform};

bindings! { game_object_checks;
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
}
