//! Typed handles to Unity objects.

use hao_il2cpp_runtime::{Il2CppValue, Object, Result, TypeInfo};

use crate::{
    bindings::*,
    il2cpp,
    values::{Color, Quaternion, RenderMode, Vector2, Vector3},
};

macro_rules! object_type {
    ($(#[$doc:meta])* $name:ident = $assembly:literal $namespace:literal $class:literal) => {
        $(#[$doc])*
        #[repr(transparent)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(pub Object);

        unsafe impl Il2CppValue for $name {
            fn is_reference() -> bool {
                true
            }

            fn matches(ty: TypeInfo) -> bool {
                <Object as Il2CppValue>::matches(ty) && ty.namespace == $namespace && ty.name == $class
            }
        }

        impl $name {
            pub const ASSEMBLY: &'static str = $assembly;
            pub const NAMESPACE: &'static str = $namespace;
            pub const NAME: &'static str = $class;

            pub fn object(&self) -> Object {
                self.0
            }

            pub fn is_null(&self) -> bool {
                self.0.is_null()
            }

            pub fn name(&self) -> Result<String> {
                let name = OBJECT_GET_NAME.call((UnityObject(self.0),))?;
                Ok(name.to_rust().unwrap_or_default())
            }

            pub fn set_name(&self, name: &str) -> Result<()> {
                OBJECT_SET_NAME.call((UnityObject(self.0), il2cpp()?.string(name)?))
            }

            /// Keeps the object when a new scene loads. Only works for GameObjects;
            /// for assets (`Material`, `Texture2D`) use [`set_hide_flags`](Self::set_hide_flags).
            pub fn dont_destroy_on_load(&self) -> Result<()> {
                OBJECT_DONT_DESTROY_ON_LOAD.call((UnityObject(self.0),))
            }

            /// Sets the object's `HideFlags`. `HideFlags::HIDE_AND_DONT_SAVE` keeps a
            /// runtime-created asset from being unloaded on scene load.
            pub fn set_hide_flags(&self, flags: crate::HideFlags) -> Result<()> {
                OBJECT_SET_HIDE_FLAGS.call((UnityObject(self.0), flags))
            }

            /// Whether the object is still alive (Unity's `(bool)obj`): `false` once
            /// it has been destroyed, even though the managed wrapper still exists.
            pub fn is_alive(&self) -> Result<bool> {
                OBJECT_IS_ALIVE.call((UnityObject(self.0),))
            }

            /// Destroys the object at the end of the frame.
            pub fn destroy(&self) -> Result<()> {
                OBJECT_DESTROY.call((UnityObject(self.0),))
            }
        }
    };
}

/// A component type, for [`GameObject::add_component`].
pub trait ComponentType: Il2CppValue {
    const ASSEMBLY: &'static str;
    const NAMESPACE: &'static str;
    const NAME: &'static str;

    fn from_object(object: Object) -> Self;
    fn object(&self) -> Object;

    fn game_object(&self) -> Result<GameObject> {
        COMPONENT_GAME_OBJECT.call((Component(self.object()),))
    }

    fn transform(&self) -> Result<Transform> {
        COMPONENT_TRANSFORM.call((Component(self.object()),))
    }

    /// `GetComponent<U>()` on this component's game object, or `None` if absent.
    fn get_component<U: ComponentType>(&self) -> Result<Option<U>> {
        let found =
            COMPONENT_GET_COMPONENT.call((Component(self.object()), component_type::<U>()?))?;
        Ok((!found.is_null()).then(|| U::from_object(found.0)))
    }

    /// Whether the component is enabled. For `Behaviour`-derived components
    /// (`MonoBehaviour`, `Canvas`, …); meaningless on a plain `Transform`.
    fn is_enabled(&self) -> Result<bool> {
        BEHAVIOUR_GET_ENABLED.call((Component(self.object()),))
    }

    fn set_enabled(&self, enabled: bool) -> Result<()> {
        BEHAVIOUR_SET_ENABLED.call((Component(self.object()), enabled))
    }
}

/// The `System.Type` of a component type `T`, for the Unity query methods.
fn component_type<T: ComponentType>() -> Result<Object> {
    Ok(il2cpp()?
        .class(T::ASSEMBLY, T::NAMESPACE, T::NAME)?
        .type_object())
}

/// Collects the elements of a managed `T[]` whose object is `array`.
fn read_object_array<T: ComponentType>(array: Object) -> Vec<T> {
    if array.is_null() {
        return Vec::new();
    }
    let Some(il2cpp) = il2cpp().ok() else {
        return Vec::new();
    };
    let length = unsafe { (il2cpp.api().il2cpp_array_length)(array.as_ptr()) };
    // Elements follow the array header (object header, bounds, length): 4 pointers.
    let base = 4 * std::mem::size_of::<usize>();
    (0..length)
        .map(|i| {
            T::from_object(unsafe {
                hao_il2cpp_runtime::read_field(array, base + i * std::mem::size_of::<usize>())
            })
        })
        .collect()
}

/// Every active object of type `T` in the loaded scenes (`Object.FindObjectsOfType`).
///
/// Call this on Unity's main thread (e.g. in [`on_update`](crate::on_update) or
/// [`on_scene_loaded`](crate::on_scene_loaded)).
pub fn find_objects_of_type<T: ComponentType>() -> Result<Vec<T>> {
    let array = OBJECT_FIND_OBJECTS_OF_TYPE.call((component_type::<T>()?,))?;
    Ok(read_object_array(array))
}

/// The first active object of type `T`, or `None` (`Object.FindObjectOfType`).
pub fn find_object_of_type<T: ComponentType>() -> Result<Option<T>> {
    Ok(find_objects_of_type::<T>()?.into_iter().next())
}

macro_rules! component_type {
    ($(#[$doc:meta])* $name:ident = $assembly:literal $namespace:literal $class:literal) => {
        object_type!($(#[$doc])* $name = $assembly $namespace $class);

        impl ComponentType for $name {
            const ASSEMBLY: &'static str = $assembly;
            const NAMESPACE: &'static str = $namespace;
            const NAME: &'static str = $class;

            fn from_object(object: Object) -> Self {
                Self(object)
            }

            fn object(&self) -> Object {
                self.0
            }
        }
    };
}

object_type!(
    /// `UnityEngine.Object`, the base of everything Unity manages.
    UnityObject = "UnityEngine.CoreModule" "UnityEngine" "Object"
);
object_type!(
    /// `UnityEngine.GameObject`.
    GameObject = "UnityEngine.CoreModule" "UnityEngine" "GameObject"
);
component_type!(
    /// `UnityEngine.Component`.
    Component = "UnityEngine.CoreModule" "UnityEngine" "Component"
);
component_type!(
    /// `UnityEngine.Transform`: position in the scene and the parent/child hierarchy.
    Transform = "UnityEngine.CoreModule" "UnityEngine" "Transform"
);
component_type!(
    /// `UnityEngine.RectTransform`, the transform of UI elements.
    RectTransform = "UnityEngine.CoreModule" "UnityEngine" "RectTransform"
);
component_type!(
    /// `UnityEngine.Canvas`, the root of a UI.
    Canvas = "UnityEngine.UIModule" "UnityEngine" "Canvas"
);
component_type!(
    /// `TMPro.TextMeshProUGUI`, TextMeshPro text in a UI.
    TextMeshPro = "Unity.TextMeshPro" "TMPro" "TextMeshProUGUI"
);
component_type!(
    /// `UnityEngine.Camera`, passed to [`on_render`](crate::on_render) callbacks.
    Camera = "UnityEngine.CoreModule" "UnityEngine" "Camera"
);
object_type!(
    /// `UnityEngine.Shader`.
    Shader = "UnityEngine.CoreModule" "UnityEngine" "Shader"
);
object_type!(
    /// `UnityEngine.Material`, e.g. for a [`gl`](crate::gl) drawing pass.
    Material = "UnityEngine.CoreModule" "UnityEngine" "Material"
);

impl Shader {
    /// `Shader.Find(name)`, or `None` if the shader isn't present.
    pub fn find(name: &str) -> Result<Option<Shader>> {
        let shader = SHADER_FIND.call((il2cpp()?.string(name)?,))?;
        Ok((!shader.is_null()).then_some(shader))
    }
}

impl Material {
    /// `new Material(shader)`.
    pub fn new(shader: Shader) -> Result<Material> {
        let object = Material(
            il2cpp()?
                .class(Self::ASSEMBLY, Self::NAMESPACE, Self::NAME)?
                .new_object(),
        );
        MATERIAL_CTOR.call((object, shader))?;
        Ok(object)
    }

    /// Activates pass `pass` for drawing (returns whether it can be rendered).
    /// Call this before a [`gl`](crate::gl) primitive so it has a material.
    pub fn set_pass(&self, pass: i32) -> Result<bool> {
        MATERIAL_SET_PASS.call((*self, pass))
    }

    pub fn set_color(&self, color: Color) -> Result<()> {
        MATERIAL_SET_COLOR.call((*self, color))
    }

    pub fn set_main_texture(&self, texture: Texture2D) -> Result<()> {
        MATERIAL_SET_MAIN_TEXTURE.call((*self, texture.0))
    }
}

object_type!(
    /// `UnityEngine.Texture2D`.
    Texture2D = "UnityEngine.CoreModule" "UnityEngine" "Texture2D"
);

impl Texture2D {
    /// `new Texture2D(width, height, format, mipChain: false)`.
    pub fn new(width: i32, height: i32, format: crate::TextureFormat) -> Result<Texture2D> {
        let object = Texture2D(
            il2cpp()?
                .class(Self::ASSEMBLY, Self::NAMESPACE, Self::NAME)?
                .new_object(),
        );
        TEXTURE2D_CTOR.call((object, width, height, format, false))?;
        Ok(object)
    }

    /// Uploads raw pixel bytes (matching the texture's format), then applies them.
    pub fn load_raw(&self, bytes: &[u8]) -> Result<()> {
        TEXTURE2D_LOAD_RAW.call((*self, il2cpp()?.byte_array(bytes)?))?;
        TEXTURE2D_APPLY.call((*self,))
    }
}

object_type!(
    /// `UnityEngine.RenderTexture`, an off-screen render target.
    RenderTexture = "UnityEngine.CoreModule" "UnityEngine" "RenderTexture"
);

impl RenderTexture {
    /// `new RenderTexture(width, height, depth)` (depth is the depth-buffer bits,
    /// e.g. 0 for a colour-only target).
    pub fn new(width: i32, height: i32, depth: i32) -> Result<RenderTexture> {
        let object = RenderTexture(
            il2cpp()?
                .class(Self::ASSEMBLY, Self::NAMESPACE, Self::NAME)?
                .new_object(),
        );
        RENDER_TEXTURE_CTOR.call((object, width, height, depth))?;
        Ok(object)
    }

    /// `RenderTexture.active`, the target GL and camera drawing goes to.
    pub fn active() -> Result<Option<RenderTexture>> {
        let rt = RENDER_TEXTURE_GET_ACTIVE.call(())?;
        Ok((!rt.is_null()).then_some(rt))
    }

    /// Sets `RenderTexture.active`. Pass `None` to restore the back buffer.
    pub fn set_active(target: Option<RenderTexture>) -> Result<()> {
        RENDER_TEXTURE_SET_ACTIVE.call((target.unwrap_or(RenderTexture(Object::NULL)),))
    }
}

component_type!(
    /// `UnityEngine.UI.RawImage`, a UI element that draws a raw [`Texture2D`] or
    /// [`RenderTexture`].
    RawImage = "UnityEngine.UI" "UnityEngine.UI" "RawImage"
);

impl RawImage {
    /// The texture the image displays.
    pub fn set_texture(&self, texture: Object) -> Result<()> {
        RAW_IMAGE_SET_TEXTURE.call((*self, texture))
    }

    /// The material the image is drawn with (its shader decides the blend mode).
    pub fn set_material(&self, material: Material) -> Result<()> {
        GRAPHIC_SET_MATERIAL.call((*self, material))
    }
}

impl GameObject {
    /// `new GameObject(name)`.
    pub fn new(name: &str) -> Result<Self> {
        let il2cpp = il2cpp()?;
        let object = GameObject(
            il2cpp
                .class(Self::ASSEMBLY, Self::NAMESPACE, Self::NAME)?
                .new_object(),
        );
        GAME_OBJECT_CTOR.call((object, il2cpp.string(name)?))?;
        Ok(object)
    }

    /// `GameObject.Find(name)`: an active object by name or path, such as `Canvas/Menu`.
    pub fn find(name: &str) -> Result<Option<Self>> {
        let found = GAME_OBJECT_FIND.call((il2cpp()?.string(name)?,))?;
        Ok((!found.is_null()).then_some(found))
    }

    /// `AddComponent(typeof(T))`.
    pub fn add_component<T: ComponentType>(&self) -> Result<T> {
        let class = il2cpp()?.class(T::ASSEMBLY, T::NAMESPACE, T::NAME)?;
        let component = GAME_OBJECT_ADD_COMPONENT.call((*self, class.type_object()))?;
        Ok(T::from_object(component.0))
    }

    /// `GetComponent<T>()`, or `None` if the object has no such component.
    pub fn get_component<T: ComponentType>(&self) -> Result<Option<T>> {
        let found = GAME_OBJECT_GET_COMPONENT.call((*self, component_type::<T>()?))?;
        Ok((!found.is_null()).then(|| T::from_object(found.0)))
    }

    pub fn transform(&self) -> Result<Transform> {
        GAME_OBJECT_TRANSFORM.call((*self,))
    }

    /// The transform as a [`RectTransform`], which objects with UI components have.
    pub fn rect_transform(&self) -> Result<Option<RectTransform>> {
        let transform = self.transform()?;
        let class = unsafe { il2cpp()?.class_of(transform.0) };
        Ok((class.name() == RectTransform::NAME).then_some(RectTransform(transform.0)))
    }

    pub fn set_active(&self, active: bool) -> Result<()> {
        GAME_OBJECT_SET_ACTIVE.call((*self, active))
    }

    /// Whether the object itself is active (its own flag, ignoring parents).
    pub fn is_active(&self) -> Result<bool> {
        GAME_OBJECT_ACTIVE_SELF.call((*self,))
    }

    /// Whether the object is active, taking its parents into account.
    pub fn is_active_in_hierarchy(&self) -> Result<bool> {
        GAME_OBJECT_ACTIVE_IN_HIERARCHY.call((*self,))
    }

    pub fn tag(&self) -> Result<String> {
        Ok(GAME_OBJECT_GET_TAG
            .call((*self,))?
            .to_rust()
            .unwrap_or_default())
    }

    pub fn set_tag(&self, tag: &str) -> Result<()> {
        GAME_OBJECT_SET_TAG.call((*self, il2cpp()?.string(tag)?))
    }

    pub fn compare_tag(&self, tag: &str) -> Result<bool> {
        GAME_OBJECT_COMPARE_TAG.call((*self, il2cpp()?.string(tag)?))
    }

    pub fn layer(&self) -> Result<i32> {
        GAME_OBJECT_GET_LAYER.call((*self,))
    }

    pub fn set_layer(&self, layer: i32) -> Result<()> {
        GAME_OBJECT_SET_LAYER.call((*self, layer))
    }
}

impl Transform {
    pub fn set_parent(&self, parent: Transform, world_position_stays: bool) -> Result<()> {
        TRANSFORM_SET_PARENT.call((*self, parent, world_position_stays))
    }

    pub fn set_local_scale(&self, scale: Vector3) -> Result<()> {
        TRANSFORM_SET_LOCAL_SCALE.call((*self, scale))
    }

    /// World-space position.
    pub fn position(&self) -> Result<Vector3> {
        TRANSFORM_GET_POSITION.call((*self,))
    }

    pub fn set_position(&self, position: Vector3) -> Result<()> {
        TRANSFORM_SET_POSITION.call((*self, position))
    }

    /// Position relative to the parent.
    pub fn local_position(&self) -> Result<Vector3> {
        TRANSFORM_GET_LOCAL_POSITION.call((*self,))
    }

    pub fn set_local_position(&self, position: Vector3) -> Result<()> {
        TRANSFORM_SET_LOCAL_POSITION.call((*self, position))
    }

    /// World-space rotation as Euler angles in degrees.
    pub fn euler_angles(&self) -> Result<Vector3> {
        TRANSFORM_GET_EULER_ANGLES.call((*self,))
    }

    pub fn rotation(&self) -> Result<Quaternion> {
        TRANSFORM_GET_ROTATION.call((*self,))
    }

    pub fn set_rotation(&self, rotation: Quaternion) -> Result<()> {
        TRANSFORM_SET_ROTATION.call((*self, rotation))
    }

    /// The parent transform, or `None` at the scene root.
    pub fn parent(&self) -> Result<Option<Transform>> {
        let parent = TRANSFORM_GET_PARENT.call((*self,))?;
        Ok((!parent.is_null()).then_some(parent))
    }

    pub fn child_count(&self) -> Result<i32> {
        TRANSFORM_GET_CHILD_COUNT.call((*self,))
    }

    pub fn child(&self, index: i32) -> Result<Transform> {
        TRANSFORM_GET_CHILD.call((*self, index))
    }

    /// A descendant by name or path (e.g. `"HUD/Health"`), or `None`.
    pub fn find(&self, path: &str) -> Result<Option<Transform>> {
        let found = TRANSFORM_FIND.call((*self, il2cpp()?.string(path)?))?;
        Ok((!found.is_null()).then_some(found))
    }
}

impl RectTransform {
    pub fn as_transform(&self) -> Transform {
        Transform(self.0)
    }

    /// Anchors and pivot, each from (0, 0) at the bottom left of the parent to (1, 1) at the top right.
    pub fn set_anchors(&self, min: Vector2, max: Vector2, pivot: Vector2) -> Result<()> {
        RECT_SET_ANCHOR_MIN.call((*self, min))?;
        RECT_SET_ANCHOR_MAX.call((*self, max))?;
        RECT_SET_PIVOT.call((*self, pivot))
    }

    /// The pivot's position relative to the anchors, in pixels with y up.
    pub fn set_anchored_position(&self, position: Vector2) -> Result<()> {
        RECT_SET_ANCHORED_POSITION.call((*self, position))
    }

    pub fn set_size(&self, size: Vector2) -> Result<()> {
        RECT_SET_SIZE_DELTA.call((*self, size))
    }
}

impl Canvas {
    pub fn set_render_mode(&self, mode: RenderMode) -> Result<()> {
        CANVAS_SET_RENDER_MODE.call((*self, mode))
    }

    /// Canvases with a higher order draw on top.
    pub fn set_sorting_order(&self, order: i32) -> Result<()> {
        CANVAS_SET_SORTING_ORDER.call((*self, order))
    }
}

impl TextMeshPro {
    pub fn set_text(&self, text: &str) -> Result<()> {
        TMP_SET_TEXT.call((*self, il2cpp()?.string(text)?))
    }

    pub fn set_font_size(&self, size: f32) -> Result<()> {
        TMP_SET_FONT_SIZE.call((*self, size))
    }

    pub fn set_color(&self, color: Color) -> Result<()> {
        TMP_SET_COLOR.call((*self, color))
    }
}
