//! The render pipeline hook, `UnityEngine.GL` drawing, and `Shader`/`Material`.

use hao_il2cpp_runtime::{Il2CppString, Object};

use crate::{
    binding::bindings, Camera, Color, Material, Rect, RenderTexture, Shader, Texture2D,
    TextureFormat,
};

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

    GL_TEXCOORD2: "UnityEngine.CoreModule" "UnityEngine" "GL" "TexCoord2" (f32, f32) -> ();
    // The 3-arg overload is a managed wrapper IL2CPP strips; this is the native one.
    GL_CLEAR: "UnityEngine.CoreModule" "UnityEngine" "GL" "Clear" (bool, bool, Color, f32) -> ();

    // An off-screen render target, so egui can draw into a texture and then be
    // shown on a top-most overlay canvas (above screen-space UI).
    RENDER_TEXTURE_CTOR: "UnityEngine.CoreModule" "UnityEngine" "RenderTexture" ".ctor" (RenderTexture, i32, i32, i32) -> ();
    RENDER_TEXTURE_GET_ACTIVE: "UnityEngine.CoreModule" "UnityEngine" "RenderTexture" "get_active" () -> RenderTexture;
    RENDER_TEXTURE_SET_ACTIVE: "UnityEngine.CoreModule" "UnityEngine" "RenderTexture" "set_active" (RenderTexture) -> ();

    SHADER_FIND: "UnityEngine.CoreModule" "UnityEngine" "Shader" "Find" (Il2CppString) -> Shader;
    MATERIAL_CTOR: "UnityEngine.CoreModule" "UnityEngine" "Material" ".ctor" (Material, Shader) -> ();
    MATERIAL_SET_PASS: "UnityEngine.CoreModule" "UnityEngine" "Material" "SetPass" (Material, i32) -> bool;
    MATERIAL_SET_COLOR: "UnityEngine.CoreModule" "UnityEngine" "Material" "set_color" (Material, Color) -> ();
    MATERIAL_SET_MAIN_TEXTURE: "UnityEngine.CoreModule" "UnityEngine" "Material" "set_mainTexture" (Material, Object) -> ();

    // The (width, height, format, mipChain) constructor.
    TEXTURE2D_CTOR: "UnityEngine.CoreModule" "UnityEngine" "Texture2D" ".ctor" (Texture2D, i32, i32, TextureFormat, bool) -> ();
    TEXTURE2D_LOAD_RAW: "UnityEngine.CoreModule" "UnityEngine" "Texture2D" "LoadRawTextureData" (Texture2D, Object) -> ();
    TEXTURE2D_APPLY: "UnityEngine.CoreModule" "UnityEngine" "Texture2D" "Apply" (Texture2D) -> ();
    // Debugging: read pixels back from the active render target to inspect them.
    TEXTURE2D_READ_PIXELS: "UnityEngine.CoreModule" "UnityEngine" "Texture2D" "ReadPixels" (Texture2D, Rect, i32, i32, bool) -> ();
    TEXTURE2D_GET_PIXEL: "UnityEngine.CoreModule" "UnityEngine" "Texture2D" "GetPixel" (Texture2D, i32, i32) -> Color;
}
