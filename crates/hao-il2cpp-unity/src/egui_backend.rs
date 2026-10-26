//! Draw an [`egui`] UI over the game from the render hook. Enable with the
//! `egui` feature.
//!
//! egui is rendered with Unity's immediate-mode [`gl`](crate::gl): its font atlas
//! is uploaded to a [`Texture2D`] (point-sampled, so solid fills stay opaque),
//! bound through a `Material`, and each tessellated triangle is emitted into a
//! [`RenderTexture`] with its colours converted from gamma to linear. That texture
//! is shown by a [`RawImage`] on our own screen-space-overlay [`Canvas`] with the
//! maximum sorting order, so the UI draws on top of the game's own overlay canvases
//! (which the engine composites after camera rendering, where a plain `gl` draw
//! would sit underneath). See [`on_egui`].

use std::sync::{Arc, Mutex};

use hao_il2cpp_runtime::{GcHandle, Result};

use crate::{
    gl, input, screen, Canvas, Color, GameObject, Material, RawImage, RenderMode, RenderTexture,
    Shader, Texture2D, TextureFormat, Vector2,
};

/// Runs `ui` every frame to build and draw an egui interface over the game.
///
/// Call it once (e.g. from `on_init`). The egui context, font texture and
/// material are created on the render thread on the first frame. Mouse input is
/// fed from `UnityEngine.Input`.
pub fn on_egui(ui: impl Fn(&egui::Context) + Send + Sync + 'static) -> Result<()> {
    let ui = Arc::new(ui);
    let state: Mutex<Option<EguiState>> = Mutex::new(None);
    // The render hook fires once per camera, so several times per frame. Run egui
    // (input + layout) only on the first callback of each frame: otherwise a click
    // edge is read in whichever camera's pass happened to catch it, so presses get
    // dropped. The render target persists, so one draw per frame is enough.
    let last_frame = std::sync::atomic::AtomicI32::new(-1);
    crate::on_render(move |_camera| {
        match crate::time::frame_count() {
            Ok(frame) if last_frame.swap(frame, std::sync::atomic::Ordering::Relaxed) == frame => {
                return
            }
            _ => {}
        }
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
    /// The off-screen target egui draws into, plus its pixel size (the screen size).
    rt: Option<(GcHandle, [i32; 2])>,
    /// The `RawImage` on the overlay canvas that displays `rt` on top of the game.
    raw_image: Option<GcHandle>,
}

/// Builds the egui material: a textured, vertex-coloured, alpha-blended shader.
/// `HideAndDontSave` keeps Unity from destroying it (and throwing NRE on later
/// use) when a scene loads, which `DontDestroyOnLoad` does not do for assets.
fn make_material() -> Result<Material> {
    let (name, shader) = match Shader::find("Sprites/Default")? {
        Some(shader) => ("Sprites/Default", shader),
        None => (
            "UI/Default",
            Shader::find("UI/Default")?.ok_or_else(|| {
                hao_il2cpp_runtime::Error::NotFound("a sprite/UI shader for egui".into())
            })?,
        ),
    };
    tracing::debug!(shader = name, "egui material shader");
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
            rt: None,
            raw_image: None,
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

    /// (Re)creates the render target and the overlay canvas that shows it. The
    /// target is remade when the screen size changes; the canvas is a
    /// `DontDestroyOnLoad` object, but is rebuilt if something ever destroys it.
    fn ensure_overlay(&mut self, w: i32, h: i32) -> Result<()> {
        let rt_ok = match &self.rt {
            Some((handle, size)) => *size == [w, h] && RenderTexture(handle.target()).is_alive()?,
            None => false,
        };
        if !rt_ok {
            let rt = RenderTexture::new(w, h, 0)?;
            rt.set_hide_flags(crate::HideFlags::HIDE_AND_DONT_SAVE)?;
            self.rt = Some((GcHandle::new(crate::il2cpp()?, rt.object()), [w, h]));
            if let Some(image) = &self.raw_image {
                RawImage(image.target()).set_texture(rt.object())?;
            }
        }

        let needs_overlay = match &self.raw_image {
            Some(image) => !RawImage(image.target()).is_alive()?,
            None => true,
        };
        if needs_overlay {
            let go = GameObject::new("hao-egui-overlay")?;
            go.dont_destroy_on_load()?;
            let canvas = go.add_component::<Canvas>()?;
            canvas.set_render_mode(RenderMode::SCREEN_SPACE_OVERLAY)?;
            // Above the game's own overlay canvases (sorting order is a short).
            canvas.set_sorting_order(i16::MAX as i32)?;
            let image = go.add_component::<RawImage>()?;
            // Stretch to fill the canvas (the whole screen).
            if let Some(rect) = go.rect_transform()? {
                rect.set_anchors(Vector2::ZERO, Vector2::ONE, Vector2::new(0.5, 0.5))?;
                rect.set_anchored_position(Vector2::ZERO)?;
                rect.set_size(Vector2::ZERO)?;
            }
            if let Some((rt, _)) = &self.rt {
                image.set_texture(RenderTexture(rt.target()).object())?;
            }
            self.raw_image = Some(GcHandle::new(crate::il2cpp()?, image.object()));
        }
        Ok(())
    }

    fn frame(&mut self, ui: &(impl Fn(&egui::Context) + ?Sized)) -> Result<()> {
        self.ensure_alive()?;
        let (sw, sh) = screen::size()?;
        let (iw, ih) = (sw.max(1), sh.max(1));
        let (w, h) = (iw as f32, ih as f32);
        self.ensure_overlay(iw, ih)?;

        let input = self.raw_input(w, h)?;
        let output = self.ctx.run(input, |ctx| ui(ctx));
        for (id, delta) in output.textures_delta.set {
            // Only the egui font atlas (managed texture 0) is handled here.
            if matches!(id, egui::TextureId::Managed(_)) {
                self.update_texture(&delta)?;
            }
        }
        let primitives = self.ctx.tessellate(output.shapes, 1.0);
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
            // Point sampling: egui's solid fills sample the atlas's white texel, and
            // bilinear filtering blends it with transparent neighbours, which capped
            // every fill's alpha below 1 and made the whole UI look see-through.
            texture.set_filter_mode(crate::FilterMode::POINT)?;
            let handle = GcHandle::new(crate::il2cpp()?, texture.object());
            self.texture = Some((handle, size));
        }
        Ok(())
    }

    fn render(&self, w: f32, h: f32, primitives: &[egui::ClippedPrimitive]) -> Result<()> {
        let Some((texture, _)) = &self.texture else {
            return Ok(());
        };
        let Some((rt, _)) = &self.rt else {
            return Ok(());
        };
        let rt = RenderTexture(rt.target());
        let material = self.material();
        material.set_main_texture(Texture2D(texture.target()))?;

        // Draw into our render target, then restore whatever the camera was using
        // even if a step fails, so the game's own rendering is left untouched.
        let previous = RenderTexture::active()?;
        RenderTexture::set_active(Some(rt))?;
        let result = (|| {
            // Start transparent so last frame's UI doesn't linger.
            gl::clear(false, Color::CLEAR)?;
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
                    // egui's Color32 is gamma (sRGB); Unity's linear colour space
                    // wants linear values from GL.Color, so convert (otherwise the
                    // dark UI is brightened into grey). Both are premultiplied.
                    let c = egui::Rgba::from(v.color);
                    gl::color(Color::rgba(c.r(), c.g(), c.b(), c.a()))?;
                    gl::tex_coord(v.uv.x, v.uv.y)?;
                    gl::vertex(v.pos.x, v.pos.y)?;
                }
                gl::end()?;
            }
            gl::pop_matrix()
        })();
        RenderTexture::set_active(previous)?;
        result
    }
}
