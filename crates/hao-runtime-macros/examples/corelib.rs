//! Binds classes from .NET's own class library and uses them from Rust.
//!
//! ```sh
//! cargo run -p hao-runtime-macros --example corelib
//! ```
//!
//! `framework = "..."` finds the assembly in the newest installed .NET when
//! the example is compiled.

mod corelib {
    hao_runtime_macros::bind! {
        framework = "System.Private.CoreLib",
        types = ["System.Text.StringBuilder", "System.IO.Path", "System.Environment"],
    }
}

use corelib::system::{io::Path, text::StringBuilder, Environment};

fn main() -> hao_runtime::Result<()> {
    let runtime = hao_runtime::Runtime::coreclr()?;

    // Constructors and instance methods. Overloads are named after their
    // parameter types, e.g. `StringBuilder(string)` is `new_str`.
    let builder = StringBuilder::new_str(&runtime, "Kia ora")?;
    builder.append_str(" from ")?;
    // `Environment.Version` is a `System.Version`, which isn't bound, so it
    // comes back as a plain object.
    let version = Environment::get_version(&runtime)?.expect("a version");
    builder.append_str(&format!(".NET {}", version.to_managed_string()?))?;
    builder.append_char('!' as u16)?;
    builder.insert_i32_str(0, "> ")?;
    println!("{}", builder.object().to_managed_string()?);
    println!("length: {}", builder.get_length()?);

    // Static methods take the runtime.
    let path = Path::combine(&runtime, "saves", "slot1.json")?.unwrap_or_default();
    println!("Path.Combine: {}", path);
    println!(
        "Path.GetExtension(\"save.json\"): {:?}",
        Path::get_extension(&runtime, "save.json")?
    );
    println!(
        "Environment.ProcessorCount: {}",
        Environment::get_processor_count(&runtime)?
    );
    println!(
        "Environment.MachineName: {:?}",
        Environment::get_machine_name(&runtime)?
    );
    Ok(())
}
