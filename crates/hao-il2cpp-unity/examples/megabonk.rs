//! A mod menu for Megabonk (Unity Il2Cpp): an egui overlay with godmode and
//! item spawning, drawn over the game. F1 toggles the menu.
//!
//! ```sh
//! cargo build --release -p hao-il2cpp-unity --features egui --example megabonk
//! # Steam > Megabonk > Properties > Launch options:
//! LD_PRELOAD=/path/to/target/release/examples/libmegabonk.so %command%
//! ```
//!
//! The class, method and enum names/values come from the game's metadata, found
//! offline with hao-il2cpp, e.g.
//! `cargo run -p hao-il2cpp --example game_info -- <game folder> <type>`.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex, OnceLock,
};

use hao_il2cpp_unity::runtime::{Il2Cpp, Method, Object, Original, Result};
use hao_il2cpp_unity::{egui, input, time, KeyCode};

const ASM: &str = "Assembly-CSharp";
const NS: &str = "Assets.Scripts.Inventory__Items__Pickups";

/// A few items with their `EItem` ids (from `game_info` on `EItem`).
const ITEMS: &[(&str, i32)] = &[
    ("Beer", 134611970),
    ("Spiky Shield", 168297988),
    ("Bonker", 201984006),
    ("Battery", 336728078),
    ("Demon Blade", 437786132),
    ("Golden Shield", 909390384),
];

/// The mod's state and behaviour. Lives in a `static` so the hook and egui
/// callbacks (which must be `'static`) can share it.
struct Mod {
    visible: AtomicBool,
    f1_down: AtomicBool,
    godmode: AtomicBool,
    /// `EItem` ids the menu asked to add, applied on the game's main thread.
    pending: Mutex<Vec<i32>>,
    /// The live `ItemInventory` captured from its `Tick`, and `AddItem(EItem)`.
    inventory: Mutex<Option<Object>>,
    add_item: OnceLock<Method>,
}

static MOD: Mod = Mod::new();

impl Mod {
    const fn new() -> Self {
        Self {
            visible: AtomicBool::new(true),
            f1_down: AtomicBool::new(false),
            godmode: AtomicBool::new(false),
            pending: Mutex::new(Vec::new()),
            inventory: Mutex::new(None),
            add_item: OnceLock::new(),
        }
    }

    /// Installs the game hooks. `&'static self` so the closures can hold it.
    fn install(&'static self, il2cpp: &'static Il2Cpp) -> Result<()> {
        // Godmode: skip `void PlayerHealth.Damage(DamageContainer dc, bool ignoreShield)`.
        let health = il2cpp.class(ASM, NS, "PlayerHealth")?;
        health.method("Damage", 2)?.hook(
            move |this: Object, dc: Object, ignore_shield: bool, original: Original<_, ()>| {
                if !self.godmode.load(Ordering::Relaxed) {
                    original.call((this, dc, ignore_shield));
                }
            },
        )?;

        // `ItemInventory.Tick()` runs every frame on the main thread: use it to
        // grab the live inventory and apply items the menu queued, where il2cpp
        // calls are safe (the egui callback is on the render thread).
        let inventory = il2cpp.class(ASM, &format!("{NS}.Items"), "ItemInventory")?;
        let _ = self.add_item.set(inventory.method("AddItem", 1)?);
        inventory
            .method("Tick", 0)?
            .hook(move |this: Object, original: Original<_, ()>| {
                self.tick(this);
                original.call((this,));
            })?;
        Ok(())
    }

    /// Runs each frame on the game thread: capture the inventory, apply queued items.
    fn tick(&self, inventory: Object) {
        *self.inventory.lock().unwrap_or_else(|e| e.into_inner()) = Some(inventory);
        let queued = std::mem::take(&mut *self.pending.lock().unwrap_or_else(|e| e.into_inner()));
        if let Some(add) = self.add_item.get() {
            for value in queued {
                // EItem is an int-backed enum, so its underlying i32 is ABI
                // compatible; skip the type check, which sees an enum.
                unsafe { add.call_unchecked::<(Object, i32), ()>((inventory, value)) };
            }
        }
    }

    /// Draws the overlay menu (on the render thread; only touches shared state).
    fn menu(&self, ctx: &egui::Context) {
        let f1 = input::key(KeyCode::F1).unwrap_or(false);
        if f1 && !self.f1_down.swap(f1, Ordering::Relaxed) {
            self.visible.fetch_xor(true, Ordering::Relaxed);
        }
        if !self.visible.load(Ordering::Relaxed) {
            return;
        }

        let frame = egui::Frame::window(&ctx.style()).fill(egui::Color32::from_rgb(24, 24, 28));
        egui::Window::new("Megabonk mod")
            .frame(frame)
            .default_pos([24.0, 24.0])
            .show(ctx, |ui| {
                ui.label("F1 to toggle.");
                ui.separator();

                let mut godmode = self.godmode.load(Ordering::Relaxed);
                if ui.checkbox(&mut godmode, "Godmode (no damage)").changed() {
                    self.godmode.store(godmode, Ordering::Relaxed);
                }

                ui.separator();
                let ready = self.inventory.lock().unwrap_or_else(|e| e.into_inner()).is_some();
                ui.add_enabled_ui(ready, |ui| {
                    ui.label(if ready {
                        "Add item:"
                    } else {
                        "Add item: (start a run first)"
                    });
                    ui.horizontal_wrapped(|ui| {
                        for (name, value) in ITEMS {
                            if ui.button(*name).clicked() {
                                self.pending.lock().unwrap_or_else(|e| e.into_inner()).push(*value);
                            }
                        }
                    });
                });

                ui.separator();
                let mut scale = time::scale().unwrap_or(1.0);
                if ui
                    .add(egui::Slider::new(&mut scale, 0.0..=3.0).text("time scale"))
                    .changed()
                {
                    let _ = time::set_scale(scale);
                }
            });
    }
}

#[used]
#[link_section = ".init_array"]
static LOAD: extern "C" fn() = load;

extern "C" fn load() {
    init_logging();
    hao_il2cpp_unity::on_init(|il2cpp| {
        MOD.install(il2cpp)?;
        egui::on_egui(|ctx| MOD.menu(ctx))?;
        tracing::info!("megabonk menu running (F1 to toggle)");
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
