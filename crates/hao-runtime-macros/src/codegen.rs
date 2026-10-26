use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::{format_ident, quote};

use crate::model::{BoundMethod, BoundType, MethodKind, Model, Ty};
use hao::dotnet::signature::PrimitiveType;

const KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false",
    "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while",
    "abstract", "become", "box", "do", "final", "macro", "override", "priv", "try", "typeof", "unsized",
    "virtual", "yield",
];

pub fn ident(name: &str) -> Ident {
    let name = if name.is_empty() || name.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{}", name)
    } else {
        name.to_owned()
    };
    match name.as_str() {
        "self" | "Self" | "super" | "crate" => format_ident!("{}_", name),
        n if KEYWORDS.contains(&n) => Ident::new_raw(n, Span::call_site()),
        n => Ident::new(n, Span::call_site()),
    }
}

fn primitive(p: PrimitiveType) -> TokenStream {
    match p {
        PrimitiveType::I1 => quote!(i8),
        PrimitiveType::U1 => quote!(u8),
        PrimitiveType::I2 => quote!(i16),
        PrimitiveType::U2 | PrimitiveType::Char => quote!(u16),
        PrimitiveType::I4 => quote!(i32),
        PrimitiveType::U4 => quote!(u32),
        PrimitiveType::I8 => quote!(i64),
        PrimitiveType::U8 => quote!(u64),
        PrimitiveType::R4 => quote!(f32),
        PrimitiveType::R8 => quote!(f64),
        PrimitiveType::I => quote!(isize),
        _ => quote!(usize),
    }
}

struct Ctx<'a> {
    model: &'a Model,
    root: TokenStream,
}

impl Ctx<'_> {
    fn bound_path(&self, index: usize) -> TokenStream {
        let ty = &self.model.types[index];
        let root = &self.root;
        let modules = ty.namespace.iter().map(|n| ident(n));
        let name = ident(&ty.rust_name);
        quote!(#root #(#modules::)* #name)
    }

    fn param_type(&self, ty: &Ty) -> TokenStream {
        match ty {
            Ty::Bool => quote!(bool),
            Ty::Primitive(p) => primitive(*p),
            Ty::String => quote!(&str),
            Ty::Object { bound: Some(i), .. } => {
                let path = self.bound_path(*i);
                quote!(&#path)
            }
            Ty::Object { bound: None, .. } => quote!(&::hao_runtime::Object),
            Ty::Void => quote!(()),
        }
    }

    fn return_type(&self, ty: &Ty) -> TokenStream {
        match ty {
            Ty::Void => quote!(()),
            Ty::String => quote!(::core::option::Option<::std::string::String>),
            Ty::Object { bound: Some(i), .. } => {
                let path = self.bound_path(*i);
                quote!(::core::option::Option<#path>)
            }
            Ty::Object { bound: None, .. } => quote!(::core::option::Option<::hao_runtime::Object>),
            ty => self.param_type(ty),
        }
    }

    fn method(&self, method: &BoundMethod) -> TokenStream {
        let root = &self.root;
        let name = ident(&method.rust_name);
        let thunk = method.thunk;

        let mut inputs = Vec::new();
        let mut prelude = Vec::new();
        let mut args = Vec::new();
        let runtime = match method.kind {
            MethodKind::Instance => {
                inputs.push(quote!(&self));
                args.push(quote!(isize = self.0.handle()));
                quote!(self.0.runtime())
            }
            _ => {
                inputs.push(quote!(runtime: &::hao_runtime::Runtime));
                quote!(runtime)
            }
        };

        for (param, ty) in &method.params {
            let param = ident(param);
            let rust_ty = self.param_type(ty);
            inputs.push(quote!(#param: #rust_ty));
            match ty {
                Ty::Bool => args.push(quote!(u8 = #param as u8)),
                Ty::Primitive(p) => {
                    let t = primitive(*p);
                    args.push(quote!(#t = #param));
                }
                Ty::String => {
                    let utf16 = format_ident!("__{}_utf16", param.to_string().trim_start_matches("r#"));
                    prelude.push(quote!(let #utf16: ::std::vec::Vec<u16> = #param.encode_utf16().collect();));
                    args.push(quote!(*const u16 = #utf16.as_ptr()));
                    args.push(quote!(i32 = #utf16.len() as i32));
                }
                Ty::Object { bound: Some(_), .. } => args.push(quote!(isize = #param.object().handle())),
                Ty::Object { bound: None, .. } => args.push(quote!(isize = #param.handle())),
                Ty::Void => {}
            }
        }

        let (output, ret_slot, convert) = match (method.kind, &method.ret) {
            (MethodKind::Constructor, _) => (
                quote!(Self),
                Some(quote!(isize)),
                quote!(Self(unsafe { ::hao_runtime::Object::from_handle(__ret, #runtime.clone(), &#root __HAO_ASSEMBLY) })),
            ),
            (_, Ty::Void) => (quote!(()), None, quote!(())),
            (_, Ty::Bool) => (quote!(bool), Some(quote!(u8)), quote!(__ret != 0)),
            (_, Ty::Primitive(p)) => (primitive(*p), Some(primitive(*p)), quote!(__ret)),
            (_, Ty::String) => (
                self.return_type(&Ty::String),
                Some(quote!(isize)),
                quote!(#root __HAO_ASSEMBLY.take_string(#runtime, __ret)?),
            ),
            (_, ty @ Ty::Object { bound, .. }) => {
                let wrap = match bound {
                    Some(i) => {
                        let path = self.bound_path(*i);
                        quote!(#path::from_object)
                    }
                    None => quote!(::core::convert::identity),
                };
                (
                    self.return_type(ty),
                    Some(quote!(isize)),
                    quote!((__ret != 0).then(|| #wrap(unsafe {
                        ::hao_runtime::Object::from_handle(__ret, #runtime.clone(), &#root __HAO_ASSEMBLY)
                    }))),
                )
            }
        };
        if let Some(slot) = &ret_slot {
            prelude.push(quote!(let mut __ret: #slot = ::core::default::Default::default();));
            args.push(quote!(*mut #slot = &mut __ret));
        }
        args.push(quote!(*mut isize = &mut __exception));

        let doc = format!(" Calls `{}`.", method.method.get().entry_name_for_docs());
        quote! {
            #[doc = #doc]
            #[allow(clippy::too_many_arguments, clippy::unnecessary_cast, clippy::needless_question_mark, clippy::bool_to_int_with_if)]
            pub fn #name(#(#inputs),*) -> ::hao_runtime::Result<#output> {
                let __runtime: &::hao_runtime::Runtime = #runtime;
                let __function = #root __HAO_ASSEMBLY.function(__runtime, #thunk)?;
                #(#prelude)*
                let mut __exception: isize = 0;
                ::hao_runtime::__call_thunk!(__runtime, __function, (#(#args),*));
                if __exception != 0 {
                    return ::core::result::Result::Err(#root __HAO_ASSEMBLY.take_exception(__runtime, __exception));
                }
                ::core::result::Result::Ok(#convert)
            }
        }
    }

    fn bound_type(&self, ty: &BoundType) -> TokenStream {
        let name = ident(&ty.rust_name);
        let doc = format!(" `{}`", ty.ty.get().full_name());
        let methods = ty.methods.iter().map(|m| self.method(m));
        quote! {
            #[doc = #doc]
            #[derive(Debug)]
            pub struct #name(::hao_runtime::Object);

            impl #name {
                /// Wraps an object without checking its type.
                pub fn from_object(object: ::hao_runtime::Object) -> Self {
                    Self(object)
                }

                pub fn object(&self) -> &::hao_runtime::Object {
                    &self.0
                }

                pub fn into_object(self) -> ::hao_runtime::Object {
                    self.0
                }

                #(#methods)*
            }
        }
    }
}

trait DocName {
    fn entry_name_for_docs(&self) -> String;
}

impl DocName for hao::dotnet::entries::Method {
    fn entry_name_for_docs(&self) -> String {
        use hao::dotnet::entries::EntryName;
        self.entry_name()
    }
}

pub struct Assembly<'a> {
    pub name: &'a str,
    pub path: &'a str,
    pub thunks_name: &'a str,
    pub coreclr: &'a [u8],
    pub mono: &'a [u8],
}

pub fn generate(model: &Model, assembly: Assembly) -> TokenStream {
    let all: Vec<&BoundType> = model.types.iter().collect();
    let ctx_for = |depth: usize| -> TokenStream {
        let supers = (0..depth).map(|_| quote!(super::));
        quote!(#(#supers)*)
    };

    // Every type refers back to the root through `super::` chains, so build
    // each namespace module with the right depth.
    fn build(model: &Model, types: &[&BoundType], depth: usize, root: &dyn Fn(usize) -> TokenStream) -> TokenStream {
        let ctx = Ctx {
            model,
            root: root(depth),
        };
        let here = types.iter().filter(|t| t.namespace.len() == depth).map(|t| ctx.bound_type(t));
        let mut children: Vec<&str> = types
            .iter()
            .filter(|t| t.namespace.len() > depth)
            .map(|t| t.namespace[depth].as_str())
            .collect();
        children.sort();
        children.dedup();
        let children = children.into_iter().map(|child| {
            let nested: Vec<&BoundType> = types
                .iter()
                .filter(|t| t.namespace.len() > depth && t.namespace[depth] == child)
                .copied()
                .collect();
            let inner = build(model, &nested, depth + 1, root);
            let child = ident(child);
            quote! {
                pub mod #child {
                    #inner
                }
            }
        });
        quote! {
            #(#here)*
            #(#children)*
        }
    }
    let out = build(model, &all, 0, &ctx_for);

    let (name, path, thunks_name) = (assembly.name, assembly.path, assembly.thunks_name);
    let coreclr = Literal::byte_string(assembly.coreclr);
    let mono = Literal::byte_string(assembly.mono);
    let count = model.thunk_count;
    quote! {
        #[doc(hidden)]
        pub static __HAO_ASSEMBLY: ::hao_runtime::BoundAssembly = ::hao_runtime::BoundAssembly::new(
            #name,
            #path,
            #thunks_name,
            #coreclr,
            #mono,
            #count,
        );
        const _: &[u8] = include_bytes!(#path);
        #out
    }
}
