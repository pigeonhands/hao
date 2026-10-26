//! Experimental: draw an [`egui`] UI over the game from the render hook.
//!
//! **Unverified rendering.** The data flow (input, tessellation, texture upload,
//! GL draw) is complete, but the shader/blend/colour-space details are the kind
//! of thing that needs tuning against a real game. Enable with the `egui`
//! feature.
//!
//! egui is rendered with Unity's immediate-mode [`gl`](crate::gl): its font atlas
//! is uploaded to a [`Texture2D`], bound through a `Material`, and each tessellated
//! triangle is emitted. See [`on_egui`].

use std::sync::{Arc, Mutex};

use hao_il2cpp_runtime::{GcHandle, Result};

use crate::{gl, input, screen, Color, Material, Shader, Texture2D, TextureFormat};

/// Runs `ui` every frame to build and draw an egui interface over the game.
///
/// Call it once (e.g. from `on_init`). The egui context, font texture and
/// material are created on the render thread on the first frame. Mouse input is
/// fed from `UnityEngine.Input`.
pub fn on_egui(ui: impl Fn(&egui::Context) + Send + Sync + 'static) -> Result<()> {
    let ui = Arc::new(ui);
    let state: Mutex<Option<EguiState>> = Mutex::new(None);
    crate::on_render(move |_camera| {
        let mut guard = state.lock().unwrap_or_else(|e| e.into_inner());
        let state = match guard.as_mut() {
            Some(state) => state,
            None => match EguiState::new() {
                Ok(state) => guard.insert(state),
                Err(e) => {
                    tracing::error!("egui init failed: {}", e);
                    return;
                }
            },
        };
        if let Err(e) = state.frame(&*ui) {
            tracing::error!("egui frame failed: {}", e);
        }
    })
}

struct EguiState {
    ctx: egui::Context,
    /// Kept alive across frames: the GC doesn't scan Rust memory.
    material: GcHandle,
    texture: Option<(GcHandle, [usize; 2])>,
    /// A CPU copy of the font atlas (RGBA), patched from egui's texture deltas.
    atlas: Vec<u8>,
    /// Previous state of the left/right mouse buttons, so a press or release is
    /// sent to egui only on the edge (egui is event-based, not level-based).
    pointer_down: [bool; 2],
}

/// Builds the egui material: a textured, vertex-coloured, alpha-blended shader.
/// `HideAndDontSave` keeps Unity from destroying it (and throwing NRE on later
/// use) when a scene loads, which `DontDestroyOnLoad` does not do for assets.
fn make_material() -> Result<Material> {
    let shader = Shader::find("Sprites/Default")?
        .or(Shader::find("UI/Default")?)
        .ok_or_else(|| hao_il2cpp_runtime::Error::NotFound("a sprite/UI shader for egui".into()))?;
    let material = Material::new(shader)?;
    material.set_hide_flags(crate::HideFlags::HIDE_AND_DONT_SAVE)?;
    Ok(material)
}

impl EguiState {
    fn new() -> Result<Self> {
        let ctx = egui::Context::default();
        ctx.set_pixels_per_point(1.0);
        let material = make_material()?;
        Ok(Self {
            ctx,
            material: GcHandle::new(crate::il2cpp()?, material.object()),
            texture: None,
            atlas: Vec::new(),
            pointer_down: [false; 2],
        })
    }

    fn material(&self) -> Material {
        Material(self.material.target())
    }

    /// Rebuilds the material and font texture if a scene load destroyed them
    /// despite `HideAndDontSave`. The GC handles keep the C# wrappers alive, so
    /// their sizes survive; only the underlying Unity objects are remade.
    fn ensure_alive(&mut self) -> Result<()> {
        if self.material().is_alive()? {
            return Ok(());
        }
        tracing::debug!("egui: resources were destroyed, recreating");
        self.material = GcHandle::new(crate::il2cpp()?, make_material()?.object());
        if let Some((_, size)) = self.texture.take() {
            self.ensure_texture(size)?;
            if !self.atlas.is_empty() {
                Texture2D(self.texture.as_ref().unwrap().0.target()).load_raw(&self.atlas)?;
            }
        }
        Ok(())
    }

    fn frame(&mut self, ui: &(impl Fn(&egui::Context) + ?Sized)) -> Result<()> {
        self.ensure_alive()?;

        // Step logs: whichever appears last in Player.log before a managed
        // exception is the call that threw. Enable with RUST_LOG=debug.
        tracing::debug!("egui frame: screen size");
        let (w, h) = screen::size()?;
        let (w, h) = (w.max(1) as f32, h.max(1) as f32);

        tracing::debug!("egui frame: input");
        let input = self.raw_input(w, h)?;
        tracing::debug!("egui frame: run ui");
        let output = self.ctx.run(input, |ctx| ui(ctx));
        for (id, delta) in output.textures_delta.set {
            // Only the egui font atlas (managed texture 0) is handled here.
            if matches!(id, egui::TextureId::Managed(_)) {
                tracing::debug!(size = ?delta.image.size(), "egui frame: upload texture");
                self.update_texture(&delta)?;
            }
        }
        let primitives = self.ctx.tessellate(output.shapes, 1.0);
        tracing::debug!(primitives = primitives.len(), "egui frame: render");
        self.render(w, h, &primitives)
    }

    fn raw_input(&mut self, w: f32, h: f32) -> Result<egui::RawInput> {
        let mouse = input::mouse_position()?;
        // Unity's origin is bottom-left; egui's is top-left.
        let pos = egui::pos2(mouse.x, h - mouse.y);
        let mut events = vec![egui::Event::PointerMoved(pos)];
        for (i, (button, egui_button)) in [
            (0, egui::PointerButton::Primary),
            (1, egui::PointerButton::Secondary),
        ]
        .into_iter()
        .enumerate()
        {
            // Send an event only when the button changes state, so egui gets one
            // press and one release per click rather than a press every frame
            // (which left drags stuck to the cursor).
            let down = input::mouse_button(button)?;
            if down != self.pointer_down[i] {
                self.pointer_down[i] = down;
                events.push(egui::Event::PointerButton {
                    pos,
                    button: egui_button,
                    pressed: down,
                    modifiers: egui::Modifiers::default(),
                });
            }
        }
        Ok(egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::pos2(0.0, 0.0),
                egui::vec2(w, h),
            )),
            events,
            ..Default::default()
        })
    }

    /// Patches the CPU atlas with a texture delta and re-uploads it.
    fn update_texture(&mut self, delta: &egui::epaint::ImageDelta) -> Result<()> {
        let [dw, dh] = delta.image.size();
        let pixels: Vec<u8> = match &delta.image {
            egui::epaint::ImageData::Font(font) => {
                font.srgba_pixels(None).flat_map(|p| p.to_array()).collect()
            }
            egui::epaint::ImageData::Color(image) => {
                image.pixels.iter().flat_map(|p| p.to_array()).collect()
            }
        };

        match delta.pos {
            None => {
                self.atlas = pixels;
                self.ensure_texture([dw, dh])?;
            }
            Some([x, y]) => {
                let full = self.texture.as_ref().map(|(_, s)| *s).unwrap_or([dw, dh]);
                for row in 0..dh {
                    let dst = ((y + row) * full[0] + x) * 4;
                    let src = row * dw * 4;
                    if dst + dw * 4 <= self.atlas.len() {
                        self.atlas[dst..dst + dw * 4].copy_from_slice(&pixels[src..src + dw * 4]);
                    }
                }
            }
        }
        Texture2D(self.texture.as_ref().unwrap().0.target()).load_raw(&self.atlas)
    }

    fn ensure_texture(&mut self, size: [usize; 2]) -> Result<()> {
        if self.texture.as_ref().map(|(_, s)| *s) != Some(size) {
            let texture = Texture2D::new(size[0] as i32, size[1] as i32, TextureFormat::RGBA32)?;
            // Survive scene loads, as for the material.
            texture.set_hide_flags(crate::HideFlags::HIDE_AND_DONT_SAVE)?;
            let handle = GcHandle::new(crate::il2cpp()?, texture.object());
            self.texture = Some((handle, size));
        }
        Ok(())
    }

    fn render(&self, w: f32, h: f32, primitives: &[egui::ClippedPrimitive]) -> Result<()> {
        let Some((texture, _)) = &self.texture else {
            return Ok(());
        };
        let material = self.material();
        material.set_main_texture(Texture2D(texture.target()))?;

        gl::push_matrix()?;
        // Flip Y so egui's top-left origin maps to Unity's bottom-left.
        gl::load_pixel_matrix(0.0, w, h, 0.0)?;
        for primitive in primitives {
            let egui::epaint::Primitive::Mesh(mesh) = &primitive.primitive else {
                continue;
            };
            material.set_pass(0)?;
            gl::begin(gl::Mode::Triangles)?;
            for &index in &mesh.indices {
                let v = mesh.vertices[index as usize];
                let [r, g, b, a] = v.color.to_array();
                let scale = 1.0 / 255.0;
                gl::color(Color::rgba(
                    r as f32 * scale,
                    g as f32 * scale,
                    b as f32 * scale,
                    a as f32 * scale,
                ))?;
                gl::tex_coord(v.uv.x, v.uv.y)?;
                gl::vertex(v.pos.x, v.pos.y)?;
            }
            gl::end()?;
        }
        gl::pop_matrix()
    }
}
