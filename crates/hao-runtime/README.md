# hao-runtime

[![crates.io](https://img.shields.io/crates/v/hao-runtime.svg)](https://crates.io/crates/hao-runtime) [![docs.rs](https://img.shields.io/docsrs/hao-runtime)](https://docs.rs/hao-runtime) [![license](https://img.shields.io/crates/l/hao-runtime.svg)](https://github.com/pigeonhands/hao)

Host the .NET runtime in a Rust process and call into managed code.

| feature | runtime | needs |
|---|---|---|
| `coreclr` (default) | .NET 8+, loading `libcoreclr` directly | a .NET install (`DOTNET_ROOT` or the standard locations) |
| `mono` | the Mono embedding API, as used by Unity | `libmonosgen-2.0` (system Mono, or a game's own Mono) |

```rust
use hao_runtime::Runtime;

let runtime = Runtime::coreclr()?;
runtime.load_assembly("MyLib.dll")?;

// A static [UnmanagedCallersOnly] method.
let add = runtime.function_pointer("MyLib", "MyLib.Exports", "Add")?;
let add: extern "C" fn(i32, i32) -> i32 = unsafe { std::mem::transmute(add) };
assert_eq!(add(40, 2), 42);
```

With Mono, any static method can be called, and the pointer it returns takes an extra trailing
`*mut *mut c_void` argument that receives an exception:

```rust
let runtime = Runtime::mono(&MonoOptions {
    library: Some("MyGame/MonoBleedingEdge/EmbedRuntime/mono-2.0-bdwgc.dll".into()),
    ..Default::default()
})?;
```

- `method_pointer` returns a native-callable pointer to a static method whether or not it is
  `[UnmanagedCallersOnly]`, by JIT-compiling it (via reflection on CoreCLR, `mono_compile_method`
  on Mono). IL you patch before loading takes effect when the method is compiled. Unlike
  `function_pointer` it has no trailing exception argument, so a managed exception crosses it
  unhandled.
- `load_assembly_bytes` loads an assembly from memory, and `load_module` (the `hao` feature, on
  with `coreclr`) loads a [`hao`](../hao) `Module` you built or edited without writing a file.
- CoreCLR is configured in memory: nothing is written to disk, and no `.runtimeconfig.json` is
  needed.
- `Runtime::mono_attach` attaches to a Mono runtime already running in the process, for a mod
  loaded into a Unity Mono game (point `MonoOptions.library` at the game's own Mono). Loading
  assemblies and calling methods then work as usual. Attaching to a foreign CoreCLR is not
  supported.
- Only one runtime of each kind can run per process, so `Runtime::coreclr()` and
  `Runtime::mono()` return the existing instance when called again.
- On Mono, threads have to be attached before running managed code. `attach_thread` does this,
  and generated bindings call it for you.

Calling methods by hand is fiddly; [`hao-runtime-macros`](../hao-runtime-macros) generates safe
bindings instead. The `bindings` module holds the support code those bindings use.

## Examples

```sh
cargo run -p hao-runtime --example generate_and_call
cargo run -p hao-runtime --example patch_in_memory
cargo run -p hao-runtime --example hook_managed
cargo run -p hao-runtime --features mono --example mono
```

- [`generate_and_call`](examples/generate_and_call.rs): builds a class with hao (arithmetic, a
  call into `System.Math`, and a loop built with the IL emitter), loads it and calls it.
- [`patch_in_memory`](examples/patch_in_memory.rs): reads an assembly, patches two methods' IL,
  and loads the patched copy next to the original to compare them.
- [`hook_managed`](examples/hook_managed.rs): hooks managed methods with Rust functions, like
  Harmony prefixes and postfixes, by rewriting their IL to call a native function pointer.
- [`mono`](examples/mono.rs): the Mono backend, including a call that throws.
- [`call_any_method`](examples/call_any_method.rs): calls plain static methods (no `[UnmanagedCallersOnly]`) with `method_pointer`, including one whose IL was patched before loading.
