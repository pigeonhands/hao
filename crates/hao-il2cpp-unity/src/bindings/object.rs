//! `UnityEngine.Object`, the base of everything Unity manages.

use hao_il2cpp_runtime::{Il2CppString, Object};

use crate::{binding::bindings, HideFlags, UnityObject};

bindings! { object_checks;
    OBJECT_GET_NAME: "UnityEngine.CoreModule" "UnityEngine" "Object" "get_name" (UnityObject) -> Il2CppString;
    OBJECT_SET_NAME: "UnityEngine.CoreModule" "UnityEngine" "Object" "set_name" (UnityObject, Il2CppString) -> ();
    OBJECT_DONT_DESTROY_ON_LOAD: "UnityEngine.CoreModule" "UnityEngine" "Object" "DontDestroyOnLoad" (UnityObject) -> ();
    OBJECT_DESTROY: "UnityEngine.CoreModule" "UnityEngine" "Object" "Destroy" (UnityObject) -> ();
    OBJECT_FIND_OBJECTS_OF_TYPE: "UnityEngine.CoreModule" "UnityEngine" "Object" "FindObjectsOfType" (Object) -> Object;
    OBJECT_SET_HIDE_FLAGS: "UnityEngine.CoreModule" "UnityEngine" "Object" "set_hideFlags" (UnityObject, HideFlags) -> ();
    // Unity's `(bool)obj`: false once the object has been destroyed.
    OBJECT_IS_ALIVE: "UnityEngine.CoreModule" "UnityEngine" "Object" "op_Implicit" (UnityObject) -> bool;
}
