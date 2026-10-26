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
value_type!(
    /// `UnityEngine.Quaternion`, a rotation. Use [`Quaternion::euler`] to build one.
    Quaternion = "UnityEngine" "Quaternion" { x: f32, y: f32, z: f32, w: f32 }
);

impl Vector2 {
    pub const ZERO: Self = Self::new(0.0, 0.0);
    pub const ONE: Self = Self::new(1.0, 1.0);

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn magnitude(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn normalized(self) -> Self {
        let m = self.magnitude();
        if m > 0.0 {
            Self::new(self.x / m, self.y / m)
        } else {
            Self::ZERO
        }
    }
}

impl std::ops::Add for Vector2 {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Self::new(self.x + o.x, self.y + o.y)
    }
}

impl std::ops::Sub for Vector2 {
    type Output = Self;
    fn sub(self, o: Self) -> Self {
        Self::new(self.x - o.x, self.y - o.y)
    }
}

impl std::ops::Mul<f32> for Vector2 {
    type Output = Self;
    fn mul(self, s: f32) -> Self {
        Self::new(self.x * s, self.y * s)
    }
}

impl Vector3 {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);
    pub const ONE: Self = Self::new(1.0, 1.0, 1.0);
    pub const UP: Self = Self::new(0.0, 1.0, 0.0);
    pub const FORWARD: Self = Self::new(0.0, 0.0, 1.0);

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn dot(self, o: Self) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    pub fn magnitude(self) -> f32 {
        self.dot(self).sqrt()
    }

    pub fn normalized(self) -> Self {
        let m = self.magnitude();
        if m > 0.0 {
            self * (1.0 / m)
        } else {
            Self::ZERO
        }
    }

    pub fn distance(self, o: Self) -> f32 {
        (self - o).magnitude()
    }
}

impl std::ops::Add for Vector3 {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Self::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl std::ops::Sub for Vector3 {
    type Output = Self;
    fn sub(self, o: Self) -> Self {
        Self::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

impl std::ops::Mul<f32> for Vector3 {
    type Output = Self;
    fn mul(self, s: f32) -> Self {
        Self::new(self.x * s, self.y * s, self.z * s)
    }
}

impl Color {
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    pub const RED: Self = Self::rgb(1.0, 0.0, 0.0);
    pub const GREEN: Self = Self::rgb(0.0, 1.0, 0.0);
    pub const BLUE: Self = Self::rgb(0.0, 0.0, 1.0);
    pub const YELLOW: Self = Self::rgb(1.0, 0.92, 0.016);
    pub const CLEAR: Self = Self::rgba(0.0, 0.0, 0.0, 0.0);

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::rgba(r, g, b, 1.0)
    }

    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// The colour with its alpha replaced.
    pub const fn with_alpha(self, a: f32) -> Self {
        Self {
            r: self.r,
            g: self.g,
            b: self.b,
            a,
        }
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
int_enum!(
    /// `UnityEngine.TextureFormat`.
    TextureFormat = "UnityEngine" "TextureFormat" { ALPHA8 = 1, RGBA32 = 4, ARGB32 = 5, RGB24 = 3 }
);
int_enum!(
    /// `UnityEngine.KeyCode`, for [`input`](crate::input). A few common keys are
    /// named; any other can be made with `KeyCode(value)`.
    KeyCode = "UnityEngine" "KeyCode" {
        BACKSPACE = 8, TAB = 9, RETURN = 13, ESCAPE = 27, SPACE = 32,
        LEFT = 276, RIGHT = 275, UP = 273, DOWN = 274,
        ALPHA0 = 48, ALPHA1 = 49, ALPHA2 = 50, ALPHA3 = 51,
        A = 97, B = 98, C = 99, D = 100, E = 101, F = 102, G = 103, H = 104,
        I = 105, J = 106, K = 107, L = 108, M = 109, N = 110, O = 111, P = 112,
        Q = 113, R = 114, S = 115, T = 116, U = 117, V = 118, W = 119, X = 120,
        Y = 121, Z = 122,
        F1 = 282, F2 = 283, F3 = 284, F4 = 285, F5 = 286, F6 = 287,
        LEFT_SHIFT = 304, LEFT_CONTROL = 306, LEFT_ALT = 308
    }
);
