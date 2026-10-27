//! Calls the game's own GodotSharp API from Rust through macro-generated
//! bindings — the payoff of [`hao_runtime_macros::bind!`] against a real game
//! assembly.
//!
//! `bind!` reads the game's `GodotSharp.dll` at compile time and generates typed
//! Rust wrappers; at runtime the attached [`Runtime`](hao_godot::Runtime) loads
//! the generated thunks and dispatches into the engine. Build with the assembly
//! path (so the macro can read it) and preload the result:
//!
//! ```sh
//! HAO_GODOTSHARP=".../GodotSharp.dll" \
//!   cargo build --release -p hao-godot --example godot_types
//! # Launch options (run from a terminal to see logs):
//! #   LD_PRELOAD=.../target/release/examples/libgodot_types.so %command%
//! ```
//!
//! Without `HAO_GODOTSHARP` the example still compiles (it just does nothing),
//! so a plain workspace build is unaffected.
//!
//! These calls currently run on the attach background thread; most Godot APIs
//! are main-thread-only, so this is limited to the thread-agnostic ones until
//! the main-thread lifecycle hook lands.

#[cfg(hao_godotsharp)]
mod bindings {
    hao_runtime_macros::bind! {
        assembly_env = "HAO_GODOTSHARP",
        types = [
            "Godot.Engine",
            "Godot.OS",
            "Godot.Time",
            "Godot.GodotObject",
            "Godot.Node",
            "Godot.SceneTree",
            "Godot.Label",
        ],
    }
}

#[cfg(hao_godotsharp)]
mod demo {
    use super::bindings::godot::{Engine, Time, OS};

    // Slay the Spire 2 runs on .NET 9.
    const DOTNET_MAJOR: u16 = 9;

    #[used]
    #[link_section = ".init_array"]
    static LOAD: extern "C" fn() = load;

    extern "C" fn load() {
        hao_godot::init_logging();
        tracing::info!("godot_types mod loaded; waiting for CoreCLR");
        hao_godot::on_clr_init(|host| {
            if let Err(e) = run(host) {
                tracing::error!("godot_types failed: {e}");
            }
        });
    }

    fn run(host: hao_godot::ClrHost) -> hao_runtime::Result<()> {
        // Let the game settle before touching managed code (see call.rs).
        std::thread::sleep(std::time::Duration::from_secs(10));
        let runtime = host.attach(DOTNET_MAJOR)?;

        // Static GodotSharp calls through the generated bindings.
        tracing::info!("OS.GetName() = {:?}", OS::get_name(&runtime)?);
        tracing::info!("Time.GetTicksMsec() = {}", Time::get_ticks_msec(&runtime)?);
        tracing::info!(
            "Engine.GetProcessFrames() = {}",
            Engine::get_process_frames(&runtime)?
        );
        tracing::info!("called the game's GodotSharp API from Rust via generated bindings");
        Ok(())
    }
}
