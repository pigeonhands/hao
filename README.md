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
use hao::{dotnet::il::{Instruction, OpCode, Operand}, Module};

let module = Module::from_path_no_resolve("Sample.dll")?;
let calc = module.find_type("Sample", "Calc").unwrap();
let add = calc.get().find_method("Add").unwrap();

{
    let mut add = add.get_mut();
    let body = add.body.as_mut().unwrap();
    let index = body.position(|i| i.opcode == OpCode::Add).unwrap();
    body.instructions[index] = Instruction::simple(OpCode::Mul);
    body.insert(0, [
        Instruction::new(OpCode::Ldstr, Operand::String("hi".into())),
        Instruction::simple(OpCode::Pop),
    ]);
}

module.write_to_path("Sample.patched.dll")?;
```

New type, member and assembly references used by the patched code are added to the metadata
automatically. See [`examples/patch_method.rs`](examples/patch_method.rs) for a complete example
that adds a `Console.WriteLine` call.

## C# output

```rust
use hao::dotnet::csharp::{CSharpOptions, WriteCSharp};

println!("{}", module.to_csharp());
println!("{}", method.csharp());

let with_il = module.to_csharp_with(CSharpOptions { il_bodies: true, ..Default::default() });
```

```sh
cargo run --example dump_csharp -- tests/fixtures/Sample.dll --il
```

## Thread safety

Entries use `Rc<RefCell<T>>` by default. Enable the `sync` feature to use `Arc<RwLock<T>>`
instead, which makes `Module` `Send + Sync`.

## Tests

`tests/fixtures/Sample.dll` is built from `tests/fixtures/Sample` with `tests/fixtures/build.sh`.
The write tests run the rewritten assembly with `dotnet` when it is installed. The ignored tests
load and rewrite every assembly in the installed shared framework:

```sh
cargo test --release -- --include-ignored
```

[`Entry`]: https://docs.rs/hao/latest/hao/dotnet/entries/struct.Entry.html
