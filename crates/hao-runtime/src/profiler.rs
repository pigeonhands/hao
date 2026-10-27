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
    ffi::c_void,
    sync::atomic::{AtomicPtr, Ordering},
};

type Hresult = i32;
const S_OK: Hresult = 0;
const E_NOINTERFACE: Hresult = 0x8000_4002u32 as Hresult;
const E_POINTER: Hresult = 0x8000_4003u32 as Hresult;
const CLASS_E_CLASSNOTAVAILABLE: Hresult = 0x8004_0111u32 as Hresult;

// COR_PRF_MONITOR flags (corprof.idl).
const COR_PRF_MONITOR_JIT_COMPILATION: u32 = 0x0000_0020;
const COR_PRF_ENABLE_REJIT: u32 = 0x0004_0000;

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

#[repr(C)]
struct CallbackVtbl {
    query_interface: unsafe extern "C" fn(*mut Callback, *const Guid, *mut *mut c_void) -> Hresult,
    add_ref: unsafe extern "C" fn(*mut Callback) -> u32,
    release: unsafe extern "C" fn(*mut Callback) -> u32,
    initialize: unsafe extern "C" fn(*mut Callback, *mut c_void) -> Hresult,
    // ICorProfilerCallback has 69 methods, +8 (v2) +3 (v3) +15 (v4) = 95; the
    // first (Initialize) is above, leaving 94 we don't handle.
    rest: [*const c_void; 94],
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

    // SetEventMask (ICorProfilerInfo slot 16): monitor JIT and enable ReJIT.
    let mask = COR_PRF_MONITOR_JIT_COMPILATION | COR_PRF_ENABLE_REJIT;
    let hr = unsafe { call_slot_u32(info, 16, mask) };
    log(&format!(
        "profiler initialized: ICorProfilerInfo={info:?}, SetEventMask(0x{mask:x})=0x{hr:x}"
    ));
    S_OK
}

static CALLBACK_VTBL: CallbackVtbl = CallbackVtbl {
    query_interface: cb_query_interface,
    add_ref: cb_add_ref,
    release: cb_release,
    initialize: cb_initialize,
    rest: [cb_stub as *const c_void; 94],
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
