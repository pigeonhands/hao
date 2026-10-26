//! Unity structs and enums, laid out as the game passes them.

use hao_il2cpp_runtime::{element, Il2CppValue, TypeInfo};

macro_rules! value_type {
    ($(#[$doc:meta])* $name:ident = $namespace:literal $class:literal { $($field:ident: $t:ty),* }) => {
        $(#[$doc])*
        #[repr(C)]
        #[derive(Debug, Clone, Copy, PartialEq, Default)]
        pub struct $name {
            $(pub $field: $t,)*
        }

        unsafe impl Il2CppValue for $name {
            fn matches(ty: TypeInfo) -> bool {
                ty.is_value_type($namespace, $class)
            }
        }
    };
}

value_type!(
    /// `UnityEngine.Vector2`.
    Vector2 = "UnityEngine" "Vector2" { x: f32, y: f32 }
);
value_type!(
    /// `UnityEngine.Vector3`.
    Vector3 = "UnityEngine" "Vector3" { x: f32, y: f32, z: f32 }
);
value_type!(
    /// `UnityEngine.Color`, with components from 0 to 1.
    Color = "UnityEngine" "Color" { r: f32, g: f32, b: f32, a: f32 }
);

impl Vector2 {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl Vector3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

impl Color {
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }
}

macro_rules! int_enum {
    ($(#[$doc:meta])* $name:ident = $namespace:literal $class:literal { $($variant:ident = $value:literal),* }) => {
        $(#[$doc])*
        #[repr(transparent)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(pub i32);

        impl $name {
            $(pub const $variant: Self = Self($value);)*
        }

        unsafe impl Il2CppValue for $name {
            fn matches(ty: TypeInfo) -> bool {
                !ty.byref && ty.is_enum && ty.element == element::I4 && ty.namespace == $namespace && ty.name == $class
            }
        }
    };
}

int_enum!(
    /// `UnityEngine.SceneManagement.LoadSceneMode`.
    LoadSceneMode = "UnityEngine.SceneManagement" "LoadSceneMode" { SINGLE = 0, ADDITIVE = 1 }
);
int_enum!(
    /// `UnityEngine.RenderMode`, how a [`Canvas`](crate::Canvas) is drawn.
    RenderMode = "UnityEngine" "RenderMode" { SCREEN_SPACE_OVERLAY = 0, SCREEN_SPACE_CAMERA = 1, WORLD_SPACE = 2 }
);
