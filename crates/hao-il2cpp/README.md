# hao-il2cpp

[![crates.io](https://img.shields.io/crates/v/hao-il2cpp.svg)](https://crates.io/crates/hao-il2cpp) [![docs.rs](https://img.shields.io/docsrs/hao-il2cpp)](https://docs.rs/hao-il2cpp) [![license](https://img.shields.io/crates/l/hao-il2cpp.svg)](https://github.com/pigeonhands/hao)

Read Unity Il2Cpp games, with the goal of generating .NET proxy assemblies for them with
[`hao`](../hao), the way Il2CppInterop does for MelonLoader.

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
- [ ] custom attributes (only `[DefaultMember]` for indexers is generated so far)
- [ ] more metadata versions (v31)
- [ ] `CodeRegistration` and method addresses
- [ ] runtime proxy assemblies that call into the game, like Il2CppInterop

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
