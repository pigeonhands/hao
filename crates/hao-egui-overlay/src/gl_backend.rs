//! OpenGL overlay: hooks `glXSwapBuffers` to inject egui rendering via
//! `egui_glow` before the frame is presented.

use std::sync::{Arc, Mutex};

use crate::input;

type Display = libc::c_void;
type GLXDrawable = libc::c_ulong;
type GlXSwapBuffersFn = unsafe extern "C" fn(*mut Display, GLXDrawable);

static REAL_SWAP: Mutex<Option<GlXSwapBuffersFn>> = Mutex::new(None);

struct GlState {
    painter: egui_glow::Painter,
    #[allow(dead_code)] // kept alive for the painter
    gl: Arc<glow::Context>,
    ctx: egui::Context,
    pointer_down: [bool; 2],
    keys_down: [bool; input::KEY_COUNT],
}

static STATE: Mutex<Option<GlState>> = Mutex::new(None);

pub fn install() {
    tracing::info!("hao-egui-overlay: OpenGL hooks ready (via symbol export)");
}

fn get_real_swap() -> GlXSwapBuffersFn {
    let mut guard = REAL_SWAP.lock().unwrap();
    if let Some(f) = *guard {
        return f;
    }
    let f = unsafe {
        let sym = libc::dlsym(libc::RTLD_NEXT, b"glXSwapBuffers\0".as_ptr() as _);
        if sym.is_null() {
            panic!("hao-egui-overlay: could not find real glXSwapBuffers");
        }
        std::mem::transmute::<*mut libc::c_void, GlXSwapBuffersFn>(sym)
    };
    *guard = Some(f);
    f
}

fn ensure_state() {
    let mut guard = STATE.lock().unwrap();
    if guard.is_some() {
        return;
    }
    tracing::info!("hao-egui-overlay: OpenGL backend activated");
    let gl = unsafe {
        glow::Context::from_loader_function(|name| {
            let c_name = std::ffi::CString::new(name).unwrap();
            let mut sym = libc::dlsym(libc::RTLD_DEFAULT, c_name.as_ptr());
            if sym.is_null() {
                sym = libc::dlsym(libc::RTLD_NEXT, c_name.as_ptr());
            }
            sym as *const _
        })
    };
    let gl = Arc::new(gl);
    let painter = egui_glow::Painter::new(Arc::clone(&gl), "", None, false)
        .expect("hao-egui-overlay: failed to create egui_glow::Painter");
    *guard = Some(GlState {
        painter,
        gl,
        ctx: egui::Context::default(),
        pointer_down: [false; 2],
        keys_down: [false; input::KEY_COUNT],
    });
}

unsafe fn frame_gl() {
    let draw = match crate::get_draw() {
        Some(d) => d,
        None => return,
    };

    ensure_state();
    let mut guard = STATE.lock().unwrap();
    let state = guard.as_mut().unwrap();

    // Query viewport for screen size
    let mut viewport = [0i32; 4];
    let gl_get_integerv: unsafe extern "C" fn(u32, *mut i32) = std::mem::transmute(
        libc::dlsym(libc::RTLD_DEFAULT, b"glGetIntegerv\0".as_ptr() as _),
    );
    gl_get_integerv(0x0BA2 /* GL_VIEWPORT */, viewport.as_mut_ptr());
    let (w, h) = (viewport[2].max(1) as u32, viewport[3].max(1) as u32);

    let scale = crate::ui_scale();
    let inv = 1.0 / scale;
    let (pw, ph) = (w as f32 * inv, h as f32 * inv);

    state.ctx.set_pixels_per_point(scale);
    let raw_input =
        input::gather(pw, ph, inv, &mut state.pointer_down, &mut state.keys_down);
    let mut output = state.ctx.run_pass(raw_input, |ctx| draw(ctx));
    let primitives = state.ctx.tessellate(output.shapes, scale);
    state.painter.paint_and_update_textures(
        [w, h],
        scale,
        &primitives,
        &mut output.textures_delta,
    );
}

/// LD_PRELOAD hook: exported as `glXSwapBuffers` so the game's GL calls
/// come here. We render egui, then forward to the real swap.
///
/// # Safety
/// Called by the OpenGL loader; `dpy` and `drawable` must be valid.
#[no_mangle]
pub unsafe extern "C" fn glXSwapBuffers(dpy: *mut Display, drawable: GLXDrawable) {
    frame_gl();
    let real = get_real_swap();
    real(dpy, drawable);
}
