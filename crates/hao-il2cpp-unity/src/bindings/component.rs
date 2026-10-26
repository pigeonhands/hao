//! `UnityEngine.Component` and `UnityEngine.Behaviour`.

use hao_il2cpp_runtime::Object;

use crate::{binding::bindings, Component, GameObject, Transform};

bindings! { component_checks;
    COMPONENT_GAME_OBJECT: "UnityEngine.CoreModule" "UnityEngine" "Component" "get_gameObject" (Component) -> GameObject;
    COMPONENT_TRANSFORM: "UnityEngine.CoreModule" "UnityEngine" "Component" "get_transform" (Component) -> Transform;
    COMPONENT_GET_COMPONENT: "UnityEngine.CoreModule" "UnityEngine" "Component" "GetComponent" (Component, Object) -> Component;

    BEHAVIOUR_GET_ENABLED: "UnityEngine.CoreModule" "UnityEngine" "Behaviour" "get_enabled" (Component) -> bool;
    BEHAVIOUR_SET_ENABLED: "UnityEngine.CoreModule" "UnityEngine" "Behaviour" "set_enabled" (Component, bool) -> ();
}
