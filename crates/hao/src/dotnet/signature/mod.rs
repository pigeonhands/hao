//! Signature blobs (ECMA-335 II.23.2).

mod reader;
mod writer;

pub(crate) use reader::{decode_compressed_u32, SignatureReader, StandAloneSig, TokenResolver};
pub(crate) use writer::{encode_compressed_u32, SignatureWriter, TokenEncoder};

use bitflags::bitflags;
use strum::{FromRepr, IntoStaticStr};

use crate::dotnet::entries::TypeDefOrRef;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromRepr)]
#[repr(u8)]
pub enum ElementType {
    End = 0x00,
    Void = 0x01,
    Boolean = 0x02,
    Char = 0x03,
    I1 = 0x04,
    U1 = 0x05,
    I2 = 0x06,
    U2 = 0x07,
    I4 = 0x08,
    U4 = 0x09,
    I8 = 0x0A,
    U8 = 0x0B,
    R4 = 0x0C,
    R8 = 0x0D,
    String = 0x0E,
    Ptr = 0x0F,
    ByRef = 0x10,
    ValueType = 0x11,
    Class = 0x12,
    Var = 0x13,
    Array = 0x14,
    GenericInst = 0x15,
    TypedByRef = 0x16,
    I = 0x18,
    U = 0x19,
    FnPtr = 0x1B,
    Object = 0x1C,
    SzArray = 0x1D,
    MVar = 0x1E,
    CModReqd = 0x1F,
    CModOpt = 0x20,
    Internal = 0x21,
    Sentinel = 0x41,
    Pinned = 0x45,
}

/// Built-in types, named with their C# keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromRepr, IntoStaticStr)]
#[repr(u8)]
pub enum PrimitiveType {
    #[strum(serialize = "void")]
    Void = 0x01,
    #[strum(serialize = "bool")]
    Boolean = 0x02,
    #[strum(serialize = "char")]
    Char = 0x03,
    #[strum(serialize = "sbyte")]
    I1 = 0x04,
    #[strum(serialize = "byte")]
    U1 = 0x05,
    #[strum(serialize = "short")]
    I2 = 0x06,
    #[strum(serialize = "ushort")]
    U2 = 0x07,
    #[strum(serialize = "int")]
    I4 = 0x08,
    #[strum(serialize = "uint")]
    U4 = 0x09,
    #[strum(serialize = "long")]
    I8 = 0x0A,
    #[strum(serialize = "ulong")]
    U8 = 0x0B,
    #[strum(serialize = "float")]
    R4 = 0x0C,
    #[strum(serialize = "double")]
    R8 = 0x0D,
    #[strum(serialize = "string")]
    String = 0x0E,
    #[strum(serialize = "TypedReference")]
    TypedByRef = 0x16,
    #[strum(serialize = "nint")]
    I = 0x18,
    #[strum(serialize = "nuint")]
    U = 0x19,
    #[strum(serialize = "object")]
    Object = 0x1C,
}

impl PrimitiveType {
    pub fn keyword(self) -> &'static str {
        self.into()
    }

    /// Size in bytes for fixed size value types.
    pub fn size(self) -> Option<usize> {
        match self {
            Self::Boolean | Self::I1 | Self::U1 => Some(1),
            Self::Char | Self::I2 | Self::U2 => Some(2),
            Self::I4 | Self::U4 | Self::R4 => Some(4),
            Self::I8 | Self::U8 | Self::R8 => Some(8),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ArrayShape {
    pub rank: u32,
    pub sizes: Vec<u32>,
    pub lo_bounds: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeSig {
    Primitive(PrimitiveType),
    ValueType(TypeDefOrRef),
    Class(TypeDefOrRef),
    Ptr(Box<TypeSig>),
    ByRef(Box<TypeSig>),
    Pinned(Box<TypeSig>),
    SzArray(Box<TypeSig>),
    Array {
        element: Box<TypeSig>,
        shape: ArrayShape,
    },
    GenericInst {
        generic: Box<TypeSig>,
        args: Vec<TypeSig>,
    },
    /// Generic parameter of the enclosing type.
    Var(u32),
    /// Generic parameter of the enclosing method.
    MVar(u32),
    FnPtr(Box<MethodSig>),
    Modified {
        required: bool,
        modifier: TypeDefOrRef,
        ty: Box<TypeSig>,
    },
}

impl Default for TypeSig {
    fn default() -> Self {
        Self::Primitive(PrimitiveType::Void)
    }
}

impl TypeSig {
    pub const VOID: Self = Self::Primitive(PrimitiveType::Void);
    pub const BOOL: Self = Self::Primitive(PrimitiveType::Boolean);
    pub const CHAR: Self = Self::Primitive(PrimitiveType::Char);
    pub const INT32: Self = Self::Primitive(PrimitiveType::I4);
    pub const INT64: Self = Self::Primitive(PrimitiveType::I8);
    pub const FLOAT64: Self = Self::Primitive(PrimitiveType::R8);
    pub const STRING: Self = Self::Primitive(PrimitiveType::String);
    pub const OBJECT: Self = Self::Primitive(PrimitiveType::Object);

    pub fn class(ty: TypeDefOrRef) -> Self {
        Self::Class(ty)
    }

    pub fn value_type(ty: TypeDefOrRef) -> Self {
        Self::ValueType(ty)
    }

    /// `ty<args...>` for a generic class, e.g. `IEnumerable<int>`.
    pub fn generic_class(ty: TypeDefOrRef, args: Vec<TypeSig>) -> Self {
        Self::GenericInst {
            generic: Box::new(Self::Class(ty)),
            args,
        }
    }

    pub fn sz_array(element: TypeSig) -> Self {
        Self::SzArray(Box::new(element))
    }

    pub fn is_void(&self) -> bool {
        matches!(self, Self::Primitive(PrimitiveType::Void))
    }

    /// The type with any custom modifiers and `pinned` removed.
    pub fn unmodified(&self) -> &TypeSig {
        match self {
            Self::Modified { ty, .. } | Self::Pinned(ty) => ty.unmodified(),
            t => t,
        }
    }

    /// The [`TypeDefOrRef`] for class / valuetype signatures.
    pub fn type_def_or_ref(&self) -> Option<&TypeDefOrRef> {
        match self.unmodified() {
            Self::ValueType(t) | Self::Class(t) => Some(t),
            Self::GenericInst { generic, .. } => generic.type_def_or_ref(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, FromRepr)]
#[repr(u8)]
pub enum CallingConvention {
    #[default]
    Default = 0x0,
    C = 0x1,
    StdCall = 0x2,
    ThisCall = 0x3,
    FastCall = 0x4,
    VarArg = 0x5,
    Field = 0x6,
    LocalSig = 0x7,
    Property = 0x8,
    Unmanaged = 0x9,
    GenericInst = 0xA,
    NativeVarArg = 0xB,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct SignatureFlags: u8 {
        const GENERIC = 0x10;
        const HAS_THIS = 0x20;
        const EXPLICIT_THIS = 0x40;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MethodSig {
    pub calling_convention: CallingConvention,
    pub flags: SignatureFlags,
    pub generic_param_count: u32,
    pub ret: TypeSig,
    pub params: Vec<TypeSig>,
    /// Parameters after the vararg sentinel (call site signatures only).
    pub varargs: Vec<TypeSig>,
}

impl MethodSig {
    pub fn has_this(&self) -> bool {
        self.flags.contains(SignatureFlags::HAS_THIS)
    }

    pub fn new_static(ret: TypeSig, params: Vec<TypeSig>) -> Self {
        Self {
            ret,
            params,
            ..Default::default()
        }
    }

    pub fn new_instance(ret: TypeSig, params: Vec<TypeSig>) -> Self {
        Self {
            flags: SignatureFlags::HAS_THIS,
            ret,
            params,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FieldSig {
    pub ty: TypeSig,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PropertySig {
    pub has_this: bool,
    pub ty: TypeSig,
    pub params: Vec<TypeSig>,
}

/// Signature of a [`MemberRef`](crate::dotnet::entries::MemberRef).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemberRefSig {
    Method(MethodSig),
    Field(FieldSig),
}

impl Default for MemberRefSig {
    fn default() -> Self {
        Self::Field(FieldSig::default())
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, collections::HashMap};

    use super::*;
    use crate::{
        dotnet::entries::{Entry, TypeRef},
        error::Result,
    };

    #[derive(Default)]
    struct Tokens(RefCell<HashMap<u32, Entry<TypeRef>>>);

    impl TokenResolver for Tokens {
        fn type_def_or_ref(&self, coded: u32) -> Result<TypeDefOrRef> {
            let entry = self
                .0
                .borrow_mut()
                .entry(coded)
                .or_insert_with(|| {
                    Entry::new(TypeRef {
                        name: format!("T{}", coded),
                        ..Default::default()
                    })
                })
                .clone();
            Ok(TypeDefOrRef::TypeRef(entry))
        }
    }

    impl TokenEncoder for Tokens {
        fn type_def_or_ref(&mut self, ty: &TypeDefOrRef) -> Result<u32> {
            let TypeDefOrRef::TypeRef(t) = ty else {
                unreachable!()
            };
            Ok(*self.0.borrow().iter().find(|(_, e)| e.ptr_eq(t)).unwrap().0)
        }
    }

    fn method(blob: &[u8]) -> MethodSig {
        SignatureReader::new(blob, &Tokens::default())
            .read_method_sig()
            .unwrap()
    }

    fn write_method(sig: &MethodSig, tokens: &mut Tokens) -> Vec<u8> {
        let mut w = SignatureWriter::new(tokens);
        w.write_method_sig(sig).unwrap();
        w.finish()
    }

    #[test]
    fn instance_method() {
        let sig = method(&[0x20, 0x02, 0x08, 0x08, 0x0E]);
        assert!(sig.has_this());
        assert_eq!(sig.ret, TypeSig::Primitive(PrimitiveType::I4));
        assert_eq!(
            sig.params,
            [
                TypeSig::Primitive(PrimitiveType::I4),
                TypeSig::Primitive(PrimitiveType::String)
            ]
        );
    }

    #[test]
    fn signed_and_unsigned_bytes_are_not_swapped() {
        let tokens = Tokens::default();
        let field = SignatureReader::new(&[0x06, 0x04], &tokens)
            .read_field_sig()
            .unwrap();
        assert_eq!(field.ty, TypeSig::Primitive(PrimitiveType::I1));
        assert_eq!(PrimitiveType::I1.keyword(), "sbyte");
        let field = SignatureReader::new(&[0x06, 0x19], &tokens)
            .read_field_sig()
            .unwrap();
        assert_eq!(field.ty, TypeSig::Primitive(PrimitiveType::U));
    }

    #[test]
    fn varargs_after_sentinel() {
        let sig = method(&[0x05, 0x03, 0x01, 0x08, 0x41, 0x0A, 0x0D]);
        assert_eq!(sig.calling_convention, CallingConvention::VarArg);
        assert_eq!(sig.params, [TypeSig::Primitive(PrimitiveType::I4)]);
        assert_eq!(
            sig.varargs,
            [
                TypeSig::Primitive(PrimitiveType::I8),
                TypeSig::Primitive(PrimitiveType::R8)
            ]
        );
    }

    #[test]
    fn array_shape_with_negative_bounds() {
        let blob = [0x06, 0x14, 0x08, 0x02, 0x02, 0x03, 0x04, 0x02, 0x00, 0x7B];
        let tokens = Tokens::default();
        let field = SignatureReader::new(&blob, &tokens)
            .read_field_sig()
            .unwrap();
        let TypeSig::Array { shape, .. } = field.ty else {
            panic!()
        };
        assert_eq!(
            shape,
            ArrayShape {
                rank: 2,
                sizes: vec![3, 4],
                lo_bounds: vec![0, -3]
            }
        );
    }

    #[test]
    fn compressed_signed_integers() {
        let mut tokens = Tokens::default();
        for (value, encoded) in [
            (3, &[0x06][..]),
            (-3, &[0x7B]),
            (64, &[0x80, 0x80]),
            (-64, &[0x01]),
            (8192, &[0xC0, 0x00, 0x40, 0x00]),
            (-8192, &[0x80, 0x01]),
            (268435455, &[0xDF, 0xFF, 0xFF, 0xFE]),
            (-268435456, &[0xC0, 0x00, 0x00, 0x01]),
        ] {
            let mut w = SignatureWriter::new(&mut tokens);
            w.write_compressed_i32(value).unwrap();
            assert_eq!(w.finish(), encoded, "encoding {}", value);
            let decoded = SignatureReader::new(encoded, &Tokens::default())
                .read_compressed_i32()
                .unwrap();
            assert_eq!(decoded, value);
        }
    }

    #[test]
    fn generic_method_with_modifiers_round_trips() {
        // generic<1> instance void (class T5, valuetype T6<!!0>[], modreq(T9) int32&, int32[0...,-3...])
        let blob = [
            0x30, 0x01, 0x04, 0x01, 0x12, 0x05, 0x1D, 0x15, 0x11, 0x06, 0x01, 0x1E, 0x00, 0x1F,
            0x09, 0x10, 0x08, 0x14, 0x08, 0x02, 0x00, 0x02, 0x00, 0x7B,
        ];
        let mut tokens = Tokens::default();
        let sig = SignatureReader::new(&blob, &tokens)
            .read_method_sig()
            .unwrap();
        assert_eq!(sig.generic_param_count, 1);
        assert_eq!(sig.params.len(), 4);
        assert!(matches!(
            &sig.params[2],
            TypeSig::Modified { required: true, .. }
        ));
        assert_eq!(write_method(&sig, &mut tokens), blob);
    }
}
