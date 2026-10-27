//! Host the .NET runtime in a Rust process and call into managed code.
//!
//! Two backends are available behind features:
//! - `coreclr` (default): .NET 8+, loading `libcoreclr` directly.
//! - `mono`: the Mono embedding API, as used by Unity.
//!
//! Managed methods are called through native function pointers, so the
//! methods have to be callable from native code: `[UnmanagedCallersOnly]`
//! static methods on CoreCLR, or any static method on Mono. Bindings generated
//! by `hao-runtime-macros` produce such methods for you.

pub mod bindings;
#[cfg(feature = "coreclr")]
mod coreclr;
mod error;
#[cfg(feature = "coreclr")]
pub mod profiler;
#[cfg(feature = "mono")]
mod mono;

use std::{ffi::c_void, path::Path, sync::Arc};

pub use bindings::{BoundAssembly, Object};
pub use error::{Error, Result};

#[cfg(feature = "coreclr")]
pub use coreclr::{find_dotnet_root, CoreClrOptions};
#[cfg(feature = "mono")]
pub use mono::MonoOptions;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    CoreClr,
    Mono,
}

enum Inner {
    #[cfg(feature = "coreclr")]
    CoreClr(coreclr::CoreClr),
    #[cfg(feature = "mono")]
    Mono(mono::Mono),
}

/// A handle to a running .NET runtime. Cheap to clone.
#[derive(Clone)]
pub struct Runtime(Arc<Inner>);

impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Runtime({:?})", self.backend())
    }
}

impl Runtime {
    /// Starts CoreCLR with the newest installed framework, or returns the
    /// already running instance. Only one CoreCLR can run per process.
    #[cfg(feature = "coreclr")]
    pub fn coreclr() -> Result<Self> {
        Self::coreclr_with(&CoreClrOptions::default())
    }

    /// Starts CoreCLR with `options`. The options are ignored if it is already running.
    #[cfg(feature = "coreclr")]
    pub fn coreclr_with(options: &CoreClrOptions) -> Result<Self> {
        Self::coreclr_from(|| coreclr::CoreClr::new(options))
    }

    /// Attaches to a CoreCLR already running in this process, for a native mod
    /// loaded into a .NET game (e.g. Godot on CoreCLR). `host_handle`, `domain_id`
    /// and `create_delegate` (`coreclr_create_delegate`) are captured as the
    /// game's runtime initialises — see the `hao-godot` crate; `major` is the
    /// .NET major version. After this, assemblies can be loaded and methods
    /// called as usual. Returns the existing handle if hao already has one.
    #[cfg(feature = "coreclr")]
    pub fn coreclr_attach(
        host_handle: *mut c_void,
        domain_id: u32,
        create_delegate: *const c_void,
        major: u16,
    ) -> Result<Self> {
        Self::coreclr_from(|| {
            coreclr::CoreClr::attach(host_handle, domain_id, create_delegate, major)
        })
    }

    #[cfg(feature = "coreclr")]
    fn coreclr_from(make: impl FnOnce() -> Result<coreclr::CoreClr>) -> Result<Self> {
        static RUNTIME: std::sync::Mutex<Option<Runtime>> = std::sync::Mutex::new(None);
        let mut runtime = RUNTIME.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(runtime) = runtime.as_ref() {
            return Ok(runtime.clone());
        }
        let started = Runtime(Arc::new(Inner::CoreClr(make()?)));
        *runtime = Some(started.clone());
        Ok(started)
    }

    /// Starts Mono, or returns the already running instance.
    #[cfg(feature = "mono")]
    pub fn mono(options: &MonoOptions) -> Result<Self> {
        Self::mono_with(|| mono::Mono::new(options))
    }

    /// Attaches to a Mono runtime already running in this process, for a mod
    /// loaded into a Unity Mono game. `options.library` should point at the
    /// game's own Mono. After this, assemblies can be loaded and methods called
    /// as usual. Returns the existing handle if hao already has one.
    #[cfg(feature = "mono")]
    pub fn mono_attach(options: &MonoOptions) -> Result<Self> {
        Self::mono_with(|| mono::Mono::attach(options))
    }

    #[cfg(feature = "mono")]
    fn mono_with(make: impl FnOnce() -> Result<mono::Mono>) -> Result<Self> {
        static RUNTIME: std::sync::Mutex<Option<Runtime>> = std::sync::Mutex::new(None);
        let mut runtime = RUNTIME.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(runtime) = runtime.as_ref() {
            return Ok(runtime.clone());
        }
        let started = Runtime(Arc::new(Inner::Mono(make()?)));
        *runtime = Some(started.clone());
        Ok(started)
    }

    pub fn backend(&self) -> Backend {
        match &*self.0 {
            #[cfg(feature = "coreclr")]
            Inner::CoreClr(_) => Backend::CoreClr,
            #[cfg(feature = "mono")]
            Inner::Mono(_) => Backend::Mono,
        }
    }

    /// Lets the calling thread run managed code. Needed for Mono on threads it
    /// didn't start, a no-op on CoreCLR. Binding calls do this automatically.
    pub fn attach_thread(&self) {
        match &*self.0 {
            #[cfg(feature = "coreclr")]
            Inner::CoreClr(_) => {}
            #[cfg(feature = "mono")]
            Inner::Mono(mono) => mono.attach_thread(),
        }
    }

    /// Loads an assembly and makes it available to later loads by name.
    pub fn load_assembly(&self, path: impl AsRef<Path>) -> Result<()> {
        match &*self.0 {
            #[cfg(feature = "coreclr")]
            Inner::CoreClr(clr) => clr.load_assembly(path.as_ref()),
            #[cfg(feature = "mono")]
            Inner::Mono(mono) => mono.load_assembly(path.as_ref()),
        }
    }

    /// Loads an assembly from memory. `name` is its assembly name.
    pub fn load_assembly_bytes(&self, name: &str, bytes: &[u8]) -> Result<()> {
        match &*self.0 {
            #[cfg(feature = "coreclr")]
            Inner::CoreClr(clr) => {
                let _ = name;
                clr.load_assembly_bytes(bytes)
            }
            #[cfg(feature = "mono")]
            Inner::Mono(mono) => mono.load_assembly_bytes(name, bytes),
        }
    }

    /// Loads a module built or edited with hao, without writing it to disk.
    #[cfg(feature = "hao")]
    pub fn load_module(&self, module: &hao::Module) -> Result<()> {
        let bytes = module
            .write()
            .map_err(|e| Error::Other(format!("could not write {}: {}", module.name(), e)))?;
        let name = match &module.assembly {
            Some(assembly) => assembly.get().name.clone(),
            None => module.name().trim_end_matches(".dll").to_string(),
        };
        self.load_assembly_bytes(&name, &bytes)
    }

    /// A native pointer to the static method `type_name::method` in `assembly`.
    ///
    /// On CoreCLR the method must be `[UnmanagedCallersOnly]`; for a method that
    /// isn't, use [`method_pointer`](Self::method_pointer) instead, which calls
    /// any method by JIT-compiling it. On Mono the returned thunk takes an extra
    /// trailing `*mut *mut c_void` exception argument.
    pub fn function_pointer(
        &self,
        assembly: &str,
        type_name: &str,
        method: &str,
    ) -> Result<*const c_void> {
        match &*self.0 {
            #[cfg(feature = "coreclr")]
            Inner::CoreClr(clr) => clr.function_pointer(assembly, type_name, method),
            #[cfg(feature = "mono")]
            Inner::Mono(mono) => mono.function_pointer(assembly, type_name, method),
        }
    }

    /// A native-callable pointer to a static method, whether or not it is
    /// `[UnmanagedCallersOnly]`. The method is JIT-compiled first, so IL patched
    /// before the assembly was loaded takes effect.
    ///
    /// The pointer uses the managed calling convention, which matches a plain
    /// `extern "C" fn(args) -> ret` for static methods with primitive and
    /// pointer arguments. Unlike [`function_pointer`](Self::function_pointer),
    /// it takes no trailing exception argument on either backend, so a managed
    /// exception crosses it as an unhandled error.
    pub fn method_pointer(
        &self,
        assembly: &str,
        type_name: &str,
        method: &str,
    ) -> Result<*const c_void> {
        match &*self.0 {
            #[cfg(feature = "coreclr")]
            Inner::CoreClr(clr) => clr.method_pointer(assembly, type_name, method),
            #[cfg(feature = "mono")]
            Inner::Mono(mono) => mono.method_pointer(assembly, type_name, method),
        }
    }
}
