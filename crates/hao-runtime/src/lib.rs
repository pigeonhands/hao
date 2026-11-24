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
#[cfg(all(feature = "coreclr", target_arch = "x86_64"))]
pub mod attach;
pub mod console;
pub mod jit;
#[cfg(feature = "coreclr")]
mod coreclr;
mod error;
#[cfg(feature = "hao")]
mod loader;
#[cfg(feature = "coreclr")]
pub mod profiler;
#[cfg(feature = "hao")]
pub mod rewrite;
#[cfg(feature = "reflect")]
pub mod reflect;
#[cfg(feature = "mono")]
mod mono;

use std::{ffi::c_void, path::Path, sync::Arc};

pub use bindings::{BoundAssembly, Object};
pub use error::{Error, Result};

/// The `hao` assembly-editing crate, re-exported so downstream crates can
/// build modules to load with [`Runtime::load_module`] without pinning
/// their own (possibly mismatched) `hao` version.
#[cfg(feature = "hao")]
pub use hao;

#[cfg(feature = "coreclr")]
pub use coreclr::{find_dotnet_root, CoreClrOptions};
#[cfg(feature = "mono")]
pub use mono::MonoOptions;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "reflect", derive(facet::Facet))]
#[cfg_attr(feature = "reflect", repr(C))]
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

    /// Like [`method_pointer`](Self::method_pointer), but resolves the type
    /// from whichever `AssemblyLoadContext` actually loaded the assembly.
    ///
    /// Mono has no `AssemblyLoadContext`, so there it is identical to
    /// [`method_pointer`](Self::method_pointer).
    pub fn method_pointer_from_loaded(
        &self,
        assembly: &str,
        type_name: &str,
        method: &str,
    ) -> Result<*const c_void> {
        match &*self.0 {
            #[cfg(feature = "coreclr")]
            Inner::CoreClr(clr) => clr.method_pointer_from_loaded(assembly, type_name, method),
            #[cfg(feature = "mono")]
            Inner::Mono(mono) => mono.method_pointer(assembly, type_name, method),
        }
    }

    /// Resolves a well-known corlib method to a native-callable pointer in
    /// the loaded `System.Private.CoreLib`, deriving the type name, method
    /// name and overload arity from the
    /// [`WellKnownMethod`](hao::dotnet::entries::well_known::WellKnownMethod)
    /// vocabulary instead of raw strings. CoreCLR only.
    #[cfg(feature = "hao")]
    pub fn corlib_method_pointer(
        &self,
        method: hao::dotnet::entries::well_known::WellKnownMethod,
    ) -> Result<*const c_void> {
        let (namespace, name) = method.declaring().full_name();
        let full_name = if namespace.is_empty() {
            name.to_string()
        } else {
            format!("{namespace}.{name}")
        };
        self.find_method_from_loaded(
            "System.Private.CoreLib",
            &full_name,
            method.method_name(),
            method.param_count() as i32,
        )
    }

    /// Like [`method_pointer_from_loaded`](Self::method_pointer_from_loaded)
    /// but matches by parameter count, so it works with overloaded methods.
    pub fn find_method_from_loaded(
        &self,
        assembly: &str,
        type_name: &str,
        method: &str,
        param_count: i32,
    ) -> Result<*const c_void> {
        match &*self.0 {
            #[cfg(feature = "coreclr")]
            Inner::CoreClr(clr) => {
                clr.find_method_from_loaded(assembly, type_name, method, param_count)
            }
            #[cfg(feature = "mono")]
            Inner::Mono(mono) => mono.find_method(assembly, type_name, method, param_count),
        }
    }

    /// Calls a managed method pointer through a P/Invoke wrapper that
    /// ensures the calling thread is registered with the CLR's GC.
    /// Falls back to `jit::call_native` on Mono (which handles thread
    /// attach separately).
    #[cfg(target_arch = "x86_64")]
    pub fn invoke(&self, ptr: usize, args: &[u64]) -> Result<u64> {
        match &*self.0 {
            #[cfg(feature = "coreclr")]
            Inner::CoreClr(clr) => clr.invoke(ptr, args),
            #[cfg(feature = "mono")]
            Inner::Mono(_) => Ok(unsafe { jit::call_native(ptr, args) }),
        }
    }

    /// Like [`invoke`](Self::invoke) but returns `f64`.
    #[cfg(target_arch = "x86_64")]
    pub fn invoke_float(&self, ptr: usize, args: &[u64]) -> Result<f64> {
        match &*self.0 {
            #[cfg(feature = "coreclr")]
            Inner::CoreClr(clr) => clr.invoke_float(ptr, args),
            #[cfg(feature = "mono")]
            Inner::Mono(_) => Ok(unsafe { jit::call_native_float(ptr, args) }),
        }
    }

    /// Requests a no-GC region for the next `budget` bytes of managed
    /// allocation.  Returns `true` if the runtime agreed to suppress GC.
    pub fn suppress_gc(&self, budget: i64) -> Result<bool> {
        match &*self.0 {
            #[cfg(feature = "coreclr")]
            Inner::CoreClr(clr) => clr.suppress_gc(budget),
            #[cfg(feature = "mono")]
            Inner::Mono(_) => Ok(false),
        }
    }

    /// Ends a no-GC region started by [`suppress_gc`](Self::suppress_gc).
    pub fn resume_gc(&self) -> Result<()> {
        match &*self.0 {
            #[cfg(feature = "coreclr")]
            Inner::CoreClr(clr) => clr.resume_gc(),
            #[cfg(feature = "mono")]
            Inner::Mono(_) => Ok(()),
        }
    }

    /// JIT-compiles and caches the `Hao.Loader::Enumerate` helper so
    /// [`enumerate`](Self::enumerate) can be called from a cooperative-mode
    /// thread. Must be called off such a thread (e.g. at init). CoreCLR only.
    pub fn warm_enumerate(&self) -> Result<()> {
        match &*self.0 {
            #[cfg(feature = "coreclr")]
            Inner::CoreClr(clr) => clr.warm_enumerate(),
            #[cfg(feature = "mono")]
            Inner::Mono(_) => Err(Error::Other("enumerate is CoreCLR-only".into())),
        }
    }

    /// Materializes any managed `IEnumerable` into the addresses of its
    /// elements, by calling a JIT-compiled `foreach` helper and reading the
    /// resulting `object[]`. This handles concrete types native code cannot
    /// read by hand — lazy iterators, LINQ chains, dictionary views.
    ///
    /// Call [`warm_enumerate`](Self::warm_enumerate) once off a cooperative-mode
    /// thread first; this call must run ON the cooperative (game) thread, like
    /// any [`method_pointer`](Self::method_pointer) call. CoreCLR only.
    pub fn enumerate(&self, obj: usize) -> Result<Vec<usize>> {
        match &*self.0 {
            #[cfg(feature = "coreclr")]
            Inner::CoreClr(clr) => clr.enumerate(obj),
            #[cfg(feature = "mono")]
            Inner::Mono(_) => Err(Error::Other("enumerate is CoreCLR-only".into())),
        }
    }

    /// The .NET major version (e.g. 9) the runtime was started or attached
    /// with. `None` on Mono, where no CoreCLR framework version applies.
    pub fn dotnet_major(&self) -> Option<u16> {
        match &*self.0 {
            #[cfg(feature = "coreclr")]
            Inner::CoreClr(clr) => Some(clr.major()),
            #[cfg(feature = "mono")]
            Inner::Mono(_) => None,
        }
    }

    /// Returns all type full names from a loaded assembly.
    pub fn type_names(&self, assembly: &str) -> Result<Vec<String>> {
        match &*self.0 {
            #[cfg(feature = "coreclr")]
            Inner::CoreClr(clr) => clr.type_names(assembly),
            #[cfg(feature = "mono")]
            Inner::Mono(_) => Err(Error::Other("type_names is CoreCLR-only".into())),
        }
    }

    /// Returns all method names from a type in a loaded assembly.
    pub fn method_names(&self, assembly: &str, type_name: &str) -> Result<Vec<String>> {
        match &*self.0 {
            #[cfg(feature = "coreclr")]
            Inner::CoreClr(clr) => clr.method_names(assembly, type_name),
            #[cfg(feature = "mono")]
            Inner::Mono(_) => Err(Error::Other("method_names is CoreCLR-only".into())),
        }
    }
}
