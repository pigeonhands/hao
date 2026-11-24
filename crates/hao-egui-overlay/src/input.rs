//! X11 input for the overlay: mouse position/buttons and keyboard state via
//! raw Xlib calls loaded through dlsym.

use std::sync::OnceLock;

pub const KEY_COUNT: usize = KEYS.len();

pub(crate) fn gather(
    pw: f32,
    ph: f32,
    inv: f32,
    pointer_down: &mut [bool; 2],
    keys_down: &mut [bool; KEY_COUNT],
    last_pos: &mut Option<egui::Pos2>,
) -> egui::RawInput {
    let mut events = Vec::new();

    let xlib = Xlib::get();
    let (mx, my, buttons) = xlib.map(|x| x.query_pointer()).unwrap_or((0, 0, 0));
    // X11 coords are top-left origin in pixels. Scale to egui point space.
    let pos = egui::pos2(mx as f32 * inv, my as f32 * inv);
    // Only emit PointerMoved when the position actually changes. egui resets its
    // "last movement time" on EVERY PointerMoved event (even a same-position
    // one), so a constant per-frame stream keeps the pointer perpetually
    // "moving" and suppresses hover tooltips (which require the pointer to be
    // still for a delay).
    if *last_pos != Some(pos) {
        events.push(egui::Event::PointerMoved(pos));
        *last_pos = Some(pos);
    }

    let keymap = xlib.map(|x| x.query_keymap()).unwrap_or([0u8; 32]);

    let shift = keybit(&keymap, KEY_LSHIFT) || keybit(&keymap, KEY_RSHIFT);
    let ctrl = keybit(&keymap, KEY_LCTRL) || keybit(&keymap, KEY_RCTRL);
    let alt = keybit(&keymap, KEY_LALT) || keybit(&keymap, KEY_RALT);
    let modifiers = egui::Modifiers {
        alt,
        ctrl,
        shift,
        mac_cmd: false,
        command: ctrl,
    };

    // Mouse buttons: X11 button mask bit 8 = Button1, bit 9 = Button3
    for (i, (mask, egui_button)) in [
        (1 << 8, egui::PointerButton::Primary),
        (1 << 10, egui::PointerButton::Secondary),
    ]
    .into_iter()
    .enumerate()
    {
        let down = buttons & mask != 0;
        if down != pointer_down[i] {
            pointer_down[i] = down;
            events.push(egui::Event::PointerButton {
                pos,
                button: egui_button,
                pressed: down,
                modifiers,
            });
        }
    }

    // Scroll: Button4 (up) / Button5 (down)
    if buttons & (1 << 11) != 0 {
        events.push(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, 1.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
            modifiers,
        });
    }
    if buttons & (1 << 12) != 0 {
        events.push(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, -1.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
            modifiers,
        });
    }

    // Keyboard
    for (i, &(keycode, key)) in KEYS.iter().enumerate() {
        let down = keybit(&keymap, keycode);
        if down != keys_down[i] {
            keys_down[i] = down;
            events.push(egui::Event::Key {
                key,
                physical_key: None,
                pressed: down,
                repeat: false,
                modifiers,
            });
            if down {
                if let Some(c) = key_to_char(key, shift) {
                    events.push(egui::Event::Text(c.to_string()));
                }
            }
        }
    }

    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(pw, ph),
        )),
        events,
        ..Default::default()
    }
}

fn key_to_char(key: egui::Key, shift: bool) -> Option<char> {
    let c = match key {
        egui::Key::A => 'a',
        egui::Key::B => 'b',
        egui::Key::C => 'c',
        egui::Key::D => 'd',
        egui::Key::E => 'e',
        egui::Key::F => 'f',
        egui::Key::G => 'g',
        egui::Key::H => 'h',
        egui::Key::I => 'i',
        egui::Key::J => 'j',
        egui::Key::K => 'k',
        egui::Key::L => 'l',
        egui::Key::M => 'm',
        egui::Key::N => 'n',
        egui::Key::O => 'o',
        egui::Key::P => 'p',
        egui::Key::Q => 'q',
        egui::Key::R => 'r',
        egui::Key::S => 's',
        egui::Key::T => 't',
        egui::Key::U => 'u',
        egui::Key::V => 'v',
        egui::Key::W => 'w',
        egui::Key::X => 'x',
        egui::Key::Y => 'y',
        egui::Key::Z => 'z',
        egui::Key::Num0 => '0',
        egui::Key::Num1 => '1',
        egui::Key::Num2 => '2',
        egui::Key::Num3 => '3',
        egui::Key::Num4 => '4',
        egui::Key::Num5 => '5',
        egui::Key::Num6 => '6',
        egui::Key::Num7 => '7',
        egui::Key::Num8 => '8',
        egui::Key::Num9 => '9',
        egui::Key::Space => return Some(' '),
        egui::Key::Minus => return Some(if shift { '_' } else { '-' }),
        egui::Key::Period => return Some(if shift { '>' } else { '.' }),
        egui::Key::Comma => return Some(if shift { '<' } else { ',' }),
        _ => return None,
    };
    Some(if shift { c.to_ascii_uppercase() } else { c })
}

fn keybit(keymap: &[u8; 32], keycode: u8) -> bool {
    keymap[keycode as usize / 8] & (1 << (keycode % 8)) != 0
}

// X11 keycodes (evdev)
const KEY_LSHIFT: u8 = 50;
const KEY_RSHIFT: u8 = 62;
const KEY_LCTRL: u8 = 37;
const KEY_RCTRL: u8 = 105;
const KEY_LALT: u8 = 64;
const KEY_RALT: u8 = 108;

const KEYS: &[(u8, egui::Key)] = &[
    // Letters
    (38, egui::Key::A), (56, egui::Key::B), (54, egui::Key::C),
    (40, egui::Key::D), (26, egui::Key::E), (41, egui::Key::F),
    (42, egui::Key::G), (43, egui::Key::H), (31, egui::Key::I),
    (44, egui::Key::J), (45, egui::Key::K), (46, egui::Key::L),
    (58, egui::Key::M), (57, egui::Key::N), (32, egui::Key::O),
    (33, egui::Key::P), (24, egui::Key::Q), (27, egui::Key::R),
    (39, egui::Key::S), (28, egui::Key::T), (30, egui::Key::U),
    (55, egui::Key::V), (25, egui::Key::W), (53, egui::Key::X),
    (29, egui::Key::Y), (52, egui::Key::Z),
    // Numbers
    (19, egui::Key::Num0), (10, egui::Key::Num1), (11, egui::Key::Num2),
    (12, egui::Key::Num3), (13, egui::Key::Num4), (14, egui::Key::Num5),
    (15, egui::Key::Num6), (16, egui::Key::Num7), (17, egui::Key::Num8),
    (18, egui::Key::Num9),
    // Navigation
    (22, egui::Key::Backspace), (119, egui::Key::Delete),
    (36, egui::Key::Enter), (23, egui::Key::Tab), (9, egui::Key::Escape),
    (113, egui::Key::ArrowLeft), (114, egui::Key::ArrowRight),
    (111, egui::Key::ArrowUp), (116, egui::Key::ArrowDown),
    (110, egui::Key::Home), (115, egui::Key::End),
    (112, egui::Key::PageUp), (117, egui::Key::PageDown),
    (65, egui::Key::Space),
    // Punctuation
    (20, egui::Key::Minus), (60, egui::Key::Period), (59, egui::Key::Comma),
    // F-keys (for toggle hotkey)
    (67, egui::Key::F1), (68, egui::Key::F2), (69, egui::Key::F3),
    (70, egui::Key::F4), (71, egui::Key::F5),
];

// --- Xlib FFI loaded at runtime via dlsym ---

type Display = libc::c_void;
type Window = libc::c_ulong;

type XOpenDisplayFn = unsafe extern "C" fn(*const libc::c_char) -> *mut Display;
type XDefaultRootWindowFn = unsafe extern "C" fn(*mut Display) -> Window;
type XQueryPointerFn = unsafe extern "C" fn(
    *mut Display, Window,
    *mut Window, *mut Window,
    *mut libc::c_int, *mut libc::c_int,
    *mut libc::c_int, *mut libc::c_int,
    *mut libc::c_uint,
) -> libc::c_int;
type XQueryKeymapFn = unsafe extern "C" fn(*mut Display, *mut [libc::c_char; 32]) -> libc::c_int;

struct Xlib {
    display: *mut Display,
    root: Window,
    query_pointer_fn: XQueryPointerFn,
    query_keymap_fn: XQueryKeymapFn,
}

unsafe impl Send for Xlib {}
unsafe impl Sync for Xlib {}

static XLIB: OnceLock<Option<Xlib>> = OnceLock::new();

impl Xlib {
    fn get() -> Option<&'static Xlib> {
        XLIB.get_or_init(|| {
            unsafe {
                let lib = libc::dlopen(b"libX11.so.6\0".as_ptr() as _, libc::RTLD_LAZY);
                if lib.is_null() {
                    let lib = libc::dlopen(b"libX11.so\0".as_ptr() as _, libc::RTLD_LAZY);
                    if lib.is_null() {
                        tracing::warn!("hao-egui-overlay: failed to load libX11");
                        return None;
                    }
                    return Self::init_from(lib);
                }
                Self::init_from(lib)
            }
        })
        .as_ref()
    }

    unsafe fn init_from(lib: *mut libc::c_void) -> Option<Xlib> {
        let open: XOpenDisplayFn =
            std::mem::transmute(libc::dlsym(lib, b"XOpenDisplay\0".as_ptr() as _));
        let root_fn: XDefaultRootWindowFn =
            std::mem::transmute(libc::dlsym(lib, b"XDefaultRootWindow\0".as_ptr() as _));
        let qp: XQueryPointerFn =
            std::mem::transmute(libc::dlsym(lib, b"XQueryPointer\0".as_ptr() as _));
        let qk: XQueryKeymapFn =
            std::mem::transmute(libc::dlsym(lib, b"XQueryKeymap\0".as_ptr() as _));

        let display = open(std::ptr::null());
        if display.is_null() {
            tracing::warn!("hao-egui-overlay: XOpenDisplay(NULL) failed");
            return None;
        }
        let root = root_fn(display);
        tracing::debug!("hao-egui-overlay: X11 input ready");
        Some(Xlib {
            display,
            root,
            query_pointer_fn: qp,
            query_keymap_fn: qk,
        })
    }

    fn query_pointer(&self) -> (i32, i32, u32) {
        unsafe {
            let (mut rx, mut ry, mut wx, mut wy) = (0, 0, 0, 0);
            let mut mask = 0u32;
            let mut child = 0;
            let mut root_ret = 0;
            (self.query_pointer_fn)(
                self.display, self.root,
                &mut root_ret, &mut child,
                &mut rx, &mut ry, &mut wx, &mut wy,
                &mut mask,
            );
            (rx, ry, mask)
        }
    }

    fn query_keymap(&self) -> [u8; 32] {
        unsafe {
            let mut keys = [0i8; 32];
            (self.query_keymap_fn)(self.display, &mut keys);
            std::mem::transmute(keys)
        }
    }
}
