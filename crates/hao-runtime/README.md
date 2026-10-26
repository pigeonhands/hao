# hao-runtime

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

- `load_assembly_bytes` loads an assembly from memory, e.g. one generated with
  [`hao`](../hao)'s `Module::new`.
- Only one runtime of each kind can run per process, so `Runtime::coreclr()` and
  `Runtime::mono()` return the existing instance when called again.
- On Mono, threads have to be attached before running managed code. `attach_thread` does this,
  and generated bindings call it for you.

Calling methods by hand is fiddly; [`hao-runtime-macros`](../hao-runtime-macros) generates safe
bindings instead. The `bindings` module holds the support code those bindings use.
