//! `UnityEngine.Input`, the legacy input manager.

use hao_il2cpp_runtime::Il2CppString;

use crate::{binding::bindings, KeyCode, Vector2, Vector3};

bindings! { input_checks;
    INPUT_GET_KEY: "UnityEngine.InputLegacyModule" "UnityEngine" "Input" "GetKey" (KeyCode) -> bool;
    INPUT_GET_KEY_DOWN: "UnityEngine.InputLegacyModule" "UnityEngine" "Input" "GetKeyDown" (KeyCode) -> bool;
    INPUT_GET_KEY_UP: "UnityEngine.InputLegacyModule" "UnityEngine" "Input" "GetKeyUp" (KeyCode) -> bool;
    INPUT_GET_MOUSE_BUTTON: "UnityEngine.InputLegacyModule" "UnityEngine" "Input" "GetMouseButton" (i32) -> bool;
    INPUT_MOUSE_POSITION: "UnityEngine.InputLegacyModule" "UnityEngine" "Input" "get_mousePosition" () -> Vector3;
    INPUT_SCROLL_DELTA: "UnityEngine.InputLegacyModule" "UnityEngine" "Input" "get_mouseScrollDelta" () -> Vector2;
    INPUT_STRING: "UnityEngine.InputLegacyModule" "UnityEngine" "Input" "get_inputString" () -> Il2CppString;
}
