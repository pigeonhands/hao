//! An example mod for Megabonk's Linux build (and most Unity games): draws a
//! crosshair and a translucent box over the game with `GL` immediate mode,
//! from the render-pipeline hook. Toggle it with F1.
//!
//! ```sh
//! cargo build --release -p hao-il2cpp-unity --example render_overlay
//! # Steam > Megabonk > Properties > Launch options:
//! LD_PRELOAD=/path/to/target/release/examples/librender_overlay.so %command%
//! ```

use std::sync::{Mutex, OnceLock};

use hao_il2cpp_unity::{gl, input, on_render, screen, Camera, Color, KeyCode, Material};

#[used]
#[link_section = ".init_array"]
static LOAD: extern "C" fn() = load;

/// A colored material is needed so `GL` has a pass to draw with. Built on the
/// first render (on the render thread) and reused.
static MATERIAL: OnceLock<Material> = OnceLock::new();
static VISIBLE: Mutex<bool> = Mutex::new(true);

fn material() -> hao_il2cpp_unity::runtime::Result<Material> {
    if let Some(m) = MATERIAL.get() {
        return Ok(*m);
    }
    // The built-in unlit vertex-colour shader Unity ships for GL drawing.
    let shader = hao_il2cpp_unity::Shader::find("Hidden/Internal-Colored")?.ok_or_else(|| {
        hao_il2cpp_unity::runtime::Error::NotFound("Hidden/Internal-Colored shader".into())
    })?;
    Ok(*MATERIAL.get_or_init(|| Material::new(shader).expect("create material")))
}

/// Draws a filled rectangle in pixel coordinates.
fn rect(x: f32, y: f32, w: f32, h: f32, color: Color) -> hao_il2cpp_unity::runtime::Result<()> {
    gl::draw(gl::Mode::Quads, || {
        let _ = gl::color(color);
        let _ = gl::vertex(x, y);
        let _ = gl::vertex(x + w, y);
        let _ = gl::vertex(x + w, y + h);
        let _ = gl::vertex(x, y + h);
    })
}

fn draw() -> hao_il2cpp_unity::runtime::Result<()> {
    if !*VISIBLE.lock().unwrap_or_else(|e| e.into_inner()) {
        return Ok(());
    }
    let (w, h) = screen::size()?;
    let (w, h) = (w as f32, h as f32);

    let material = material()?;
    material.set_pass(0)?;
    gl::push_matrix()?;
    gl::load_pixel_matrix(0.0, w, 0.0, h)?;

    // A crosshair at the centre.
    let (cx, cy, s) = (w / 2.0, h / 2.0, 12.0);
    rect(cx - s, cy - 1.0, 2.0 * s, 2.0, Color::WHITE)?;
    rect(cx - 1.0, cy - s, 2.0, 2.0 * s, Color::WHITE)?;

    // A translucent panel in the corner.
    rect(16.0, h - 96.0, 240.0, 80.0, Color::BLACK.with_alpha(0.5))?;
    rect(16.0, h - 20.0, 240.0, 4.0, Color::YELLOW)?;

    gl::pop_matrix()
}

extern "C" fn load() {
    init_logging();
    hao_il2cpp_unity::on_init(|_| {
        on_render(|_camera: Camera| {
            if input::key_down(KeyCode::F1).unwrap_or(false) {
                let mut v = VISIBLE.lock().unwrap_or_else(|e| e.into_inner());
                *v = !*v;
            }
            if let Err(e) = draw() {
                tracing::error!("render_overlay: {}", e);
            }
        })?;
        tracing::info!("render overlay running (F1 to toggle)");
        Ok(())
    });
}

fn init_logging() {
    let filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}
