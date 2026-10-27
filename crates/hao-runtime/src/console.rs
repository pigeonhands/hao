//! Give an injected mod somewhere to see its logs.
//!
//! A game is usually a windowed process with no console, so `stdout`/`stderr`
//! (and a `tracing` subscriber writing to them) go nowhere. [`alloc_console`]
//! fixes that on Windows by allocating a console and pointing the standard
//! handles at it. On other platforms the process already has a `stderr`, so it
//! is a no-op; run the game from a terminal to see the output.

/// Allocates a console and attaches the standard output/error handles to it.
/// Returns whether a console was allocated (always `false` off Windows).
pub fn alloc_console() -> bool {
    imp::alloc_console()
}

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::System::Console::AllocConsole;

    pub fn alloc_console() -> bool {
        // AllocConsole points the process's standard handles at the new console.
        // Rust reads those handles lazily, so as long as this runs before the
        // first stdout/stderr use (it does, at mod load) the output reaches it.
        unsafe { AllocConsole() != 0 }
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn alloc_console() -> bool {
        false
    }
}
