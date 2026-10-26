//! UI: `RectTransform`, `Canvas`, TextMeshPro text.

use hao_il2cpp_runtime::{Il2CppString, Object};

use crate::{
    binding::bindings, Canvas, Color, Material, RawImage, RectTransform, RenderMode, TextMeshPro,
    Vector2,
};

bindings! { ui_checks;
    RAW_IMAGE_SET_TEXTURE: "UnityEngine.UI" "UnityEngine.UI" "RawImage" "set_texture" (RawImage, Object) -> ();
    // set_material is declared on Graphic, which RawImage derives from.
    GRAPHIC_SET_MATERIAL: "UnityEngine.UI" "UnityEngine.UI" "Graphic" "set_material" (RawImage, Material) -> ();

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
}
