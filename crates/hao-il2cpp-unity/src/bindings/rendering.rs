//! The render pipeline hook, `UnityEngine.GL` drawing, and `Shader`/`Material`.

use hao_il2cpp_runtime::Il2CppString;

use crate::{binding::bindings, Camera, Color, Material, Shader};

bindings! { rendering_checks;
    // Unity calls FireOnPostRender from native after each camera renders.
    CAMERA_FIRE_ON_POST_RENDER: "UnityEngine.CoreModule" "UnityEngine" "Camera" "FireOnPostRender" (Camera) -> ();

    GL_BEGIN: "UnityEngine.CoreModule" "UnityEngine" "GL" "Begin" (i32) -> ();
    GL_END: "UnityEngine.CoreModule" "UnityEngine" "GL" "End" () -> ();
    GL_COLOR: "UnityEngine.CoreModule" "UnityEngine" "GL" "Color" (Color) -> ();
    GL_VERTEX3: "UnityEngine.CoreModule" "UnityEngine" "GL" "Vertex3" (f32, f32, f32) -> ();
    GL_LOAD_PIXEL_MATRIX: "UnityEngine.CoreModule" "UnityEngine" "GL" "LoadPixelMatrix" (f32, f32, f32, f32) -> ();
    GL_LOAD_ORTHO: "UnityEngine.CoreModule" "UnityEngine" "GL" "LoadOrtho" () -> ();
    GL_PUSH_MATRIX: "UnityEngine.CoreModule" "UnityEngine" "GL" "PushMatrix" () -> ();
    GL_POP_MATRIX: "UnityEngine.CoreModule" "UnityEngine" "GL" "PopMatrix" () -> ();

    SHADER_FIND: "UnityEngine.CoreModule" "UnityEngine" "Shader" "Find" (Il2CppString) -> Shader;
    MATERIAL_CTOR: "UnityEngine.CoreModule" "UnityEngine" "Material" ".ctor" (Material, Shader) -> ();
    MATERIAL_SET_PASS: "UnityEngine.CoreModule" "UnityEngine" "Material" "SetPass" (Material, i32) -> bool;
    MATERIAL_SET_COLOR: "UnityEngine.CoreModule" "UnityEngine" "Material" "set_color" (Material, Color) -> ();
}
