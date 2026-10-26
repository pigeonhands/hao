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

/// A `char_t`, the character type the hosting APIs take. Wide on Windows.
#[cfg(windows)]
type CharT = u16;
#[cfg(not(windows))]
type CharT = c_char;

/// Encodes a string as a nul-terminated `char_t` array.
fn wide(value: &str) -> Vec<CharT> {
    #[cfg(windows)]
    return value.encode_utf16().chain([0]).collect();
    #[cfg(not(windows))]
    return value.bytes().map(|b| b as CharT).chain([0]).collect();
}

fn c_string(value: impl Into<Vec<u8>>) -> Result<CString> {
    CString::new(value).map_err(|e| Error::Other(format!("nul byte in {:?}", e.into_vec())))
}

// coreclr_initialize / coreclr_create_delegate.
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

// Delegates from ComponentActivator.
type GetFunctionPointer = unsafe extern "system" fn(
    *const CharT,
    *const CharT,
    *const CharT,
    *mut c_void,
    *mut c_void,
    *mut *mut c_void,
) -> i32;
type LoadAssemblyBytes =
    unsafe extern "system" fn(*const u8, usize, *const u8, usize, *mut c_void, *mut c_void) -> i32;
type LoadFrom = unsafe extern "system" fn(*const CharT) -> i32;
type GetMethod = unsafe extern "system" fn(*const CharT, *const CharT) -> *mut c_void;

/// `GetFunctionPointer`'s sentinel for a `[UnmanagedCallersOnly]` method.
const UNMANAGED_CALLERS_ONLY: *const CharT = usize::MAX as *const CharT;

const ACTIVATOR: &[u8] = b"Internal.Runtime.InteropServices.ComponentActivator\0";

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

const DOTNET_ROOTS: &[&str] = &[
    "/usr/share/dotnet",
    "/usr/lib/dotnet",
    "/usr/local/share/dotnet",
    "/opt/dotnet",
    r"C:\Program Files\dotnet",
];

pub fn find_dotnet_root() -> Option<PathBuf> {
    std::env::var_os("DOTNET_ROOT")
        .map(PathBuf::from)
        .into_iter()
        .chain(DOTNET_ROOTS.iter().map(PathBuf::from))
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

/// A resolved `Microsoft.NETCore.App` framework directory.
struct Framework {
    dir: PathBuf,
    major: u16,
}

impl Framework {
    fn find(options: &CoreClrOptions) -> Result<Self> {
        let root = options
            .dotnet_root
            .clone()
            .or_else(find_dotnet_root)
            .ok_or_else(|| Error::NotFound("a .NET installation (set DOTNET_ROOT)".into()))?;
        let frameworks = root.join("shared").join("Microsoft.NETCore.App");
        let dir = match &options.framework_version {
            Some(v) => frameworks.join(v),
            None => newest_dir(&frameworks).ok_or_else(|| {
                Error::NotFound(format!("Microsoft.NETCore.App in {}", root.display()))
            })?,
        };
        let version = dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let major = parse_version(&version)[0] as u16;
        if major < 8 {
            return Err(Error::Other(format!(
                "hosting needs .NET 8 or newer, found {}",
                version
            )));
        }
        Ok(Self { dir, major })
    }

    /// The runtime properties CoreCLR needs: the trusted assemblies, where to
    /// find native libraries, and the hosting switch the delegates need.
    fn properties(&self) -> Result<Vec<(&'static str, String)>> {
        let separator = if cfg!(windows) { ";" } else { ":" };
        let trusted = std::fs::read_dir(&self.dir)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|e| e == "dll"))
            .map(|p| p.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(separator);
        let exe = std::env::current_exe()?;
        let app_dir = exe.parent().unwrap_or(Path::new("."));
        Ok(vec![
            ("TRUSTED_PLATFORM_ASSEMBLIES", trusted),
            (
                "NATIVE_DLL_SEARCH_DIRECTORIES",
                format!("{}{}", self.dir.display(), separator),
            ),
            (
                "APP_CONTEXT_BASE_DIRECTORY",
                format!("{}{}", app_dir.display(), std::path::MAIN_SEPARATOR),
            ),
            (
                "System.Runtime.InteropServices.EnableConsumingManagedCodeFromNativeHosting",
                "true".into(),
            ),
        ])
    }
}

/// A started runtime's host handle, used to fetch delegates.
struct Host {
    handle: *mut c_void,
    domain: u32,
    create_delegate: CreateDelegate,
}

impl Host {
    /// Starts the runtime from `library` with `framework`'s properties.
    fn start(library: &Library, framework: &Framework) -> Result<Self> {
        let properties = framework.properties()?;
        let keys = properties
            .iter()
            .map(|(k, _)| c_string(*k))
            .collect::<Result<Vec<_>>>()?;
        let values = properties
            .iter()
            .map(|(_, v)| c_string(v.as_str()))
            .collect::<Result<Vec<_>>>()?;
        let key_ptrs = keys.iter().map(|k| k.as_ptr()).collect::<Vec<_>>();
        let value_ptrs = values.iter().map(|v| v.as_ptr()).collect::<Vec<_>>();
        let exe = c_string(std::env::current_exe()?.to_string_lossy().into_owned())?;

        unsafe {
            let initialize: Initialize = *library
                .get(b"coreclr_initialize\0")
                .map_err(|e| Error::Library("coreclr_initialize".into(), e))?;
            let create_delegate: CreateDelegate = *library
                .get(b"coreclr_create_delegate\0")
                .map_err(|e| Error::Library("coreclr_create_delegate".into(), e))?;
            let (mut handle, mut domain) = (ptr::null_mut(), 0);
            let code = initialize(
                exe.as_ptr(),
                c"hao-runtime".as_ptr(),
                key_ptrs.len() as i32,
                key_ptrs.as_ptr(),
                value_ptrs.as_ptr(),
                &mut handle,
                &mut domain,
            );
            check("coreclr_initialize", code)?;
            Ok(Self {
                handle,
                domain,
                create_delegate,
            })
        }
    }

    /// A delegate for a static method of `ComponentActivator`.
    fn activator_delegate<T>(&self, method: &'static str) -> Result<T> {
        let name = c_string(method)?;
        let mut delegate = ptr::null_mut();
        let code = unsafe {
            (self.create_delegate)(
                self.handle,
                self.domain,
                c"System.Private.CoreLib".as_ptr(),
                ACTIVATOR.as_ptr().cast(),
                name.as_ptr(),
                &mut delegate,
            )
        };
        check(method, code)?;
        assert_eq!(std::mem::size_of::<T>(), std::mem::size_of::<*mut c_void>());
        Ok(unsafe { std::mem::transmute_copy(&delegate) })
    }
}

/// The `Hao.Loader` helpers, built and loaded on first use.
struct Loader {
    load_from: LoadFrom,
    get_method: GetMethod,
}

pub(crate) struct CoreClr {
    _coreclr: Library,
    major: u16,
    get_function_pointer: GetFunctionPointer,
    load_assembly_bytes: LoadAssemblyBytes,
    loader: OnceLock<Loader>,
}

// The ComponentActivator delegates are thread safe.
unsafe impl Send for CoreClr {}
unsafe impl Sync for CoreClr {}

impl CoreClr {
    pub fn new(options: &CoreClrOptions) -> Result<Self> {
        let framework = Framework::find(options)?;
        let path = framework.dir.join(coreclr_name());
        let coreclr = unsafe { Library::new(&path) }
            .map_err(|e| Error::Library(path.display().to_string(), e))?;
        let host = Host::start(&coreclr, &framework)?;
        Ok(Self {
            major: framework.major,
            get_function_pointer: host.activator_delegate("GetFunctionPointer")?,
            load_assembly_bytes: host.activator_delegate("LoadAssemblyBytes")?,
            loader: OnceLock::new(),
            _coreclr: coreclr,
        })
    }

    /// The `Hao.Loader` helpers, generated with hao and loaded once.
    fn loader(&self) -> Result<&Loader> {
        if let Some(loader) = self.loader.get() {
            return Ok(loader);
        }
        self.load_assembly_bytes(&loader_assembly(self.major)?)?;
        let load_from = self.function_pointer(LOADER, "Hao.Loader", "LoadFrom")?;
        let get_method = self.function_pointer(LOADER, "Hao.Loader", "GetMethod")?;
        Ok(self.loader.get_or_init(|| unsafe {
            Loader {
                load_from: std::mem::transmute::<*const c_void, LoadFrom>(load_from),
                get_method: std::mem::transmute::<*const c_void, GetMethod>(get_method),
            }
        }))
    }

    /// Loads with `Assembly.LoadFrom`, so dependencies next to the assembly
    /// are found. `ComponentActivator.LoadAssembly` would need hostfxr.
    pub fn load_assembly(&self, path: &Path) -> Result<()> {
        let load_from = self.loader()?.load_from;
        check("Assembly.LoadFrom", unsafe {
            load_from(wide(&path.to_string_lossy()).as_ptr())
        })
    }

    /// A native-callable pointer to a static method, whether or not it is
    /// `[UnmanagedCallersOnly]`. The method is JIT-compiled first (so IL patched
    /// before the assembly was loaded takes effect), and the pointer uses the
    /// managed calling convention, which matches the C ABI for static methods
    /// with primitive and pointer arguments.
    pub fn method_pointer(
        &self,
        assembly: &str,
        type_name: &str,
        method: &str,
    ) -> Result<*const c_void> {
        let full_name = wide(&format!("{}, {}", type_name, assembly));
        let name = wide(method);
        let pointer = unsafe { (self.loader()?.get_method)(full_name.as_ptr(), name.as_ptr()) };
        if pointer.is_null() {
            return Err(Error::NotFound(format!(
                "method {}::{} (is it a unique static method?)",
                type_name, method
            )));
        }
        Ok(pointer)
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
        let type_name = wide(&format!("{}, {}", type_name, assembly));
        let method = wide(method);
        let mut function = ptr::null_mut();
        let code = unsafe {
            (self.get_function_pointer)(
                type_name.as_ptr(),
                method.as_ptr(),
                UNMANAGED_CALLERS_ONLY,
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

/// A small `Hao.Loader` assembly with two `[UnmanagedCallersOnly]` helpers:
///
/// - `int LoadFrom(IntPtr path)`: `Assembly.LoadFrom`, returning the exception's HResult on failure.
/// - `IntPtr GetMethod(IntPtr fullName, IntPtr name)`: looks up a static method by reflection,
///   JIT-compiles it and returns its native entry point (0 on failure), so any method can be
///   called, not only `[UnmanagedCallersOnly]` ones.
fn loader_assembly(major: u16) -> Result<Vec<u8>> {
    use hao::{
        dotnet::{
            entries::{Method, TypeDef},
            il::{Emitter, OpCode},
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
        let exception = module.corlib_type("System.Exception")?;

        // int LoadFrom(IntPtr path):
        //   try { Assembly.LoadFrom(Marshal.PtrToStringAuto(path)); } catch (Exception e) { result = e.HResult; }
        let assembly = module.corlib_type("System.Reflection.Assembly")?;
        let load_from = module.import_method_on(
            &assembly,
            "LoadFrom",
            MethodSig::new_static(TypeSig::class(assembly.clone()), vec![TypeSig::STRING]),
        )?;
        let hresult = module.import_method_on(
            &exception,
            "get_HResult",
            MethodSig::new_instance(TypeSig::INT32, vec![]),
        )?;
        let load_from_body = Emitter::build(|e| {
            let result = e.declare_local(TypeSig::INT32);
            e.try_catch(
                exception.clone(),
                |e| {
                    e.ldarg(0).call(&to_string).call(load_from).pop();
                },
                |e| {
                    e.callvirt(hresult).stloc(result);
                },
            );
            e.ldloc(result).ret();
        });

        // IntPtr GetMethod(IntPtr fullName, IntPtr name):
        //   var m = Type.GetType(fullName).GetMethod(name, flags);
        //   RuntimeHelpers.PrepareMethod(m.MethodHandle);
        //   return m.MethodHandle.GetFunctionPointer();   // 0 on any exception
        let type_ty = module.corlib_type("System.Type")?;
        let method_info = module.corlib_type("System.Reflection.MethodInfo")?;
        let method_base = module.corlib_type("System.Reflection.MethodBase")?;
        let handle_ty = module.corlib_type("System.RuntimeMethodHandle")?;
        let binding_flags = module.corlib_type("System.Reflection.BindingFlags")?;
        let get_type = module.import_method_on(
            &type_ty,
            "GetType",
            MethodSig::new_static(TypeSig::class(type_ty.clone()), vec![TypeSig::STRING]),
        )?;
        let get_method = module.import_method_on(
            &type_ty,
            "GetMethod",
            MethodSig::new_instance(
                TypeSig::class(method_info.clone()),
                vec![TypeSig::STRING, TypeSig::value_type(binding_flags)],
            ),
        )?;
        let method_handle = module.import_method_on(
            &method_base,
            "get_MethodHandle",
            MethodSig::new_instance(TypeSig::value_type(handle_ty.clone()), vec![]),
        )?;
        let prepare = module.import_corlib_method(
            "System.Runtime.CompilerServices.RuntimeHelpers",
            "PrepareMethod",
            MethodSig::new_static(TypeSig::VOID, vec![TypeSig::value_type(handle_ty.clone())]),
        )?;
        // GetFunctionPointer returns `nint`, encoded as the native-int primitive, not System.IntPtr.
        let function_pointer = module.import_method_on(
            &handle_ty,
            "GetFunctionPointer",
            MethodSig::new_instance(TypeSig::NATIVE_INT, vec![]),
        )?;

        // Instance | Static | Public | NonPublic
        const BINDING_FLAGS: i32 = 4 | 8 | 16 | 32;
        let get_method_body = Emitter::build(|e| {
            let found_type = e.declare_local(TypeSig::class(type_ty.clone()));
            let method = e.declare_local(TypeSig::class(method_info.clone()));
            let handle = e.declare_local(TypeSig::value_type(handle_ty.clone()));
            let result = e.declare_local(TypeSig::NATIVE_INT);
            e.ldc_i4(0).op(OpCode::Conv_I).stloc(result);
            e.try_catch(
                exception,
                |e| {
                    e.ldarg(0).call(&to_string).call(get_type).stloc(found_type);
                    e.ldloc(found_type)
                        .ldarg(1)
                        .call(&to_string)
                        .ldc_i4(BINDING_FLAGS)
                        .callvirt(get_method)
                        .stloc(method);
                    e.ldloc(method).callvirt(method_handle).stloc(handle);
                    e.ldloc(handle).call(prepare);
                    e.ldloca(handle).call(function_pointer).stloc(result);
                },
                |e| {
                    e.pop(); // discard the exception; result stays 0
                },
            );
            e.ldloc(result).ret();
        });

        let loader = module.add_static_class("Hao", "Loader")?;
        TypeDef::add_method(
            &loader,
            Method::public_static(
                "LoadFrom",
                MethodSig::new_static(TypeSig::INT32, vec![TypeSig::NATIVE_INT]),
            )
            .with_attribute(uco.clone())
            .with_body(load_from_body),
        );
        TypeDef::add_method(
            &loader,
            Method::public_static(
                "GetMethod",
                MethodSig::new_static(
                    TypeSig::NATIVE_INT,
                    vec![TypeSig::NATIVE_INT, TypeSig::NATIVE_INT],
                ),
            )
            .with_attribute(uco)
            .with_body(get_method_body),
        );
        module.write()
    };
    build().map_err(|e| Error::Other(format!("could not build the loader assembly: {}", e)))
}
