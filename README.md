hao
=======

(verb) (-a) to catch in a net, fish (with a net). 
> Kua taha ngā rā i hao ai i te ika o te moana, o te wai māori. / The days have passed to net the fish of the ocean and fresh water. 


<!-- [![Actions][actions-badge]][actions-url] -->

[![github-badge]][github-link]
[![crates-hao-badge]][crates-hao]
[![docs][docs-hao-badge]][docs-hao]
![License][license-badge]

[license-badge]:https://img.shields.io/crates/l/hao.svg?style=for-the-badge
[github-badge]: https://img.shields.io/badge/github-pigeonhands/hao-8da0cb?style=for-the-badge&labelColor=555555&logo=github
[github-link]: https://github.com/pigeonhands/hao
[actions-badge]: https://img.shields.io/github/actions/workflow/status/pigeonhands/hao/ci.yml?branch=master&style=for-the-badge
[actions-url]: https://github.com/pigeonhands/hao/actions
[crates-hao-badge]: https://img.shields.io/crates/v/hao.svg?style=for-the-badge&color=fc8d62&logo=rust
[crates-hao]: https://crates.io/crates/hao
[docs-hao-badge]: https://img.shields.io/badge/docs.rs-hao-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs
[docs-hao]: https://docs.rs/hao


-----

A library for reading, editing and writing .NET assemblies in Rust.

- Loads every metadata table into a graph of shared [`Entry`] handles. Types own their
  fields, methods, properties, events, nested types and generic parameters.
- Decodes signatures and CIL method bodies. Branch targets are instruction indices and
  operands are resolved to their entries, so edits don't need manual offset fixups.
- Writes modules back out by rebuilding the metadata from scratch, like dnlib's `ModuleWriter`.
  ReadyToRun images are converted back to IL only images.
- Writes C#-like declarations (optionally with IL listings) through `CSharpWriter`.
- Supports compressed (`#~`) and uncompressed (`#-`) metadata.

## Crates

| crate | |
|---|---|
| [`hao`](crates/hao) | read, edit and write .NET assemblies |
| [`hao-runtime`](crates/hao-runtime) | host CoreCLR (`coreclr` feature, .NET 8+) or Mono (`mono` feature) in a Rust process |
| [`hao-runtime-macros`](crates/hao-runtime-macros) | `bind!` generates Rust bindings to a .NET assembly for `hao-runtime` |

## Loading

```rust
use hao::Module;

let module = Module::from_path("Example.dll")?;

for ty in module.all_types() {
    let ty = ty.get();
    println!("{}", ty.full_name());
    for method in &ty.methods {
        println!("    {}", method.get().name);
    }
}
```

`Module::from_path` also resolves referenced assemblies from the module's directory and the
installed .NET shared framework. Use `Module::from_path_no_resolve` to skip that.

## Patching IL

```rust
use hao::{
    dotnet::{il::{Instruction, OpCode}, signature::{MethodSig, TypeSig}},
    Module,
};

let mut module = Module::from_path_no_resolve("Sample.dll")?;

let write_line = module.import_method(
    "System.Console",
    "WriteLine",
    MethodSig::new_static(TypeSig::VOID, vec![TypeSig::STRING]),
)?;

let add = module.find_method("Sample.Calc", "Add").unwrap();
add.edit_body(|body| {
    let index = body.position(|i| i.opcode == OpCode::Add).unwrap();
    body.instructions[index] = Instruction::simple(OpCode::Mul);
    body.insert(0, [Instruction::ldstr("Add called"), Instruction::call(write_line)]);
});

module.write_to_path("Sample.patched.dll")?;
```

`import_type`/`import_method`/`import_field` reuse existing references or definitions and add new
ones when needed. `insert` and `remove` keep branch targets and exception ranges pointing at the
same instructions. See [`examples/patch_method.rs`](crates/hao/examples/patch_method.rs).

## C# output

```rust
use hao::dotnet::csharp::{CSharpOptions, WriteCSharp};

println!("{}", module.to_csharp());
println!("{}", method.csharp());
println!("{}", method.get().body.as_ref().unwrap().csharp()); // IL listing

let with_il = module.to_csharp_with(CSharpOptions { il_bodies: true, ..Default::default() });
```

```sh
crates/hao/tests/fixtures/build.sh
cargo run --example dump_csharp -- crates/hao/tests/fixtures/out/Sample.dll --il
```

## Calling .NET from Rust

`bind!` reads an assembly with hao and generates a struct per public class. Calls go through
a small thunk assembly the macro builds and embeds, so there's no reflection at runtime.

```rust
mod my_lib {
    hao_runtime_macros::bind! { assembly = "dotnet/MyLib.dll" }
}

let runtime = hao_runtime::Runtime::coreclr()?; // or Runtime::mono(&MonoOptions::default())
let calculator = my_lib::my_lib::Calculator::new(&runtime, 10)?;
assert_eq!(calculator.add(5)?, 15);
assert_eq!(calculator.describe("total=")?.as_deref(), Some("total=15"));
```

Primitives, strings and classes can be passed and returned. Exceptions come back as
`hao_runtime::Error::Managed`. Generic types and methods, structs and arrays aren't bound yet.

## Thread safety

Entries use `Rc<RefCell<T>>` by default. Enable the `sync` feature to use `Arc<RwLock<T>>`
instead, which makes `Module` `Send + Sync`.

## Tests

The tests build `crates/hao/tests/fixtures/Sample` with the dotnet SDK and run the rewritten assemblies with
it. Tests that need the sample are skipped when the SDK is not installed. The ignored tests
load and rewrite every assembly in the installed shared framework:

```sh
cargo test --release -- --include-ignored
```

[`Entry`]: https://docs.rs/hao/latest/hao/dotnet/entries/struct.Entry.html
