# hao

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
same instructions. See [`examples/patch_method.rs`](examples/patch_method.rs).

## Creating assemblies

```rust
use hao::dotnet::{
    entries::{Method, TypeDef},
    il::{Emitter, OpCode},
    module::{ModuleKind, TargetRuntime},
    signature::{MethodSig, TypeSig},
};

let mut module = Module::new("Generated", ModuleKind::Dll, TargetRuntime::NetCore(9));
let answer = module.add_static_class("Generated", "Answer")?;
let max = module.import_corlib_method(
    "System.Math",
    "Max",
    MethodSig::new_static(TypeSig::INT32, vec![TypeSig::INT32; 2]),
)?;

// public static int Get(int a) => a > 0 ? Math.Max(a, 42) : 0;
let body = Emitter::build(|e| {
    e.ldarg(0).ldc_i4(0).op(OpCode::Cgt).if_else(
        |e| {
            e.ldarg(0).ldc_i4(42).call(&max);
        },
        |e| {
            e.ldc_i4(0);
        },
    );
    e.ret();
});
let signature = MethodSig::new_static(TypeSig::INT32, vec![TypeSig::INT32]);
TypeDef::add_method(&answer, Method::public_static("Get", signature).with_body(body));
module.write_to_path("Generated.dll")?;
```

`TargetRuntime::NetFramework` references `mscorlib` instead, for Mono and Unity.

`il::Emitter` has a method per common instruction (`ldarg`, `call`, `stloc`, ...), structured
control flow (`if_else`, `if_then`, `while_loop`, `try_catch`) and `call_native` for calling a
native function pointer. To patch an existing method, `MethodBody::prefix` runs code at its start
and `postfix` before every return. `Method::with_attribute` with `Module::corlib_attribute` adds
attributes such as `[UnmanagedCallersOnly]`.

Members of other loaded modules are referenced with `import_foreign_method`,
`import_foreign_type` and `import_foreign_signature`, which copy the needed type and assembly
references across.

## Custom attributes

```rust
let value = attribute.decode(&module)?;      // fixed and named arguments
let tag = value.fixed[0].as_str();
let weight = value.named("Weight").and_then(|v| v.as_i64());

let attribute = CustomAttribute::new(ctor, &CustomAttributeValue {
    fixed: vec!["text".into()],
    named: vec![CANamedArgument::property("Weight", 3.into())],
})?;
```

Enum arguments need the enum's type, so load the module with its dependencies when an attribute
uses enums from another assembly.

## Unity and Mono

`PathAssemblyResolver::for_game(game_dir)` resolves from a Unity game's `*_Data/Managed` folder
and its bundled Mono. Assemblies referencing `mscorlib` are written the same way, and the tests
run a patched .NET Framework build with Mono when it is installed.

## C# output

```rust
use hao::dotnet::csharp::{CSharpOptions, WriteCSharp};

println!("{}", module.to_csharp());
println!("{}", method.csharp());
println!("{}", method.get().body.as_ref().unwrap().csharp()); // IL listing

let with_il = module.to_csharp_with(CSharpOptions { il_bodies: true, ..Default::default() });
```

```sh
tests/fixtures/build.sh
cargo run --example dump_csharp -- tests/fixtures/out/Sample.dll --il
```

## Thread safety

Entries use `Rc<RefCell<T>>` by default. Enable the `sync` feature to use `Arc<RwLock<T>>`
instead, which makes `Module` `Send + Sync`.

## Tests

The tests build `tests/fixtures/Sample` with the dotnet SDK and run the rewritten assemblies with
it. Tests that need the sample are skipped when the SDK is not installed. The ignored tests
load and rewrite every assembly in the installed shared framework:

```sh
cargo test --release -- --include-ignored
```

[`Entry`]: https://docs.rs/hao/latest/hao/dotnet/entries/struct.Entry.html
