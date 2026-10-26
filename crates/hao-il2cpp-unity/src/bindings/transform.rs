//! `UnityEngine.Transform`.

use hao_il2cpp_runtime::Il2CppString;

use crate::{binding::bindings, Quaternion, Transform, Vector3};

bindings! { transform_checks;
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
}
