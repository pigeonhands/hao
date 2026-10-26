# hao-il2cpp-unity

Typed Unity wrappers for mods of Unity Il2Cpp games, on top of
[`hao-il2cpp-runtime`](../hao-il2cpp-runtime).

The runtime crate finds and hooks methods by name and knows nothing about Unity. This crate wraps
common Unity functionality the same way for any Unity game:

- `GameObject` (create, find, add components, activate), `Transform`, `RectTransform`, `Canvas`
  and TextMeshPro text, with `Vector2`, `Vector3` and `Color`
- `TextOverlay`: a line of text drawn over the game in one call
- `Scene` and `on_scene_loaded`, which runs code on Unity's main thread whenever a scene loads
- `log`/`log_warning`/`log_error` (`Debug.Log`, to the game's `Player.log`) and `time`

Each Unity method is looked up the first time it's used and checked against the game's real
signature, so a game built with a different Unity version gives an error instead of a crash.
`check_bindings` checks them all at once.

```rust
use std::sync::OnceLock;

use hao_il2cpp_unity::{on_scene_loaded, TextOverlay};

static OVERLAY: OnceLock<TextOverlay> = OnceLock::new();

#[used]
#[link_section = ".init_array"]
static LOAD: extern "C" fn() = load;

extern "C" fn load() {
    hao_il2cpp_unity::on_init(|_| {
        on_scene_loaded(|scene, _| {
            let overlay = OVERLAY.get_or_init(|| TextOverlay::new("my mod").unwrap());
            overlay.set_text(&format!("scene: {}", scene.name().unwrap())).unwrap();
        })
    });
}
```

Unity's API may only be used from its main thread, which hooks of methods the game calls there
(like `on_scene_loaded`'s callbacks) run on.

## Examples

Both are mods for Megabonk's Linux build. Build one, then add
`LD_PRELOAD=/path/to/target/release/examples/lib<name>.so %command%` to the game's Steam launch
options.

- [`scene_overlay`](examples/scene_overlay.rs): draws text over the game with the current scene's
  name and how many scenes have loaded. Works in most Unity games with TextMeshPro.
- [`heal_mod`](examples/heal_mod.rs): doubles healing by hooking the game's own
  `PlayerHealth.Heal`, reads the player's `hp` field and logs to `Player.log`.

```sh
cargo build --release -p hao-il2cpp-unity --example scene_overlay
```

## Tests

`HAO_IL2CPP_GAME=/path/to/game cargo test -p hao-il2cpp-unity` starts the game's Il2Cpp runtime
without Unity and checks every wrapped method exists with the expected signature, and (for
Megabonk) that the `heal_mod` example installs its hook. Without the variable the tests skip.
