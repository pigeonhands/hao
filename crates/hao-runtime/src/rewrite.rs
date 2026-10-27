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
}
