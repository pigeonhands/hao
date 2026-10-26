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
//! - `framework = "Name"`: an assembly of the installed .NET, such as
//!   `System.Private.CoreLib`, found when compiling. CoreCLR only.
//! - `types = ["Ns.Type", ...]`: only bind these types (nested types use `+`).
//!
//! Every public, non-generic class gets a struct in a module named after its
//! namespace. Public, non-generic methods whose parameters are primitives,
//! strings or classes are bound. Static methods and constructors take the
//! [`Runtime`](https://docs.rs/hao-runtime) to call into.

mod codegen;
mod model;
mod names;
mod options;
mod thunks;
mod ty;

use proc_macro::TokenStream;

use options::Options;

fn expand(options: Options) -> syn::Result<proc_macro2::TokenStream> {
    let Options {
        path,
        span,
        framework,
        types,
    } = options;
    let error = |e: hao::error::HaoError| {
        syn::Error::new_spanned(&span, format!("{}: {}", path.display(), e))
    };

    let library = hao::Module::from_path_no_resolve(&path).map_err(error)?;
    let name = library
        .assembly
        .as_ref()
        .map(|a| a.get().name.clone())
        .ok_or_else(|| syn::Error::new_spanned(&span, "the module has no assembly manifest"))?;

    let model = model::Model::new(&library, types.as_deref());
    for wanted in types.iter().flatten() {
        if !model
            .types
            .iter()
            .any(|t| &t.ty.get().full_name().replace('/', "+") == wanted)
        {
            return Err(syn::Error::new_spanned(
                &span,
                format!("{} is not a public, non-generic class in {}", wanted, name),
            ));
        }
    }

    let thunks_name = format!("{}.HaoThunks", name);
    let coreclr =
        thunks::build(&library, &model, &thunks_name, thunks::Target::CoreClr).map_err(error)?;
    let mono =
        thunks::build(&library, &model, &thunks_name, thunks::Target::Mono).map_err(error)?;
    // Framework assemblies are already available to the runtime, so they aren't loaded by path.
    let path = match framework {
        true => String::new(),
        false => path
            .canonicalize()
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned(),
    };
    Ok(codegen::generate(
        &model,
        codegen::Assembly {
            name: &name,
            path: &path,
            thunks_name: &thunks_name,
            coreclr: &coreclr,
            mono: &mono,
        },
    ))
}

/// Generates bindings for a .NET assembly. See the crate docs.
#[proc_macro]
pub fn bind(input: TokenStream) -> TokenStream {
    let options = syn::parse_macro_input!(input as Options);
    expand(options)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
