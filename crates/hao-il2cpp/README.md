# hao-il2cpp

Read Unity Il2Cpp games, with the goal of generating .NET proxy assemblies for them with
[`hao`](../hao), the way Il2CppInterop does for MelonLoader.

**Work in progress.** What's here so far:

- [x] `global-metadata.dat` reader for metadata v29 (Unity 2021.2 – 2023.x): images, assemblies,
      types, methods, fields, parameters, properties, events and generic parameters, with names
- [x] `GameAssembly` binary (ELF): finds `MetadataRegistration` and decodes the `Il2CppType`
      table, giving the types of fields, parameters and return values
- [x] declaration assemblies ("dummy DLLs"): one assembly per image with every type, field,
      constant, method signature, property, event, generic parameter and interface, with stub
      bodies. Other code compiles against them.
- [ ] custom attributes (only `[DefaultMember]` for indexers is generated so far)
- [ ] PE game binaries (`GameAssembly.dll`)
- [ ] more metadata versions (v31, v39 for Unity 6)
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
    println!("    {}", method); // `int Heal(float amount, bool allowOverheal)`, ...
}

// One declaration assembly per image, for ILSpy or to compile against.
game.write_assemblies("out")?;
```

`Game` is built on the lower level `Metadata` (`global-metadata.dat`) and `Binary` (the
`Il2CppType` table from `GameAssembly`), which are public for anything the views don't cover.

```sh
cargo run --release -p hao-il2cpp --example game_info -- path/to/game System.String
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
