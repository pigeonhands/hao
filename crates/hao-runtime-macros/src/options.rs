//! The macro's input: `assembly = "..."` (or `assembly_env`/`framework`) and `types = [...]`.

use std::path::PathBuf;

use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Expr, ExprArray, ExprLit, Ident, Lit, LitStr, Token,
};

pub struct Options {
    pub path: PathBuf,
    /// The option giving the path, for error spans.
    pub span: LitStr,
    /// From the installed .NET, so already available to the runtime.
    pub framework: bool,
    pub types: Option<Vec<String>>,
}

struct KeyValue {
    key: Ident,
    value: Expr,
}

impl Parse for KeyValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key = input.parse()?;
        input.parse::<Token![=]>()?;
        Ok(Self {
            key,
            value: input.parse()?,
        })
    }
}

fn string(expr: &Expr) -> syn::Result<LitStr> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Str(s), ..
        }) => Ok(s.clone()),
        other => Err(syn::Error::new_spanned(other, "expected a string literal")),
    }
}

fn strings(expr: &Expr) -> syn::Result<Vec<String>> {
    match expr {
        Expr::Array(ExprArray { elems, .. }) => {
            elems.iter().map(|e| string(e).map(|s| s.value())).collect()
        }
        other => Err(syn::Error::new_spanned(
            other,
            "expected an array of strings",
        )),
    }
}

impl Parse for Options {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut source = None;
        let mut types = None;
        for KeyValue { key, value } in Punctuated::<KeyValue, Token![,]>::parse_terminated(input)? {
            match key.to_string().as_str() {
                "assembly" => {
                    let lit = string(&value)?;
                    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
                    source = Some((PathBuf::from(manifest).join(lit.value()), lit, false));
                }
                "assembly_env" => {
                    let lit = string(&value)?;
                    let path = std::env::var(lit.value()).map_err(|_| {
                        syn::Error::new_spanned(
                            &lit,
                            format!("environment variable {} is not set", lit.value()),
                        )
                    })?;
                    source = Some((path.into(), lit, false));
                }
                "framework" => {
                    let lit = string(&value)?;
                    let dir = hao::resolver::PathAssemblyResolver::find_framework_dir()
                        .ok_or_else(|| {
                            syn::Error::new_spanned(&lit, "no .NET install found (set DOTNET_ROOT)")
                        })?;
                    source = Some((dir.join(format!("{}.dll", lit.value())), lit, true));
                }
                "types" => types = Some(strings(&value)?),
                _ => return Err(syn::Error::new_spanned(
                    key,
                    "unknown option, expected `assembly`, `assembly_env`, `framework` or `types`",
                )),
            }
        }
        let (path, span, framework) = source.ok_or_else(|| {
            input.error(
                "missing `assembly = \"...\"`, `assembly_env = \"...\"` or `framework = \"...\"`",
            )
        })?;
        Ok(Self {
            path,
            span,
            framework,
            types,
        })
    }
}
