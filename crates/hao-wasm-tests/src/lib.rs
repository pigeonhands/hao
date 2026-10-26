//! Functions exported from a `wasm32-unknown-unknown` build, called by `run.mjs`
//! to check hao works inside a wasm runtime. Results are counts, or -1 on error.

use hao::{
    dotnet::module::{ModuleKind, TargetRuntime},
    Module,
};

#[no_mangle]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut buffer = Vec::<u8>::with_capacity(len);
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr
}

/// # Safety
/// `ptr` must come from [`alloc`] with the same `len`.
#[no_mangle]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    drop(unsafe { Vec::from_raw_parts(ptr, 0, len) });
}

unsafe fn bytes<'a>(ptr: *const u8, len: usize) -> &'a [u8] {
    unsafe { std::slice::from_raw_parts(ptr, len) }
}

/// Loads an assembly, writes it back out and reloads it. Returns the number of types.
///
/// # Safety
/// `ptr` must point to `len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn rewrite_assembly(ptr: *const u8, len: usize) -> i64 {
    let rewrite = || -> hao::error::Result<i64> {
        let module = Module::from_bytes(unsafe { bytes(ptr, len) })?;
        let reloaded = Module::from_bytes(&module.write()?)?;
        Ok(reloaded.all_types().len() as i64)
    };
    rewrite().unwrap_or(-1)
}

/// Creates an assembly with one type and reloads it. Returns the number of types.
#[no_mangle]
pub extern "C" fn create_assembly() -> i64 {
    let create = || -> hao::error::Result<i64> {
        let mut module = Module::new("FromWasm", ModuleKind::Dll, TargetRuntime::NetCore(9));
        let object = module.corlib_type("System.Object")?;
        module.add_type(hao::dotnet::entries::TypeDef::new(
            hao::dotnet::md::streams::tables_stream::TypeAttributes::Public,
            "Wasm",
            "Created",
            Some(object),
        ));
        let reloaded = Module::from_bytes(&module.write()?)?;
        Ok(reloaded.all_types().len() as i64)
    };
    create().unwrap_or(-1)
}

/// Reads an Il2Cpp game and builds its declaration assemblies. Returns how many were written.
///
/// # Safety
/// Both pointers must point to their lengths of readable bytes.
#[no_mangle]
pub unsafe extern "C" fn il2cpp_assemblies(metadata: *const u8, metadata_len: usize, binary: *const u8, binary_len: usize) -> i64 {
    let generate = || -> hao_il2cpp::Result<i64> {
        let game = hao_il2cpp::Game::from_bytes(unsafe { bytes(metadata, metadata_len) }.to_vec(), unsafe { bytes(binary, binary_len) })?;
        let assemblies = game.generate()?;
        for assembly in &assemblies {
            assembly.module.write()?;
        }
        Ok(assemblies.len() as i64)
    };
    generate().unwrap_or(-1)
}
