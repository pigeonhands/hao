//! `LD_PRELOAD` proof for the CoreCLR attach: capture the Godot game's running
//! CoreCLR host handle and log it. If this works, we can create delegates into
//! the game's managed code (GodotSharp) from Rust.
//!
//! ```sh
//! cargo build --release -p hao-godot --example capture
//! # Steam > game > Properties > Launch options:
//! LD_PRELOAD=/path/to/target/release/examples/libcapture.so %command%
//! # Then read ~/.local/share/hao-godot/hao-godot.log
//! ```

#[used]
#[link_section = ".init_array"]
static LOAD: extern "C" fn() = load;

extern "C" fn load() {
    hao_godot::log("capture mod loaded (LD_PRELOAD); waiting for CoreCLR");
    hao_godot::on_clr_init(|host| {
        hao_godot::log(&format!(
            "mod callback ran: CoreCLR host captured (handle={:?}, domain={}) — attach works",
            host.handle, host.domain
        ));
    });
}
