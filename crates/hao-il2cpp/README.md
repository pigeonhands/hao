# hao-il2cpp

Read Unity Il2Cpp games, with the goal of generating .NET proxy assemblies for them with
[`hao`](../hao), the way Il2CppInterop does for MelonLoader.

**Work in progress.** What's here so far:

- [x] `global-metadata.dat` reader for metadata v29 (Unity 2021.2 – 2023.x): images, assemblies,
      types, methods, fields, parameters, properties, events and generic parameters, with names
- [ ] more metadata versions (v31, v39 for Unity 6)
- [ ] `GameAssembly` binary: `CodeRegistration` / `MetadataRegistration` and the `Il2CppType`
      table, needed for the types of fields, parameters and return values (ELF and PE)
- [ ] proxy assembly generation

```rust
use hao_il2cpp::Metadata;

let metadata = Metadata::from_path("Game_Data/il2cpp_data/Metadata/global-metadata.dat")?;
for image in &metadata.images {
    println!("{} ({} types)", metadata.image_name(image), image.type_count);
}
```

```sh
cargo run -p hao-il2cpp --example il2cpp_info -- path/to/global-metadata.dat System.String
```

## Tests

The tests need a real game. Set `HAO_IL2CPP_METADATA` to a v29 `global-metadata.dat`, or they
skip themselves. Game files are never committed.
