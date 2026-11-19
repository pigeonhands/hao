# hao-il2cpp

[![crates.io](https://img.shields.io/crates/v/hao-il2cpp.svg)](https://crates.io/crates/hao-il2cpp) [![docs.rs](https://img.shields.io/docsrs/hao-il2cpp)](https://docs.rs/hao-il2cpp) [![license](https://img.shields.io/crates/l/hao-il2cpp.svg)](https://github.com/pigeonhands/hao)

Read Unity Il2Cpp games and regenerate their .NET assemblies with [`hao`](../hao): typed
declaration ("dummy") DLLs to browse in ILSpy and compile against. Calling the game at runtime is
done from Rust with [`hao-il2cpp-runtime`](../hao-il2cpp-runtime), not a managed C# host — so
these assemblies are reference artifacts, and the execution path is Rust rather than
Il2CppInterop-style managed proxies.

**Work in progress.** What's here so far:

- [x] `global-metadata.dat` reader for metadata **v29** (Unity 2021.2 – 2023.x) and **v39**
      (Unity 6): images, assemblies, types, methods, fields, parameters, properties, events and
      generic parameters, with names
- [x] `GameAssembly` binary, **ELF (`.so`) and PE (`.dll`)**: finds `MetadataRegistration` and
      decodes the `Il2CppType` table, giving the types of fields, parameters and return values.
      Validated end-to-end on a native Linux game (v29) and a Windows / Unity 6 game (v39).
- [x] declaration assemblies ("dummy DLLs"): one assembly per image with every type, field,
      constant, method signature, property, event, generic parameter and interface, with stub
      bodies. Other code compiles against them.
- [x] `CodeRegistration`: resolves each method's native code address in `GameAssembly`
      (`MethodView::code_address`, `Game::method_pointer`)
- [x] custom attributes: recovers each member's attributes and their fixed/named argument
      values from v29+ metadata (primitives, strings, enums, simple `System.Type`, arrays);
      attributes whose blob can't be re-encoded exactly (`object` params, generic/array `Type`)
      are dropped, never guessed
- [x] metadata **v31** (later Unity 2022.3 LTS / 2023.x): implemented from the
      Il2CppDumper/Il2CppInspector reference layout — **unverified against a real v31 game**
- [x] call the game's methods at runtime — done from **Rust**, not a managed C# host. The
      generated assemblies are reference artifacts (browse in ILSpy, compile against); live calls
      and hooks run through [`hao-il2cpp-runtime`](../hao-il2cpp-runtime)'s typed `call`/`hook`,
      and the `(image, token)` identity each method carries resolves to a live `MethodInfo` with
      `Il2Cpp::method(image, token)` — the online mirror of `Binary::method_pointer`. A managed
      C#-consumer stack à la Il2CppInterop is intentionally out of scope. (`proxy.rs` can still
      emit seam-routed "proxy" assemblies, but their seam is a throwing stub by design.)

```rust
use hao_il2cpp::Game;

let game = Game::open("path/to/game")?; // the folder with GameAssembly.so and <Game>_Data

let health = game.find_type("Assets.Scripts.Inventory__Items__Pickups.PlayerHealth").unwrap();
println!("{} in {}", health.full_name(), health.image_name());
for field in health.fields() {
    println!("    {}", field); // `static int maxMaxHp`, `float shield`, ...
}
for method in health.methods() {
    // The whole signature via Display...
    println!("    {}", method); // `int Heal(float amount, bool allowOverheal)`

    // ...or each part on its own: name, static flag, return type, and the
    // parameters with their own names and types.
    println!("      name:    {}", method.name());        // "Heal"
    println!("      static:  {}", method.is_static());
    println!("      returns: {}", method.return_type_name());
    for parameter in method.parameters() {
        println!("      param:   {} {}", parameter.type_name(), parameter.name());
    }
}

// One declaration assembly per image, for ILSpy or to compile against.
game.write_assemblies("out")?;
```

### Declaration assemblies

An Il2Cpp game ships no .NET assemblies: its C# was compiled to C++ and then to native code in
`GameAssembly`. What's left of the original assemblies is `global-metadata.dat`, which has every
type, method and field name but no code. `write_assemblies` (and the `generate_dlls` example)
rebuild one ordinary .NET assembly per original assembly from it: every type, field, constant,
property, event and method signature, with method bodies that just throw. They're useful for:

- browsing the game in ILSpy, dnSpy or Rider to find what a mod should hook or read
  (for [`hao-il2cpp-runtime`](../hao-il2cpp-runtime)), with exact names and signatures
- compiling C# against the game's API, since the declarations match the real types
- comparing game versions to see what an update changed

They contain no game logic: for that you'd disassemble `GameAssembly`.

`Game` is built on the lower level `Metadata` (`global-metadata.dat`) and `Binary` (the
`Il2CppType` table from `GameAssembly`), which are public for anything the views don't cover.

```sh
cargo run --release -p hao-il2cpp --example game_info -- path/to/game System.String
cargo run --release -p hao-il2cpp --example game_info -- path/to/game System.String Concat
cargo run --release -p hao-il2cpp --example generate_dlls -- path/to/game out
```

## Tests

The tests need a real game, which you point them at:

```sh
HAO_IL2CPP_GAME=/path/to/game cargo test --release -p hao-il2cpp
```

`HAO_IL2CPP_GAME` is a game folder with a v29 `GameAssembly.so` and `*_Data/il2cpp_data`.
`HAO_IL2CPP_METADATA` can point the metadata tests at a `global-metadata.dat` directly. Without
either the tests skip themselves. Game files are never committed.
