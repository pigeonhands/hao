//! The .NET types bindings can pass, and how each crosses the native boundary.

use hao::dotnet::{
    entries::{Entry, TypeDef, TypeDefOrRef},
    il::OpCode,
    signature::{PrimitiveType, TypeSig},
};

use crate::names::snake_case;

#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    Void,
    Bool,
    Primitive(PrimitiveType),
    String,
    /// A reference type, passed as a `GCHandle`. `bound` is the index of its
    /// generated struct, if it has one.
    Object {
        ty: Option<TypeDefOrRef>,
        bound: Option<usize>,
    },
}

impl Ty {
    /// `None` for types bindings can't pass (structs, arrays, generics, ...).
    pub fn from_sig(sig: &TypeSig, bound_types: &[Entry<TypeDef>]) -> Option<Self> {
        Some(match sig.unmodified() {
            TypeSig::Primitive(PrimitiveType::Void) => Ty::Void,
            TypeSig::Primitive(PrimitiveType::Boolean) => Ty::Bool,
            TypeSig::Primitive(PrimitiveType::String) => Ty::String,
            TypeSig::Primitive(PrimitiveType::Object) => Ty::Object {
                ty: None,
                bound: None,
            },
            TypeSig::Primitive(PrimitiveType::TypedByRef) => return None,
            TypeSig::Primitive(p) => Ty::Primitive(*p),
            TypeSig::Class(TypeDefOrRef::TypeSpec(_)) => return None,
            TypeSig::Class(ty) => {
                let bound = match ty {
                    TypeDefOrRef::TypeDef(def) => bound_types.iter().position(|b| b.ptr_eq(def)),
                    _ => None,
                };
                Ty::Object {
                    ty: Some(ty.clone()),
                    bound,
                }
            }
            _ => return None,
        })
    }

    /// The thunk parameters carrying a value of this type: strings are a
    /// pointer and length, objects a `GCHandle`.
    pub fn native(&self) -> Vec<TypeSig> {
        match self {
            Ty::Void => vec![],
            Ty::Bool => vec![TypeSig::Primitive(PrimitiveType::U1)],
            Ty::Primitive(PrimitiveType::Char) => vec![TypeSig::Primitive(PrimitiveType::U2)],
            Ty::Primitive(p) => vec![TypeSig::Primitive(*p)],
            Ty::String => vec![TypeSig::NATIVE_INT, TypeSig::INT32],
            Ty::Object { .. } => vec![TypeSig::NATIVE_INT],
        }
    }

    /// Strings and objects come back as `GCHandle`s.
    pub fn returned_as_handle(&self) -> bool {
        matches!(self, Ty::String | Ty::Object { .. })
    }

    /// The `stind` that writes a returned value through a pointer.
    pub fn store_op(&self) -> OpCode {
        match self {
            Ty::Bool => OpCode::Stind_I1,
            Ty::Primitive(p) => match p {
                PrimitiveType::I1 | PrimitiveType::U1 => OpCode::Stind_I1,
                PrimitiveType::I2 | PrimitiveType::U2 | PrimitiveType::Char => OpCode::Stind_I2,
                PrimitiveType::I4 | PrimitiveType::U4 => OpCode::Stind_I4,
                PrimitiveType::I8 | PrimitiveType::U8 => OpCode::Stind_I8,
                PrimitiveType::R4 => OpCode::Stind_R4,
                PrimitiveType::R8 => OpCode::Stind_R8,
                _ => OpCode::Stind_I,
            },
            _ => OpCode::Stind_I,
        }
    }

    /// A short name telling overloads apart: `append_i32`, `new_str`.
    pub fn suffix(&self) -> String {
        match self {
            Ty::Void => "void".into(),
            Ty::Bool => "bool".into(),
            Ty::String => "str".into(),
            Ty::Primitive(PrimitiveType::Char) => "char".into(),
            Ty::Primitive(p) => primitive_name(*p).into(),
            Ty::Object { ty: Some(ty), .. } => snake_case(&ty.name()),
            Ty::Object { ty: None, .. } => "object".into(),
        }
    }
}

/// The Rust type of a primitive. `char` is a UTF-16 code unit.
pub fn primitive_name(p: PrimitiveType) -> &'static str {
    match p {
        PrimitiveType::I1 => "i8",
        PrimitiveType::U1 => "u8",
        PrimitiveType::I2 => "i16",
        PrimitiveType::U2 | PrimitiveType::Char => "u16",
        PrimitiveType::I4 => "i32",
        PrimitiveType::U4 => "u32",
        PrimitiveType::I8 => "i64",
        PrimitiveType::U8 => "u64",
        PrimitiveType::R4 => "f32",
        PrimitiveType::R8 => "f64",
        PrimitiveType::I => "isize",
        _ => "usize",
    }
}
