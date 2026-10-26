# hao-il2cpp-runtime

Hook and call into a Unity Il2Cpp game from inside its process, the way MelonLoader mods do.

A mod is a `cdylib` that gets loaded into the game. `on_init` runs once the game's Il2Cpp
runtime has started; from there, classes and methods are found by name through the game's own
`il2cpp_*` API, and methods are hooked with [`hooking`](https://crates.io/crates/hooking).

```rust
use hao_il2cpp_runtime::{Object, Original};

// Runs `load` when the game loads the library.
#[used]
#[link_section = ".init_array"]
static LOAD: extern "C" fn() = load;

extern "C" fn load() {
    hao_il2cpp_runtime::on_init(|il2cpp| {
        let health = il2cpp.class("Assembly-CSharp", "Assets.Scripts.Inventory__Items__Pickups", "PlayerHealth")?;
        // `int PlayerHealth.Heal(float amount, bool allowOverheal)`: the instance comes first.
        health.method("Heal", 2)?.hook(|this: Object, amount: f32, overheal: bool, original: Original<_, i32>| {
            original.call((this, amount * 2.0, overheal))
        })?;
        Ok(())
    });
}
```

The closure's types are checked against the method's real signature when the hook is installed,
and a mismatch is an `Error::Signature` rather than a crash. Arguments and return values are
Rust primitives for the matching .NET ones, `Object` for references, `Il2CppString` for strings
and `*mut c_void` for pointers and `ref` parameters. `Method::call` calls a method the same way,
and `Class::methods` lists every overload with its `Method::signature`. Managed exceptions pass
through hooks.

## Examples

- [`heal_mod`](examples/heal_mod.rs): a Megabonk mod that doubles healing by hooking
  `PlayerHealth.Heal` and reads the player's `hp` field.
- [`scene_overlay`](examples/scene_overlay.rs): a Megabonk mod that hooks Unity's
  `SceneManager.Internal_SceneLoaded` and adds a TextMeshPro text overlay to the game's UI with
  the current scene's name. It shows creating `GameObject`s and components, passing Unity structs
  (`Scene`, `Vector2`, `Color`) declared in Rust, and keeping objects alive with a `GcHandle`.
- [`call_methods`](examples/call_methods.rs): starts a game's Il2Cpp runtime without Unity and
  lists and calls methods. Run it with `cargo run -p hao-il2cpp-runtime --example call_methods --
  <game folder>`.

The mods are loaded by adding `LD_PRELOAD=/path/to/libheal_mod.so %command%` to the game's Steam
launch options. Class, method and field names come from the game's metadata, which
[`hao-il2cpp`](../hao-il2cpp)'s `game_info` example shows.

Managed structs are passed as `#[repr(C)]` Rust structs that implement `Il2CppValue`:

```rust
#[repr(C)]
#[derive(Clone, Copy)]
struct Vector2 { x: f32, y: f32 }

unsafe impl Il2CppValue for Vector2 {
    fn matches(ty: TypeInfo) -> bool {
        ty.is_value_type("UnityEngine", "Vector2")
    }
}
```

Enums are passed as their underlying integer. The garbage collector doesn't scan Rust memory, so
objects kept between calls should be held through a `GcHandle`.

- `Method::hook_raw` takes an `extern "C"` detour instead, with Il2Cpp's hidden `MethodInfo*` as
  the last parameter. `original!` gets the original function and must be the first thing the
  detour does: it reads the trampoline address from registers the hook sets.
- `Class::field_offset` with `read_field`/`write_field` reads and writes fields of game objects.
- `Il2Cpp::load` + `init_standalone` run a game's Il2Cpp runtime without Unity, which is how the
  tests work. Only code that doesn't call into Unity can run that way.

The game binary is found at runtime, whatever it's called (`GameAssembly.so`, `GameAssembly.dll`):
`find_game_assembly(dir)` in a game folder, `loaded_game_assembly()` in the running process.

Status: native Linux games with `LD_PRELOAD` so far. A Windows build of a game running under
Proton is detected and reported as unsupported for now: its `GameAssembly.dll` is loaded by
Wine rather than `dlopen`, and uses the Windows calling convention.

## Tests

`HAO_IL2CPP_GAME=/path/to/game cargo test -p hao-il2cpp-runtime` loads the game's Il2Cpp runtime
into the test process, hooks `String.Concat` and `Int32.Parse` with raw and typed hooks, checks
Unity struct and enum types, and (for Megabonk) checks the example mod installs its hook. Without
the variable the tests skip.
