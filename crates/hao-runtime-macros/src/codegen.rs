//! Generates the Rust side: a struct per bound type in modules per namespace,
//! with a method per bound method that calls its thunk.

use std::collections::BTreeMap;

use hao::dotnet::entries::EntryName;
use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};

use crate::{
    model::{BoundMethod, BoundType, MethodKind, Model},
    names::ident,
    ty::{primitive_name, Ty},
};

pub struct Assembly<'a> {
    pub name: &'a str,
    /// Empty for framework assemblies, which the runtime already has.
    pub path: &'a str,
    pub thunks_name: &'a str,
    pub coreclr: &'a [u8],
    pub mono: &'a [u8],
}

pub fn generate(model: &Model, assembly: Assembly) -> TokenStream {
    let mut tree = ModuleTree::default();
    for ty in &model.types {
        tree.insert(&ty.module, ty);
    }
    let modules = tree.render(model, 0);

    let Assembly {
        name,
        path,
        thunks_name,
        coreclr,
        mono,
    } = assembly;
    let (coreclr, mono) = (Literal::byte_string(coreclr), Literal::byte_string(mono));
    let count = model.thunk_count;
    // Rebuild when the bound assembly changes.
    let track = (!path.is_empty()).then(|| {
        quote!(
            const _: &[u8] = include_bytes!(#path);
        )
    });
    quote! {
        #[doc(hidden)]
        pub static __HAO_ASSEMBLY: ::hao_runtime::BoundAssembly =
            ::hao_runtime::BoundAssembly::new(#name, #path, #thunks_name, #coreclr, #mono, #count);
        #track
        #modules
    }
}

/// Bound types grouped into nested modules by namespace.
#[derive(Default)]
struct ModuleTree<'a> {
    types: Vec<&'a BoundType>,
    children: BTreeMap<&'a str, ModuleTree<'a>>,
}

impl<'a> ModuleTree<'a> {
    fn insert(&mut self, path: &'a [String], ty: &'a BoundType) {
        match path.split_first() {
            None => self.types.push(ty),
            Some((first, rest)) => self.children.entry(first).or_default().insert(rest, ty),
        }
    }

    fn render(&self, model: &Model, depth: usize) -> TokenStream {
        let types = Types { model, depth };
        let structs = self.types.iter().map(|ty| types.bound_type(ty));
        let children = self.children.iter().map(|(name, child)| {
            let name = ident(name);
            let inner = child.render(model, depth + 1);
            quote!(pub mod #name { #inner })
        });
        quote!(#(#structs)* #(#children)*)
    }
}

/// Code for the types in one module, `depth` modules below the macro call.
struct Types<'a> {
    model: &'a Model,
    depth: usize,
}

impl Types<'_> {
    /// `super::super::` back to the macro call.
    fn root(&self) -> TokenStream {
        let supers = (0..self.depth).map(|_| quote!(super::));
        quote!(#(#supers)*)
    }

    /// The path to a bound type's struct.
    fn path(&self, index: usize) -> TokenStream {
        let ty = &self.model.types[index];
        let root = self.root();
        let modules = ty.module.iter().map(|m| ident(m));
        let name = ident(&ty.rust_name);
        quote!(#root #(#modules::)* #name)
    }

    fn bound_type(&self, ty: &BoundType) -> TokenStream {
        let name = ident(&ty.rust_name);
        let doc = format!(" `{}`", ty.ty.get().full_name());
        let methods = ty.methods.iter().map(|m| self.method(m));
        // Both are transparent wrappers of `Object`, so the reference cast is sound.
        let base = ty.base.map(|base| {
            let base = self.path(base);
            quote! {
                impl ::core::ops::Deref for #name {
                    type Target = #base;
                    fn deref(&self) -> &#base {
                        unsafe { &*(self as *const Self as *const #base) }
                    }
                }
            }
        });
        quote! {
            #[doc = #doc]
            #[derive(Debug)]
            #[repr(transparent)]
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

            #base
        }
    }

    fn param_type(&self, ty: &Ty) -> TokenStream {
        match ty {
            Ty::Void => quote!(()),
            Ty::Bool => quote!(bool),
            Ty::Primitive(p) => rust_primitive(*p),
            Ty::String => quote!(&str),
            Ty::Object { bound: Some(i), .. } => {
                let path = self.path(*i);
                quote!(&#path)
            }
            Ty::Object { bound: None, .. } => quote!(&::hao_runtime::Object),
        }
    }

    /// The thunk arguments for a parameter, as `type = value`, and any setup they need.
    fn marshal(
        &self,
        name: &proc_macro2::Ident,
        ty: &Ty,
    ) -> (Option<TokenStream>, Vec<TokenStream>) {
        match ty {
            Ty::Void => (None, vec![]),
            Ty::Bool => (None, vec![quote!(u8 = #name as u8)]),
            Ty::Primitive(p) => {
                let t = rust_primitive(*p);
                (None, vec![quote!(#t = #name)])
            }
            Ty::String => {
                let utf16 = format_ident!("__{}_utf16", name.to_string().trim_start_matches("r#"));
                (
                    Some(
                        quote!(let #utf16: ::std::vec::Vec<u16> = #name.encode_utf16().collect();),
                    ),
                    vec![
                        quote!(*const u16 = #utf16.as_ptr()),
                        quote!(i32 = #utf16.len() as i32),
                    ],
                )
            }
            Ty::Object { bound: Some(_), .. } => {
                (None, vec![quote!(isize = #name.object().handle())])
            }
            Ty::Object { bound: None, .. } => (None, vec![quote!(isize = #name.handle())]),
        }
    }

    /// The Rust return type, the thunk's result slot type, and how to turn `__ret` into the return value.
    fn unmarshal(
        &self,
        method: &BoundMethod,
        runtime: &TokenStream,
    ) -> (TokenStream, Option<TokenStream>, TokenStream) {
        let root = self.root();
        let object = quote!(unsafe { ::hao_runtime::Object::from_handle(__ret, #runtime.clone(), &#root __HAO_ASSEMBLY) });
        if method.kind == MethodKind::Constructor {
            return (quote!(Self), Some(quote!(isize)), quote!(Self(#object)));
        }
        match &method.ret {
            Ty::Void => (quote!(()), None, quote!(())),
            Ty::Bool => (quote!(bool), Some(quote!(u8)), quote!(__ret != 0)),
            Ty::Primitive(p) => (rust_primitive(*p), Some(rust_primitive(*p)), quote!(__ret)),
            Ty::String => (
                quote!(::core::option::Option<::std::string::String>),
                Some(quote!(isize)),
                quote!(#root __HAO_ASSEMBLY.take_string(#runtime, __ret)?),
            ),
            Ty::Object { bound, .. } => {
                let (ty, wrap) = match bound {
                    Some(i) => {
                        let path = self.path(*i);
                        (path.clone(), quote!(#path::from_object))
                    }
                    None => (
                        quote!(::hao_runtime::Object),
                        quote!(::core::convert::identity),
                    ),
                };
                (
                    quote!(::core::option::Option<#ty>),
                    Some(quote!(isize)),
                    quote!((__ret != 0).then(|| #wrap(#object))),
                )
            }
        }
    }

    fn method(&self, method: &BoundMethod) -> TokenStream {
        let root = self.root();
        let name = ident(&method.rust_name);
        let thunk = method.thunk;

        let (mut inputs, mut args, runtime) = match method.kind {
            MethodKind::Instance => (
                vec![quote!(&self)],
                vec![quote!(isize = self.0.handle())],
                quote!(self.0.runtime()),
            ),
            _ => (
                vec![quote!(runtime: &::hao_runtime::Runtime)],
                vec![],
                quote!(runtime),
            ),
        };
        let mut setup = Vec::new();
        for (param, ty) in &method.params {
            let param = ident(param);
            let rust_ty = self.param_type(ty);
            inputs.push(quote!(#param: #rust_ty));
            let (prepare, thunk_args) = self.marshal(&param, ty);
            setup.extend(prepare);
            args.extend(thunk_args);
        }
        let (output, slot, convert) = self.unmarshal(method, &runtime);
        if let Some(slot) = &slot {
            setup.push(quote!(let mut __ret: #slot = ::core::default::Default::default();));
            args.push(quote!(*mut #slot = &mut __ret));
        }
        args.push(quote!(*mut isize = &mut __exception));

        let doc = format!(" Calls `{}`.", method.method.get().entry_name());
        quote! {
            #[doc = #doc]
            #[allow(clippy::too_many_arguments, clippy::unnecessary_cast, clippy::needless_question_mark)]
            pub fn #name(#(#inputs),*) -> ::hao_runtime::Result<#output> {
                let __runtime: &::hao_runtime::Runtime = #runtime;
                let __function = #root __HAO_ASSEMBLY.function(__runtime, #thunk)?;
                #(#setup)*
                let mut __exception: isize = 0;
                ::hao_runtime::__call_thunk!(__runtime, __function, (#(#args),*));
                if __exception != 0 {
                    return ::core::result::Result::Err(#root __HAO_ASSEMBLY.take_exception(__runtime, __exception));
                }
                ::core::result::Result::Ok(#convert)
            }
        }
    }
}

fn rust_primitive(p: hao::dotnet::signature::PrimitiveType) -> TokenStream {
    let name = format_ident!("{}", primitive_name(p));
    quote!(#name)
}
