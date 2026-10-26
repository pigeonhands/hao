//! Hook and call into a Unity Il2Cpp game from inside its process, the way
//! MelonLoader mods do.
//!
//! A mod is a `cdylib` loaded into the game (on Linux with `LD_PRELOAD`).
//! [`on_init`] runs once the game's Il2Cpp runtime has started, and from there
//! methods are found by name and hooked with ordinary closures. The closure's
//! types are checked against the method's real signature when the hook is
//! installed:
//!
//! ```no_run
//! use hao_il2cpp_runtime::{Il2Cpp, Object, Original};
//!
//! hao_il2cpp_runtime::on_init(|il2cpp: &'static Il2Cpp| {
//!     let health = il2cpp.class("Assembly-CSharp", "Assets.Scripts.Inventory__Items__Pickups", "PlayerHealth")?;
//!     // `int Heal(float amount, bool allowOverheal)`: the instance comes first.
//!     health.method("Heal", 2)?.hook(|this: Object, amount: f32, overheal: bool, original: Original<_, i32>| {
//!         original.call((this, amount * 2.0, overheal))
//!     })?;
//!     Ok(())
//! });
//! ```

mod api;
mod error;
#[cfg(unix)]
mod loader;
mod typed;

use std::{
    ffi::{c_void, CStr, CString},
    path::{Path, PathBuf},
    sync::OnceLock,
};

use libloading::Library;

pub use api::{Api, Ptr};
pub use error::{Error, Result};
#[cfg(unix)]
pub use loader::on_init;
pub use typed::{
    element, HookCallback, Il2CppString, Il2CppValue, MethodArgs, Object, Original, Parameter,
    Signature, TypeInfo, ValueCheck,
};

/// The hidden last argument of every Il2Cpp method (`const MethodInfo*`).
pub type MethodInfo = Ptr;

/// Whether `path` is a game binary: `GameAssembly.so`, `GameAssembly.dll` or any other `GameAssembly.*`.
/// Both `/` and `\\` separate directories, as Windows paths can show up on Linux under Wine.
pub fn is_game_assembly(path: impl AsRef<Path>) -> bool {
    let path = path.as_ref().to_string_lossy();
    path.rsplit(['/', '\\'])
        .next()
        .is_some_and(|name| name.starts_with("GameAssembly."))
}

/// The game binary in a game folder, whichever platform the game was built for.
pub fn find_game_assembly(dir: impl AsRef<Path>) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .find(|p| p.is_file() && is_game_assembly(p))
}

/// The game binary mapped into this process, if any.
#[cfg(unix)]
pub fn loaded_game_assembly() -> Option<PathBuf> {
    let maps = std::fs::read_to_string("/proc/self/maps").ok()?;
    maps.lines()
        .filter_map(|line| line.split_whitespace().nth(5))
        .map(PathBuf::from)
        .find(|p| is_game_assembly(p))
}

/// The game binary mapped into this process, if any.
#[cfg(windows)]
pub fn loaded_game_assembly() -> Option<PathBuf> {
    libloading::os::windows::Library::open_already_loaded("GameAssembly.dll")
        .ok()
        .map(|_| PathBuf::from("GameAssembly.dll"))
}

/// Errors unless `path` is a binary this build can load: ELF on Unix, PE on Windows.
fn check_format(path: &Path) -> Result<()> {
    use std::io::Read;
    let mut magic = [0u8; 4];
    let read = std::fs::File::open(path).and_then(|mut f| f.read_exact(&mut magic));
    let (elf, pe) = (magic == *b"\x7fELF", magic.starts_with(b"MZ"));
    match read {
        Ok(()) if cfg!(unix) && pe => Err(Error::UnsupportedBinary(format!(
            "{} is a Windows binary (a Windows build running under Wine/Proton), which a Linux mod can't load yet",
            path.display()
        ))),
        Ok(()) if cfg!(windows) && elf => Err(Error::UnsupportedBinary(format!("{} is not a Windows binary", path.display()))),
        _ => Ok(()),
    }
}

static INSTANCE: OnceLock<Il2Cpp> = OnceLock::new();

/// The game's Il2Cpp runtime.
pub struct Il2Cpp {
    api: Api,
    _library: Library,
}

fn cstring(value: &str) -> Result<CString> {
    CString::new(value).map_err(|_| Error::Nul(value.to_owned()))
}

pub(crate) unsafe fn c_str<'a>(value: *const std::ffi::c_char) -> &'a str {
    if value.is_null() {
        ""
    } else {
        unsafe { CStr::from_ptr(value) }.to_str().unwrap_or("")
    }
}

impl Il2Cpp {
    fn install(library: Library) -> Result<&'static Il2Cpp> {
        let api = Api::load(&library)?;
        tracing::debug!("loaded the il2cpp_* API from the game binary");
        Ok(INSTANCE.get_or_init(|| Il2Cpp {
            api,
            _library: library,
        }))
    }

    /// The runtime of the game binary already loaded in this process.
    pub fn loaded() -> Result<&'static Il2Cpp> {
        if let Some(il2cpp) = INSTANCE.get() {
            return Ok(il2cpp);
        }
        let path = loaded_game_assembly()
            .ok_or_else(|| Error::NotFound("a loaded GameAssembly".into()))?;
        check_format(&path)?;
        tracing::debug!(path = %path.display(), "using the loaded game binary");
        #[cfg(unix)]
        let library = unsafe {
            libloading::os::unix::Library::open(Some(&path), libc::RTLD_NOW | libc::RTLD_NOLOAD)
                .map(Library::from)
        };
        #[cfg(windows)]
        let library =
            libloading::os::windows::Library::open_already_loaded(&path).map(Library::from);
        Self::install(library.map_err(Error::Load)?)
    }

    /// Loads a game binary into this process, e.g. to run a game's code without Unity.
    pub fn load(path: impl AsRef<Path>) -> Result<&'static Il2Cpp> {
        if let Some(il2cpp) = INSTANCE.get() {
            return Ok(il2cpp);
        }
        check_format(path.as_ref())?;
        tracing::debug!(path = %path.as_ref().display(), "loading the game binary");
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
        tracing::debug!(data_dir = %data_dir.as_ref().display(), "starting Il2Cpp without Unity");
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
        let (assembly_c, namespace_c, name_c) =
            (cstring(assembly)?, cstring(namespace)?, cstring(name)?);
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
            return Err(Error::NotFound(format!(
                "class {}.{} in {}",
                namespace, name, assembly
            )));
        }
        tracing::trace!(assembly, namespace, name, "found class");
        Ok(Class {
            il2cpp: self,
            ptr: class,
        })
    }

    /// A new managed string.
    pub fn string(&self, value: &str) -> Result<Il2CppString> {
        let value = cstring(value)?;
        Ok(Il2CppString(Object(unsafe {
            (self.api.il2cpp_string_new)(value.as_ptr())
        })))
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
            let length = (self.api.il2cpp_string_length)(string.0).max(0) as usize;
            let chars =
                std::slice::from_raw_parts((self.api.il2cpp_string_chars)(string.0), length);
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
            ptr: unsafe { (self.api.il2cpp_object_get_class)(object.0) },
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
        let method = unsafe {
            (self.il2cpp.api.il2cpp_class_get_method_from_name)(
                self.ptr,
                c_name.as_ptr(),
                parameter_count,
            )
        };
        if method.is_null() {
            return Err(Error::NotFound(format!(
                "method {}.{}({} parameters)",
                self.name(),
                name,
                parameter_count
            )));
        }
        tracing::trace!(class = self.name(), name, parameter_count, "found method");
        Ok(Method {
            il2cpp: self.il2cpp,
            ptr: method,
        })
    }

    /// Allocates an object of this class without running a constructor; call
    /// `.ctor` on it with [`Method::call`].
    pub fn new_object(&self) -> Object {
        Object(unsafe { (self.il2cpp.api.il2cpp_object_new)(self.ptr) })
    }

    /// The class's `System.Type`, as `typeof` gives it.
    pub fn type_object(&self) -> Object {
        Object(unsafe {
            (self.il2cpp.api.il2cpp_type_get_object)((self.il2cpp.api.il2cpp_class_get_type)(
                self.ptr,
            ))
        })
    }

    /// Every method declared by the class, e.g. to pick an overload by [`Method::signature`].
    pub fn methods(&self) -> impl Iterator<Item = Method> + '_ {
        let mut iter: Ptr = std::ptr::null_mut();
        std::iter::from_fn(move || {
            let method = unsafe { (self.il2cpp.api.il2cpp_class_get_methods)(self.ptr, &mut iter) };
            (!method.is_null()).then_some(Method {
                il2cpp: self.il2cpp,
                ptr: method,
            })
        })
    }

    /// A field by name, with its type, offset and whether it is static.
    pub fn field(&self, name: &str) -> Result<Field> {
        let c_name = cstring(name)?;
        let field = unsafe {
            (self.il2cpp.api.il2cpp_class_get_field_from_name)(self.ptr, c_name.as_ptr())
        };
        if field.is_null() {
            return Err(Error::NotFound(format!("field {}.{}", self.name(), name)));
        }
        Ok(Field {
            il2cpp: self.il2cpp,
            ptr: field,
        })
    }

    /// Every field declared by the class.
    pub fn fields(&self) -> impl Iterator<Item = Field> + '_ {
        let mut iter: Ptr = std::ptr::null_mut();
        std::iter::from_fn(move || {
            let field = unsafe { (self.il2cpp.api.il2cpp_class_get_fields)(self.ptr, &mut iter) };
            (!field.is_null()).then_some(Field {
                il2cpp: self.il2cpp,
                ptr: field,
            })
        })
    }

    /// The byte offset of an instance field from the start of an object.
    pub fn field_offset(&self, name: &str) -> Result<usize> {
        Ok(self.field(name)?.offset())
    }
}

/// A managed field (`FieldInfo*`).
#[derive(Clone, Copy)]
pub struct Field {
    il2cpp: &'static Il2Cpp,
    ptr: Ptr,
}

unsafe impl Send for Field {}
unsafe impl Sync for Field {}

/// `FIELD_ATTRIBUTE_STATIC` in the field's flags.
const FIELD_STATIC: i32 = 0x0010;

impl Field {
    pub fn as_ptr(&self) -> Ptr {
        self.ptr
    }

    pub fn name(&self) -> &'static str {
        unsafe { c_str((self.il2cpp.api.il2cpp_field_get_name)(self.ptr)) }
    }

    /// The managed type name, e.g. `System.Single`.
    pub fn type_name(&self) -> String {
        let ty = unsafe { (self.il2cpp.api.il2cpp_field_get_type)(self.ptr) };
        unsafe { c_str((self.il2cpp.api.il2cpp_type_get_name)(ty)) }.to_owned()
    }

    /// The byte offset from the start of an object (for instance fields), with
    /// [`read_field`]/[`write_field`].
    pub fn offset(&self) -> usize {
        unsafe { (self.il2cpp.api.il2cpp_field_get_offset)(self.ptr) }
    }

    pub fn is_static(&self) -> bool {
        unsafe { (self.il2cpp.api.il2cpp_field_get_flags)(self.ptr) & FIELD_STATIC != 0 }
    }

    /// Reads this instance field's value from `object`.
    ///
    /// # Safety
    /// `T` must match the field's layout, and `object` must be an instance of
    /// the field's class.
    pub unsafe fn read<T: Copy>(&self, object: Object) -> T {
        unsafe { read_field(object, self.offset()) }
    }

    /// Writes this instance field's value on `object`.
    ///
    /// # Safety
    /// As for [`Field::read`].
    pub unsafe fn write<T: Copy>(&self, object: Object, value: T) {
        unsafe { write_field(object, self.offset(), value) }
    }
}

/// Keeps an object alive and findable after the garbage collector moves or
/// would free it. The collector doesn't scan Rust memory, so objects kept
/// between calls (in statics or closures) need one. Freed on drop.
pub struct GcHandle {
    il2cpp: &'static Il2Cpp,
    handle: Ptr,
}

unsafe impl Send for GcHandle {}
unsafe impl Sync for GcHandle {}

impl GcHandle {
    pub fn new(il2cpp: &'static Il2Cpp, object: Object) -> Self {
        Self {
            il2cpp,
            handle: unsafe { (il2cpp.api.il2cpp_gchandle_new)(object.0, false) },
        }
    }

    pub fn target(&self) -> Object {
        Object(unsafe { (self.il2cpp.api.il2cpp_gchandle_get_target)(self.handle) })
    }
}

impl Drop for GcHandle {
    fn drop(&mut self) {
        unsafe { (self.il2cpp.api.il2cpp_gchandle_free)(self.handle) }
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
    /// gets a function that calls the original code. [`Method::hook`] is the safe,
    /// typed version of this.
    ///
    /// The hook stays in place when the returned value is dropped.
    ///
    /// # Safety
    /// `detour` must be an `extern "C"` function with exactly the method's signature.
    pub unsafe fn hook_raw(&self, detour: *mut u8) -> Result<MethodHook> {
        let mut hook = unsafe { hooking::Hook::create(self.code() as *mut u8, detour) }
            .map_err(Error::Hook)?;
        unsafe { hook.apply_hook() }.map_err(Error::Hook)?;
        tracing::debug!(method = self.name(), code = ?self.code(), "hooked (raw)");
        Ok(MethodHook(hook))
    }
}

/// An applied method hook. Dropping it leaves the hook in place.
pub struct MethodHook(pub(crate) hooking::Hook<'static>);

impl MethodHook {
    pub fn remove(mut self) -> Result<()> {
        unsafe { self.0.remove_hook() }.map_err(Error::Hook)?;
        tracing::debug!(code = ?self.0.data.symbol_address, "removed hook");
        Ok(())
    }
}

/// Reads a managed object's field at `offset` (from [`Class::field_offset`]).
///
/// # Safety
/// `object` must be a live object with a field of type `T` at `offset`.
pub unsafe fn read_field<T: Copy>(object: Object, offset: usize) -> T {
    unsafe { std::ptr::read_unaligned((object.0 as *const u8).add(offset) as *const T) }
}

/// Writes a managed object's field at `offset`. Only for fields that don't hold references.
///
/// # Safety
/// `object` must be a live object with a field of type `T` at `offset`.
pub unsafe fn write_field<T: Copy>(object: Object, offset: usize, value: T) {
    unsafe { std::ptr::write_unaligned((object.0 as *mut u8).add(offset) as *mut T, value) }
}

/// Inside a detour from [`Method::hook_raw`], gets the function that calls the
/// original method. Must be the first thing the detour does.
#[macro_export]
macro_rules! original {
    ($fn_type:ty) => {{
        let original =
            $crate::__original_function_ptr().expect("original! must be used inside a hook");
        unsafe { ::core::mem::transmute::<*mut ::core::ffi::c_void, $fn_type>(original.as_ptr()) }
    }};
}

#[doc(hidden)]
pub use hooking::original_function_ptr as __original_function_ptr;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_assembly_names() {
        assert!(is_game_assembly("/games/Example/GameAssembly.so"));
        assert!(is_game_assembly("C:\\Games\\Example\\GameAssembly.dll"));
        assert!(!is_game_assembly("/games/Example/UnityPlayer.so"));
        assert!(!is_game_assembly("/games/Example/NotGameAssembly.dll"));
    }

    #[test]
    fn finds_and_checks_the_game_binary() {
        let dir = std::env::temp_dir().join(format!("hao-il2cpp-runtime-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("UnityPlayer.dll"), b"MZ\x90\x00").unwrap();
        std::fs::write(dir.join("GameAssembly.dll"), b"MZ\x90\x00").unwrap();
        let found = find_game_assembly(&dir).unwrap();
        assert_eq!(found, dir.join("GameAssembly.dll"));
        if cfg!(unix) {
            assert!(matches!(
                check_format(&found),
                Err(Error::UnsupportedBinary(_))
            ));
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
}
