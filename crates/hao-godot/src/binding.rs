//! GodotSharp method bindings resolved at runtime by name + param count.

use std::ffi::c_void;
use std::sync::OnceLock;

use hao_runtime::jit::call_native;
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
        let ptr = rt
            .find_method_from_loaded("GodotSharp", self.type_name, self.method, self.param_count)
            .map(|p| p as usize)?;
        let _ = self.cache.set(ptr);
        Ok(ptr)
    }

    pub fn call0(&self, rt: &Runtime) -> Result<u64> {
        let ptr = self.resolve(rt)?;
        Ok(unsafe { call_native(ptr, &[]) })
    }

    pub fn call1(&self, rt: &Runtime, a: u64) -> Result<u64> {
        let ptr = self.resolve(rt)?;
        Ok(unsafe { call_native(ptr, &[a]) })
    }

    pub fn call2(&self, rt: &Runtime, a: u64, b: u64) -> Result<u64> {
        let ptr = self.resolve(rt)?;
        Ok(unsafe { call_native(ptr, &[a, b]) })
    }

    pub fn call3(&self, rt: &Runtime, a: u64, b: u64, c: u64) -> Result<u64> {
        let ptr = self.resolve(rt)?;
        Ok(unsafe { call_native(ptr, &[a, b, c]) })
    }

    pub fn call_ptr(&self, rt: &Runtime, args: &[u64]) -> Result<*const c_void> {
        let ptr = self.resolve(rt)?;
        Ok(unsafe { call_native(ptr, args) } as *const c_void)
    }
}

/// Declares GodotSharp bindings with lazy runtime resolution.
///
/// ```ignore
/// bindings! {
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
