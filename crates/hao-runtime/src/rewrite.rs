//! Rewrite a managed assembly's IL with hao before the runtime JITs it.
//!
//! The Godot/CoreCLR modding path: a native mod intercepts the game assembly's
//! load (see [`intercept_assembly`]) and hands CoreCLR a hao-rewritten copy. The
//! rewrite ([`patch_call`]) adds a `[DllImport]`-style P/Invoke to a function the
//! mod exports and calls it from a lifecycle method — so the method the engine
//! already runs each frame on the main thread now also calls back into Rust. No
//! injected assembly, no C#; pure LD_PRELOAD + hao.

use hao::{
    dotnet::{
        entries::{Entry, ImplMap, Method, ModuleRef, TypeDef},
        il::Instruction,
        md::streams::tables_stream::{MethodFlags, PInvokeMapFlags},
        signature::{MethodSig, TypeSig},
    },
    Module,
};

use crate::{Error, Result};

/// Rewrites `assembly` so `type_name::method_name` begins by calling the native
/// function `symbol`, exported from `library` (resolved as a `[DllImport]`).
/// Returns the patched assembly bytes.
///
/// The native function is bound as `static extern void symbol()`, so it takes no
/// arguments; it runs wherever the engine calls the patched method.
pub fn patch_call(
    assembly: &[u8],
    type_name: &str,
    method_name: &str,
    library: &str,
    symbol: &str,
) -> Result<Vec<u8>> {
    let mut module =
        Module::from_bytes(assembly).map_err(|e| Error::Other(format!("reading assembly: {e}")))?;

    let target = module
        .find_method(type_name, method_name)
        .ok_or_else(|| Error::NotFound(format!("{type_name}::{method_name}")))?;
    let declaring = target
        .get()
        .declaring_type
        .upgrade()
        .ok_or_else(|| Error::Other(format!("{type_name} has no declaring type")))?;

    // Add `static extern void symbol()`, forwarded to `library` via P/Invoke.
    let module_ref = Entry::new(ModuleRef {
        name: library.to_string(),
        ..Default::default()
    });
    module.module_refs.push(module_ref.clone());
    let mut pinvoke = Method::public_static(symbol, MethodSig::new_static(TypeSig::VOID, vec![]));
    pinvoke.flags |= MethodFlags::PinvokeImpl;
    pinvoke.impl_map = Some(ImplMap {
        flags: PInvokeMapFlags::CallConvCdecl | PInvokeMapFlags::NoMangle,
        import_name: symbol.to_string(),
        scope: module_ref,
    });
    let pinvoke = TypeDef::add_method(&declaring, pinvoke);

    // Call it first thing in the target method.
    {
        let mut method = target.get_mut();
        let body = method
            .body
            .as_mut()
            .ok_or_else(|| Error::Other(format!("{type_name}::{method_name} has no body")))?;
        body.insert(0, [Instruction::call(pinvoke)]);
    }

    module
        .write()
        .map_err(|e| Error::Other(format!("writing assembly: {e}")))
}

#[cfg(all(unix, target_arch = "x86_64"))]
pub use intercept::{arm as arm_interception, intercept_assembly};

/// Hooks the file loader so that when the runtime opens a target assembly, it
/// gets a rewritten copy instead. The only platform-specific part of the
/// assembly-rewrite path.
#[cfg(all(unix, target_arch = "x86_64"))]
mod intercept {
    use std::{
        cell::Cell,
        ffi::{c_char, c_int, c_void, CStr, CString},
        sync::{
            atomic::{AtomicBool, Ordering},
            Mutex,
        },
    };

    const RTLD_DEFAULT: *mut c_void = std::ptr::null_mut();

    unsafe extern "C" {
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
        fn memfd_create(name: *const c_char, flags: c_int) -> c_int;
    }

    use crate::Result;

    type PatchFn = Box<dyn Fn(&[u8]) -> Result<Vec<u8>> + Send + Sync>;

    struct Target {
        /// The file name (not path) to match, e.g. `sts2.dll`.
        basename: String,
        patch: PatchFn,
        /// A `/proc/self/fd/{fd}` path to the rewritten copy, built on first open.
        patched: Mutex<Option<CString>>,
    }

    static TARGETS: Mutex<Vec<Target>> = Mutex::new(Vec::new());
    static HOOKED: AtomicBool = AtomicBool::new(false);

    thread_local! {
        // Set while we read/write the rewritten copy, so our own file I/O isn't
        // itself intercepted.
        static BUILDING: Cell<bool> = const { Cell::new(false) };
    }

    /// Serves a rewritten copy of `basename` (matched by file name) whenever the
    /// runtime opens it. `patch` receives the original bytes and returns the
    /// replacement (see [`patch_call`](super::patch_call)). Call it from a mod's
    /// entry point, before the runtime loads the assembly.
    ///
    /// The file-open hooks are NOT installed immediately — they are armed by
    /// [`arm`] (called automatically from `coreclr_init_detour` after
    /// `coreclr_initialize` returns). This avoids the open/openat hooks
    /// interfering with `dlopen` during early game startup.
    pub fn intercept_assembly(
        basename: &str,
        patch: impl Fn(&[u8]) -> Result<Vec<u8>> + Send + Sync + 'static,
    ) {
        TARGETS.lock().unwrap_or_else(|e| e.into_inner()).push(Target {
            basename: basename.to_string(),
            patch: Box::new(patch),
            patched: Mutex::new(None),
        });
    }

    /// Installs the open/openat hooks so registered targets are intercepted.
    /// Called automatically after `coreclr_initialize` returns — the CLR is
    /// loaded but managed assemblies haven't been opened yet.
    pub fn arm() {
        if TARGETS.lock().unwrap_or_else(|e| e.into_inner()).is_empty() {
            return;
        }
        install();
    }

    fn install() {
        if HOOKED.swap(true, Ordering::SeqCst) {
            return;
        }
        // Hook libc's file-open entry points. On many systems open==open64 and
        // openat==openat64 (64-bit offsets are the default), so deduplicate by
        // resolved address to avoid double-patching the same function.
        let mut hooked_addrs = Vec::new();
        hook_dedup(c"open", open_detour as *mut u8, &mut hooked_addrs);
        hook_dedup(c"open64", open_detour as *mut u8, &mut hooked_addrs);
        hook_dedup(c"openat", openat_detour as *mut u8, &mut hooked_addrs);
        hook_dedup(c"openat64", openat_detour as *mut u8, &mut hooked_addrs);
    }

    fn hook_dedup(name: &CStr, detour: *mut u8, seen: &mut Vec<usize>) {
        let addr = unsafe { dlsym(RTLD_DEFAULT, name.as_ptr()) } as usize;
        if addr == 0 {
            tracing::debug!("not hooking {name:?}: symbol not found");
            return;
        }
        if seen.contains(&addr) {
            tracing::debug!("{name:?} shares address with an already-hooked function, skipping");
            return;
        }
        seen.push(addr);
        match unsafe { hooking::Hook::by_name(None, name, detour) } {
            Ok(mut hook) => {
                if let Err(e) = unsafe { hook.apply_hook() } {
                    tracing::warn!("applying the {name:?} hook failed: {e:?}");
                }
                std::mem::forget(hook);
            }
            Err(e) => tracing::debug!("not hooking {name:?}: {e:?}"),
        }
    }

    /// If `path` names a target assembly, the path of its rewritten copy.
    fn redirect(path: *const c_char) -> Option<CString> {
        if path.is_null() || BUILDING.with(Cell::get) {
            return None;
        }
        let path = unsafe { CStr::from_ptr(path) }.to_string_lossy().into_owned();
        let name = path.rsplit('/').next().unwrap_or(&path);
        // Lock scope: find target and get/build cached result, then drop lock
        // before any I/O (diag opens files → open_detour → redirect → deadlock
        // if the lock is still held).
        let result = {
            let targets = TARGETS.lock().unwrap_or_else(|e| e.into_inner());
            let target = targets.iter().find(|t| t.basename == name)?;
            let mut cache = target.patched.lock().unwrap_or_else(|e| e.into_inner());
            if cache.is_none() {
                *cache = build(&path, &target.patch);
            }
            cache.clone()
        };
        result
    }

    /// Reads the original assembly, rewrites it, and serves the copy from an
    /// anonymous in-memory file (`memfd_create`), returning a
    /// `/proc/self/fd/{fd}` path that the CLR can open.
    fn build(path: &str, patch: &PatchFn) -> Option<CString> {
        BUILDING.with(|b| b.set(true));
        // diag here is safe: BUILDING is true so the open() for the diag
        // file won't re-enter redirect().
        crate::attach::diag(&format!("rewrite::build intercepted: {path}"));
        let result = (|| {
            let original = std::fs::read(path).ok()?;
            BUILDING.with(|b| b.set(false));
            let patched = match patch(&original) {
                Ok(bytes) => bytes,
                Err(e) => {
                    BUILDING.with(|b| b.set(true));
                    crate::attach::diag(&format!("rewrite: patch FAILED for {path}: {e}"));
                    tracing::error!("rewriting {path} failed: {e}");
                    return None;
                }
            };
            BUILDING.with(|b| b.set(true));
            crate::attach::diag(&format!(
                "rewrite: patch OK, {} -> {} bytes, creating memfd",
                original.len(),
                patched.len()
            ));
            BUILDING.with(|b| b.set(false));
            let fd = unsafe { memfd_create(c"hao-rewrite".as_ptr(), 0) };
            if fd < 0 {
                tracing::error!("memfd_create failed for {path}");
                return None;
            }
            let written = unsafe {
                libc_write(fd, patched.as_ptr() as *const c_void, patched.len())
            };
            if written < 0 || written as usize != patched.len() {
                unsafe { libc_close(fd) };
                tracing::error!("write to memfd failed for {path}");
                return None;
            }
            BUILDING.with(|b| b.set(true));
            crate::attach::diag(&format!(
                "rewrite: serving {path} from /proc/self/fd/{fd}"
            ));
            tracing::info!("serving a rewritten {path} from memfd (fd {fd})");
            CString::new(format!("/proc/self/fd/{fd}")).ok()
        })();
        BUILDING.with(|b| b.set(false));
        result
    }

    unsafe fn libc_write(fd: c_int, buf: *const c_void, count: usize) -> isize {
        unsafe extern "C" {
            fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;
        }
        unsafe { write(fd, buf, count) }
    }

    unsafe fn libc_close(fd: c_int) -> c_int {
        unsafe extern "C" {
            fn close(fd: c_int) -> c_int;
        }
        unsafe { close(fd) }
    }

    extern "C" fn open_detour(path: *const c_char, flags: c_int, mode: c_int) -> c_int {
        let original = hooking::original_function_ptr().expect("original! inside the open hook");
        let original: unsafe extern "C" fn(*const c_char, c_int, c_int) -> c_int =
            unsafe { std::mem::transmute(original.as_ptr()) };
        match redirect(path) {
            Some(temp) => unsafe { original(temp.as_ptr(), flags, mode) },
            None => unsafe { original(path, flags, mode) },
        }
    }

    extern "C" fn openat_detour(
        dirfd: c_int,
        path: *const c_char,
        flags: c_int,
        mode: c_int,
    ) -> c_int {
        let original = hooking::original_function_ptr().expect("original! inside the openat hook");
        let original: unsafe extern "C" fn(c_int, *const c_char, c_int, c_int) -> c_int =
            unsafe { std::mem::transmute(original.as_ptr()) };
        match redirect(path) {
            Some(temp) => unsafe { original(dirfd, temp.as_ptr(), flags, mode) },
            None => unsafe { original(dirfd, path, flags, mode) },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hao::dotnet::{
        entries::Version,
        il::{MethodBody, OpCode},
        md::streams::tables_stream::TypeAttributes,
        module::{ModuleKind, TargetRuntime},
    };

    /// A tiny assembly with `Game.Loop::Tick()` whose body is just `ret`.
    fn sample() -> Vec<u8> {
        let mut module = Module::new("Patchee", ModuleKind::Dll, TargetRuntime::NetCore(9));
        module.add_assembly_ref(
            "System.Runtime",
            Version {
                major: 9,
                ..Default::default()
            },
            &TargetRuntime::NetCore(9).framework_token(),
        );
        let object = module.corlib_type("System.Object").unwrap();
        let ty = module.add_type(TypeDef::new(
            TypeAttributes::Public | TypeAttributes::BeforeFieldInit,
            "Game",
            "Loop",
            Some(object),
        ));
        let tick = TypeDef::add_method(
            &ty,
            Method::public_static("Tick", MethodSig::new_static(TypeSig::VOID, vec![])),
        );
        tick.get_mut().body = Some(MethodBody {
            instructions: vec![Instruction::ret()],
            ..Default::default()
        });
        module.write().unwrap()
    }

    #[test]
    fn splices_a_pinvoke_call() {
        let patched = patch_call(&sample(), "Game.Loop", "Tick", "libhook.so", "hao_tick").unwrap();
        let reloaded = Module::from_bytes(&patched).unwrap();

        // The P/Invoke method was added, forwarded to our library.
        let pinvoke = reloaded
            .find_method("Game.Loop", "hao_tick")
            .expect("pinvoke method");
        let pinvoke = pinvoke.get();
        let impl_map = pinvoke.impl_map.as_ref().expect("impl map");
        assert_eq!(impl_map.import_name, "hao_tick");
        assert_eq!(impl_map.scope.get().name, "libhook.so");

        // Tick now calls it before anything else.
        let tick = reloaded.find_method("Game.Loop", "Tick").unwrap();
        let tick = tick.get();
        let body = tick.body.as_ref().unwrap();
        assert_eq!(body.instructions[0].opcode, OpCode::Call);
        assert_eq!(body.instructions.last().unwrap().opcode, OpCode::Ret);
    }

    #[test]
    fn roundtrip_size() {
        let path = "/games/SteamLibrary/steamapps/common/Slay the Spire 2/data_sts2_linuxbsd_x86_64/GodotSharp.dll";
        let Ok(original) = std::fs::read(path) else {
            eprintln!("skipping: {path} not found");
            return;
        };
        let module = Module::from_bytes(&original).unwrap();
        let roundtripped = module.write().unwrap();
        eprintln!(
            "original: {} bytes, roundtripped: {} bytes, ratio: {:.2}x",
            original.len(),
            roundtripped.len(),
            roundtripped.len() as f64 / original.len() as f64
        );
        assert!(
            roundtripped.len() < original.len() * 3 / 2,
            "roundtripped size ({}) is more than 1.5x the original ({})",
            roundtripped.len(),
            original.len()
        );
    }

    #[test]
    fn patches_real_godotsharp() {
        let path = "/games/SteamLibrary/steamapps/common/Slay the Spire 2/data_sts2_linuxbsd_x86_64/GodotSharp.dll";
        let Ok(original) = std::fs::read(path) else {
            eprintln!("skipping: {path} not found");
            return;
        };
        let patched = patch_call(
            &original,
            "Godot.SceneTree",
            "InvokeGodotClassMethod",
            "/home/samm/src/projects/xur/target/release/libxur_stub.so",
            "hao_godot_tick",
        )
        .expect("patch_call failed");

        let reloaded = Module::from_bytes(&patched).expect("reload failed");
        let pinvoke = reloaded
            .find_method("Godot.SceneTree", "hao_godot_tick")
            .expect("hao_godot_tick not found");
        let p = pinvoke.get();
        eprintln!("flags: {:?}", p.flags);
        let impl_map = p.impl_map.as_ref().expect("NO IMPL_MAP on hao_godot_tick");
        eprintln!(
            "impl_map: import={} scope={}",
            impl_map.import_name,
            impl_map.scope.get().name
        );
        assert_eq!(impl_map.import_name, "hao_godot_tick");
        assert!(
            impl_map.scope.get().name.contains("libxur_stub"),
            "scope should reference libxur_stub.so, got: {}",
            impl_map.scope.get().name
        );

        eprintln!("module_refs: {}", reloaded.module_refs.len());
        for mr in &reloaded.module_refs {
            eprintln!("  {}", mr.get().name);
        }
    }
}
