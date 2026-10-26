//! Experimental: draw an [`egui`] UI over the game from the render hook.
//!
//! **Unverified rendering.** The data flow (input, tessellation, texture upload,
//! GL draw) is complete, but the shader/blend/colour-space details are the kind
//! of thing that needs tuning against a real game. Enable with the `egui`
//! feature.
//!
//! egui is rendered with Unity's immediate-mode [`gl`](crate::gl): its font atlas
//! is uploaded to a [`Texture2D`], bound through a `Material`, and each tessellated
//! triangle is emitted into a [`RenderTexture`]. That texture is shown by a
//! [`RawImage`] on our own screen-space-overlay [`Canvas`] with the maximum sorting
//! order, so the UI draws on top of the game's own overlay canvases (which the
//! engine composites after camera rendering, where a plain `gl` draw would sit
//! underneath). See [`on_egui`].

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
    /// A 1x1 texture used to read a pixel back from the target for debugging.
    readback: Option<GcHandle>,
    /// Counts rendered frames, to throttle the debug pixel log.
    frame_index: u64,
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
    tracing::info!(shader = name, "egui material shader");
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
            readback: None,
            frame_index: 0,
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

        // Step logs: whichever appears last in Player.log before a managed
        // exception is the call that threw. Enable with RUST_LOG=debug.
        tracing::debug!("egui frame: screen size");
        let (sw, sh) = screen::size()?;
        let (iw, ih) = (sw.max(1), sh.max(1));
        let (w, h) = (iw as f32, ih as f32);
        self.ensure_overlay(iw, ih)?;

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

    fn render(&mut self, w: f32, h: f32, primitives: &[egui::ClippedPrimitive]) -> Result<()> {
        let Some((texture, _)) = &self.texture else {
            return Ok(());
        };
        let Some((rt, _)) = &self.rt else {
            return Ok(());
        };
        let rt = RenderTexture(rt.target());
        let material = self.material();
        material.set_main_texture(Texture2D(texture.target()))?;

        self.frame_index += 1;
        // Log the pipeline state once a second or so (at ~60fps), without spamming.
        let trace = self.frame_index % 60 == 0;
        if trace {
            let (meshes, verts, mut max_a) = (
                primitives.len(),
                primitives
                    .iter()
                    .filter_map(|p| match &p.primitive {
                        egui::epaint::Primitive::Mesh(m) => Some(m.vertices.len()),
                        _ => None,
                    })
                    .sum::<usize>(),
                0u8,
            );
            // The largest alpha egui emitted this frame: tells us whether the
            // transparency is in egui's output or added later in the pipeline.
            for p in primitives {
                if let egui::epaint::Primitive::Mesh(m) = &p.primitive {
                    for v in &m.vertices {
                        max_a = max_a.max(v.color.to_array()[3]);
                    }
                }
            }
            tracing::info!(
                meshes,
                verts,
                max_vertex_alpha = max_a,
                screen = ?(w, h),
                "egui draw"
            );
        }

        // egui's content bounding box, so the readback samples where the UI is.
        let content_center = if trace {
            let mut min = (f32::MAX, f32::MAX);
            let mut max = (f32::MIN, f32::MIN);
            for p in primitives {
                if let egui::epaint::Primitive::Mesh(m) = &p.primitive {
                    for v in &m.vertices {
                        min = (min.0.min(v.pos.x), min.1.min(v.pos.y));
                        max = (max.0.max(v.pos.x), max.1.max(v.pos.y));
                    }
                }
            }
            tracing::info!(?min, ?max, "egui content bounds (egui coords)");
            Some((min, max))
        } else {
            None
        };

        // Draw into our render target, then restore whatever the camera was using
        // even if a step fails, so the game's own rendering is left untouched.
        let previous = RenderTexture::active()?;
        RenderTexture::set_active(Some(rt))?;
        if trace {
            // Confirm our target is actually the active render target now.
            let active = RenderTexture::active()?;
            tracing::info!(
                bound = active.map(|a| a.object() == rt.object()).unwrap_or(false),
                "render target set active"
            );
        }
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
            // While the target is still active, read pixels back from egui's
            // content to see what landed in the target: the centre (likely a
            // widget) and a point just below the title bar (plain window fill).
            if let Some((min, max)) = content_center {
                let center = ((min.0 + max.0) / 2.0, (min.1 + max.1) / 2.0);
                let fill = (min.0 + 20.0, min.1 + 40.0);
                for (label, (x, y)) in [("center", center), ("fill", fill)] {
                    if let Err(e) = self.sample_target(label, x, y, h) {
                        tracing::warn!("target readback failed: {}", e);
                    }
                }
            }
            gl::pop_matrix()
        })();
        RenderTexture::set_active(previous)?;
        result
    }

    /// Debug: reads the active target at egui's content centre `(cx, cy)` (in
    /// egui coords) and logs the colour, so we can see what landed in the target.
    fn sample_target(&mut self, label: &str, cx: f32, cy: f32, h: f32) -> Result<()> {
        let readback = match &self.readback {
            Some(handle) => Texture2D(handle.target()),
            None => {
                let tex = Texture2D::new(1, 1, crate::TextureFormat::RGBA32)?;
                tex.set_hide_flags(crate::HideFlags::HIDE_AND_DONT_SAVE)?;
                self.readback = Some(GcHandle::new(crate::il2cpp()?, tex.object()));
                tex
            }
        };
        // The target is bottom-left origin, egui is top-left, so flip Y.
        let py = h - cy;
        readback.read_pixels(crate::Rect::new(cx, py, 1.0, 1.0), 0, 0)?;
        let c = readback.get_pixel(0, 0)?;
        tracing::info!(r = c.r, g = c.g, b = c.b, a = c.a, at = label, "target pixel");
        Ok(())
    }
}
