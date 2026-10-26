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
use hao_il2cpp::Metadata;

let metadata = Metadata::from_path("Game_Data/il2cpp_data/Metadata/global-metadata.dat")?;
for image in &metadata.images {
    println!("{} ({} types)", metadata.image_name(image), image.type_count);
}
```

```rust
use hao_il2cpp::{generate, Binary, ElfImage, Metadata};

let metadata = Metadata::from_path("Game_Data/il2cpp_data/Metadata/global-metadata.dat")?;
let data = std::fs::read("GameAssembly.so")?;
let binary = Binary::read(&ElfImage::parse(&data)?, &metadata)?;
for assembly in generate(&metadata, &binary)? {
    assembly.module.write_to_path(format!("out/{}", assembly.file_name))?;
}
```

```sh
cargo run -p hao-il2cpp --example il2cpp_info -- path/to/global-metadata.dat System.String path/to/GameAssembly.so
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
