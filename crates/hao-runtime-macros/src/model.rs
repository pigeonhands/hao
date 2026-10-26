//! Which types and methods of an assembly can be bound, and how their values
//! cross the native boundary.

use hao::{
    dotnet::{
        entries::{Entry, Method, TypeDef, TypeDefOrRef},
        md::streams::tables_stream::{MethodFlags, TypeAttributes},
        signature::{PrimitiveType, TypeSig},
    },
    Module,
};

/// How a value is passed between Rust and a thunk.
#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    Void,
    Bool,
    Primitive(PrimitiveType),
    String,
    /// A reference type. `bound` is the index of its bound type, if any.
    Object {
        ty: Option<TypeDefOrRef>,
        bound: Option<usize>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MethodKind {
    Constructor,
    Static,
    Instance,
}

pub struct BoundMethod {
    pub method: Entry<Method>,
    pub kind: MethodKind,
    pub params: Vec<(String, Ty)>,
    pub ret: Ty,
    pub rust_name: String,
    pub thunk: usize,
}

pub struct BoundType {
    pub ty: Entry<TypeDef>,
    pub namespace: Vec<String>,
    pub rust_name: String,
    pub methods: Vec<BoundMethod>,
}

fn is_public(t: &TypeDef) -> bool {
    let visibility = t.flags & TypeAttributes::VisibilityMask;
    let public = visibility == TypeAttributes::Public || visibility == TypeAttributes::NestedPublic;
    public
        && t.declaring_type
            .upgrade()
            .is_none_or(|parent| is_public(&parent.get()))
}

fn bindable_type(t: &TypeDef) -> bool {
    is_public(t)
        && t.generic_params.is_empty()
        && !t.is_value_type()
        && !t.is_delegate()
        && !t.name.starts_with('<')
}

pub fn snake_case(name: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = name.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        if c.is_uppercase() {
            let prev_lower = i > 0 && (chars[i - 1].is_lowercase() || chars[i - 1].is_ascii_digit());
            let next_lower = chars.get(i + 1).is_some_and(|n| n.is_lowercase());
            let prev_upper = i > 0 && chars[i - 1].is_uppercase();
            if !out.is_empty() && !out.ends_with('_') && (prev_lower || (prev_upper && next_lower)) {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else if c.is_alphanumeric() || *c == '_' {
            out.push(*c);
        } else {
            out.push('_');
        }
    }
    out
}

fn rust_type_name(t: &TypeDef) -> String {
    let own = t.name.split('`').next().unwrap_or(&t.name).replace(|c: char| !c.is_alphanumeric(), "_");
    match t.declaring_type.upgrade() {
        Some(parent) => format!("{}_{}", rust_type_name(&parent.get()), own),
        None => own,
    }
}

fn top_level_namespace(t: &Entry<TypeDef>) -> String {
    match t.get().declaring_type.upgrade() {
        Some(parent) => top_level_namespace(&parent),
        None => t.get().namespace.clone(),
    }
}

pub struct Model {
    pub types: Vec<BoundType>,
    pub thunk_count: usize,
}

impl Model {
    pub fn new(module: &Module, filter: Option<&[String]>) -> Self {
        let types: Vec<Entry<TypeDef>> = module
            .all_types()
            .into_iter()
            .filter(|t| bindable_type(&t.get()))
            .filter(|t| filter.is_none_or(|f| f.contains(&t.get().full_name().replace('/', "+"))))
            .collect();

        let mut thunk = hao_runtime_thunk_base();
        let mut bound_types = Vec::new();
        for ty in &types {
            let namespace = top_level_namespace(ty)
                .split('.')
                .filter(|s| !s.is_empty())
                .map(snake_case)
                .collect();
            let mut methods: Vec<BoundMethod> = Vec::new();
            for method in ty.get().methods.iter() {
                let Some(mut bound) = bind_method(method, &types) else {
                    continue;
                };
                let base = bound.rust_name.clone();
                let mut n = 1;
                while methods.iter().any(|m| m.rust_name == bound.rust_name) {
                    bound.rust_name = format!("{}_{}", base, n);
                    n += 1;
                }
                bound.thunk = thunk;
                thunk += 1;
                methods.push(bound);
            }
            bound_types.push(BoundType {
                ty: ty.clone(),
                namespace,
                rust_name: rust_type_name(&ty.get()),
                methods,
            });
        }
        Self {
            types: bound_types,
            thunk_count: thunk,
        }
    }
}

/// The number of helper thunks before the bound methods (`hao_runtime::bindings::helper::COUNT`).
fn hao_runtime_thunk_base() -> usize {
    4
}

fn value_ty(sig: &TypeSig, types: &[Entry<TypeDef>]) -> Option<Ty> {
    Some(match sig.unmodified() {
        TypeSig::Primitive(PrimitiveType::Void) => Ty::Void,
        TypeSig::Primitive(PrimitiveType::Boolean) => Ty::Bool,
        TypeSig::Primitive(PrimitiveType::String) => Ty::String,
        TypeSig::Primitive(PrimitiveType::Object) => Ty::Object { ty: None, bound: None },
        TypeSig::Primitive(PrimitiveType::TypedByRef) => return None,
        TypeSig::Primitive(p) => Ty::Primitive(*p),
        TypeSig::Class(t) => {
            let bound = match t {
                TypeDefOrRef::TypeDef(def) => types.iter().position(|b| b.ptr_eq(def)),
                _ => None,
            };
            if matches!(t, TypeDefOrRef::TypeSpec(_)) {
                return None;
            }
            Ty::Object {
                ty: Some(t.clone()),
                bound,
            }
        }
        _ => return None,
    })
}

fn bind_method(method: &Entry<Method>, types: &[Entry<TypeDef>]) -> Option<BoundMethod> {
    let m = method.get();
    let public = m.flags & MethodFlags::MemberAccessMask == MethodFlags::Public;
    if !public || m.signature.generic_param_count > 0 || !m.signature.varargs.is_empty() || m.is_cctor() {
        return None;
    }
    let kind = if m.is_ctor() {
        let declaring = m.declaring_type.upgrade()?;
        if declaring.get().flags.contains(TypeAttributes::Abstract) {
            return None;
        }
        MethodKind::Constructor
    } else if m.is_static() {
        MethodKind::Static
    } else {
        MethodKind::Instance
    };
    let params = m
        .signature
        .params
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let ty = value_ty(p, types)?;
            (ty != Ty::Void).then(|| (snake_case(&m.param_name(i)), ty))
        })
        .collect::<Option<Vec<_>>>()?;
    let ret = if kind == MethodKind::Constructor {
        Ty::Void
    } else {
        value_ty(&m.signature.ret, types)?
    };
    let rust_name = if kind == MethodKind::Constructor {
        "new".to_owned()
    } else {
        snake_case(&m.name)
    };
    Some(BoundMethod {
        method: method.clone(),
        kind,
        params,
        ret,
        rust_name,
        thunk: 0,
    })
}
