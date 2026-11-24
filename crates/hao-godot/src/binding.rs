//! GodotSharp method bindings resolved at runtime by name + param count.
//!
//! # Declaring bindings
//!
//! Use the [`bindings!`](crate::bindings) macro to declare static bindings.
//! Each entry specifies a fully-qualified C# type, the method name, and the
//! number of parameters (used to disambiguate overloads):
//!
//! ```no_run
//! mod player {
//!     use hao_runtime::{Result, Runtime};
//!
//!     hao_godot::bindings! {
//!         GET_POSITION: "Godot.Node2D" "get_Position" 0;
//!         SET_POSITION: "Godot.Node2D" "set_Position" 1;
//!         GET_HEALTH:   "Game.Player"  "get_Health"   0;
//!     }
//!
//!     pub fn get_position(rt: &Runtime, node: usize) -> Result<u64> {
//!         GET_POSITION.call1(rt, node as u64)
//!     }
//!
//!     pub fn set_position(rt: &Runtime, node: usize, pos: u64) -> Result<()> {
//!         SET_POSITION.call2(rt, node as u64, pos)?;
//!         Ok(())
//!     }
//!
//!     pub fn get_health(rt: &Runtime, instance: usize) -> Result<f32> {
//!         Ok(f32::from_bits(GET_HEALTH.call1(rt, instance as u64)? as u32))
//!     }
//! }
//! ```
//!
//! Bindings resolve lazily on first use — no build-time dependency on
//! GodotSharp or any game assembly. The resolved pointer is cached in a
//! thread-safe [`OnceLock`] for the process lifetime.
//!
//! Pre-built bindings for common Godot types live in [`engine`](crate::engine).

use std::ffi::c_void;
use std::sync::OnceLock;

use hao_runtime::{Result, Runtime};

pub struct Binding {
    pub type_name: &'static str,
    pub method: &'static str,
    pub param_count: i32,
    cache: OnceLock<usize>,
}

impl Binding {
    pub const fn new(type_name: &'static str, method: &'static str, param_count: i32) -> Self {
        Self {
            type_name,
            method,
            param_count,
            cache: OnceLock::new(),
        }
    }

    pub fn resolve(&self, rt: &Runtime) -> Result<usize> {
        if let Some(&ptr) = self.cache.get() {
            return Ok(ptr);
        }
        tracing::info!(
            "Binding::resolve: {}::{} ({})",
            self.type_name,
            self.method,
            self.param_count
        );
        let ptr = rt
            .find_method_from_loaded("GodotSharp", self.type_name, self.method, self.param_count)
            .map(|p| p as usize)?;
        tracing::info!(
            "Binding::resolve: {}::{} -> 0x{:x}",
            self.type_name,
            self.method,
            ptr
        );
        let _ = self.cache.set(ptr);
        Ok(ptr)
    }

    pub fn call0(&self, rt: &Runtime) -> Result<u64> {
        let ptr = self.resolve(rt)?;
        Ok(unsafe { hao_runtime::jit::call_native(ptr, &[]) })
    }

    pub fn call1(&self, rt: &Runtime, a: u64) -> Result<u64> {
        let ptr = self.resolve(rt)?;
        Ok(unsafe { hao_runtime::jit::call_native(ptr, &[a]) })
    }

    pub fn call2(&self, rt: &Runtime, a: u64, b: u64) -> Result<u64> {
        let ptr = self.resolve(rt)?;
        Ok(unsafe { hao_runtime::jit::call_native(ptr, &[a, b]) })
    }

    pub fn call3(&self, rt: &Runtime, a: u64, b: u64, c: u64) -> Result<u64> {
        let ptr = self.resolve(rt)?;
        Ok(unsafe { hao_runtime::jit::call_native(ptr, &[a, b, c]) })
    }

    pub fn call_ptr(&self, rt: &Runtime, args: &[u64]) -> Result<*const c_void> {
        let ptr = self.resolve(rt)?;
        Ok(unsafe { hao_runtime::jit::call_native(ptr, args) } as *const c_void)
    }

    /// Call the method and read a 4×f32 struct return (e.g. Rect2, Vector4)
    /// from xmm0+xmm1 on x86-64.
    pub fn call_f32x4(&self, rt: &Runtime, args: &[u64]) -> Result<[f32; 4]> {
        let ptr = self.resolve(rt)?;
        Ok(unsafe { hao_runtime::jit::call_native_f32x4(ptr, args) })
    }
}

/// Declares GodotSharp bindings with lazy runtime resolution.
///
/// ```no_run
/// hao_godot::bindings! {
///     ENGINE_GET_MAIN_LOOP: "Godot.Engine" "GetMainLoop" 0;
///     NODE_GET_CHILD:       "Godot.Node"   "GetChild"    1;
/// }
/// ```
///
/// Each entry becomes a `pub static Binding` resolved on first use via
/// `find_method_from_loaded` with param-count matching (handles overloads).
#[macro_export]
macro_rules! bindings {
    ($($name:ident: $type_name:literal $method:literal $param_count:literal;)*) => {
        $(pub static $name: $crate::binding::Binding =
            $crate::binding::Binding::new($type_name, $method, $param_count);)*
    };
}
