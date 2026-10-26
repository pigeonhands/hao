//! Text drawn over the game.

use hao_il2cpp_runtime::{GcHandle, Result};

use crate::{il2cpp, Canvas, Color, ComponentType, GameObject, RenderMode, TextMeshPro, Vector2};

/// A line of TextMeshPro text on its own canvas, drawn over the game's UI and
/// kept across scene loads. Positioned from the top left of the screen.
///
/// Create it on Unity's main thread, e.g. in [`on_scene_loaded`](crate::on_scene_loaded).
pub struct TextOverlay {
    root: GcHandle,
    text: GcHandle,
}

impl TextOverlay {
    pub fn new(name: &str) -> Result<Self> {
        let il2cpp = il2cpp()?;
        let root = GameObject::new(name)?;
        root.dont_destroy_on_load()?;
        let canvas = root.add_component::<Canvas>()?;
        canvas.set_render_mode(RenderMode::SCREEN_SPACE_OVERLAY)?;
        canvas.set_sorting_order(i16::MAX as i32)?;

        let label = GameObject::new(&format!("{} text", name))?;
        let text = label.add_component::<TextMeshPro>()?;
        let rect = label
            .rect_transform()?
            .expect("UI components use a RectTransform");
        rect.as_transform().set_parent(root.transform()?, false)?;
        let top_left = Vector2::new(0.0, 1.0);
        rect.set_anchors(top_left, top_left, top_left)?;
        rect.set_size(Vector2::new(1200.0, 400.0))?;

        let overlay = Self {
            root: GcHandle::new(il2cpp, root.0),
            text: GcHandle::new(il2cpp, text.0),
        };
        overlay.set_position(16.0, 16.0)?;
        overlay.set_font_size(28.0)?;
        Ok(overlay)
    }

    pub fn game_object(&self) -> GameObject {
        GameObject(self.root.target())
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
