//! CoreCLR JIT helpers — resolve precode indirections and call JIT'd methods.
//!
//! CoreCLR compiles methods lazily. Before a method is JIT'd, calls go through
//! a *precode* stub that triggers compilation and then patches itself to jump
//! directly to the native code. The two common precode forms on x86-64 are:
//!
//! - **`ff 25 <disp32>`** — indirect jump through a pointer slot
//! - **`e9 <disp32>`** — relative jump to the JIT'd body
//!
//! [`resolve_jit`] follows one level of indirection so you get the actual
//! native address, and [`patch_precode_target`] rewrites the indirect-jump
//! slot to redirect a method.
//!
//! [`call_native`] and [`call_native_float`] are small trampolines that call a
//! native function pointer with up to 6 integer arguments, used by script
//! runtimes to invoke managed methods generically.

use std::fmt;

/// Errors returned by JIT helpers.
#[derive(Debug)]
pub enum JitError {
    NotAPrecode,
}

impl fmt::Display for JitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JitError::NotAPrecode => write!(f, "not a precode (expected ff 25)"),
        }
    }
}

impl std::error::Error for JitError {}

/// Follow one level of CoreCLR precode indirection.
///
/// Given a method's precode address, returns the address it currently points
/// to — either the JIT'd native body or the prestub. If the address is not a
/// recognised precode form, returns the input unchanged.
///
/// # Safety
///
/// `addr` must point to readable memory containing a valid precode or native
/// code. The caller must ensure the memory remains mapped.
#[cfg(target_arch = "x86_64")]
pub unsafe fn resolve_jit(addr: usize) -> usize {
    let p = addr as *const u8;
    let b0 = unsafe { *p };
    let b1 = unsafe { *p.add(1) };
    if b0 == 0xff && b1 == 0x25 {
        let disp = unsafe { std::ptr::read_unaligned(p.add(2) as *const i32) };
        let slot = unsafe { p.add(6).offset(disp as isize) as *const usize };
        unsafe { *slot }
    } else if b0 == 0xe9 {
        let disp = unsafe { std::ptr::read_unaligned(p.add(1) as *const i32) };
        unsafe { p.add(5).offset(disp as isize) as usize }
    } else {
        addr
    }
}

/// Rewrite a `ff 25` precode's target pointer to redirect the method.
///
/// # Safety
///
/// `precode_addr` must point to an `ff 25 <disp32>` instruction whose
/// pointer slot is writable (or has been made writable).
#[cfg(target_arch = "x86_64")]
pub unsafe fn patch_precode_target(precode_addr: usize, new_target: usize) -> Result<(), JitError> {
    let p = precode_addr as *const u8;
    let b0 = unsafe { *p };
    let b1 = unsafe { *p.add(1) };
    if b0 == 0xff && b1 == 0x25 {
        let disp = unsafe { std::ptr::read_unaligned(p.add(2) as *const i32) };
        let slot = unsafe { p.add(6).offset(disp as isize) as *mut usize };
        unsafe { *slot = new_target };
        Ok(())
    } else {
        Err(JitError::NotAPrecode)
    }
}

/// Read `count` bytes starting at `addr` as a hex string, capped at 64.
///
/// # Safety
///
/// `addr` must point to at least `min(count, 64)` bytes of readable memory.
pub unsafe fn peek_bytes(addr: usize, count: usize) -> String {
    let n = count.min(64);
    let bytes: Vec<u8> = (0..n).map(|i| unsafe { *((addr + i) as *const u8) }).collect();
    let hex: Vec<String> = bytes.iter().map(|b| format!("{b:02x}")).collect();
    hex.join(" ")
}

/// Call a native function pointer with up to 6 integer/pointer arguments.
///
/// This is the generic trampoline for calling managed methods through the
/// raw pointers returned by [`Runtime::method_pointer`](crate::Runtime::method_pointer).
///
/// # Safety
///
/// `ptr` must be a valid function pointer. `args` must contain the correct
/// number and types of arguments for the target function.
#[cfg(target_arch = "x86_64")]
pub unsafe fn call_native(ptr: usize, args: &[u64]) -> u64 {
    type F0 = unsafe extern "C" fn() -> u64;
    type F1 = unsafe extern "C" fn(u64) -> u64;
    type F2 = unsafe extern "C" fn(u64, u64) -> u64;
    type F3 = unsafe extern "C" fn(u64, u64, u64) -> u64;
    type F4 = unsafe extern "C" fn(u64, u64, u64, u64) -> u64;
    type F5 = unsafe extern "C" fn(u64, u64, u64, u64, u64) -> u64;
    type F6 = unsafe extern "C" fn(u64, u64, u64, u64, u64, u64) -> u64;

    unsafe {
        match args.len() {
            0 => std::mem::transmute::<usize, F0>(ptr)(),
            1 => std::mem::transmute::<usize, F1>(ptr)(args[0]),
            2 => std::mem::transmute::<usize, F2>(ptr)(args[0], args[1]),
            3 => std::mem::transmute::<usize, F3>(ptr)(args[0], args[1], args[2]),
            4 => std::mem::transmute::<usize, F4>(ptr)(args[0], args[1], args[2], args[3]),
            5 => std::mem::transmute::<usize, F5>(ptr)(args[0], args[1], args[2], args[3], args[4]),
            6 => std::mem::transmute::<usize, F6>(ptr)(
                args[0], args[1], args[2], args[3], args[4], args[5],
            ),
            _ => panic!("too many arguments for call_native (max 6)"),
        }
    }
}

/// Like [`call_native`], but for methods returning a float (xmm0 on x86-64).
/// Supports up to 3 integer arguments.
///
/// # Safety
///
/// Same requirements as [`call_native`].
#[cfg(target_arch = "x86_64")]
pub unsafe fn call_native_float(ptr: usize, args: &[u64]) -> f64 {
    type F0 = unsafe extern "C" fn() -> f64;
    type F1 = unsafe extern "C" fn(u64) -> f64;
    type F2 = unsafe extern "C" fn(u64, u64) -> f64;
    type F3 = unsafe extern "C" fn(u64, u64, u64) -> f64;

    unsafe {
        match args.len() {
            0 => std::mem::transmute::<usize, F0>(ptr)(),
            1 => std::mem::transmute::<usize, F1>(ptr)(args[0]),
            2 => std::mem::transmute::<usize, F2>(ptr)(args[0], args[1]),
            3 => std::mem::transmute::<usize, F3>(ptr)(args[0], args[1], args[2]),
            _ => panic!("too many arguments for call_native_float (max 3)"),
        }
    }
}
