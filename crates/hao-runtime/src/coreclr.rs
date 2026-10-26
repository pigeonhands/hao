//! Hosting CoreCLR (.NET 8+) by loading `libcoreclr` directly.
//!
//! The runtime is configured in memory, and the delegates hostfxr would hand
//! out come straight from `ComponentActivator` in `System.Private.CoreLib`, so
//! no `.runtimeconfig.json` is needed.

use std::{
    ffi::{c_char, c_void, CString},
    path::{Path, PathBuf},
    ptr,
    sync::OnceLock,
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

type Initialize = unsafe extern "C" fn(
    *const c_char,
    *const c_char,
    i32,
    *const *const c_char,
    *const *const c_char,
    *mut *mut c_void,
    *mut u32,
) -> i32;
type CreateDelegate = unsafe extern "C" fn(
    *mut c_void,
    u32,
    *const c_char,
    *const c_char,
    *const c_char,
    *mut *mut c_void,
) -> i32;

type GetFunctionPointer = unsafe extern "system" fn(
    *const CharT,
    *const CharT,
    *const CharT,
    *mut c_void,
    *mut c_void,
    *mut *mut c_void,
) -> i32;
type LoadFrom = unsafe extern "system" fn(*const CharT) -> i32;
type LoadAssemblyBytes =
    unsafe extern "system" fn(*const u8, usize, *const u8, usize, *mut c_void, *mut c_void) -> i32;

const UNMANAGED_CALLERS_ONLY_METHOD: *const CharT = usize::MAX as *const CharT;

fn check(function: &'static str, code: i32) -> Result<()> {
    // An HRESULT, negative on failure.
    if code >= 0 {
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
    _coreclr: Library,
    major: u16,
    get_function_pointer: GetFunctionPointer,
    load_from: OnceLock<LoadFrom>,
    load_assembly_bytes: LoadAssemblyBytes,
}

// The ComponentActivator delegates are thread safe.
unsafe impl Send for CoreClr {}
unsafe impl Sync for CoreClr {}

fn parse_version(name: &str) -> Vec<u32> {
    name.split(['.', '-'])
        .map(|p| p.parse().unwrap_or(0))
        .collect()
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
        .find(|p| p.join("shared").join("Microsoft.NETCore.App").is_dir())
}

fn coreclr_name() -> &'static str {
    if cfg!(windows) {
        "coreclr.dll"
    } else if cfg!(target_os = "macos") {
        "libcoreclr.dylib"
    } else {
        "libcoreclr.so"
    }
}

fn c_string(value: impl Into<Vec<u8>>) -> Result<CString> {
    CString::new(value).map_err(|e| Error::Other(format!("nul byte in {:?}", e.into_vec())))
}

impl CoreClr {
    pub fn new(options: &CoreClrOptions) -> Result<Self> {
        let root = options
            .dotnet_root
            .clone()
            .or_else(find_dotnet_root)
            .ok_or_else(|| Error::NotFound("a .NET installation (set DOTNET_ROOT)".into()))?;

        let frameworks = root.join("shared").join("Microsoft.NETCore.App");
        let framework_dir = match &options.framework_version {
            Some(v) => frameworks.join(v),
            None => newest_dir(&frameworks).ok_or_else(|| {
                Error::NotFound(format!(
                    "the Microsoft.NETCore.App framework in {}",
                    root.display()
                ))
            })?,
        };
        let framework = framework_dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        if parse_version(&framework)[0] < 8 {
            return Err(Error::Other(format!(
                "hosting needs .NET 8 or newer, found {}",
                framework
            )));
        }

        let separator = if cfg!(windows) { ";" } else { ":" };
        let trusted: Vec<String> = std::fs::read_dir(&framework_dir)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|e| e == "dll"))
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        let exe = std::env::current_exe()?;
        let app_dir = exe.parent().unwrap_or(Path::new("."));
        let properties = [
            ("TRUSTED_PLATFORM_ASSEMBLIES", trusted.join(separator)),
            (
                "NATIVE_DLL_SEARCH_DIRECTORIES",
                format!("{}{}", framework_dir.display(), separator),
            ),
            (
                "APP_CONTEXT_BASE_DIRECTORY",
                format!("{}{}", app_dir.display(), std::path::MAIN_SEPARATOR),
            ),
            (
                "System.Runtime.InteropServices.EnableConsumingManagedCodeFromNativeHosting",
                "true".into(),
            ),
        ];
        let keys = properties
            .iter()
            .map(|(k, _)| c_string(*k))
            .collect::<Result<Vec<_>>>()?;
        let values = properties
            .iter()
            .map(|(_, v)| c_string(v.as_str()))
            .collect::<Result<Vec<_>>>()?;
        let key_ptrs: Vec<_> = keys.iter().map(|k| k.as_ptr()).collect();
        let value_ptrs: Vec<_> = values.iter().map(|v| v.as_ptr()).collect();

        let path = framework_dir.join(coreclr_name());
        let coreclr = unsafe { Library::new(&path) }
            .map_err(|e| Error::Library(path.display().to_string(), e))?;

        unsafe {
            let initialize: Initialize = *coreclr
                .get(b"coreclr_initialize\0")
                .map_err(|e| Error::Library("coreclr_initialize".into(), e))?;
            let create_delegate: CreateDelegate = *coreclr
                .get(b"coreclr_create_delegate\0")
                .map_err(|e| Error::Library("coreclr_create_delegate".into(), e))?;

            let exe_path = c_string(exe.to_string_lossy().into_owned())?;
            let (mut handle, mut domain) = (ptr::null_mut(), 0);
            let code = initialize(
                exe_path.as_ptr(),
                c"hao-runtime".as_ptr(),
                key_ptrs.len() as i32,
                key_ptrs.as_ptr(),
                value_ptrs.as_ptr(),
                &mut handle,
                &mut domain,
            );
            check("coreclr_initialize", code)?;

            let delegate = |method: &'static str| -> Result<*mut c_void> {
                let name = c_string(method)?;
                let mut delegate = ptr::null_mut();
                let code = create_delegate(
                    handle,
                    domain,
                    c"System.Private.CoreLib".as_ptr(),
                    c"Internal.Runtime.InteropServices.ComponentActivator".as_ptr(),
                    name.as_ptr(),
                    &mut delegate,
                );
                check(method, code)?;
                Ok(delegate)
            };

            Ok(Self {
                get_function_pointer: std::mem::transmute::<*mut c_void, GetFunctionPointer>(
                    delegate("GetFunctionPointer")?,
                ),
                load_assembly_bytes: std::mem::transmute::<*mut c_void, LoadAssemblyBytes>(
                    delegate("LoadAssemblyBytes")?,
                ),
                _coreclr: coreclr,
                major: parse_version(&framework)[0] as u16,
                load_from: OnceLock::new(),
            })
        }
    }

    /// Loads with `Assembly.LoadFrom`, so dependencies next to the assembly
    /// are found. `ComponentActivator.LoadAssembly` would need hostfxr.
    pub fn load_assembly(&self, path: &Path) -> Result<()> {
        let load_from = match self.load_from.get() {
            Some(f) => *f,
            None => {
                self.load_assembly_bytes(&loader_assembly(self.major)?)?;
                let f = self.function_pointer(LOADER, "Hao.Loader", "LoadFrom")?;
                *self
                    .load_from
                    .get_or_init(|| unsafe { std::mem::transmute::<*const c_void, LoadFrom>(f) })
            }
        };
        let path = to_char_t(&path.to_string_lossy());
        check("Assembly.LoadFrom", unsafe { load_from(path.as_ptr()) })
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
    pub fn function_pointer(
        &self,
        assembly: &str,
        type_name: &str,
        method: &str,
    ) -> Result<*const c_void> {
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

const LOADER: &str = "Hao.Loader";

/// `static class Hao.Loader { [UnmanagedCallersOnly] static int LoadFrom(IntPtr path) }`, which
/// calls `Assembly.LoadFrom` and returns the exception's HResult on failure.
fn loader_assembly(major: u16) -> Result<Vec<u8>> {
    use hao::{
        dotnet::{
            entries::{Method, TypeDef},
            il::Emitter,
            module::{ModuleKind, TargetRuntime},
            signature::{MethodSig, TypeSig},
        },
        Module,
    };
    let build = || -> hao::error::Result<Vec<u8>> {
        let mut module = Module::new(
            LOADER,
            ModuleKind::Dll,
            TargetRuntime::NetCoreImplementation(major),
        );
        let uco = module
            .corlib_attribute("System.Runtime.InteropServices.UnmanagedCallersOnlyAttribute")?;
        let to_string = module.import_corlib_method(
            "System.Runtime.InteropServices.Marshal",
            "PtrToStringAuto",
            MethodSig::new_static(TypeSig::STRING, vec![TypeSig::NATIVE_INT]),
        )?;
        let assembly = module.corlib_type("System.Reflection.Assembly")?;
        let load_from = module.import_method_on(
            &assembly,
            "LoadFrom",
            MethodSig::new_static(TypeSig::class(assembly.clone()), vec![TypeSig::STRING]),
        )?;
        let exception = module.corlib_type("System.Exception")?;
        let hresult = module.import_method_on(
            &exception,
            "get_HResult",
            MethodSig::new_instance(TypeSig::INT32, vec![]),
        )?;

        // try { Assembly.LoadFrom(Marshal.PtrToStringAuto(path)); } catch (Exception e) { result = e.HResult; }
        let mut e = Emitter::new();
        let result = e.declare_local(TypeSig::INT32);
        e.try_catch(
            exception,
            |e| {
                e.ldarg(0).call(to_string).call(load_from).pop();
            },
            |e| {
                e.callvirt(hresult).stloc(result);
            },
        );
        e.ldloc(result).ret();

        let loader = module.add_static_class("Hao", "Loader")?;
        let signature = MethodSig::new_static(TypeSig::INT32, vec![TypeSig::NATIVE_INT]);
        TypeDef::add_method(
            &loader,
            Method::public_static("LoadFrom", signature)
                .with_attribute(uco)
                .with_body(e.finish()),
        );
        module.write()
    };
    build().map_err(|e| Error::Other(format!("could not build the loader assembly: {}", e)))
}
