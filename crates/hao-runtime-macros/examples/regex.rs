//! Uses .NET's `Regex` from Rust through generated bindings, including
//! objects returned from calls and exceptions thrown by .NET.
//!
//! ```sh
//! cargo run -p hao-runtime-macros --example regex
//! ```

mod regex {
    hao_runtime_macros::bind! {
        framework = "System.Text.RegularExpressions",
        types = [
            "System.Text.RegularExpressions.Regex",
            "System.Text.RegularExpressions.Match",
            "System.Text.RegularExpressions.Group",
            "System.Text.RegularExpressions.Capture",
        ],
    }
}

use hao_runtime::Error;
use regex::system::text::regular_expressions::Regex;

fn main() -> hao_runtime::Result<()> {
    let runtime = hao_runtime::Runtime::coreclr()?;

    let version = Regex::new(&runtime, r"(\d+)\.(\d+)\.(\d+)")?;
    println!("pattern: {:?}", version.object().to_managed_string()?);
    println!(
        "IsMatch(\"hao 0.0.4\"): {}",
        version.is_match_str("hao 0.0.4")?
    );

    // Overloads after the first are named after their parameters: the static
    // `IsMatch(input, pattern)` comes first, so the instance `IsMatch(input)`
    // is `is_match_str`.

    // Calls returning bound classes give typed objects back. `Match` derives
    // from `Group` and `Capture`, whose methods it gets through `Deref`.
    let found = version
        .match_str("released hao 0.0.4 today")?
        .expect("a match");
    println!(
        "Match.Value: {:?}, at index {}",
        found.get_value()?,
        found.get_index()?
    );
    let next = found.next_match()?.expect("a match object");
    println!("another match: {}", next.get_success()?);

    // Static methods take the runtime.
    let replaced = Regex::replace(&runtime, "a1b22c333", r"\d+", "#")?;
    println!("Regex.Replace: {:?}", replaced);

    // Exceptions thrown by .NET come back as errors.
    match Regex::new(&runtime, "(unclosed") {
        Err(Error::Managed(message)) => {
            println!("invalid pattern: {}", message.lines().next().unwrap_or(""))
        }
        other => println!("unexpected: {:?}", other.map(|_| ())),
    }
    Ok(())
}
