//! `LD_PRELOAD` proof for the CoreCLR attach: capture the Godot game's running
//! CoreCLR host handle and log it. If this works, we can create delegates into
//! the game's managed code (GodotSharp) from Rust.
//!
//! ```sh
//! cargo build --release -p hao-godot --example capture
//! # Steam > game > Properties > Launch options (run from a terminal to see logs):
//! LD_PRELOAD=/path/to/target/release/examples/libcapture.so %command%
//! ```

#[used]
#[link_section = ".init_array"]
static LOAD: extern "C" fn() = load;

extern "C" fn load() {
    hao_godot::init_logging();
    tracing::info!("capture mod loaded (LD_PRELOAD); waiting for CoreCLR");
    hao_godot::on_clr_init(|host| {
        tracing::info!(
            "mod callback ran: CoreCLR host captured (handle={:?}, domain={}) — attach works",
            host.handle,
            host.domain
        );
    });
}
