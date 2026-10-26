//! Hosting CoreCLR (.NET 8+) through hostfxr, the same way `dotnet` does.

use std::{
    ffi::c_void,
    path::{Path, PathBuf},
    ptr,
};

use libloading::Library;

use crate::error::{Error, Result};

#[cfg(windows)]
type CharT = u16;
#[cfg(not(windows))]
type CharT = std::ffi::c_char;

fn to_char_t(value: &str) -> Vec<CharT> {
    #[cfg(windows)]
    {
        value.encode_utf16().chain([0]).collect()
    }
    #[cfg(not(windows))]
    {
        value.bytes().map(|b| b as CharT).chain([0]).collect()
    }
}

type HostfxrHandle = *mut c_void;
type InitializeForRuntimeConfig =
    unsafe extern "C" fn(*const CharT, *const c_void, *mut HostfxrHandle) -> i32;
type GetRuntimeDelegate = unsafe extern "C" fn(HostfxrHandle, i32, *mut *mut c_void) -> i32;

type GetFunctionPointer = unsafe extern "system" fn(
    *const CharT,
    *const CharT,
    *const CharT,
    *mut c_void,
    *mut c_void,
    *mut *mut c_void,
) -> i32;
type LoadAssembly = unsafe extern "system" fn(*const CharT, *mut c_void, *mut c_void) -> i32;
type LoadAssemblyBytes = unsafe extern "system" fn(
    *const u8,
    usize,
    *const u8,
    usize,
    *mut c_void,
    *mut c_void,
) -> i32;

const HDT_GET_FUNCTION_POINTER: i32 = 6;
const HDT_LOAD_ASSEMBLY: i32 = 7;
const HDT_LOAD_ASSEMBLY_BYTES: i32 = 8;
const UNMANAGED_CALLERS_ONLY_METHOD: *const CharT = usize::MAX as *const CharT;

fn check(function: &'static str, code: i32) -> Result<()> {
    // 0 = Success, 1 = Success_HostAlreadyInitialized, 2 = Success_DifferentRuntimeProperties
    if (0..=2).contains(&code) {
        Ok(())
    } else {
        Err(Error::Host {
            function,
            code: code as u32,
        })
    }
}

/// Where to find .NET and which framework version to run.
#[derive(Debug, Clone, Default)]
pub struct CoreClrOptions {
    /// Defaults to `DOTNET_ROOT` or the standard install locations.
    pub dotnet_root: Option<PathBuf>,
    /// A `Microsoft.NETCore.App` version such as `9.0.21`. Defaults to the newest installed.
    pub framework_version: Option<String>,
}

pub(crate) struct CoreClr {
    _hostfxr: Library,
    get_function_pointer: GetFunctionPointer,
    load_assembly: LoadAssembly,
    load_assembly_bytes: LoadAssemblyBytes,
}

// The hostfxr delegates are thread safe.
unsafe impl Send for CoreClr {}
unsafe impl Sync for CoreClr {}

fn parse_version(name: &str) -> Vec<u32> {
    name.split(['.', '-']).map(|p| p.parse().unwrap_or(0)).collect()
}

fn newest_dir(dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .max_by_key(|p| parse_version(&p.file_name().unwrap_or_default().to_string_lossy()))
}

pub fn find_dotnet_root() -> Option<PathBuf> {
    std::env::var_os("DOTNET_ROOT")
        .map(PathBuf::from)
        .into_iter()
        .chain(
            [
                "/usr/share/dotnet",
                "/usr/lib/dotnet",
                "/usr/local/share/dotnet",
                "/opt/dotnet",
                r"C:\Program Files\dotnet",
            ]
            .map(PathBuf::from),
        )
        .find(|p| p.join("host").join("fxr").is_dir())
}

fn hostfxr_name() -> &'static str {
    if cfg!(windows) {
        "hostfxr.dll"
    } else if cfg!(target_os = "macos") {
        "libhostfxr.dylib"
    } else {
        "libhostfxr.so"
    }
}

impl CoreClr {
    pub fn new(options: &CoreClrOptions) -> Result<Self> {
        let root = options
            .dotnet_root
            .clone()
            .or_else(find_dotnet_root)
            .ok_or_else(|| Error::NotFound("a .NET installation (set DOTNET_ROOT)".into()))?;

        let fxr_dir = newest_dir(&root.join("host").join("fxr"))
            .ok_or_else(|| Error::NotFound(format!("hostfxr in {}", root.display())))?;
        let framework = match &options.framework_version {
            Some(v) => v.clone(),
            None => newest_dir(&root.join("shared").join("Microsoft.NETCore.App"))
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                .ok_or_else(|| Error::NotFound("the Microsoft.NETCore.App framework".into()))?,
        };
        let major = parse_version(&framework)[0];
        if major < 8 {
            return Err(Error::Other(format!(
                "hosting needs .NET 8 or newer, found {}",
                framework
            )));
        }

        let config = std::env::temp_dir().join(format!("hao-runtime-{}.runtimeconfig.json", std::process::id()));
        std::fs::write(
            &config,
            format!(
                r#"{{"runtimeOptions":{{"tfm":"net{major}.0","framework":{{"name":"Microsoft.NETCore.App","version":"{framework}"}}}}}}"#
            ),
        )?;

        let path = fxr_dir.join(hostfxr_name());
        let hostfxr = unsafe { Library::new(&path) }
            .map_err(|e| Error::Library(path.display().to_string(), e))?;

        unsafe {
            let initialize: InitializeForRuntimeConfig = *hostfxr
                .get(b"hostfxr_initialize_for_runtime_config\0")
                .map_err(|e| Error::Library("hostfxr_initialize_for_runtime_config".into(), e))?;
            let get_delegate: GetRuntimeDelegate = *hostfxr
                .get(b"hostfxr_get_runtime_delegate\0")
                .map_err(|e| Error::Library("hostfxr_get_runtime_delegate".into(), e))?;

            let mut handle = ptr::null_mut();
            let config_path = to_char_t(&config.to_string_lossy());
            let code = initialize(config_path.as_ptr(), ptr::null(), &mut handle);
            let _ = std::fs::remove_file(&config);
            check("hostfxr_initialize_for_runtime_config", code)?;

            let delegate = |kind: i32, name: &'static str| -> Result<*mut c_void> {
                let mut delegate = ptr::null_mut();
                check(name, get_delegate(handle, kind, &mut delegate))?;
                Ok(delegate)
            };
            let get_function_pointer = delegate(HDT_GET_FUNCTION_POINTER, "hdt_get_function_pointer")?;
            let load_assembly = delegate(HDT_LOAD_ASSEMBLY, "hdt_load_assembly")?;
            let load_assembly_bytes = delegate(HDT_LOAD_ASSEMBLY_BYTES, "hdt_load_assembly_bytes")?;

            Ok(Self {
                get_function_pointer: std::mem::transmute::<*mut c_void, GetFunctionPointer>(get_function_pointer),
                load_assembly: std::mem::transmute::<*mut c_void, LoadAssembly>(load_assembly),
                load_assembly_bytes: std::mem::transmute::<*mut c_void, LoadAssemblyBytes>(load_assembly_bytes),
                _hostfxr: hostfxr,
            })
        }
    }

    pub fn load_assembly(&self, path: &Path) -> Result<()> {
        let path = to_char_t(&path.to_string_lossy());
        let code = unsafe { (self.load_assembly)(path.as_ptr(), ptr::null_mut(), ptr::null_mut()) };
        check("load_assembly", code)
    }

    pub fn load_assembly_bytes(&self, bytes: &[u8]) -> Result<()> {
        let code = unsafe {
            (self.load_assembly_bytes)(
                bytes.as_ptr(),
                bytes.len(),
                ptr::null(),
                0,
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };
        check("load_assembly_bytes", code)
    }

    /// A pointer to a static `[UnmanagedCallersOnly]` method.
    pub fn function_pointer(&self, assembly: &str, type_name: &str, method: &str) -> Result<*const c_void> {
        let type_name = to_char_t(&format!("{}, {}", type_name, assembly));
        let method = to_char_t(method);
        let mut function = ptr::null_mut();
        let code = unsafe {
            (self.get_function_pointer)(
                type_name.as_ptr(),
                method.as_ptr(),
                UNMANAGED_CALLERS_ONLY_METHOD,
                ptr::null_mut(),
                ptr::null_mut(),
                &mut function,
            )
        };
        check("get_function_pointer", code)?;
        Ok(function)
    }
}
