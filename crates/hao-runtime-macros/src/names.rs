//! Rust names for .NET names.

use proc_macro2::{Ident, Span};
use quote::format_ident;

use hao::dotnet::entries::TypeDef;

const KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "dyn", "else", "enum", "extern", "false",
    "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
    "return", "static", "struct", "trait", "true", "type", "unsafe", "use", "where", "while",
    "abstract", "become", "box", "do", "final", "macro", "override", "priv", "try", "typeof",
    "unsized", "virtual", "yield",
];

/// An identifier for `name`, escaping keywords.
pub fn ident(name: &str) -> Ident {
    let name = if name.is_empty() || name.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{}", name)
    } else {
        name.to_owned()
    };
    match name.as_str() {
        // These can't be raw identifiers.
        "self" | "Self" | "super" | "crate" => format_ident!("{}_", name),
        n if KEYWORDS.contains(&n) => Ident::new_raw(n, Span::call_site()),
        n => Ident::new(n, Span::call_site()),
    }
}

/// `GetHashCode` → `get_hash_code`, `IOStream` → `io_stream`.
pub fn snake_case(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if !c.is_alphanumeric() {
            out.push('_');
            continue;
        }
        if c.is_uppercase() && i > 0 && !out.ends_with('_') {
            let prev = chars[i - 1];
            let next_lower = chars.get(i + 1).is_some_and(|n| n.is_lowercase());
            if prev.is_lowercase() || prev.is_ascii_digit() || (prev.is_uppercase() && next_lower) {
                out.push('_');
            }
        }
        out.extend(c.to_lowercase());
    }
    out
}

/// The struct name for a type: its name without generic arity, prefixed by
/// its declaring types for nested types (`Outer_Inner`).
pub fn type_name(t: &TypeDef) -> String {
    let own = t
        .name
        .split('`')
        .next()
        .unwrap_or(&t.name)
        .replace(|c: char| !c.is_alphanumeric(), "_");
    match t.declaring_type.upgrade() {
        Some(parent) => format!("{}_{}", type_name(&parent.get()), own),
        None => own,
    }
}

/// `MyLib.Shapes` → `["my_lib", "shapes"]`.
pub fn module_path(namespace: &str) -> Vec<String> {
    namespace
        .split('.')
        .filter(|s| !s.is_empty())
        .map(snake_case)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snake_case_names() {
        assert_eq!(snake_case("GetHashCode"), "get_hash_code");
        assert_eq!(snake_case("IOStream"), "io_stream");
        assert_eq!(snake_case("get_Length"), "get_length");
        assert_eq!(snake_case("UTF8Encoding"), "utf8_encoding");
        assert_eq!(snake_case("value"), "value");
    }
}
