use super::{Entry, MemberRef, Method, ModuleRef};
use crate::dotnet::{
    md::streams::tables_stream::PInvokeMapFlags,
    signature::ElementType,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomAttributeCtor {
    Method(Entry<Method>),
    MemberRef(Entry<MemberRef>),
}

/// A custom attribute. The value is kept as the raw blob (II.23.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomAttribute {
    pub constructor: CustomAttributeCtor,
    pub value: Vec<u8>,
}

impl CustomAttribute {
    /// Full name of the attribute type.
    pub fn type_name(&self) -> String {
        match &self.constructor {
            CustomAttributeCtor::Method(m) => m
                .get()
                .declaring_type
                .upgrade()
                .map(|t| t.get().full_name())
                .unwrap_or_default(),
            CustomAttributeCtor::MemberRef(m) => m
                .get()
                .parent
                .as_type()
                .map(|t| t.full_name())
                .unwrap_or_default(),
        }
    }
}

/// A compile time constant for a field, parameter or property.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Constant {
    pub element_type: ElementType,
    pub value: Vec<u8>,
}

impl Constant {
    pub fn as_i64(&self) -> Option<i64> {
        let v = &self.value;
        Some(match self.element_type {
            ElementType::Boolean | ElementType::U1 => *v.first()? as i64,
            ElementType::I1 => *v.first()? as i8 as i64,
            ElementType::I2 => i16::from_le_bytes(v.get(..2)?.try_into().ok()?) as i64,
            ElementType::U2 | ElementType::Char => {
                u16::from_le_bytes(v.get(..2)?.try_into().ok()?) as i64
            }
            ElementType::I4 => i32::from_le_bytes(v.get(..4)?.try_into().ok()?) as i64,
            ElementType::U4 => u32::from_le_bytes(v.get(..4)?.try_into().ok()?) as i64,
            ElementType::I8 | ElementType::U8 => {
                i64::from_le_bytes(v.get(..8)?.try_into().ok()?)
            }
            _ => return None,
        })
    }

    pub fn as_string(&self) -> Option<String> {
        if self.element_type != ElementType::String {
            return None;
        }
        let units: Vec<u16> = self
            .value
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16(&units).ok()
    }
}

/// Declarative security (II.22.11). The permission set is the raw blob.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclSecurity {
    pub action: u16,
    pub permission_set: Vec<u8>,
    pub custom_attributes: Vec<super::CustomAttribute>,
}

/// P/Invoke information for a method or field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImplMap {
    pub flags: PInvokeMapFlags,
    pub import_name: String,
    pub scope: Entry<ModuleRef>,
}
