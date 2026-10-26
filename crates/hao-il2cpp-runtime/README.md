# hao-il2cpp-runtime

Hook and call into a Unity Il2Cpp game from inside its process, the way MelonLoader mods do.

A mod is a `cdylib` that gets loaded into the game. `on_init` runs once the game's Il2Cpp
runtime has started; from there, classes and methods are found by name through the game's own
`il2cpp_*` API, and methods are hooked with [`hooking`](https://crates.io/crates/hooking).

```rust
use hao_il2cpp_runtime::{original, MethodInfo, Object};

// `int PlayerHealth.Heal(float amount, bool allowOverheal)`. Il2Cpp methods take the
// instance first and a hidden `MethodInfo*` last.
extern "C" fn heal(this: Object, amount: f32, allow_overheal: bool, method: MethodInfo) -> i32 {
    let heal = original!(extern "C" fn(Object, f32, bool, MethodInfo) -> i32);
    heal(this, amount * 2.0, allow_overheal, method)
}

#[ctor::ctor]
fn load() {
    hao_il2cpp_runtime::on_init(|il2cpp| {
        let health = il2cpp.class("Assembly-CSharp", "Assets.Scripts.Inventory__Items__Pickups", "PlayerHealth")?;
        unsafe { health.method("Heal", 2)?.hook(heal as *mut u8)? };
        Ok(())
    });
}
```

[`examples/heal_mod.rs`](examples/heal_mod.rs) is a complete mod for Megabonk's Linux build. Load it
by adding `LD_PRELOAD=/path/to/libheal_mod.so %command%` to the game's Steam launch options.
The class, method and field names come from the game's metadata, which
[`hao-il2cpp`](../hao-il2cpp)'s `game_info` example shows.

- `original!` must be the first thing a detour does: it reads the trampoline address from
  registers the hook sets.
- `Class::field_offset` with `read_field`/`write_field` reads and writes fields of game objects.
- `Il2Cpp::load` + `init_standalone` run a game's Il2Cpp runtime without Unity, which is how the
  tests work. Only code that doesn't call into Unity can run that way.

Status: Linux (`LD_PRELOAD`) only so far. Windows needs a proxy DLL to get into the process.

## Tests

`HAO_IL2CPP_GAME=/path/to/game cargo test -p hao-il2cpp-runtime` loads the game's Il2Cpp runtime
into the test process, hooks `String.Concat`, and (for Megabonk) checks the example mod installs
its hook. Without the variable the tests skip.
