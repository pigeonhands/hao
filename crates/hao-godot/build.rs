//! Enables the `godot_types` example only when `HAO_GODOTSHARP` points at a
//! game's `GodotSharp.dll`, so a plain build (without it) still compiles.

fn main() {
    println!("cargo::rustc-check-cfg=cfg(hao_godotsharp)");
    println!("cargo::rerun-if-env-changed=HAO_GODOTSHARP");
    if std::env::var_os("HAO_GODOTSHARP").is_some() {
        println!("cargo::rustc-cfg=hao_godotsharp");
    }
}
