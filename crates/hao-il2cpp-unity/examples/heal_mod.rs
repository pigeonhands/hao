//! An example mod for Megabonk's Linux build: doubles all healing.
//!
//! Build it, then set the game's Steam launch options to load it:
//!
//! ```sh
//! cargo build --release -p hao-il2cpp-unity --example heal_mod
//! # Steam > Megabonk > Properties > Launch options:
//! LD_PRELOAD=/path/to/target/release/examples/libheal_mod.so %command%
//! ```
//!
//! Method and field names come from the game's metadata, e.g. with
//! `cargo run -p hao-il2cpp --example game_info -- <game folder> <type>`.

use hao_il2cpp_unity::runtime::{read_field, Object, Original};

/// Runs `load` when the game loads the library, like a C constructor.
#[used]
#[link_section = ".init_array"]
static LOAD: extern "C" fn() = load;

extern "C" fn load() {
    hao_il2cpp_unity::on_init(|il2cpp| {
        let health = il2cpp.class(
            "Assembly-CSharp",
            "Assets.Scripts.Inventory__Items__Pickups",
            "PlayerHealth",
        )?;
        let hp = health.field_offset("hp")?;
        // `int PlayerHealth.Heal(float amount, bool allowOverheal)`, checked against
        // the closure's types when the hook is installed.
        health.method("Heal", 2)?.hook(
            move |this: Object, amount: f32, overheal: bool, original: Original<_, i32>| {
                let healed = original.call((this, amount * 2.0, overheal));
                let hp: i32 = unsafe { read_field(this, hp) };
                // Shows up in the game's Player.log.
                let _ = hao_il2cpp_unity::log(&format!(
                    "[heal_mod] healed {} instead of {}, hp is now {}",
                    healed, amount, hp
                ));
                healed
            },
        )?;
        eprintln!("[heal_mod] hooked PlayerHealth.Heal");
        Ok(())
    });
}
