//! An example mod for Megabonk's Linux build: adds a text overlay to the game's
//! UI that shows the current scene.
//!
//! It hooks `SceneManager.Internal_SceneLoaded`, which Unity calls on the main
//! thread whenever a scene finishes loading, and builds the UI from the game's
//! own Unity types the first time it runs: a `GameObject` with a `Canvas`, and a
//! child with a TextMeshPro text component.
//!
//! ```sh
//! cargo build --release -p hao-il2cpp-runtime --example scene_overlay
//! # Steam > Megabonk > Properties > Launch options:
//! LD_PRELOAD=/path/to/target/release/examples/libscene_overlay.so %command%
//! ```

use std::{ffi::c_void, sync::OnceLock};

use hao_il2cpp_runtime::{
    GcHandle, Il2Cpp, Il2CppString, Il2CppValue, Object, Original, Result, TypeInfo,
};

/// `UnityEngine.SceneManagement.Scene`, a struct holding a handle.
#[repr(C)]
#[derive(Clone, Copy)]
struct Scene {
    handle: i32,
}

unsafe impl Il2CppValue for Scene {
    fn matches(ty: TypeInfo) -> bool {
        ty.is_value_type("UnityEngine.SceneManagement", "Scene")
    }
}

/// `UnityEngine.Vector2`.
#[repr(C)]
#[derive(Clone, Copy)]
struct Vector2 {
    x: f32,
    y: f32,
}

unsafe impl Il2CppValue for Vector2 {
    fn matches(ty: TypeInfo) -> bool {
        ty.is_value_type("UnityEngine", "Vector2")
    }
}

/// `UnityEngine.Color`.
#[repr(C)]
#[derive(Clone, Copy)]
struct Color {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

unsafe impl Il2CppValue for Color {
    fn matches(ty: TypeInfo) -> bool {
        ty.is_value_type("UnityEngine", "Color")
    }
}

/// `RenderMode.ScreenSpaceOverlay`. Enums are passed as their underlying integer.
const SCREEN_SPACE_OVERLAY: i32 = 0;

/// The overlay's text component, kept alive across scene loads.
static OVERLAY: OnceLock<GcHandle> = OnceLock::new();

/// Calls an instance method found by name and parameter count.
fn call<Args: hao_il2cpp_runtime::MethodArgs, R: Il2CppValue>(
    class: &hao_il2cpp_runtime::Class,
    name: &str,
    argc: i32,
    args: Args,
) -> Result<R> {
    class.method(name, argc)?.call(args)
}

/// `new GameObject(name)`, kept across scene loads.
fn game_object(il2cpp: &'static Il2Cpp, name: &str) -> Result<Object> {
    let class = il2cpp.class("UnityEngine.CoreModule", "UnityEngine", "GameObject")?;
    let object = class.new_object();
    call::<_, ()>(&class, ".ctor", 1, (object, il2cpp.string(name)?))?;
    let unity_object = il2cpp.class("UnityEngine.CoreModule", "UnityEngine", "Object")?;
    unity_object
        .method("DontDestroyOnLoad", 1)?
        .call::<_, ()>((object,))?;
    Ok(object)
}

/// `game_object.AddComponent(typeof(component))`.
fn add_component(
    il2cpp: &'static Il2Cpp,
    game_object: Object,
    component: &hao_il2cpp_runtime::Class,
) -> Result<Object> {
    let class = il2cpp.class("UnityEngine.CoreModule", "UnityEngine", "GameObject")?;
    call(
        &class,
        "AddComponent",
        1,
        (game_object, component.type_object()),
    )
}

fn create_overlay(il2cpp: &'static Il2Cpp) -> Result<Object> {
    let canvas_class = il2cpp.class("UnityEngine.UIModule", "UnityEngine", "Canvas")?;
    let transform = il2cpp.class("UnityEngine.CoreModule", "UnityEngine", "Transform")?;
    let rect = il2cpp.class("UnityEngine.CoreModule", "UnityEngine", "RectTransform")?;
    let text_class = il2cpp.class("Unity.TextMeshPro", "TMPro", "TextMeshProUGUI")?;
    let game_object_class = il2cpp.class("UnityEngine.CoreModule", "UnityEngine", "GameObject")?;

    let root = game_object(il2cpp, "hao overlay")?;
    let canvas = add_component(il2cpp, root, &canvas_class)?;
    call::<_, ()>(
        &canvas_class,
        "set_renderMode",
        1,
        (canvas, SCREEN_SPACE_OVERLAY),
    )?;
    call::<_, ()>(&canvas_class, "set_sortingOrder", 1, (canvas, 32000))?;

    let label = game_object(il2cpp, "hao overlay text")?;
    let text = add_component(il2cpp, label, &text_class)?;
    let parent: Object = call(&game_object_class, "get_transform", 0, (root,))?;
    // Adding a UI component turned the label's transform into a RectTransform.
    let label_rect: Object = call(&game_object_class, "get_transform", 0, (label,))?;
    call::<_, ()>(&transform, "SetParent", 2, (label_rect, parent, false))?;

    // Pin the text to the top left corner.
    let top_left = Vector2 { x: 0.0, y: 1.0 };
    call::<_, ()>(&rect, "set_anchorMin", 1, (label_rect, top_left))?;
    call::<_, ()>(&rect, "set_anchorMax", 1, (label_rect, top_left))?;
    call::<_, ()>(&rect, "set_pivot", 1, (label_rect, top_left))?;
    call::<_, ()>(
        &rect,
        "set_anchoredPosition",
        1,
        (label_rect, Vector2 { x: 16.0, y: -16.0 }),
    )?;
    call::<_, ()>(
        &rect,
        "set_sizeDelta",
        1,
        (label_rect, Vector2 { x: 800.0, y: 60.0 }),
    )?;

    call::<_, ()>(&text_class, "set_fontSize", 1, (text, 28.0f32))?;
    let color = Color {
        r: 1.0,
        g: 0.85,
        b: 0.2,
        a: 1.0,
    };
    call::<_, ()>(&text_class, "set_color", 1, (text, color))?;
    Ok(text)
}

fn show_scene(il2cpp: &'static Il2Cpp, mut scene: Scene) -> Result<()> {
    let text = match OVERLAY.get() {
        Some(overlay) => overlay.target(),
        None => {
            let text = create_overlay(il2cpp)?;
            OVERLAY.get_or_init(|| GcHandle::new(il2cpp, text)).target()
        }
    };
    // Instance methods of structs take a pointer to the struct.
    let scene_class = il2cpp.class(
        "UnityEngine.CoreModule",
        "UnityEngine.SceneManagement",
        "Scene",
    )?;
    let name: Il2CppString = call(
        &scene_class,
        "get_name",
        0,
        (&mut scene as *mut Scene as *mut c_void,),
    )?;
    let message = format!("hao | scene: {}", name.to_rust().unwrap_or_default());
    let text_class = il2cpp.class("Unity.TextMeshPro", "TMPro", "TMP_Text")?;
    call::<_, ()>(&text_class, "set_text", 1, (text, il2cpp.string(&message)?))
}

#[used]
#[link_section = ".init_array"]
static LOAD: extern "C" fn() = load;

extern "C" fn load() {
    hao_il2cpp_runtime::on_init(|il2cpp| {
        let scenes = il2cpp.class(
            "UnityEngine.CoreModule",
            "UnityEngine.SceneManagement",
            "SceneManager",
        )?;
        // `static void Internal_SceneLoaded(Scene scene, LoadSceneMode mode)`
        scenes.method("Internal_SceneLoaded", 2)?.hook(
            move |scene: Scene, mode: i32, original: Original<_, ()>| {
                original.call((scene, mode));
                if let Err(e) = show_scene(il2cpp, scene) {
                    eprintln!("[scene_overlay] {}", e);
                }
            },
        )?;
        eprintln!("[scene_overlay] waiting for a scene to load");
        Ok(())
    });
}
