//! An example mod for Megabonk's Linux build: doubles all healing.
//!
//! Build it, then set the game's Steam launch options to load it:
//!
//! ```sh
//! cargo build --release -p hao-il2cpp-runtime --example heal_mod
//! # Steam > Megabonk > Properties > Launch options:
//! LD_PRELOAD=/path/to/target/release/examples/libheal_mod.so %command%
//! ```
//!
//! Method and field names come from the game's metadata, e.g. with
//! `cargo run -p hao-il2cpp --example game_info -- <game folder> <type>`.

use std::sync::OnceLock;

use hao_il2cpp_runtime::{original, read_field, MethodInfo, Object};

static HP_OFFSET: OnceLock<usize> = OnceLock::new();

/// `int PlayerHealth.Heal(float amount, bool allowOverheal)`: the instance comes
/// first and Il2Cpp's `MethodInfo*` last.
extern "C" fn heal(this: Object, amount: f32, allow_overheal: bool, method: MethodInfo) -> i32 {
    let heal = original!(extern "C" fn(Object, f32, bool, MethodInfo) -> i32);
    let healed = heal(this, amount * 2.0, allow_overheal, method);
    if let Some(offset) = HP_OFFSET.get() {
        let hp: i32 = unsafe { read_field(this, *offset) };
        eprintln!("[heal_mod] healed {} instead of {}, hp is now {}", healed, amount, hp);
    }
    healed
}

/// Runs when the library is loaded into the game.
#[ctor::ctor(unsafe)]
fn load() {
    hao_il2cpp_runtime::on_init(|il2cpp| {
        let health = il2cpp.class("Assembly-CSharp", "Assets.Scripts.Inventory__Items__Pickups", "PlayerHealth")?;
        let _ = HP_OFFSET.set(health.field_offset("hp")?);
        unsafe { health.method("Heal", 2)?.hook(heal as *mut u8)? };
        eprintln!("[heal_mod] hooked PlayerHealth.Heal");
        Ok(())
    });
}
