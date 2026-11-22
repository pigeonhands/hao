# hao-egui-overlay

Engine-agnostic [egui](https://github.com/emilk/egui) overlay for games, via
OpenGL/Vulkan swapchain hooks. Inject as an `LD_PRELOAD` library and draw an
egui UI on top of any game — no engine bindings required.

Both backends are hooked at init. Whichever renderer the game activates first
is used; the draw callback is the same either way.

## Features

| Feature   | Default | Description |
|-----------|---------|-------------|
| `opengl`  | yes     | Hook `glXSwapBuffers`, render via `egui_glow` |
| `vulkan`  | yes     | Hook `vkQueuePresentKHR`, render via `ash` + custom pipeline |

## Quick start

```rust
use std::sync::atomic::{AtomicBool, Ordering};

static VISIBLE: AtomicBool = AtomicBool::new(true);

#[used]
#[link_section = ".init_array"]
static LOAD: extern "C" fn() = load;

extern "C" fn load() {
    hao_egui_overlay::on_frame(|ctx| {
        // F1 toggles the overlay
        ctx.input(|i| {
            if i.key_pressed(egui::Key::F1) {
                VISIBLE.fetch_xor(true, Ordering::Relaxed);
            }
        });
        if !VISIBLE.load(Ordering::Relaxed) { return; }

        egui::Window::new("My Overlay").show(ctx, |ui| {
            ui.label("Hello from the overlay!");
            if ui.button("Click me").clicked() {
                tracing::info!("button clicked");
            }
        });
    });

    hao_egui_overlay::set_scale(0.75);
    hao_egui_overlay::init();
}
```

Build as a cdylib and inject:

```sh
cargo build --release --example overlay_demo
LD_PRELOAD=target/release/examples/liboverlay_demo.so ./my_game
```

## How it works

### OpenGL

The crate exports a `glXSwapBuffers` symbol. Because the library is
`LD_PRELOAD`'d, calls to `glXSwapBuffers` land here first. On each call:

1. Query the GL viewport for screen dimensions
2. Gather mouse + keyboard input from X11 (`XQueryPointer`, `XQueryKeymap`)
3. Run the egui frame (input → callback → tessellate)
4. Render via `egui_glow::Painter` into the current framebuffer
5. Forward to the real `glXSwapBuffers`

### Vulkan

The crate exports `vkGetInstanceProcAddr`. Vulkan functions are loaded through
it, so we can intercept any call:

- `vkCreateSwapchainKHR` — capture swapchain images, format, and extent
- `vkQueuePresentKHR` — render egui into the swapchain image before presenting
- `vkDestroySwapchainKHR` — clean up overlay resources

A render pass, pipeline, and per-image framebuffers/command buffers are created
when the swapchain is created (and recreated on resize).

### Input

Mouse position and buttons come from `XQueryPointer` on a separate X11
connection (so we don't interfere with the game's event loop). Keyboard state
comes from `XQueryKeymap`. Both work regardless of whether the game uses X11,
Wayland-via-XWayland, or SDL.
