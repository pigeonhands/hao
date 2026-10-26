//! Immediate-mode drawing with `UnityEngine.GL`, for use inside an
//! [`on_render`](crate::on_render) callback (it's only valid during rendering).
//!
//! ```no_run
//! use hao_il2cpp_unity::{gl, on_render, Color, Vector3};
//!
//! on_render(|_camera| {
//!     gl::load_pixel_matrix(0.0, 1920.0, 0.0, 1080.0);
//!     gl::draw(gl::Mode::Triangles, || {
//!         gl::color(Color::rgb(1.0, 0.0, 0.0));
//!         gl::vertex(16.0, 16.0);
//!         gl::vertex(216.0, 16.0);
//!         gl::vertex(16.0, 116.0);
//!     });
//! })?;
//! # Ok::<(), hao_il2cpp_unity::runtime::Error>(())
//! ```
//!
//! An egui (or other GPU UI) backend would translate its tessellated output into
//! these calls, or hook the native graphics present for a full GPU surface — that
//! part is renderer-specific and up to the mod.

use hao_il2cpp_runtime::Result;

use crate::{
    bindings::{
        GL_BEGIN, GL_COLOR, GL_END, GL_LOAD_ORTHO, GL_LOAD_PIXEL_MATRIX, GL_POP_MATRIX,
        GL_PUSH_MATRIX, GL_TEXCOORD2, GL_VERTEX3,
    },
    Color,
};

/// The primitive topology passed to [`draw`]/[`begin`], matching Unity's `GL` modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Lines = 1,
    LineStrip = 2,
    Triangles = 4,
    TriangleStrip = 5,
    Quads = 7,
}

/// Begins a primitive of `mode`. Pair with [`end`]; prefer [`draw`].
pub fn begin(mode: Mode) -> Result<()> {
    GL_BEGIN.call((mode as i32,))
}

pub fn end() -> Result<()> {
    GL_END.call(())
}

/// Draws a primitive: calls [`begin`], runs `body` (which emits [`color`] and
/// [`vertex`]/[`vertex3`]), then [`end`], even if `body` returns an error.
pub fn draw(mode: Mode, body: impl FnOnce()) -> Result<()> {
    begin(mode)?;
    body();
    end()
}

/// Sets the current vertex colour.
pub fn color(color: Color) -> Result<()> {
    GL_COLOR.call((color,))
}

/// Emits a 3D vertex.
pub fn vertex3(x: f32, y: f32, z: f32) -> Result<()> {
    GL_VERTEX3.call((x, y, z))
}

/// Emits a 2D vertex at z = 0.
pub fn vertex(x: f32, y: f32) -> Result<()> {
    vertex3(x, y, 0.0)
}

/// Sets the texture coordinate for the next vertex.
pub fn tex_coord(u: f32, v: f32) -> Result<()> {
    GL_TEXCOORD2.call((u, v))
}

/// Sets up a pixel-space projection: `(left, bottom)` to `(right, top)` in pixels.
pub fn load_pixel_matrix(left: f32, right: f32, bottom: f32, top: f32) -> Result<()> {
    GL_LOAD_PIXEL_MATRIX.call((left, right, bottom, top))
}

/// Sets up an orthographic projection with coordinates from (0, 0) to (1, 1).
pub fn load_ortho() -> Result<()> {
    GL_LOAD_ORTHO.call(())
}

/// Saves the projection/model-view matrix; restore with [`pop_matrix`].
pub fn push_matrix() -> Result<()> {
    GL_PUSH_MATRIX.call(())
}

pub fn pop_matrix() -> Result<()> {
    GL_POP_MATRIX.call(())
}
