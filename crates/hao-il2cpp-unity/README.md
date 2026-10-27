# hao-il2cpp-unity

[![crates.io](https://img.shields.io/crates/v/hao-il2cpp-unity.svg)](https://crates.io/crates/hao-il2cpp-unity) [![docs.rs](https://img.shields.io/docsrs/hao-il2cpp-unity)](https://docs.rs/hao-il2cpp-unity) [![license](https://img.shields.io/crates/l/hao-il2cpp-unity.svg)](https://github.com/pigeonhands/hao)

Typed Unity wrappers for mods of Unity Il2Cpp games, on top of
[`hao-il2cpp-runtime`](../hao-il2cpp-runtime).

The runtime crate finds and hooks methods by name and knows nothing about Unity. This crate wraps
common Unity functionality the same way for any Unity game:

- `GameObject` (create, find, add/get components, activate, tag, layer), `Transform` (position,
  rotation, scale, parent/children, find), `RectTransform`, `Canvas`, `Camera` and TextMeshPro
  text; `Component::is_enabled`/`set_enabled`
- value types `Vector2`, `Vector3` (with arithmetic, `dot`, `magnitude`, `normalized`, `distance`),
  `Quaternion` (`Quaternion::euler`), `Color` (named constants), and the `KeyCode` enum
- `TextOverlay`: a line of text drawn over the game in one call
- `find_objects_of_type::<T>()` / `find_object_of_type::<T>()` and `GameObject::get_component`
  / `Component::get_component` query live objects by type (`Object.FindObjectsOfType`,
  `GetComponent`)
- lifecycle: `on_scene_loaded` (a scene loads), `on_update` (each frame), and `on_render` (on the
  render thread after each camera renders — where to draw an overlay)
- `gl`: immediate-mode drawing (`GL.Begin`/`Vertex3`/`Color`/…) for use inside `on_render`
- `input` (keys and mouse), `screen` (size), `time`, `application`, and
  `log`/`log_warning`/`log_error` (`Debug.Log`, to the game's `Player.log`)

### Drawing on top of the game

`on_render` runs your callback in Unity's render pipeline (it hooks
`Camera.FireOnPostRender`, which the engine calls from native code per camera render). From there
you can draw with the `gl` immediate-mode module.

A plain `gl` draw sits *under* the game's screen-space UI, which the engine composites after the
cameras. To draw on top of it, use an `OverlaySurface`: it renders your `gl` drawing into a
`RenderTexture` shown by a top-most screen-space-overlay canvas, following the screen size and
surviving scene loads. (`OverlayCanvas` is the bare top-most canvas, and `TextOverlay` a line of
text built on it.)

For a full UI, enable the `egui` feature and call `egui::on_egui(|ctx| { … })`: it renders an
[egui](https://github.com/emilk/egui) interface onto an `OverlaySurface`, above the game's own UI
and taking mouse input. See `examples/egui_demo.rs`.

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

- [`scene_overlay`](examples/scene_overlay.rs): draws a live overlay over the game with the
  current scene, scenes loaded and a smoothed FPS, refreshed every frame via `on_update`. Works
  in most Unity games with TextMeshPro.
- [`render_overlay`](examples/render_overlay.rs): draws a crosshair and a panel over the game with
  `GL` immediate mode from the `on_render` hook (using a `Hidden/Internal-Colored` material),
  toggled with F1.
- [`heal_mod`](examples/heal_mod.rs): doubles healing by hooking the game's own
  `PlayerHealth.Heal`, reads the player's `hp` field and logs to `Player.log`.

```sh
cargo build --release -p hao-il2cpp-unity --example scene_overlay
```

## Logging

Errors, debug and trace messages (the game binary being found, `il2cpp_init`, hooks being
installed and removed, lookups) go through [`tracing`](https://crates.io/crates/tracing). Nothing
is printed unless the mod installs a subscriber, e.g. with `tracing-subscriber`:

```rust
let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
let _ = tracing_subscriber::fmt().with_env_filter(filter).with_writer(std::io::stderr).try_init();
```

Then `RUST_LOG=hao_il2cpp_runtime=debug` in the game's launch options shows what hao is doing.

## Tests

`HAO_IL2CPP_GAME=/path/to/game cargo test -p hao-il2cpp-unity` starts the game's Il2Cpp runtime
without Unity and checks every wrapped method exists with the expected signature, and (for
Megabonk) that the `heal_mod` example installs its hook. Without the variable the tests skip.
