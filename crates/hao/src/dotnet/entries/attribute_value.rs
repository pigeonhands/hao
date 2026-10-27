//! Decoding and encoding of custom attribute values (ECMA-335 II.23.3).

use super::{CustomAttribute, CustomAttributeCtor, Entry, TypeDef, TypeDefOrRef};
use crate::{
    dotnet::signature::{
        decode_compressed_u32, encode_compressed_u32, MemberRefSig, PrimitiveType, TypeSig,
    },
    error::{HaoError, Result},
    Module,
};

const PROLOG: u16 = 0x0001;
const NAMED_FIELD: u8 = 0x53;
const NAMED_PROPERTY: u8 = 0x54;
const TAG_SZARRAY: u8 = 0x1D;
const TAG_TYPE: u8 = 0x50;
const TAG_BOXED: u8 = 0x51;
const TAG_ENUM: u8 = 0x55;

/// The type of a custom attribute argument.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "reflect", derive(facet::Facet))]
#[cfg_attr(feature = "reflect", repr(C))]
pub enum CAType {
    /// `bool` through `double`, and `string`.
    Primitive(PrimitiveType),
    /// `System.Type`, stored as a serialized type name.
    Type,
    /// `object`, the value carries its own type.
    Object,
    Enum {
        type_name: String,
        underlying: PrimitiveType,
    },
    SzArray(Box<CAType>),
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "reflect", derive(facet::Facet))]
#[cfg_attr(feature = "reflect", repr(C))]
pub enum CAValue {
    Bool(bool),
    Char(u16),
    I1(i8),
    U1(u8),
    I2(i16),
    U2(u16),
    I4(i32),
    U4(u32),
    I8(i64),
    U8(u64),
    R4(f32),
    R8(f64),
    String(Option<String>),
    /// A serialized type name such as `System.String, System.Runtime`.
    Type(Option<String>),
    Enum {
        type_name: String,
        value: Box<CAValue>,
    },
    /// An `object` argument.
    Boxed(Box<CAValue>),
    Array {
        element: CAType,
        values: Option<Vec<CAValue>>,
    },
}

impl CAValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(Some(s)) | Self::Type(Some(s)) => Some(s),
            Self::Boxed(v) => v.as_str(),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        Some(match self {
            Self::Bool(v) => *v as i64,
            Self::Char(v) => *v as i64,
            Self::I1(v) => *v as i64,
            Self::U1(v) => *v as i64,
            Self::I2(v) => *v as i64,
            Self::U2(v) => *v as i64,
            Self::I4(v) => *v as i64,
            Self::U4(v) => *v as i64,
            Self::I8(v) => *v,
            Self::U8(v) => *v as i64,
            Self::Enum { value, .. } | Self::Boxed(value) => return value.as_i64(),
            _ => return None,
        })
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(v) => Some(*v),
            Self::Boxed(v) => v.as_bool(),
            _ => None,
        }
    }

    /// The type this value would be serialized with when boxed or named.
    pub fn ca_type(&self) -> CAType {
        let p = |p| CAType::Primitive(p);
        match self {
            Self::Bool(_) => p(PrimitiveType::Boolean),
            Self::Char(_) => p(PrimitiveType::Char),
            Self::I1(_) => p(PrimitiveType::I1),
            Self::U1(_) => p(PrimitiveType::U1),
            Self::I2(_) => p(PrimitiveType::I2),
            Self::U2(_) => p(PrimitiveType::U2),
            Self::I4(_) => p(PrimitiveType::I4),
            Self::U4(_) => p(PrimitiveType::U4),
            Self::I8(_) => p(PrimitiveType::I8),
            Self::U8(_) => p(PrimitiveType::U8),
            Self::R4(_) => p(PrimitiveType::R4),
            Self::R8(_) => p(PrimitiveType::R8),
            Self::String(_) => p(PrimitiveType::String),
            Self::Type(_) => CAType::Type,
            Self::Boxed(_) => CAType::Object,
            Self::Enum { type_name, value } => CAType::Enum {
                type_name: type_name.clone(),
                underlying: match value.ca_type() {
                    CAType::Primitive(p) => p,
                    _ => PrimitiveType::I4,
                },
            },
            Self::Array { element, .. } => CAType::SzArray(Box::new(element.clone())),
        }
    }
}

impl From<&str> for CAValue {
    fn from(value: &str) -> Self {
        Self::String(Some(value.into()))
    }
}

impl From<i32> for CAValue {
    fn from(value: i32) -> Self {
        Self::I4(value)
    }
}

impl From<bool> for CAValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "reflect", derive(facet::Facet))]
pub struct CANamedArgument {
    pub is_field: bool,
    pub name: String,
    pub ty: CAType,
    pub value: CAValue,
}

impl CANamedArgument {
    pub fn property(name: impl Into<String>, value: CAValue) -> Self {
        Self {
            is_field: false,
            name: name.into(),
            ty: value.ca_type(),
            value,
        }
    }

    pub fn field(name: impl Into<String>, value: CAValue) -> Self {
        Self {
            is_field: true,
            ..Self::property(name, value)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "reflect", derive(facet::Facet))]
pub struct CustomAttributeValue {
    pub fixed: Vec<CAValue>,
    pub named: Vec<CANamedArgument>,
}

impl CustomAttributeValue {
    pub fn named(&self, name: &str) -> Option<&CAValue> {
        self.named.iter().find(|n| n.name == name).map(|n| &n.value)
    }
}

fn error(msg: impl Into<String>) -> HaoError {
    HaoError::InvalidCustomAttribute(msg.into())
}

fn constructor_params(ctor: &CustomAttributeCtor) -> Result<Vec<TypeSig>> {
    match ctor {
        CustomAttributeCtor::Method(m) => Ok(m.get().signature.params.clone()),
        CustomAttributeCtor::MemberRef(m) => match &m.get().signature {
            MemberRefSig::Method(sig) => Ok(sig.params.clone()),
            MemberRefSig::Field(_) => Err(error("constructor is a field reference")),
        },
    }
}

fn underlying_type(ty: &Entry<TypeDef>) -> Result<PrimitiveType> {
    let ty = ty.get();
    let field = ty
        .enum_underlying_field()
        .ok_or_else(|| error(format!("{} is not an enum", ty.full_name())))?;
    let underlying = field.get().signature.ty.clone();
    match underlying {
        TypeSig::Primitive(p) => Ok(p),
        _ => Err(error(format!(
            "{} has a non primitive underlying type",
            ty.full_name()
        ))),
    }
}

/// `Ns.Outer+Inner, Assembly, Version=...` without the assembly qualification.
fn strip_assembly(type_name: &str) -> &str {
    type_name.split(',').next().unwrap_or(type_name).trim()
}

impl Module {
    /// Finds a type by its serialized (reflection) name, e.g. `Ns.Outer+Inner, Assembly`,
    /// in this module or a loaded dependency.
    pub fn resolve_serialized_type(&self, type_name: &str) -> Option<Entry<TypeDef>> {
        let full_name = strip_assembly(type_name).replace('+', "/");
        if let Some(t) = self.find_type_by_full_name(&full_name) {
            return Some(t);
        }
        let (outer, nested) = full_name.split_once('/').unwrap_or((&full_name, ""));
        let (namespace, name) = outer.rsplit_once('.').unwrap_or(("", outer));
        let mut ty = self.assembly_refs.iter().find_map(|a| {
            let module = a.get().resolved.clone()?;
            module.resolve_type(namespace, name)
        })?;
        for part in nested.split('/').filter(|p| !p.is_empty()) {
            let next = ty
                .get()
                .nested_types
                .iter()
                .find(|t| t.get().name == part)
                .cloned()?;
            ty = next;
        }
        Some(ty)
    }
}

fn serialized_name(ty: &TypeDefOrRef) -> String {
    ty.full_name().replace('/', "+")
}

struct Reader<'a> {
    data: &'a [u8],
    position: usize,
    module: &'a Module,
}

impl<'a> Reader<'a> {
    fn bytes<const N: usize>(&mut self) -> Result<[u8; N]> {
        let slice = self
            .data
            .get(self.position..self.position + N)
            .ok_or_else(|| error("unexpected end of blob"))?;
        self.position += N;
        Ok(slice.try_into().unwrap())
    }

    fn u8(&mut self) -> Result<u8> {
        self.bytes::<1>().map(|b| b[0])
    }

    fn compressed(&mut self) -> Result<u32> {
        let (value, len) = decode_compressed_u32(&self.data[self.position.min(self.data.len())..])
            .ok_or_else(|| error("invalid compressed integer"))?;
        self.position += len;
        Ok(value)
    }

    fn ser_string(&mut self) -> Result<Option<String>> {
        if self.data.get(self.position) == Some(&0xFF) {
            self.position += 1;
            return Ok(None);
        }
        let len = self.compressed()? as usize;
        let bytes = self
            .data
            .get(self.position..self.position + len)
            .ok_or_else(|| error("string runs past the end of the blob"))?;
        self.position += len;
        String::from_utf8(bytes.to_vec())
            .map(Some)
            .map_err(|_| error("invalid utf-8 string"))
    }

    fn sig_type(&self, sig: &TypeSig) -> Result<CAType> {
        Ok(match sig.unmodified() {
            TypeSig::Primitive(PrimitiveType::Object) => CAType::Object,
            TypeSig::Primitive(p) => CAType::Primitive(*p),
            TypeSig::SzArray(element) => CAType::SzArray(Box::new(self.sig_type(element)?)),
            TypeSig::Class(t) if t.full_name_is("System", "Type") => CAType::Type,
            TypeSig::ValueType(t) => {
                let def = t.resolve().ok_or_else(|| {
                    error(format!(
                        "can't resolve enum {}, load the module's dependencies",
                        t.full_name()
                    ))
                })?;
                CAType::Enum {
                    type_name: serialized_name(t),
                    underlying: underlying_type(&def)?,
                }
            }
            other => return Err(error(format!("unsupported argument type {:?}", other))),
        })
    }

    fn tagged_type(&mut self) -> Result<CAType> {
        let tag = self.u8()?;
        Ok(match tag {
            TAG_SZARRAY => CAType::SzArray(Box::new(self.tagged_type()?)),
            TAG_TYPE => CAType::Type,
            TAG_BOXED => CAType::Object,
            TAG_ENUM => {
                let type_name = self
                    .ser_string()?
                    .ok_or_else(|| error("null enum type name"))?;
                let def = self
                    .module
                    .resolve_serialized_type(&type_name)
                    .ok_or_else(|| {
                        error(format!(
                            "can't resolve enum {}, load the module's dependencies",
                            type_name
                        ))
                    })?;
                CAType::Enum {
                    underlying: underlying_type(&def)?,
                    type_name,
                }
            }
            tag => CAType::Primitive(
                PrimitiveType::from_repr(tag)
                    .filter(|p| p.size().is_some() || *p == PrimitiveType::String)
                    .ok_or_else(|| error(format!("invalid element type {:#x}", tag)))?,
            ),
        })
    }

    fn value(&mut self, ty: &CAType) -> Result<CAValue> {
        Ok(match ty {
            CAType::Primitive(p) => match p {
                PrimitiveType::Boolean => CAValue::Bool(self.u8()? != 0),
                PrimitiveType::Char => CAValue::Char(u16::from_le_bytes(self.bytes()?)),
                PrimitiveType::I1 => CAValue::I1(self.u8()? as i8),
                PrimitiveType::U1 => CAValue::U1(self.u8()?),
                PrimitiveType::I2 => CAValue::I2(i16::from_le_bytes(self.bytes()?)),
                PrimitiveType::U2 => CAValue::U2(u16::from_le_bytes(self.bytes()?)),
                PrimitiveType::I4 => CAValue::I4(i32::from_le_bytes(self.bytes()?)),
                PrimitiveType::U4 => CAValue::U4(u32::from_le_bytes(self.bytes()?)),
                PrimitiveType::I8 => CAValue::I8(i64::from_le_bytes(self.bytes()?)),
                PrimitiveType::U8 => CAValue::U8(u64::from_le_bytes(self.bytes()?)),
                PrimitiveType::R4 => CAValue::R4(f32::from_le_bytes(self.bytes()?)),
                PrimitiveType::R8 => CAValue::R8(f64::from_le_bytes(self.bytes()?)),
                PrimitiveType::String => CAValue::String(self.ser_string()?),
                p => {
                    return Err(error(format!(
                        "{} can't be an attribute argument",
                        p.keyword()
                    )))
                }
            },
            CAType::Type => CAValue::Type(self.ser_string()?),
            CAType::Object => {
                let ty = self.tagged_type()?;
                CAValue::Boxed(Box::new(self.value(&ty)?))
            }
            CAType::Enum {
                type_name,
                underlying,
            } => CAValue::Enum {
                type_name: type_name.clone(),
                value: Box::new(self.value(&CAType::Primitive(*underlying))?),
            },
            CAType::SzArray(element) => {
                let count = u32::from_le_bytes(self.bytes()?);
                let values = if count == u32::MAX {
                    None
                } else {
                    if count as usize > self.data.len() {
                        return Err(error("array is larger than the blob"));
                    }
                    Some(
                        (0..count)
                            .map(|_| self.value(element))
                            .collect::<Result<_>>()?,
                    )
                };
                CAValue::Array {
                    element: (**element).clone(),
                    values,
                }
            }
        })
    }
}

struct Writer(Vec<u8>);

impl Writer {
    fn ser_string(&mut self, value: &Option<String>) -> Result<()> {
        match value {
            None => self.0.push(0xFF),
            Some(s) => {
                encode_compressed_u32(&mut self.0, s.len() as u32)?;
                self.0.extend_from_slice(s.as_bytes());
            }
        }
        Ok(())
    }

    fn tagged_type(&mut self, ty: &CAType) -> Result<()> {
        match ty {
            CAType::Primitive(p) => self.0.push(*p as u8),
            CAType::Type => self.0.push(TAG_TYPE),
            CAType::Object => self.0.push(TAG_BOXED),
            CAType::Enum { type_name, .. } => {
                self.0.push(TAG_ENUM);
                self.ser_string(&Some(type_name.clone()))?;
            }
            CAType::SzArray(element) => {
                self.0.push(TAG_SZARRAY);
                self.tagged_type(element)?;
            }
        }
        Ok(())
    }

    fn value(&mut self, value: &CAValue) -> Result<()> {
        match value {
            CAValue::Bool(v) => self.0.push(*v as u8),
            CAValue::Char(v) => self.0.extend_from_slice(&v.to_le_bytes()),
            CAValue::I1(v) => self.0.push(*v as u8),
            CAValue::U1(v) => self.0.push(*v),
            CAValue::I2(v) => self.0.extend_from_slice(&v.to_le_bytes()),
            CAValue::U2(v) => self.0.extend_from_slice(&v.to_le_bytes()),
            CAValue::I4(v) => self.0.extend_from_slice(&v.to_le_bytes()),
            CAValue::U4(v) => self.0.extend_from_slice(&v.to_le_bytes()),
            CAValue::I8(v) => self.0.extend_from_slice(&v.to_le_bytes()),
            CAValue::U8(v) => self.0.extend_from_slice(&v.to_le_bytes()),
            CAValue::R4(v) => self.0.extend_from_slice(&v.to_le_bytes()),
            CAValue::R8(v) => self.0.extend_from_slice(&v.to_le_bytes()),
            CAValue::String(s) | CAValue::Type(s) => self.ser_string(s)?,
            CAValue::Enum { value, .. } => self.value(value)?,
            CAValue::Boxed(inner) => {
                self.tagged_type(&inner.ca_type())?;
                self.value(inner)?;
            }
            CAValue::Array { values, .. } => match values {
                None => self.0.extend_from_slice(&u32::MAX.to_le_bytes()),
                Some(values) => {
                    self.0
                        .extend_from_slice(&(values.len() as u32).to_le_bytes());
                    for v in values {
                        self.value(v)?;
                    }
                }
            },
        }
        Ok(())
    }
}

impl CustomAttributeValue {
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut w = Writer(PROLOG.to_le_bytes().to_vec());
        for value in &self.fixed {
            w.value(value)?;
        }
        w.0.extend_from_slice(&(self.named.len() as u16).to_le_bytes());
        for named in &self.named {
            w.0.push(if named.is_field {
                NAMED_FIELD
            } else {
                NAMED_PROPERTY
            });
            w.tagged_type(&named.ty)?;
            w.ser_string(&Some(named.name.clone()))?;
            w.value(&named.value)?;
        }
        Ok(w.0)
    }
}

impl CustomAttribute {
    pub fn new(constructor: CustomAttributeCtor, value: &CustomAttributeValue) -> Result<Self> {
        Ok(Self {
            constructor,
            value: value.encode()?,
        })
    }

    /// Decodes the arguments. `module` is used to find enum types, so load its
    /// dependencies first if attributes use enums from other assemblies.
    pub fn decode(&self, module: &Module) -> Result<CustomAttributeValue> {
        let params = constructor_params(&self.constructor)?;
        let mut r = Reader {
            data: &self.value,
            position: 0,
            module,
        };
        if self.value.is_empty() {
            return Ok(CustomAttributeValue::default());
        }
        if u16::from_le_bytes(r.bytes()?) != PROLOG {
            return Err(error("missing prolog"));
        }

        let fixed = params
            .iter()
            .map(|p| {
                let ty = r.sig_type(p)?;
                r.value(&ty)
            })
            .collect::<Result<Vec<_>>>()?;

        let count = u16::from_le_bytes(r.bytes()?);
        let named = (0..count)
            .map(|_| {
                let is_field = match r.u8()? {
                    NAMED_FIELD => true,
                    NAMED_PROPERTY => false,
                    _ => return Err(error("invalid named argument kind")),
                };
                let ty = r.tagged_type()?;
                let name = r.ser_string()?.ok_or_else(|| error("null argument name"))?;
                let value = r.value(&ty)?;
                Ok(CANamedArgument {
                    is_field,
                    name,
                    ty,
                    value,
                })
            })
            .collect::<Result<_>>()?;

        Ok(CustomAttributeValue { fixed, named })
    }
}
