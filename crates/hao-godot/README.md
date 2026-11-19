# hao-godot

[![crates.io](https://img.shields.io/crates/v/hao-godot.svg)](https://crates.io/crates/hao-godot) [![docs.rs](https://img.shields.io/docsrs/hao-godot)](https://docs.rs/hao-godot) [![license](https://img.shields.io/crates/l/hao-godot.svg)](https://github.com/pigeonhands/hao)

Attach to a Godot (or any CoreCLR / .NET) game from a native mod and call into its managed code.

A mod is a `cdylib` loaded before the game starts its runtime (on Linux with `LD_PRELOAD`). Unlike
Mono, CoreCLR has no "attach to the running runtime" API, so hao captures the host as the game
creates it — hooking the loader to see `libcoreclr` arrive, then `coreclr_initialize` to read its
handle. `on_clr_init` hands you that [`ClrHost`], and `host.attach(major)` builds a
[`hao_runtime::Runtime`](https://docs.rs/hao-runtime) bound to the game's CLR.

```rust
#[used]
#[link_section = ".init_array"]
static LOAD: extern "C" fn() = load;

extern "C" fn load() {
    hao_godot::init_logging();
    hao_godot::on_clr_init(|host| {
        // Attach to the game's CLR (9 = its .NET major version), then call into
        // its managed code through the returned Runtime.
        let _runtime = host.attach(9).unwrap();
    });
}
```

- the attach machinery lives in [`hao_runtime::attach`](https://docs.rs/hao-runtime) and is
  re-exported here; `init_logging` is a one-call `tracing` setup (console + stderr) for an injected
  mod
- the callback runs on a background thread, not the game's init thread — managed work during
  `coreclr_initialize` corrupts the runtime's bootstrap
- with the `Runtime` you can call any managed method; generate typed bindings for the game's own
  assemblies (e.g. GodotSharp) with [`hao-runtime-macros`](../hao-runtime-macros)

## Examples

Build one, then add `LD_PRELOAD=/path/to/target/release/examples/lib<name>.so %command%` to the
game's Steam launch options and run it from a terminal to see the logs.

```sh
cargo build --release -p hao-godot --example capture
```

- [`capture`](examples/capture.rs): captures the game's running CoreCLR host and logs it — the
  proof the attach works.
- [`call`](examples/call.rs): attaches hao's runtime and calls a managed method
  (`Environment.ProcessId`) to prove the whole call path works.
- [`godot_types`](examples/godot_types.rs): calls the game's own GodotSharp API (`OS.GetName`, …)
  through bindings generated from its `GodotSharp.dll`.
- [`profiler`](examples/profiler.rs): registers hao-runtime's CoreCLR profiler, which observes JIT
  compilation and exposes JITed native code addresses — the basis for patching the game's methods.
- [`rewrite`](examples/rewrite.rs): rewrites the game assembly's IL before CoreCLR JITs it, so one
  of the game's own methods calls back into Rust, with no injected assembly.

## Logging

Logs go through [`tracing`](https://crates.io/crates/tracing). `init_logging` installs a subscriber
writing to stderr (and allocates a console where one is needed); set the level with `RUST_LOG`,
e.g. `RUST_LOG=hao_godot=debug,hao_runtime=debug`.
