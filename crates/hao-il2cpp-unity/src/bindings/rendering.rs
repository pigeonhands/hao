//! The render pipeline hook and `UnityEngine.GL` immediate-mode drawing.

use crate::{binding::bindings, Camera, Color};

bindings! { rendering_checks;
    // Unity calls FireOnPostRender from native after each camera renders.
    CAMERA_FIRE_ON_POST_RENDER: "UnityEngine.CoreModule" "UnityEngine" "Camera" "FireOnPostRender" (Camera) -> ();

    GL_BEGIN: "UnityEngine.CoreModule" "UnityEngine" "GL" "Begin" (i32) -> ();
    GL_END: "UnityEngine.CoreModule" "UnityEngine" "GL" "End" () -> ();
    GL_COLOR: "UnityEngine.CoreModule" "UnityEngine" "GL" "Color" (Color) -> ();
    GL_VERTEX3: "UnityEngine.CoreModule" "UnityEngine" "GL" "Vertex3" (f32, f32, f32) -> ();
    GL_LOAD_PIXEL_MATRIX: "UnityEngine.CoreModule" "UnityEngine" "GL" "LoadPixelMatrix" (f32, f32, f32, f32) -> ();
    GL_LOAD_ORTHO: "UnityEngine.CoreModule" "UnityEngine" "GL" "LoadOrtho" () -> ();
}
