//! Which types and methods of an assembly get bindings, and their Rust names.

use hao::{
    dotnet::{
        entries::{Entry, Method, TypeDef, TypeDefOrRef},
        md::streams::tables_stream::TypeAttributes,
    },
    Module,
};

use crate::{
    names::{module_path, snake_case, type_name},
    thunks::HELPER_COUNT,
    ty::Ty,
};

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
    /// Index of the method's thunk, `f{thunk}`.
    pub thunk: usize,
}

impl BoundMethod {
    /// Whether the thunk writes a result through a pointer.
    pub fn returns(&self) -> bool {
        self.kind == MethodKind::Constructor || self.ret != Ty::Void
    }
}

pub struct BoundType {
    pub ty: Entry<TypeDef>,
    /// Rust modules, from the namespace.
    pub module: Vec<String>,
    pub rust_name: String,
    /// The nearest base class that is bound too.
    pub base: Option<usize>,
    pub methods: Vec<BoundMethod>,
}

pub struct Model {
    pub types: Vec<BoundType>,
    pub thunk_count: usize,
}

fn bindable_type(t: &TypeDef) -> bool {
    t.is_visible()
        && t.generic_params.is_empty()
        && !t.is_value_type()
        && !t.is_delegate()
        && !t.name.starts_with('<')
}

impl Model {
    pub fn new(module: &Module, filter: Option<&[String]>) -> Self {
        let types: Vec<Entry<TypeDef>> = module
            .all_types()
            .into_iter()
            .filter(|t| bindable_type(&t.get()))
            .filter(|t| filter.is_none_or(|f| f.contains(&t.get().full_name().replace('/', "+"))))
            .collect();

        let mut thunk = HELPER_COUNT;
        let bound = types
            .iter()
            .map(|ty| {
                let mut methods: Vec<BoundMethod> = Vec::new();
                for method in &ty.get().methods {
                    let Some(mut bound) = bind_method(method, &types) else {
                        continue;
                    };
                    bound.rust_name = overload_name(&bound, &methods);
                    bound.thunk = thunk;
                    thunk += 1;
                    methods.push(bound);
                }
                BoundType {
                    ty: ty.clone(),
                    module: module_path(&ty.get().top_level_namespace()),
                    rust_name: type_name(&ty.get()),
                    base: bound_base(ty, &types),
                    methods,
                }
            })
            .collect();
        Self {
            types: bound,
            thunk_count: thunk,
        }
    }
}

/// The first overload keeps the plain name, later ones are named after their
/// parameter types: `append`, `append_i32`, `append_str_i32_i32`.
fn overload_name(method: &BoundMethod, earlier: &[BoundMethod]) -> String {
    let taken = |name: &str| earlier.iter().any(|m| m.rust_name == name);
    if !taken(&method.rust_name) {
        return method.rust_name.clone();
    }
    let suffix: Vec<String> = method.params.iter().map(|(_, ty)| ty.suffix()).collect();
    let base = match suffix.is_empty() {
        true => format!("{}_empty", method.rust_name),
        false => format!("{}_{}", method.rust_name, suffix.join("_")),
    };
    let mut name = base.clone();
    let mut n = 1;
    while taken(&name) {
        name = format!("{}_{}", base, n);
        n += 1;
    }
    name
}

fn bound_base(ty: &Entry<TypeDef>, types: &[Entry<TypeDef>]) -> Option<usize> {
    let mut current = ty.get().extends.clone();
    while let Some(TypeDefOrRef::TypeDef(def)) = current {
        if let Some(index) = types.iter().position(|t| t.ptr_eq(&def)) {
            return Some(index);
        }
        current = def.get().extends.clone();
    }
    None
}

fn bind_method(method: &Entry<Method>, types: &[Entry<TypeDef>]) -> Option<BoundMethod> {
    let m = method.get();
    if !m.is_public()
        || m.signature.generic_param_count > 0
        || !m.signature.varargs.is_empty()
        || m.is_cctor()
    {
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
        .map(|(i, p)| match Ty::from_sig(p, types)? {
            Ty::Void => None,
            ty => Some((snake_case(&m.param_name(i)), ty)),
        })
        .collect::<Option<Vec<_>>>()?;
    let ret = match kind {
        MethodKind::Constructor => Ty::Void,
        _ => Ty::from_sig(&m.signature.ret, types)?,
    };
    let rust_name = match kind {
        MethodKind::Constructor => "new".to_owned(),
        _ => snake_case(&m.name),
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
