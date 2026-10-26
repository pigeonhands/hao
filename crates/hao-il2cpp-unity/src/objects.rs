//! Typed handles to Unity objects.

use hao_il2cpp_runtime::{Il2CppValue, Object, Result, TypeInfo};

use crate::{
    bindings::*,
    il2cpp,
    values::{Color, RenderMode, Vector2, Vector3},
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

            /// Keeps the object when a new scene loads.
            pub fn dont_destroy_on_load(&self) -> Result<()> {
                OBJECT_DONT_DESTROY_ON_LOAD.call((UnityObject(self.0),))
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

    pub fn is_active(&self) -> Result<bool> {
        GAME_OBJECT_ACTIVE_SELF.call((*self,))
    }
}

impl Transform {
    pub fn set_parent(&self, parent: Transform, world_position_stays: bool) -> Result<()> {
        TRANSFORM_SET_PARENT.call((*self, parent, world_position_stays))
    }

    pub fn set_local_scale(&self, scale: Vector3) -> Result<()> {
        TRANSFORM_SET_LOCAL_SCALE.call((*self, scale))
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
