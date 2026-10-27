//! Proof that we can call managed code in the game via the CoreCLR attach:
//! capture the host, attach hao's runtime, load the loader shim, then JIT and
//! call `System.Environment.get_ProcessId` — a corlib static with a primitive
//! return, so it needs no GodotSharp or running engine. Its result should equal
//! the process id, which proves the whole call path works.
//!
//! ```sh
//! cargo build --release -p hao-godot --example call
//! # Launch options: LD_PRELOAD=/path/to/target/release/examples/libcall.so %command%
//! # Then read ~/.local/share/hao-godot/hao-godot.log
//! ```

// Slay the Spire 2 runs on .NET 9.
const DOTNET_MAJOR: u16 = 9;

#[used]
#[link_section = ".init_array"]
static LOAD: extern "C" fn() = load;

extern "C" fn load() {
    hao_godot::log("call mod loaded (LD_PRELOAD); waiting for CoreCLR");
    hao_godot::on_clr_init(|host| {
        if let Err(e) = call(host) {
            hao_godot::log(&format!("call failed: {e}"));
        }
    });
}

fn call(host: hao_godot::ClrHost) -> Result<(), String> {
    // This runs on a background thread right after capture, while the game is
    // still starting. Wait for it to settle before touching managed code — a
    // stopgap until we drive calls from a Godot main-thread lifecycle callback.
    std::thread::sleep(std::time::Duration::from_secs(10));

    hao_godot::log("attaching hao runtime to the game's CoreCLR");
    let runtime = host.attach(DOTNET_MAJOR).map_err(|e| e.to_string())?;

    hao_godot::log("resolving System.Environment.get_ProcessId");
    let pointer = runtime
        .method_pointer("System.Private.CoreLib", "System.Environment", "get_ProcessId")
        .map_err(|e| e.to_string())?;

    let get_process_id: extern "C" fn() -> i32 = unsafe { std::mem::transmute(pointer) };
    let managed_pid = get_process_id() as u32;
    let native_pid = std::process::id();
    hao_godot::log(&format!(
        "managed Environment.ProcessId = {managed_pid}, native pid = {native_pid} — {}",
        if managed_pid == native_pid {
            "MATCH: calling the game's managed code from Rust works"
        } else {
            "mismatch"
        }
    ));
    Ok(())
}
