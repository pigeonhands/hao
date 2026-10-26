//! Engine globals: `Time`, `Screen`, `Application`, `Debug`, `Quaternion`.

use hao_il2cpp_runtime::Object;

use crate::{binding::bindings, Quaternion};

bindings! { engine_checks;
    TIME_DELTA: "UnityEngine.CoreModule" "UnityEngine" "Time" "get_deltaTime" () -> f32;
    TIME_GET_SCALE: "UnityEngine.CoreModule" "UnityEngine" "Time" "get_timeScale" () -> f32;
    TIME_SET_SCALE: "UnityEngine.CoreModule" "UnityEngine" "Time" "set_timeScale" (f32) -> ();
    TIME_GET_TIME: "UnityEngine.CoreModule" "UnityEngine" "Time" "get_time" () -> f32;
    TIME_REALTIME: "UnityEngine.CoreModule" "UnityEngine" "Time" "get_realtimeSinceStartup" () -> f32;

    SCREEN_WIDTH: "UnityEngine.CoreModule" "UnityEngine" "Screen" "get_width" () -> i32;
    SCREEN_HEIGHT: "UnityEngine.CoreModule" "UnityEngine" "Screen" "get_height" () -> i32;

    APPLICATION_IS_PLAYING: "UnityEngine.CoreModule" "UnityEngine" "Application" "get_isPlaying" () -> bool;
    APPLICATION_QUIT: "UnityEngine.CoreModule" "UnityEngine" "Application" "Quit" (i32) -> ();

    DEBUG_LOG: "UnityEngine.CoreModule" "UnityEngine" "Debug" "Log" (Object) -> ();
    DEBUG_LOG_WARNING: "UnityEngine.CoreModule" "UnityEngine" "Debug" "LogWarning" (Object) -> ();
    DEBUG_LOG_ERROR: "UnityEngine.CoreModule" "UnityEngine" "Debug" "LogError" (Object) -> ();

    QUATERNION_EULER: "UnityEngine.CoreModule" "UnityEngine" "Quaternion" "Euler" (f32, f32, f32) -> Quaternion;
}
