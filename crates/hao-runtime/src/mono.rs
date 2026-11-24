//! Hosting Mono through its embedding API.

use std::{
    collections::HashMap,
    ffi::{c_char, c_int, c_void, CString},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use libloading::Library;

use crate::error::{Error, Result};

type Ptr = *mut c_void;

/// Where to find Mono.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "reflect", derive(facet::Facet))]
pub struct MonoOptions {
    /// Path to the Mono shared library (`libmonosgen-2.0.so`, `mono-2.0-bdwgc.dll`, ...).
    /// Defaults to the system Mono.
    pub library: Option<PathBuf>,
    /// The directory holding the `mono/4.5` class libraries, e.g. `/usr/lib` or a
    /// Unity game's `MonoBleedingEdge/lib`. Defaults to Mono's built in path.
    pub assemblies_dir: Option<PathBuf>,
    /// Mono's config directory, e.g. `/etc`. Defaults to Mono's built in path.
    pub config_dir: Option<PathBuf>,
}

struct Api {
    set_dirs: unsafe extern "C" fn(*const c_char, *const c_char),
    jit_init_version: unsafe extern "C" fn(*const c_char, *const c_char) -> Ptr,
    thread_attach: unsafe extern "C" fn(Ptr) -> Ptr,
    domain_assembly_open: unsafe extern "C" fn(Ptr, *const c_char) -> Ptr,
    image_open_from_data_with_name:
        unsafe extern "C" fn(*const c_char, u32, c_int, *mut c_int, c_int, *const c_char) -> Ptr,
    assembly_load_from_full: unsafe extern "C" fn(Ptr, *const c_char, *mut c_int, c_int) -> Ptr,
    assembly_get_image: unsafe extern "C" fn(Ptr) -> Ptr,
    class_from_name: unsafe extern "C" fn(Ptr, *const c_char, *const c_char) -> Ptr,
    class_get_method_from_name: unsafe extern "C" fn(Ptr, *const c_char, c_int) -> Ptr,
    method_get_unmanaged_thunk: unsafe extern "C" fn(Ptr) -> Ptr,
    compile_method: unsafe extern "C" fn(Ptr) -> Ptr,
    get_root_domain: unsafe extern "C" fn() -> Ptr,
    // Bound for completeness; the `enumerate` path resolves virtual methods on
    // the object directly via `object_get_virtual_method`, which needs no
    // separate class lookup.
    #[allow(dead_code)]
    object_get_class: unsafe extern "C" fn(Ptr) -> Ptr,
    object_get_virtual_method: unsafe extern "C" fn(Ptr, Ptr) -> Ptr,
    runtime_invoke: unsafe extern "C" fn(Ptr, Ptr, *mut Ptr, *mut Ptr) -> Ptr,
    object_unbox: unsafe extern "C" fn(Ptr) -> Ptr,
    class_get_methods: unsafe extern "C" fn(Ptr, *mut Ptr) -> Ptr,
    method_get_name: unsafe extern "C" fn(Ptr) -> *const c_char,
    class_get_name: unsafe extern "C" fn(Ptr) -> *const c_char,
    class_get_namespace: unsafe extern "C" fn(Ptr) -> *const c_char,
    image_get_table_rows: unsafe extern "C" fn(Ptr, c_int) -> c_int,
    class_get: unsafe extern "C" fn(Ptr, u32) -> Ptr,
    get_corlib: unsafe extern "C" fn() -> Ptr,
}

/// The `MONO_TABLE_TYPEDEF` index for `mono_image_get_table_rows`.
const MONO_TABLE_TYPEDEF: c_int = 2;
/// The top byte of a `TypeDef` metadata token.
const MONO_TOKEN_TYPE_DEF: u32 = 0x0200_0000;

/// The three corlib `MonoMethod*` (as `usize`) the `enumerate` walk needs,
/// resolved once so each call avoids a corlib lookup.
struct EnumMethods {
    get_enumerator: usize,
    move_next: usize,
    get_current: usize,
}

pub(crate) struct Mono {
    _library: Library,
    api: Api,
    domain: Ptr,
    images: Mutex<HashMap<String, usize>>,
    enum_methods: OnceLock<EnumMethods>,
}

// Mono is thread safe once threads are attached, which every call does.
unsafe impl Send for Mono {}
unsafe impl Sync for Mono {}

fn default_library() -> Option<PathBuf> {
    let names: &[&str] = if cfg!(windows) {
        &["mono-2.0-sgen.dll", "mono-2.0-bdwgc.dll"]
    } else if cfg!(target_os = "macos") {
        &["libmonosgen-2.0.dylib"]
    } else {
        &["libmonosgen-2.0.so", "libmonosgen-2.0.so.1"]
    };
    let dirs = [
        "/usr/lib",
        "/usr/local/lib",
        "/usr/lib/x86_64-linux-gnu",
        "/Library/Frameworks/Mono.framework/Versions/Current/lib",
        r"C:\Program Files\Mono\bin",
    ];
    dirs.iter()
        .flat_map(|d| names.iter().map(move |n| Path::new(d).join(n)))
        .find(|p| p.exists())
}

fn cstring(value: &str) -> Result<CString> {
    CString::new(value).map_err(|_| Error::Other(format!("{:?} contains a nul byte", value)))
}

/// Copies a Mono-owned C string into an owned `String`. The memory belongs to
/// Mono (interned class/method names), so it is read but never freed. A null
/// pointer, or one holding invalid UTF-8, yields an empty string.
fn cstr(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { std::ffi::CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned()
}

impl Mono {
    /// Opens the Mono library and resolves its symbols, without initialising it.
    /// On `attach` the library is already loaded, so this hands back the same one.
    fn open(options: &MonoOptions) -> Result<(Library, Api)> {
        let path = options
            .library
            .clone()
            .or_else(default_library)
            .ok_or_else(|| Error::NotFound("the Mono runtime library".into()))?;
        let library = unsafe { Library::new(&path) }
            .map_err(|e| Error::Library(path.display().to_string(), e))?;

        macro_rules! symbol {
            ($name:literal) => {
                *unsafe { library.get(concat!($name, "\0").as_bytes()) }
                    .map_err(|e| Error::Library($name.into(), e))?
            };
        }
        let api = Api {
            set_dirs: symbol!("mono_set_dirs"),
            jit_init_version: symbol!("mono_jit_init_version"),
            thread_attach: symbol!("mono_thread_attach"),
            domain_assembly_open: symbol!("mono_domain_assembly_open"),
            image_open_from_data_with_name: symbol!("mono_image_open_from_data_with_name"),
            assembly_load_from_full: symbol!("mono_assembly_load_from_full"),
            assembly_get_image: symbol!("mono_assembly_get_image"),
            class_from_name: symbol!("mono_class_from_name"),
            class_get_method_from_name: symbol!("mono_class_get_method_from_name"),
            method_get_unmanaged_thunk: symbol!("mono_method_get_unmanaged_thunk"),
            compile_method: symbol!("mono_compile_method"),
            get_root_domain: symbol!("mono_get_root_domain"),
            object_get_class: symbol!("mono_object_get_class"),
            object_get_virtual_method: symbol!("mono_object_get_virtual_method"),
            runtime_invoke: symbol!("mono_runtime_invoke"),
            object_unbox: symbol!("mono_object_unbox"),
            class_get_methods: symbol!("mono_class_get_methods"),
            method_get_name: symbol!("mono_method_get_name"),
            class_get_name: symbol!("mono_class_get_name"),
            class_get_namespace: symbol!("mono_class_get_namespace"),
            image_get_table_rows: symbol!("mono_image_get_table_rows"),
            class_get: symbol!("mono_class_get"),
            get_corlib: symbol!("mono_get_corlib"),
        };
        Ok((library, api))
    }

    /// Attaches to a Mono runtime already running in this process, for a mod
    /// loaded into a Unity Mono game. `options.library` should point at the
    /// game's own Mono (e.g. `MonoBleedingEdge/.../libmonobdwgc-2.0.so`); loading
    /// it by name returns the already-loaded instance rather than a fresh copy.
    pub fn attach(options: &MonoOptions) -> Result<Self> {
        let (library, api) = Self::open(options)?;
        let domain = unsafe { (api.get_root_domain)() };
        if domain.is_null() {
            return Err(Error::Other(
                "no Mono runtime is running in this process".into(),
            ));
        }
        unsafe { (api.thread_attach)(domain) };
        Ok(Self {
            _library: library,
            api,
            domain,
            images: Mutex::new(HashMap::new()),
            enum_methods: OnceLock::new(),
        })
    }

    pub fn new(options: &MonoOptions) -> Result<Self> {
        let (library, api) = Self::open(options)?;

        let assemblies = options
            .assemblies_dir
            .as_ref()
            .map(|p| cstring(&p.to_string_lossy()))
            .transpose()?;
        let config = options
            .config_dir
            .as_ref()
            .map(|p| cstring(&p.to_string_lossy()))
            .transpose()?;
        let domain_name = cstring("hao")?;
        let version = cstring("v4.0.30319")?;
        let domain = unsafe {
            if assemblies.is_some() || config.is_some() {
                let default_config = cstring("/etc")?;
                let default_assemblies = cstring("/usr/lib")?;
                (api.set_dirs)(
                    assemblies.as_ref().unwrap_or(&default_assemblies).as_ptr(),
                    config.as_ref().unwrap_or(&default_config).as_ptr(),
                );
            }
            (api.jit_init_version)(domain_name.as_ptr(), version.as_ptr())
        };
        if domain.is_null() {
            return Err(Error::Other("mono_jit_init_version failed".into()));
        }

        Ok(Self {
            _library: library,
            api,
            domain,
            images: Mutex::new(HashMap::new()),
            enum_methods: OnceLock::new(),
        })
    }

    /// Attaches the calling thread so it can run managed code.
    pub fn attach_thread(&self) {
        unsafe { (self.api.thread_attach)(self.domain) };
    }

    fn register(&self, name: &str, assembly: Ptr) -> Result<()> {
        let image = unsafe { (self.api.assembly_get_image)(assembly) };
        if image.is_null() {
            return Err(Error::Other(format!("{} has no image", name)));
        }
        self.images
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(name.to_owned(), image as usize);
        Ok(())
    }

    pub fn load_assembly(&self, path: &Path) -> Result<()> {
        self.attach_thread();
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let c_path = cstring(&path.to_string_lossy())?;
        let assembly = unsafe { (self.api.domain_assembly_open)(self.domain, c_path.as_ptr()) };
        if assembly.is_null() {
            return Err(Error::NotFound(path.display().to_string()));
        }
        self.register(&name, assembly)
    }

    pub fn load_assembly_bytes(&self, name: &str, bytes: &[u8]) -> Result<()> {
        self.attach_thread();
        let c_name = cstring(name)?;
        let mut status = 0;
        unsafe {
            let image = (self.api.image_open_from_data_with_name)(
                bytes.as_ptr().cast(),
                bytes.len() as u32,
                1,
                &mut status,
                0,
                c_name.as_ptr(),
            );
            if image.is_null() {
                return Err(Error::Other(format!(
                    "could not open image {} ({})",
                    name, status
                )));
            }
            let assembly =
                (self.api.assembly_load_from_full)(image, c_name.as_ptr(), &mut status, 0);
            if assembly.is_null() {
                return Err(Error::Other(format!(
                    "could not load assembly {} ({})",
                    name, status
                )));
            }
            self.register(name, assembly)
        }
    }

    /// The `MonoImage*` of a loaded assembly, from the registration cache.
    fn image(&self, assembly: &str) -> Result<Ptr> {
        let image = *self
            .images
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(assembly)
            .ok_or_else(|| Error::NotFound(format!("loaded assembly {}", assembly)))?;
        Ok(image as Ptr)
    }

    /// Resolves `type_name::method` to a `MonoMethod*`, matching any arity.
    fn resolve_method(&self, assembly: &str, type_name: &str, method: &str) -> Result<Ptr> {
        self.resolve_method_arity(assembly, type_name, method, -1)
    }

    /// Resolves `type_name::method` to a `MonoMethod*`, matching the overload
    /// with `params` parameters (`-1` matches any arity).
    fn resolve_method_arity(
        &self,
        assembly: &str,
        type_name: &str,
        method: &str,
        params: c_int,
    ) -> Result<Ptr> {
        self.attach_thread();
        let image = self.image(assembly)?;
        let (namespace, name) = type_name.rsplit_once('.').unwrap_or(("", type_name));
        let (namespace, name, method_name) =
            (cstring(namespace)?, cstring(name)?, cstring(method)?);
        unsafe {
            let class = (self.api.class_from_name)(image, namespace.as_ptr(), name.as_ptr());
            if class.is_null() {
                return Err(Error::NotFound(format!(
                    "type {} in {}",
                    type_name, assembly
                )));
            }
            let method_ptr =
                (self.api.class_get_method_from_name)(class, method_name.as_ptr(), params);
            if method_ptr.is_null() {
                return Err(Error::NotFound(format!("method {}::{}", type_name, method)));
            }
            Ok(method_ptr)
        }
    }

    pub fn function_pointer(
        &self,
        assembly: &str,
        type_name: &str,
        method: &str,
    ) -> Result<*const c_void> {
        let method_ptr = self.resolve_method(assembly, type_name, method)?;
        let thunk = unsafe { (self.api.method_get_unmanaged_thunk)(method_ptr) };
        if thunk.is_null() {
            return Err(Error::Other(format!(
                "no unmanaged thunk for {}::{}",
                type_name, method
            )));
        }
        Ok(thunk)
    }

    /// The method's JIT-compiled native code, callable with the managed calling
    /// convention and with no trailing exception argument. Mono can do this for
    /// any static method, so there is no `[UnmanagedCallersOnly]` requirement.
    pub fn method_pointer(
        &self,
        assembly: &str,
        type_name: &str,
        method: &str,
    ) -> Result<*const c_void> {
        let method_ptr = self.resolve_method(assembly, type_name, method)?;
        let code = unsafe { (self.api.compile_method)(method_ptr) };
        if code.is_null() {
            return Err(Error::Other(format!(
                "could not compile {}::{}",
                type_name, method
            )));
        }
        Ok(code)
    }

    /// Like [`method_pointer`](Self::method_pointer) but selects the overload
    /// with `param_count` parameters, so it works with overloaded methods.
    /// `mono_class_get_method_from_name` matches on name and arity natively.
    pub fn find_method(
        &self,
        assembly: &str,
        type_name: &str,
        method: &str,
        param_count: c_int,
    ) -> Result<*const c_void> {
        let method_ptr = self.resolve_method_arity(assembly, type_name, method, param_count)?;
        let code = unsafe { (self.api.compile_method)(method_ptr) };
        if code.is_null() {
            return Err(Error::Other(format!(
                "could not compile {}::{}({} params)",
                type_name, method, param_count
            )));
        }
        Ok(code)
    }

    /// Resolves a well-known corlib method to a native-callable pointer, using
    /// Mono's own corlib image (`mono_get_corlib`) rather than a loaded
    /// assembly. The type is `namespace.name`, the overload is picked by
    /// `arity` (`-1` matches any).
    #[cfg(feature = "hao")]
    pub fn corlib_method_pointer(
        &self,
        namespace: &str,
        name: &str,
        method: &str,
        arity: c_int,
    ) -> Result<*const c_void> {
        self.attach_thread();
        let corlib = unsafe { (self.api.get_corlib)() };
        if corlib.is_null() {
            return Err(Error::Other("mono_get_corlib returned null".into()));
        }
        let (namespace_c, name_c, method_c) =
            (cstring(namespace)?, cstring(name)?, cstring(method)?);
        unsafe {
            let class = (self.api.class_from_name)(corlib, namespace_c.as_ptr(), name_c.as_ptr());
            if class.is_null() {
                return Err(Error::NotFound(format!("corlib type {}.{}", namespace, name)));
            }
            let method_ptr = (self.api.class_get_method_from_name)(class, method_c.as_ptr(), arity);
            if method_ptr.is_null() {
                return Err(Error::NotFound(format!(
                    "corlib method {}.{}::{}",
                    namespace, name, method
                )));
            }
            let code = (self.api.compile_method)(method_ptr);
            if code.is_null() {
                return Err(Error::Other(format!(
                    "could not compile corlib {}::{}",
                    name, method
                )));
            }
            Ok(code)
        }
    }

    /// Returns the full names (`namespace.name`, or bare `name` when the type
    /// has no namespace) of every type in a loaded assembly, by walking the
    /// `TypeDef` metadata table.
    pub fn type_names(&self, assembly: &str) -> Result<Vec<String>> {
        self.attach_thread();
        let image = self.image(assembly)?;
        let rows = unsafe { (self.api.image_get_table_rows)(image, MONO_TABLE_TYPEDEF) };
        let mut out = Vec::new();
        // Row 1 is the compiler-synthesised `<Module>` type; real types start at 2.
        for row in 2..=rows.max(0) as u32 {
            let class = unsafe { (self.api.class_get)(image, MONO_TOKEN_TYPE_DEF | row) };
            if class.is_null() {
                continue;
            }
            let namespace = cstr(unsafe { (self.api.class_get_namespace)(class) });
            let name = cstr(unsafe { (self.api.class_get_name)(class) });
            out.push(if namespace.is_empty() {
                name
            } else {
                format!("{}.{}", namespace, name)
            });
        }
        Ok(out)
    }

    /// Returns the names of every method declared on a type in a loaded
    /// assembly, by iterating the class with `mono_class_get_methods`.
    pub fn method_names(&self, assembly: &str, type_name: &str) -> Result<Vec<String>> {
        self.attach_thread();
        let image = self.image(assembly)?;
        let (namespace, name) = type_name.rsplit_once('.').unwrap_or(("", type_name));
        let (namespace_c, name_c) = (cstring(namespace)?, cstring(name)?);
        let class =
            unsafe { (self.api.class_from_name)(image, namespace_c.as_ptr(), name_c.as_ptr()) };
        if class.is_null() {
            return Err(Error::NotFound(format!(
                "type {} in {}",
                type_name, assembly
            )));
        }
        let mut out = Vec::new();
        // `iter` must start null; Mono advances it and returns null when done.
        let mut iter: Ptr = std::ptr::null_mut();
        loop {
            let method = unsafe { (self.api.class_get_methods)(class, &mut iter) };
            if method.is_null() {
                break;
            }
            out.push(cstr(unsafe { (self.api.method_get_name)(method) }));
        }
        Ok(out)
    }

    /// Resolves and caches the three `System.Collections` enumerator methods
    /// (`IEnumerable::GetEnumerator`, `IEnumerator::MoveNext`,
    /// `IEnumerator::get_Current`) from Mono's corlib, so `enumerate` does no
    /// per-call lookups. Idempotent.
    fn enum_methods(&self) -> Result<&EnumMethods> {
        if let Some(methods) = self.enum_methods.get() {
            return Ok(methods);
        }
        self.attach_thread();
        let corlib = unsafe { (self.api.get_corlib)() };
        if corlib.is_null() {
            return Err(Error::Other("mono_get_corlib returned null".into()));
        }
        let resolve = |ns: &str, ty: &str, m: &str| -> Result<usize> {
            let (ns_c, ty_c, m_c) = (cstring(ns)?, cstring(ty)?, cstring(m)?);
            unsafe {
                let class = (self.api.class_from_name)(corlib, ns_c.as_ptr(), ty_c.as_ptr());
                if class.is_null() {
                    return Err(Error::NotFound(format!("corlib type {}.{}", ns, ty)));
                }
                let method = (self.api.class_get_method_from_name)(class, m_c.as_ptr(), 0);
                if method.is_null() {
                    return Err(Error::NotFound(format!("corlib method {}.{}::{}", ns, ty, m)));
                }
                Ok(method as usize)
            }
        };
        let methods = EnumMethods {
            get_enumerator: resolve("System.Collections", "IEnumerable", "GetEnumerator")?,
            move_next: resolve("System.Collections", "IEnumerator", "MoveNext")?,
            get_current: resolve("System.Collections", "IEnumerator", "get_Current")?,
        };
        Ok(self.enum_methods.get_or_init(|| methods))
    }

    /// Invokes `method` virtually on `target`: resolves the concrete override
    /// with `mono_object_get_virtual_method`, then `mono_runtime_invoke` with no
    /// arguments. Returns `None` if the method can't be resolved or the call
    /// threw a managed exception; otherwise the (possibly null) returned object.
    ///
    /// # Safety
    /// `target` must be a live `MonoObject*` and `method` a `MonoMethod*`
    /// applicable to it. Each call is a GC safepoint.
    unsafe fn invoke_virtual(&self, target: Ptr, method: Ptr) -> Option<Ptr> {
        let concrete = (self.api.object_get_virtual_method)(target, method);
        if concrete.is_null() {
            return None;
        }
        let mut exc: Ptr = std::ptr::null_mut();
        let result = (self.api.runtime_invoke)(concrete, target, std::ptr::null_mut(), &mut exc);
        if exc.is_null() {
            Some(result)
        } else {
            None
        }
    }

    /// Resolves and caches the enumerator methods so [`enumerate`](Self::enumerate)
    /// does no corlib lookups per call. `enumerate` resolves them lazily too, so
    /// this is only an optimisation and is always safe to skip.
    pub fn warm_enumerate(&self) -> Result<()> {
        self.enum_methods()?;
        Ok(())
    }

    /// Materializes any managed `IEnumerable` into the addresses of its
    /// elements, by driving the enumerator through the embedding API:
    /// `GetEnumerator`, then `MoveNext`/`get_Current` until `MoveNext` is false.
    ///
    /// Each element address is a live `MonoObject*`. Unlike CoreCLR's one-shot
    /// `object[]` read, Mono's SGen is a moving collector and every
    /// `mono_runtime_invoke` here is a GC safepoint, so a returned address is
    /// only valid until the next GC — the same raw-pointer contract as the rest
    /// of this Mono backend. Read or pin what you need before iterating further.
    pub fn enumerate(&self, obj: usize) -> Result<Vec<usize>> {
        self.attach_thread();
        if obj == 0 {
            return Ok(Vec::new());
        }
        let methods = self.enum_methods()?;
        let target = obj as Ptr;
        // GetEnumerator -> the enumerator object (boxed when it is a struct).
        let enumerator = match unsafe { self.invoke_virtual(target, methods.get_enumerator as Ptr) }
        {
            Some(e) if !e.is_null() => e,
            _ => return Ok(Vec::new()),
        };
        let mut out = Vec::new();
        loop {
            // MoveNext -> a boxed bool; unbox and read the byte.
            let moved = match unsafe { self.invoke_virtual(enumerator, methods.move_next as Ptr) } {
                Some(m) if !m.is_null() => m,
                _ => break,
            };
            let unboxed = unsafe { (self.api.object_unbox)(moved) };
            if unboxed.is_null() || unsafe { std::ptr::read(unboxed as *const u8) } == 0 {
                break;
            }
            // get_Current -> the element (a reference type, returned directly).
            match unsafe { self.invoke_virtual(enumerator, methods.get_current as Ptr) } {
                Some(current) => out.push(current as usize),
                None => break,
            }
        }
        Ok(out)
    }
}
