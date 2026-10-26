//! Hosting Mono through its embedding API.

use std::{
    collections::HashMap,
    ffi::{c_char, c_int, c_void, CString},
    path::{Path, PathBuf},
    sync::Mutex,
};

use libloading::Library;

use crate::error::{Error, Result};

type Ptr = *mut c_void;

/// Where to find Mono.
#[derive(Debug, Clone, Default)]
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
}

pub(crate) struct Mono {
    _library: Library,
    api: Api,
    domain: Ptr,
    images: Mutex<HashMap<String, usize>>,
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

impl Mono {
    pub fn new(options: &MonoOptions) -> Result<Self> {
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
        };

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

    pub fn function_pointer(
        &self,
        assembly: &str,
        type_name: &str,
        method: &str,
    ) -> Result<*const c_void> {
        self.attach_thread();
        let image = *self
            .images
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(assembly)
            .ok_or_else(|| Error::NotFound(format!("loaded assembly {}", assembly)))?;
        let (namespace, name) = type_name.rsplit_once('.').unwrap_or(("", type_name));
        let (namespace, name, method_name) =
            (cstring(namespace)?, cstring(name)?, cstring(method)?);
        unsafe {
            let class = (self.api.class_from_name)(image as Ptr, namespace.as_ptr(), name.as_ptr());
            if class.is_null() {
                return Err(Error::NotFound(format!(
                    "type {} in {}",
                    type_name, assembly
                )));
            }
            let method_ptr = (self.api.class_get_method_from_name)(class, method_name.as_ptr(), -1);
            if method_ptr.is_null() {
                return Err(Error::NotFound(format!("method {}::{}", type_name, method)));
            }
            let thunk = (self.api.method_get_unmanaged_thunk)(method_ptr);
            if thunk.is_null() {
                return Err(Error::Other(format!(
                    "no unmanaged thunk for {}::{}",
                    type_name, method
                )));
            }
            Ok(thunk)
        }
    }
}
