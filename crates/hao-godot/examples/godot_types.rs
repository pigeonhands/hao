//! Compile-time proof that `hao-runtime-macros` can generate Rust bindings from
//! a Godot game's `GodotSharp.dll` — the typed structs a mod uses to call,
//! create and query Godot objects through the attached runtime.
//!
//! The assembly path comes from an environment variable, so it isn't hardcoded:
//!
//! ```sh
//! HAO_GODOTSHARP="/path/to/GodotSharp.dll" \
//!   cargo build -p hao-godot --example godot_types
//! ```
//!
//! If this compiles, hao's reader parsed GodotSharp and the macro generated
//! usable bindings. A real mod would call these on Godot's main thread (via the
//! lifecycle hook) using a `Runtime` from `ClrHost::attach`.

#[cfg(hao_godotsharp)]
mod godot {
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

fn main() {}
