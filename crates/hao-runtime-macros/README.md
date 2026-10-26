# hao-runtime-macros

`bind!` generates Rust bindings to a .NET assembly, for use with
[`hao-runtime`](../hao-runtime).

```rust
mod my_lib {
    hao_runtime_macros::bind! {
        assembly = "dotnet/MyLib.dll",
    }
}

let runtime = hao_runtime::Runtime::coreclr()?;
let calculator = my_lib::my_lib::Calculator::new(&runtime, 10)?;
assert_eq!(calculator.add(5)?, 15);
```

## Options

| option | |
|---|---|
| `assembly = "path"` | the assembly, relative to the crate's `Cargo.toml` |
| `assembly_env = "VAR"` | read the path from an environment variable at compile time, e.g. one set by a build script |
| `types = ["Ns.Type", ...]` | only bind these types (`+` for nested types) |

## What gets generated

Each namespace becomes a module (`MyLib.Shapes` → `my_lib::shapes`) and each public,
non-generic class becomes a struct wrapping a `hao_runtime::Object`:

- constructors become `new`, `new_1`, ... in declaration order
- static methods take `&Runtime` as their first argument
- instance methods take `&self`
- every call returns `hao_runtime::Result`, with .NET exceptions as `Error::Managed`

| .NET | parameter | return |
|---|---|---|
| `bool`, `int`, `double`, ... | `bool`, `i32`, `f64`, ... | same |
| `char` | `u16` | `u16` |
| `string` | `&str` | `Option<String>` |
| bound class | `&Class` | `Option<Class>` |
| other class / `object` | `&hao_runtime::Object` | `Option<hao_runtime::Object>` |

Methods using anything else are skipped: generic types and methods, structs, arrays, and `ref`/`out`
parameters.

## How it works

The macro reads the assembly with [`hao`](../hao) and builds a small thunk assembly with
`Module::new`: one static wrapper per bound method that converts arguments, makes the call inside
a try/catch, and passes objects as `GCHandle`s. Two variants are embedded in your binary, one
marked `[UnmanagedCallersOnly]` for CoreCLR and one for Mono's unmanaged thunks. At runtime the
library and the thunks are loaded on first use, so calls go straight through function pointers,
with no reflection.

The library's path is baked in at compile time and loaded from there.
