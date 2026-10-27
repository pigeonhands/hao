//! Drawing on top of the game: a top-most [`OverlayCanvas`], a render-to-texture
//! [`OverlaySurface`] for `gl` drawing, and a [`TextOverlay`] for a line of text.

use hao_il2cpp_runtime::{GcHandle, Result};

use crate::{
    gl, il2cpp, Canvas, Color, ComponentType, GameObject, HideFlags, RawImage, RenderMode,
    RenderTexture, TextMeshPro, Vector2,
};

/// A screen-space-overlay `Canvas` drawn on top of the game's own UI (the
/// maximum sorting order) and kept across scene loads. Parent UI elements, or a
/// [`RawImage`], under [`game_object`](Self::game_object).
///
/// Create it on Unity's main thread, e.g. in [`on_scene_loaded`](crate::on_scene_loaded).
pub struct OverlayCanvas {
    object: GcHandle,
}

impl OverlayCanvas {
    pub fn new(name: &str) -> Result<Self> {
        let go = GameObject::new(name)?;
        go.dont_destroy_on_load()?;
        let canvas = go.add_component::<Canvas>()?;
        canvas.set_render_mode(RenderMode::SCREEN_SPACE_OVERLAY)?;
        // Above the game's own overlay canvases (the sorting order is a short).
        canvas.set_sorting_order(i16::MAX as i32)?;
        Ok(Self {
            object: GcHandle::new(il2cpp()?, go.0),
        })
    }

    pub fn game_object(&self) -> GameObject {
        GameObject(self.object.target())
    }

    /// Whether the canvas is still alive (`false` once destroyed).
    pub fn is_alive(&self) -> Result<bool> {
        self.game_object().is_alive()
    }
}

/// A full-screen off-screen target shown on top of the game's UI, for drawing an
/// overlay with the [`gl`](crate::gl) module.
///
/// Immediate-mode `gl` drawn straight from [`on_render`](crate::on_render) sits
/// *under* the game's screen-space UI, which the engine composites after the
/// cameras. An `OverlaySurface` instead draws into a `RenderTexture` that a
/// [`RawImage`] on a top-most [`OverlayCanvas`] composites over everything. The
/// target follows the screen size and is rebuilt if a scene load destroys it.
///
/// ```no_run
/// use hao_il2cpp_unity::{gl, on_render, screen, Color, OverlaySurface};
/// use std::sync::Mutex;
///
/// let surface = Mutex::new(OverlaySurface::new("my-overlay"));
/// on_render(move |_camera| {
///     let (w, h) = screen::size().unwrap();
///     let mut surface = surface.lock().unwrap();
///     let _ = surface.draw((w, h), Color::CLEAR, || {
///         gl::load_pixel_matrix(0.0, w as f32, 0.0, h as f32);
///         gl::draw(gl::Mode::Quads, || {
///             gl::color(Color::RED);
///             for (x, y) in [(10.0, 10.0), (110.0, 10.0), (110.0, 60.0), (10.0, 60.0)] {
///                 let _ = gl::vertex(x, y);
///             }
///         })
///     });
/// })?;
/// # Ok::<(), hao_il2cpp_unity::runtime::Error>(())
/// ```
pub struct OverlaySurface {
    name: String,
    canvas: Option<OverlayCanvas>,
    image: Option<GcHandle>,
    target: Option<(GcHandle, [i32; 2])>,
}

impl OverlaySurface {
    /// `name` is the overlay GameObject's name (useful when inspecting the scene).
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            canvas: None,
            image: None,
            target: None,
        }
    }

    /// Binds the render target, clears it to `clear`, runs `body` (which draws
    /// with the [`gl`](crate::gl) module), then restores the previous target —
    /// rebuilding the target and canvas first if the screen resized or a scene
    /// load destroyed them. Call once per frame from [`on_render`](crate::on_render).
    pub fn draw(
        &mut self,
        size: (i32, i32),
        clear: Color,
        body: impl FnOnce() -> Result<()>,
    ) -> Result<()> {
        self.ensure(size.0.max(1), size.1.max(1))?;
        let Some((target, _)) = &self.target else {
            return Ok(());
        };
        let target = RenderTexture(target.target());
        // Restore whatever the camera was drawing to even if a step fails, so the
        // game's own rendering is left untouched.
        let previous = RenderTexture::active()?;
        RenderTexture::set_active(Some(target))?;
        let result = (|| {
            gl::clear(false, clear)?;
            body()
        })();
        RenderTexture::set_active(previous)?;
        result
    }

    /// The render texture, e.g. to bind as a material's texture while drawing.
    /// `None` until the first [`draw`](Self::draw).
    pub fn target(&self) -> Option<RenderTexture> {
        self.target.as_ref().map(|(h, _)| RenderTexture(h.target()))
    }

    /// The overlay canvas, once created (after the first [`draw`](Self::draw)).
    pub fn canvas(&self) -> Option<&OverlayCanvas> {
        self.canvas.as_ref()
    }

    fn ensure(&mut self, w: i32, h: i32) -> Result<()> {
        let target_ok = match &self.target {
            Some((handle, size)) => *size == [w, h] && RenderTexture(handle.target()).is_alive()?,
            None => false,
        };
        if !target_ok {
            let target = RenderTexture::new(w, h, 0)?;
            target.set_hide_flags(HideFlags::HIDE_AND_DONT_SAVE)?;
            self.target = Some((GcHandle::new(il2cpp()?, target.object()), [w, h]));
            if let Some(image) = &self.image {
                RawImage(image.target()).set_texture(target.object())?;
            }
        }

        let needs_canvas = match &self.image {
            Some(image) => !RawImage(image.target()).is_alive()?,
            None => true,
        };
        if needs_canvas {
            let canvas = OverlayCanvas::new(&self.name)?;
            let image = canvas.game_object().add_component::<RawImage>()?;
            // Stretch to fill the canvas (the whole screen).
            if let Some(rect) = canvas.game_object().rect_transform()? {
                rect.set_anchors(Vector2::ZERO, Vector2::ONE, Vector2::new(0.5, 0.5))?;
                rect.set_anchored_position(Vector2::ZERO)?;
                rect.set_size(Vector2::ZERO)?;
            }
            if let Some(target) = self.target() {
                image.set_texture(target.object())?;
            }
            self.image = Some(GcHandle::new(il2cpp()?, image.object()));
            self.canvas = Some(canvas);
        }
        Ok(())
    }
}

/// A line of TextMeshPro text on its own [`OverlayCanvas`], drawn over the game's
/// UI and kept across scene loads. Positioned from the top left of the screen.
///
/// Create it on Unity's main thread, e.g. in [`on_scene_loaded`](crate::on_scene_loaded).
pub struct TextOverlay {
    canvas: OverlayCanvas,
    text: GcHandle,
}

impl TextOverlay {
    pub fn new(name: &str) -> Result<Self> {
        let canvas = OverlayCanvas::new(name)?;

        let label = GameObject::new(&format!("{name} text"))?;
        let text = label.add_component::<TextMeshPro>()?;
        let rect = label
            .rect_transform()?
            .expect("UI components use a RectTransform");
        rect.as_transform()
            .set_parent(canvas.game_object().transform()?, false)?;
        let top_left = Vector2::new(0.0, 1.0);
        rect.set_anchors(top_left, top_left, top_left)?;
        rect.set_size(Vector2::new(1200.0, 400.0))?;

        let overlay = Self {
            canvas,
            text: GcHandle::new(il2cpp()?, text.0),
        };
        overlay.set_position(16.0, 16.0)?;
        overlay.set_font_size(28.0)?;
        Ok(overlay)
    }

    pub fn game_object(&self) -> GameObject {
        self.canvas.game_object()
    }

    pub fn text(&self) -> TextMeshPro {
        TextMeshPro(self.text.target())
    }

    pub fn set_text(&self, text: &str) -> Result<()> {
        self.text().set_text(text)
    }

    /// Pixels from the top left of the screen.
    pub fn set_position(&self, x: f32, y: f32) -> Result<()> {
        let transform = self.text().transform()?;
        crate::RectTransform(transform.0).set_anchored_position(Vector2::new(x, -y))
    }

    pub fn set_font_size(&self, size: f32) -> Result<()> {
        self.text().set_font_size(size)
    }

    pub fn set_color(&self, color: Color) -> Result<()> {
        self.text().set_color(color)
    }

    pub fn set_visible(&self, visible: bool) -> Result<()> {
        self.game_object().set_active(visible)
    }
}
