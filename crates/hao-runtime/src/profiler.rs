//! A minimal CoreCLR profiler, implemented as COM objects by hand.
//!
//! Registered as a startup profiler (the host sets `CORECLR_ENABLE_PROFILING=1`,
//! `CORECLR_PROFILER={`[`CLSID_STRING`]`}` and `CORECLR_PROFILER_PATH` to the
//! `.so` that re-exports [`dll_get_class_object`] as `DllGetClassObject`), the
//! runtime loads it and calls [`ICorProfilerCallback::Initialize`], where we grab
//! `ICorProfilerInfo` and ask to monitor JIT compilation and to enable ReJIT.
//!
//! This is the registration layer. ReJIT (replacing a method's IL with hao's)
//! and native code-address lookup (for patching machine code with `iced`) build
//! on the `ICorProfilerInfo` captured here.
//!
//! COM on Linux uses the C calling convention; every callback returns an
//! `HRESULT`, so unused vtable slots share one stub that returns `S_OK`.

#![allow(clippy::missing_safety_doc)]

use std::{
    collections::HashMap,
    ffi::c_void,
    sync::{
        atomic::{AtomicPtr, Ordering},
        Mutex,
    },
};

type Hresult = i32;
const S_OK: Hresult = 0;
const E_NOINTERFACE: Hresult = 0x8000_4002u32 as Hresult;
const E_POINTER: Hresult = 0x8000_4003u32 as Hresult;
const CLASS_E_CLASSNOTAVAILABLE: Hresult = 0x8004_0111u32 as Hresult;

// COR_PRF_MONITOR flags (corprof.idl).
const COR_PRF_MONITOR_MODULE_LOADS: u32 = 0x0000_0004;
const COR_PRF_MONITOR_JIT_COMPILATION: u32 = 0x0000_0020;
const COR_PRF_ENABLE_REJIT: u32 = 0x0004_0000;

// ICorProfilerInfo vtable slots (3 IUnknown methods precede ICorProfilerInfo;
// Info2 follows all 33 Info1 methods). Resolved from corprof.idl.
const SLOT_GET_FUNCTION_FROM_TOKEN: usize = 8;
const SLOT_GET_FUNCTION_INFO: usize = 15;
const SLOT_SET_EVENT_MASK: usize = 16;
const SLOT_GET_MODULE_INFO: usize = 20;
const SLOT_GET_CODE_INFO2: usize = 42;

/// `COR_PRF_CODE_INFO`: one contiguous block of a method's native code.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CodeInfo {
    start: usize,
    size: usize,
}

/// Loaded modules by id, for naming JIT events and resolving `native_code`.
static MODULES: Mutex<Option<HashMap<usize, String>>> = Mutex::new(None);
/// A substring: JIT events are logged only for modules whose name contains it.
static WATCH: Mutex<Option<String>> = Mutex::new(None);

/// Log JITed methods of modules whose name contains `substring` (e.g. the game
/// assembly). Without this, module loads are still logged but JITs are not.
pub fn watch_module(substring: &str) {
    *WATCH.lock().unwrap_or_else(|e| e.into_inner()) = Some(substring.to_string());
}

/// A COM interface id / class id.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

const fn guid(data1: u32, data2: u16, data3: u16, data4: [u8; 8]) -> Guid {
    Guid { data1, data2, data3, data4 }
}

// The CLSID a host passes as CORECLR_PROFILER. Ours; not a well-known value.
const CLSID_HAO_PROFILER: Guid = guid(0x7F9A_1B2C, 0x3D4E, 0x4F50, [0x9A, 0x1B, 0x2C, 0x3D, 0x4E, 0x5F, 0x60, 0x71]);
/// The string form of our profiler CLSID, for `CORECLR_PROFILER`.
pub const CLSID_STRING: &str = "{7F9A1B2C-3D4E-4F50-9A1B-2C3D4E5F6071}";

const IID_IUNKNOWN: Guid = guid(0, 0, 0, [0xC0, 0, 0, 0, 0, 0, 0, 0x46]);
const IID_ICLASSFACTORY: Guid = guid(1, 0, 0, [0xC0, 0, 0, 0, 0, 0, 0, 0x46]);
const IID_ICORPROFILERCALLBACK: Guid = guid(0x176F_BED1, 0xA55C, 0x4796, [0x98, 0xCA, 0xA9, 0xDA, 0x0E, 0xF8, 0x83, 0xE7]);
const IID_ICORPROFILERCALLBACK2: Guid = guid(0x8A8C_C829, 0xCCF2, 0x49FE, [0xBB, 0xAE, 0x0F, 0x02, 0x22, 0x28, 0x07, 0x1A]);
const IID_ICORPROFILERCALLBACK3: Guid = guid(0x4FD2_ED52, 0x7731, 0x4B8D, [0x94, 0x69, 0x03, 0xD2, 0xCC, 0x30, 0x86, 0xC5]);
const IID_ICORPROFILERCALLBACK4: Guid = guid(0x7B63_B2E3, 0x107D, 0x4D48, [0xB2, 0xF6, 0xF6, 0x1E, 0x22, 0x94, 0x70, 0xD2]);
const IID_ICORPROFILERINFO: Guid = guid(0x28B5_557D, 0x3F3F, 0x48B4, [0x90, 0xB2, 0x5F, 0x9E, 0xEA, 0x2F, 0x6C, 0x48]);

// --- logging -------------------------------------------------------------

static LOGGER: AtomicPtr<()> = AtomicPtr::new(std::ptr::null_mut());

/// Routes profiler log lines to `f`. Without one, lines go to stderr (which a
/// sandboxed game may swallow, so a host usually points this at a file).
pub fn set_logger(f: fn(&str)) {
    LOGGER.store(f as *mut (), Ordering::SeqCst);
}

fn log(msg: &str) {
    let f = LOGGER.load(Ordering::SeqCst);
    if f.is_null() {
        eprintln!("[hao-profiler] {msg}");
    } else {
        let f: fn(&str) = unsafe { std::mem::transmute(f) };
        f(msg);
    }
}

/// The captured `ICorProfilerInfo` (really the highest version the runtime gave
/// us via the base IID). ReJIT and code-info layers read this.
static INFO: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

// --- ICorProfilerCallback ------------------------------------------------

/// The callback object: a pointer to its vtable, as COM requires, and nothing
/// else — it is a process-lifetime singleton, so it needs no ref counting.
#[repr(C)]
struct Callback {
    vtable: *const CallbackVtbl,
}

// The vtables are immutable, and the objects are process-lifetime singletons
// reached only through raw pointers, so sharing them across threads is sound.
unsafe impl Sync for CallbackVtbl {}
unsafe impl Sync for FactoryVtbl {}

// ICorProfilerCallback(1..4) has 95 methods after IUnknown's 3, so the vtable
// is 98 slots. We implement Initialize (slot 3), ModuleLoadFinished (14) and
// JITCompilationFinished (24); the gaps between them are filled with one S_OK
// stub. Slot positions are from the method order in corprof.idl.
#[repr(C)]
struct CallbackVtbl {
    query_interface: unsafe extern "C" fn(*mut Callback, *const Guid, *mut *mut c_void) -> Hresult,
    add_ref: unsafe extern "C" fn(*mut Callback) -> u32,
    release: unsafe extern "C" fn(*mut Callback) -> u32,
    initialize: unsafe extern "C" fn(*mut Callback, *mut c_void) -> Hresult,
    gap_a: [*const c_void; 10], // slots 4..=13
    module_load_finished: unsafe extern "C" fn(*mut Callback, usize, Hresult) -> Hresult, // 14
    gap_b: [*const c_void; 9], // slots 15..=23
    jit_compilation_finished: unsafe extern "C" fn(*mut Callback, usize, Hresult, i32) -> Hresult, // 24
    gap_c: [*const c_void; 73], // slots 25..=97
}

unsafe extern "C" fn cb_query_interface(
    this: *mut Callback,
    riid: *const Guid,
    ppv: *mut *mut c_void,
) -> Hresult {
    if riid.is_null() || ppv.is_null() {
        return E_POINTER;
    }
    let riid = unsafe { *riid };
    if riid == IID_IUNKNOWN
        || riid == IID_ICORPROFILERCALLBACK
        || riid == IID_ICORPROFILERCALLBACK2
        || riid == IID_ICORPROFILERCALLBACK3
        || riid == IID_ICORPROFILERCALLBACK4
    {
        unsafe { *ppv = this as *mut c_void };
        S_OK
    } else {
        unsafe { *ppv = std::ptr::null_mut() };
        E_NOINTERFACE
    }
}

unsafe extern "C" fn cb_add_ref(_this: *mut Callback) -> u32 {
    1
}

unsafe extern "C" fn cb_release(_this: *mut Callback) -> u32 {
    1
}

/// A vtable slot we don't implement. Every `ICorProfilerCallback` method returns
/// `HRESULT`, and on the C calling convention the callee ignores extra argument
/// registers, so one `S_OK` stub serves them all.
unsafe extern "C" fn cb_stub(_this: *mut Callback) -> Hresult {
    S_OK
}

unsafe extern "C" fn cb_initialize(_this: *mut Callback, unk: *mut c_void) -> Hresult {
    if unk.is_null() {
        return E_POINTER;
    }
    // QueryInterface the given IUnknown for ICorProfilerInfo.
    let info = unsafe { query_interface(unk, &IID_ICORPROFILERINFO) };
    let Some(info) = info else {
        log("Initialize: ICorProfilerInfo not available");
        return E_NOINTERFACE;
    };
    INFO.store(info, Ordering::SeqCst);
    *MODULES.lock().unwrap_or_else(|e| e.into_inner()) = Some(HashMap::new());

    // Monitor module loads and JIT, and enable ReJIT.
    let mask =
        COR_PRF_MONITOR_MODULE_LOADS | COR_PRF_MONITOR_JIT_COMPILATION | COR_PRF_ENABLE_REJIT;
    let hr = unsafe { call_slot_u32(info, SLOT_SET_EVENT_MASK, mask) };
    log(&format!(
        "profiler initialized: ICorProfilerInfo={info:?}, SetEventMask(0x{mask:x})=0x{hr:x}"
    ));
    S_OK
}

unsafe extern "C" fn cb_module_load_finished(
    _this: *mut Callback,
    module_id: usize,
    _hr_status: Hresult,
) -> Hresult {
    let info = INFO.load(Ordering::SeqCst);
    if let Some(name) = unsafe { get_module_name(info, module_id) } {
        log(&format!("module loaded: {name} (id={module_id:#x})"));
        if let Some(map) = MODULES.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            map.insert(module_id, name);
        }
    }
    S_OK
}

unsafe extern "C" fn cb_jit_compilation_finished(
    _this: *mut Callback,
    function_id: usize,
    _hr_status: Hresult,
    _is_safe_to_block: i32,
) -> Hresult {
    let watch = WATCH.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let Some(watch) = watch else { return S_OK };
    let info = INFO.load(Ordering::SeqCst);
    let Some((module_id, token)) = (unsafe { get_function_info(info, function_id) }) else {
        return S_OK;
    };
    let name = MODULES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .and_then(|m| m.get(&module_id).cloned());
    let Some(name) = name else { return S_OK };
    if !name.contains(&watch) {
        return S_OK;
    }
    match unsafe { get_code_info(info, function_id) } {
        Some(code) => log(&format!(
            "jit {name} token={token:#010x} -> code {:#x} ({} bytes)",
            code.start, code.size
        )),
        None => log(&format!("jit {name} token={token:#010x} (no code info)")),
    }
    S_OK
}

static CALLBACK_VTBL: CallbackVtbl = CallbackVtbl {
    query_interface: cb_query_interface,
    add_ref: cb_add_ref,
    release: cb_release,
    initialize: cb_initialize,
    gap_a: [cb_stub as *const c_void; 10],
    module_load_finished: cb_module_load_finished,
    gap_b: [cb_stub as *const c_void; 9],
    jit_compilation_finished: cb_jit_compilation_finished,
    gap_c: [cb_stub as *const c_void; 73],
};

static mut CALLBACK: Callback = Callback {
    vtable: &CALLBACK_VTBL,
};

// --- IClassFactory -------------------------------------------------------

#[repr(C)]
struct Factory {
    vtable: *const FactoryVtbl,
}

#[repr(C)]
struct FactoryVtbl {
    query_interface: unsafe extern "C" fn(*mut Factory, *const Guid, *mut *mut c_void) -> Hresult,
    add_ref: unsafe extern "C" fn(*mut Factory) -> u32,
    release: unsafe extern "C" fn(*mut Factory) -> u32,
    create_instance:
        unsafe extern "C" fn(*mut Factory, *mut c_void, *const Guid, *mut *mut c_void) -> Hresult,
    lock_server: unsafe extern "C" fn(*mut Factory, i32) -> Hresult,
}

unsafe extern "C" fn factory_query_interface(
    this: *mut Factory,
    riid: *const Guid,
    ppv: *mut *mut c_void,
) -> Hresult {
    if riid.is_null() || ppv.is_null() {
        return E_POINTER;
    }
    let riid = unsafe { *riid };
    if riid == IID_IUNKNOWN || riid == IID_ICLASSFACTORY {
        unsafe { *ppv = this as *mut c_void };
        S_OK
    } else {
        unsafe { *ppv = std::ptr::null_mut() };
        E_NOINTERFACE
    }
}

unsafe extern "C" fn factory_add_ref(_this: *mut Factory) -> u32 {
    1
}

unsafe extern "C" fn factory_release(_this: *mut Factory) -> u32 {
    1
}

unsafe extern "C" fn factory_create_instance(
    _this: *mut Factory,
    outer: *mut c_void,
    _riid: *const Guid,
    ppv: *mut *mut c_void,
) -> Hresult {
    if ppv.is_null() {
        return E_POINTER;
    }
    if !outer.is_null() {
        // No aggregation.
        return 0x8004_0110u32 as Hresult; // CLASS_E_NOAGGREGATION
    }
    unsafe { *ppv = std::ptr::addr_of_mut!(CALLBACK) as *mut c_void };
    S_OK
}

unsafe extern "C" fn factory_lock_server(_this: *mut Factory, _lock: i32) -> Hresult {
    S_OK
}

static FACTORY_VTBL: FactoryVtbl = FactoryVtbl {
    query_interface: factory_query_interface,
    add_ref: factory_add_ref,
    release: factory_release,
    create_instance: factory_create_instance,
    lock_server: factory_lock_server,
};

static mut FACTORY: Factory = Factory {
    vtable: &FACTORY_VTBL,
};

/// The entry CoreCLR looks up by name in the profiler library. A host re-exports
/// it as `DllGetClassObject`.
pub unsafe extern "C" fn dll_get_class_object(
    rclsid: *const Guid,
    riid: *const Guid,
    ppv: *mut *mut c_void,
) -> Hresult {
    if rclsid.is_null() || riid.is_null() || ppv.is_null() {
        return E_POINTER;
    }
    if unsafe { *rclsid } != CLSID_HAO_PROFILER {
        return CLASS_E_CLASSNOTAVAILABLE;
    }
    let riid = unsafe { *riid };
    if riid == IID_ICLASSFACTORY || riid == IID_IUNKNOWN {
        unsafe { *ppv = std::ptr::addr_of_mut!(FACTORY) as *mut c_void };
        S_OK
    } else {
        E_NOINTERFACE
    }
}

// --- COM call helpers ----------------------------------------------------

/// Calls `IUnknown::QueryInterface(riid)` on a raw interface pointer.
unsafe fn query_interface(unk: *mut c_void, riid: *const Guid) -> Option<*mut c_void> {
    let vtable = unsafe { *(unk as *const *const *const c_void) };
    let qi: unsafe extern "C" fn(*mut c_void, *const Guid, *mut *mut c_void) -> Hresult =
        unsafe { std::mem::transmute(*vtable) };
    let mut out = std::ptr::null_mut();
    if unsafe { qi(unk, riid, &mut out) } == S_OK && !out.is_null() {
        Some(out)
    } else {
        None
    }
}

/// Calls the method at `slot` on `iface`, passing one `u32`, returning its HRESULT.
unsafe fn call_slot_u32(iface: *mut c_void, slot: usize, arg: u32) -> Hresult {
    let vtable = unsafe { *(iface as *const *const *const c_void) };
    let f: unsafe extern "C" fn(*mut c_void, u32) -> Hresult =
        unsafe { std::mem::transmute(*vtable.add(slot)) };
    unsafe { f(iface, arg) }
}

/// The raw function pointer at a vtable `slot` on `iface`.
unsafe fn slot(iface: *mut c_void, slot: usize) -> *const c_void {
    let vtable = unsafe { *(iface as *const *const *const c_void) };
    unsafe { *vtable.add(slot) }
}

/// `ICorProfilerInfo::GetModuleInfo`, returning just the module name.
unsafe fn get_module_name(info: *mut c_void, module_id: usize) -> Option<String> {
    if info.is_null() {
        return None;
    }
    type Fn = unsafe extern "C" fn(
        *mut c_void,
        usize,            // moduleId
        *mut *const u8,   // ppBaseLoadAddress
        u32,              // cchName
        *mut u32,         // pcchName
        *mut u16,         // szName[]
        *mut usize,       // pAssemblyId
    ) -> Hresult;
    let f: Fn = unsafe { std::mem::transmute(slot(info, SLOT_GET_MODULE_INFO)) };
    let mut base = std::ptr::null();
    let mut needed = 0u32;
    let mut assembly = 0usize;
    // First call learns the name length.
    unsafe {
        f(info, module_id, &mut base, 0, &mut needed, std::ptr::null_mut(), &mut assembly);
    }
    if needed == 0 {
        return None;
    }
    let mut buf = vec![0u16; needed as usize];
    let hr = unsafe {
        f(info, module_id, &mut base, needed, &mut needed, buf.as_mut_ptr(), &mut assembly)
    };
    if hr != S_OK {
        return None;
    }
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..end]))
}

/// `ICorProfilerInfo::GetFunctionInfo`, returning `(moduleId, metadata token)`.
unsafe fn get_function_info(info: *mut c_void, function_id: usize) -> Option<(usize, u32)> {
    if info.is_null() {
        return None;
    }
    type Fn = unsafe extern "C" fn(*mut c_void, usize, *mut usize, *mut usize, *mut u32) -> Hresult;
    let f: Fn = unsafe { std::mem::transmute(slot(info, SLOT_GET_FUNCTION_INFO)) };
    let (mut class_id, mut module_id, mut token) = (0usize, 0usize, 0u32);
    if unsafe { f(info, function_id, &mut class_id, &mut module_id, &mut token) } == S_OK {
        Some((module_id, token))
    } else {
        None
    }
}

/// `ICorProfilerInfo::GetFunctionFromToken`, resolving a `FunctionID`.
unsafe fn get_function_from_token(info: *mut c_void, module_id: usize, token: u32) -> Option<usize> {
    if info.is_null() {
        return None;
    }
    type Fn = unsafe extern "C" fn(*mut c_void, usize, u32, *mut usize) -> Hresult;
    let f: Fn = unsafe { std::mem::transmute(slot(info, SLOT_GET_FUNCTION_FROM_TOKEN)) };
    let mut function_id = 0usize;
    if unsafe { f(info, module_id, token, &mut function_id) } == S_OK && function_id != 0 {
        Some(function_id)
    } else {
        None
    }
}

/// `ICorProfilerInfo2::GetCodeInfo2`, returning the first native code block.
unsafe fn get_code_info(info: *mut c_void, function_id: usize) -> Option<CodeInfo> {
    if info.is_null() {
        return None;
    }
    type Fn = unsafe extern "C" fn(*mut c_void, usize, u32, *mut u32, *mut CodeInfo) -> Hresult;
    let f: Fn = unsafe { std::mem::transmute(slot(info, SLOT_GET_CODE_INFO2)) };
    let mut count = 0u32;
    let mut block = CodeInfo::default();
    if unsafe { f(info, function_id, 1, &mut count, &mut block) } == S_OK && count > 0 {
        Some(block)
    } else {
        None
    }
}

/// The native code address and size of a method, for patching it in place (for
/// example with `iced`). `module_substring` matches a loaded module's name, and
/// `token` is the method's metadata token (`mdMethodDef`). Returns `None` until
/// the method has been JIT-compiled.
///
/// Available once the profiler is initialised; the method must belong to an
/// already-loaded module.
pub fn native_code(module_substring: &str, token: u32) -> Option<(usize, usize)> {
    let info = INFO.load(Ordering::SeqCst);
    if info.is_null() {
        return None;
    }
    let module_id = *MODULES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()?
        .iter()
        .find(|(_, name)| name.contains(module_substring))
        .map(|(id, _)| id)?;
    let function_id = unsafe { get_function_from_token(info, module_id, token) }?;
    let code = unsafe { get_code_info(info, function_id) }?;
    Some((code.start, code.size))
}
