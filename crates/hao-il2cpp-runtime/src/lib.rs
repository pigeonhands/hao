//! Hook and call into a Unity Il2Cpp game from inside its process, the way
//! MelonLoader mods do.
//!
//! A mod is a `cdylib` loaded into the game (on Linux with `LD_PRELOAD`).
//! [`on_init`] runs once the game's Il2Cpp runtime has started, and from there
//! methods are found by name and hooked:
//!
//! ```no_run
//! use hao_il2cpp_runtime::{original, Il2Cpp, MethodInfo, Object};
//!
//! // `int PlayerHealth.Heal(float amount, bool allowOverheal)`. Il2Cpp passes the
//! // instance first and the method's `MethodInfo*` last.
//! extern "C" fn heal(this: Object, amount: f32, overheal: bool, method: MethodInfo) -> i32 {
//!     let original = original!(extern "C" fn(Object, f32, bool, MethodInfo) -> i32);
//!     original(this, amount * 2.0, overheal, method)
//! }
//!
//! hao_il2cpp_runtime::on_init(|il2cpp: &Il2Cpp| {
//!     let class = il2cpp.class("Assembly-CSharp", "Assets.Scripts.Inventory__Items__Pickups", "PlayerHealth")?;
//!     unsafe { class.method("Heal", 2)?.hook(heal as *mut u8)? };
//!     Ok(())
//! });
//! ```

mod api;
mod error;
#[cfg(unix)]
mod loader;

use std::{
    ffi::{c_void, CStr, CString},
    path::Path,
    sync::OnceLock,
};

use libloading::Library;

pub use api::{Api, Ptr};
pub use error::{Error, Result};
#[cfg(unix)]
pub use loader::on_init;

/// A managed object (`Il2CppObject*`).
pub type Object = Ptr;
/// The hidden last argument of every Il2Cpp method (`const MethodInfo*`).
pub type MethodInfo = Ptr;

/// The game binary's file name.
pub const GAME_ASSEMBLY: &str = if cfg!(windows) { "GameAssembly.dll" } else { "GameAssembly.so" };

static INSTANCE: OnceLock<Il2Cpp> = OnceLock::new();

/// The game's Il2Cpp runtime.
pub struct Il2Cpp {
    api: Api,
    _library: Library,
}

fn cstring(value: &str) -> Result<CString> {
    CString::new(value).map_err(|_| Error::Nul(value.to_owned()))
}

unsafe fn c_str<'a>(value: *const std::ffi::c_char) -> &'a str {
    if value.is_null() {
        ""
    } else {
        unsafe { CStr::from_ptr(value) }.to_str().unwrap_or("")
    }
}

impl Il2Cpp {
    fn install(library: Library) -> Result<&'static Il2Cpp> {
        let api = Api::load(&library)?;
        Ok(INSTANCE.get_or_init(|| Il2Cpp { api, _library: library }))
    }

    /// The runtime of the game binary already loaded in this process.
    pub fn loaded() -> Result<&'static Il2Cpp> {
        if let Some(il2cpp) = INSTANCE.get() {
            return Ok(il2cpp);
        }
        #[cfg(unix)]
        let library = unsafe {
            libloading::os::unix::Library::open(Some(GAME_ASSEMBLY), libc::RTLD_NOW | libc::RTLD_NOLOAD)
                .map(Library::from)
        };
        #[cfg(windows)]
        let library = libloading::os::windows::Library::open_already_loaded(GAME_ASSEMBLY).map(Library::from);
        Self::install(library.map_err(Error::Load)?)
    }

    /// Loads a game binary into this process, e.g. to run a game's code without Unity.
    pub fn load(path: impl AsRef<Path>) -> Result<&'static Il2Cpp> {
        if let Some(il2cpp) = INSTANCE.get() {
            return Ok(il2cpp);
        }
        let library = unsafe { Library::new(path.as_ref()) }.map_err(Error::Load)?;
        Self::install(library)
    }

    /// The instance set up by [`Il2Cpp::loaded`] or [`Il2Cpp::load`].
    pub fn get() -> Option<&'static Il2Cpp> {
        INSTANCE.get()
    }

    pub fn api(&self) -> &Api {
        &self.api
    }

    /// Starts the runtime without Unity, given the game's `<Game>_Data/il2cpp_data` folder.
    /// Only the game's pure .NET code can run this way; anything calling into Unity will fail.
    pub fn init_standalone(&self, data_dir: impl AsRef<Path>) -> Result<()> {
        let data_dir = cstring(&data_dir.as_ref().to_string_lossy())?;
        unsafe {
            (self.api.il2cpp_set_data_dir)(data_dir.as_ptr());
            if (self.api.il2cpp_init)(c"hao".as_ptr()) == 0 {
                return Err(Error::Init);
            }
        }
        Ok(())
    }

    /// Lets the calling thread run managed code. Needed on threads the game didn't start.
    pub fn attach_thread(&self) {
        unsafe { (self.api.il2cpp_thread_attach)((self.api.il2cpp_domain_get)()) };
    }

    /// Finds a class, e.g. `class("Assembly-CSharp", "Game.Player", "Health")`.
    pub fn class(&'static self, assembly: &str, namespace: &str, name: &str) -> Result<Class> {
        let (assembly_c, namespace_c, name_c) = (cstring(assembly)?, cstring(namespace)?, cstring(name)?);
        let class = unsafe {
            let domain = (self.api.il2cpp_domain_get)();
            let opened = (self.api.il2cpp_domain_assembly_open)(domain, assembly_c.as_ptr());
            if opened.is_null() {
                return Err(Error::NotFound(format!("assembly {}", assembly)));
            }
            let image = (self.api.il2cpp_assembly_get_image)(opened);
            (self.api.il2cpp_class_from_name)(image, namespace_c.as_ptr(), name_c.as_ptr())
        };
        if class.is_null() {
            return Err(Error::NotFound(format!("class {}.{} in {}", namespace, name, assembly)));
        }
        Ok(Class { il2cpp: self, ptr: class })
    }

    /// A new managed string.
    pub fn string(&self, value: &str) -> Result<Object> {
        let value = cstring(value)?;
        Ok(unsafe { (self.api.il2cpp_string_new)(value.as_ptr()) })
    }

    /// Reads a managed string, `None` for null.
    ///
    /// # Safety
    /// `string` must be null or a live `System.String`.
    pub unsafe fn read_string(&self, string: Object) -> Option<String> {
        if string.is_null() {
            return None;
        }
        unsafe {
            let length = (self.api.il2cpp_string_length)(string).max(0) as usize;
            let chars = std::slice::from_raw_parts((self.api.il2cpp_string_chars)(string), length);
            Some(String::from_utf16_lossy(chars))
        }
    }

    /// The class of a managed object.
    ///
    /// # Safety
    /// `object` must be a live managed object.
    pub unsafe fn class_of(&'static self, object: Object) -> Class {
        Class {
            il2cpp: self,
            ptr: unsafe { (self.api.il2cpp_object_get_class)(object) },
        }
    }
}

/// A managed class (`Il2CppClass*`).
#[derive(Clone, Copy)]
pub struct Class {
    il2cpp: &'static Il2Cpp,
    ptr: Ptr,
}

unsafe impl Send for Class {}
unsafe impl Sync for Class {}

impl Class {
    pub fn as_ptr(&self) -> Ptr {
        self.ptr
    }

    pub fn name(&self) -> &'static str {
        unsafe { c_str((self.il2cpp.api.il2cpp_class_get_name)(self.ptr)) }
    }

    pub fn namespace(&self) -> &'static str {
        unsafe { c_str((self.il2cpp.api.il2cpp_class_get_namespace)(self.ptr)) }
    }

    /// Finds a method by name and parameter count (`-1` for any).
    pub fn method(&self, name: &str, parameter_count: i32) -> Result<Method> {
        let c_name = cstring(name)?;
        let method = unsafe { (self.il2cpp.api.il2cpp_class_get_method_from_name)(self.ptr, c_name.as_ptr(), parameter_count) };
        if method.is_null() {
            return Err(Error::NotFound(format!("method {}.{}({} parameters)", self.name(), name, parameter_count)));
        }
        Ok(Method { il2cpp: self.il2cpp, ptr: method })
    }

    /// The byte offset of an instance field from the start of an object.
    pub fn field_offset(&self, name: &str) -> Result<usize> {
        let c_name = cstring(name)?;
        let field = unsafe { (self.il2cpp.api.il2cpp_class_get_field_from_name)(self.ptr, c_name.as_ptr()) };
        if field.is_null() {
            return Err(Error::NotFound(format!("field {}.{}", self.name(), name)));
        }
        Ok(unsafe { (self.il2cpp.api.il2cpp_field_get_offset)(field) })
    }
}

/// A managed method (`MethodInfo*`).
#[derive(Clone, Copy)]
pub struct Method {
    il2cpp: &'static Il2Cpp,
    ptr: Ptr,
}

unsafe impl Send for Method {}
unsafe impl Sync for Method {}

impl Method {
    /// The `MethodInfo*`, passed as the last argument when calling the method.
    pub fn as_ptr(&self) -> MethodInfo {
        self.ptr
    }

    pub fn name(&self) -> &'static str {
        unsafe { c_str((self.il2cpp.api.il2cpp_method_get_name)(self.ptr)) }
    }

    /// The method's compiled code. Call it with the instance first (for instance
    /// methods), then the arguments, then [`Method::as_ptr`].
    pub fn code(&self) -> *mut c_void {
        // `methodPointer` is the first field of `MethodInfo`.
        unsafe { *(self.ptr as *const *mut c_void) }
    }

    /// Redirects every call of the method to `detour`, which has the method's
    /// signature as described on [`Method::code`]. Inside the detour, [`original!`]
    /// gets a function that calls the original code.
    ///
    /// The hook stays in place when the returned value is dropped.
    ///
    /// # Safety
    /// `detour` must be an `extern "C"` function with exactly the method's signature.
    pub unsafe fn hook(&self, detour: *mut u8) -> Result<MethodHook> {
        let mut hook = unsafe { hooking::Hook::create(self.code() as *mut u8, detour) }.map_err(Error::Hook)?;
        unsafe { hook.apply_hook() }.map_err(Error::Hook)?;
        Ok(MethodHook(hook))
    }
}

/// An applied method hook.
pub struct MethodHook(hooking::Hook<'static>);

impl MethodHook {
    pub fn remove(mut self) -> Result<()> {
        unsafe { self.0.remove_hook() }.map_err(Error::Hook)
    }
}

/// Reads a managed object's field at `offset` (from [`Class::field_offset`]).
///
/// # Safety
/// `object` must be a live object with a field of type `T` at `offset`.
pub unsafe fn read_field<T: Copy>(object: Object, offset: usize) -> T {
    unsafe { std::ptr::read_unaligned((object as *const u8).add(offset) as *const T) }
}

/// Writes a managed object's field at `offset`. Only for fields that don't hold references.
///
/// # Safety
/// `object` must be a live object with a field of type `T` at `offset`.
pub unsafe fn write_field<T: Copy>(object: Object, offset: usize, value: T) {
    unsafe { std::ptr::write_unaligned((object as *mut u8).add(offset) as *mut T, value) }
}

/// Inside a detour from [`Method::hook`], gets the function that calls the
/// original method. Must be the first thing the detour does.
#[macro_export]
macro_rules! original {
    ($fn_type:ty) => {{
        let original = $crate::__original_function_ptr().expect("original! must be used inside a hook");
        unsafe { ::core::mem::transmute::<*mut ::core::ffi::c_void, $fn_type>(original.as_ptr()) }
    }};
}

#[doc(hidden)]
pub use hooking::original_function_ptr as __original_function_ptr;
