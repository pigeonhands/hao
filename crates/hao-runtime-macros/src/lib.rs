//! Generates Rust bindings for a .NET assembly, for use with `hao-runtime`.
//!
//! ```ignore
//! mod my_lib {
//!     hao_runtime_macros::bind! {
//!         assembly = "dotnet/MyLib.dll",
//!     }
//! }
//!
//! let runtime = hao_runtime::Runtime::coreclr()?;
//! let calculator = my_lib::my_lib::Calculator::new(&runtime, 10)?;
//! assert_eq!(calculator.add(5)?, 15);
//! ```
//!
//! Options:
//! - `assembly = "path"`: the assembly, relative to the crate's `Cargo.toml`.
//! - `assembly_env = "VAR"`: read the path from an environment variable at
//!   compile time instead, e.g. one set by a build script.
//! - `types = ["Ns.Type", ...]`: only bind these types (nested types use `+`).
//!
//! Every public, non-generic class gets a struct in a module named after its
//! namespace. Public, non-generic methods whose parameters are primitives,
//! strings or classes are bound. Static methods and constructors take the
//! [`Runtime`](https://docs.rs/hao-runtime) to call into.

mod codegen;
mod model;
mod thunks;

use proc_macro::TokenStream;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Expr, ExprArray, ExprLit, Ident, Lit, LitStr, Token,
};

struct Option_ {
    key: Ident,
    value: Expr,
}

impl Parse for Option_ {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key = input.parse()?;
        input.parse::<Token![=]>()?;
        Ok(Self {
            key,
            value: input.parse()?,
        })
    }
}

struct Input {
    options: Punctuated<Option_, Token![,]>,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            options: Punctuated::parse_terminated(input)?,
        })
    }
}

fn string(expr: &Expr) -> syn::Result<LitStr> {
    match expr {
        Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) => Ok(s.clone()),
        other => Err(syn::Error::new_spanned(other, "expected a string literal")),
    }
}

fn expand(input: Input) -> syn::Result<proc_macro2::TokenStream> {
    let mut path = None;
    let mut types = None;
    for option in &input.options {
        match option.key.to_string().as_str() {
            "assembly" => {
                let lit = string(&option.value)?;
                let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
                path = Some((std::path::Path::new(&manifest).join(lit.value()), lit));
            }
            "assembly_env" => {
                let lit = string(&option.value)?;
                let value = std::env::var(lit.value())
                    .map_err(|_| syn::Error::new_spanned(&lit, format!("environment variable {} is not set", lit.value())))?;
                path = Some((value.into(), lit));
            }
            "types" => match &option.value {
                Expr::Array(ExprArray { elems, .. }) => {
                    types = Some(elems.iter().map(|e| string(e).map(|s| s.value())).collect::<syn::Result<Vec<_>>>()?);
                }
                other => return Err(syn::Error::new_spanned(other, "expected an array of type names")),
            },
            _ => {
                return Err(syn::Error::new_spanned(
                    &option.key,
                    "unknown option, expected `assembly`, `assembly_env` or `types`",
                ))
            }
        }
    }
    let (path, span) = path.ok_or_else(|| syn::Error::new(proc_macro2::Span::call_site(), "missing `assembly = \"...\"`"))?;
    let error = |e: hao::error::HaoError| syn::Error::new_spanned(&span, format!("{}: {}", path.display(), e));

    let library = hao::Module::from_path_no_resolve(&path).map_err(error)?;
    let name = library
        .assembly
        .as_ref()
        .map(|a| a.get().name.clone())
        .ok_or_else(|| syn::Error::new_spanned(&span, "the module has no assembly manifest"))?;
    let thunks_name = format!("{}.HaoThunks", name);

    let model = model::Model::new(&library, types.as_deref());
    if let Some(filter) = &types {
        for wanted in filter {
            if !model.types.iter().any(|t| &t.ty.get().full_name().replace('/', "+") == wanted) {
                return Err(syn::Error::new_spanned(&span, format!("{} is not a public, non-generic class in {}", wanted, name)));
            }
        }
    }
    let coreclr = thunks::build(&library, &model, &thunks_name, thunks::Target::CoreClr).map_err(error)?;
    let mono = thunks::build(&library, &model, &thunks_name, thunks::Target::Mono).map_err(error)?;

    let path_string = path.canonicalize().unwrap_or(path.clone()).to_string_lossy().into_owned();
    Ok(codegen::generate(
        &model,
        codegen::Assembly {
            name: &name,
            path: &path_string,
            thunks_name: &thunks_name,
            coreclr: &coreclr,
            mono: &mono,
        },
    ))
}

/// Generates bindings for a .NET assembly. See the crate docs.
#[proc_macro]
pub fn bind(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as Input);
    expand(input).unwrap_or_else(|e| e.to_compile_error()).into()
}
