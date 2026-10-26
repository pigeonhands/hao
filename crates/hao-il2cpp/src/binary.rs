//! The game binary (`GameAssembly`): the registration tables and the
//! `Il2CppType` table that metadata type indices refer to.

use pewter::elf::{ElfFile, RelocatedPointers, SegmentFlags, SegmentType};

use crate::{
    error::{Error, Result},
    metadata::{Metadata, TypeDefinitionIndex},
};

/// Reads virtual addresses of a loaded binary as they will be after relocation.
pub trait BinaryImage {
    fn is_64(&self) -> bool;
    fn bytes(&self, vaddr: u64, len: usize) -> Option<&[u8]>;
    /// A pointer sized value, after relocation.
    fn pointer(&self, vaddr: u64) -> Option<u64>;
    /// Address ranges of writable or read-only data that may hold registration tables.
    fn data_ranges(&self) -> Vec<(u64, u64)>;

    fn pointer_size(&self) -> u64 {
        if self.is_64() {
            8
        } else {
            4
        }
    }

    fn u32(&self, vaddr: u64) -> Option<u32> {
        self.bytes(vaddr, 4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
    }

    fn u8(&self, vaddr: u64) -> Option<u8> {
        self.bytes(vaddr, 1).map(|b| b[0])
    }

    fn is_mapped(&self, vaddr: u64) -> bool {
        vaddr != 0 && self.bytes(vaddr, 1).is_some()
    }
}

/// A `GameAssembly.so` (or any Il2Cpp ELF binary).
pub struct ElfImage<'a> {
    elf: ElfFile<'a>,
    pointers: RelocatedPointers,
}

impl<'a> ElfImage<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self> {
        let elf = ElfFile::parse(data).map_err(|e| Error::Binary(format!("{:?}", e)))?;
        let pointers = elf
            .relocated_pointers()
            .map_err(|e| Error::Binary(format!("{:?}", e)))?;
        Ok(Self { elf, pointers })
    }
}

impl BinaryImage for ElfImage<'_> {
    fn is_64(&self) -> bool {
        self.elf.is_64()
    }

    fn bytes(&self, vaddr: u64, len: usize) -> Option<&[u8]> {
        self.elf.data_at_vaddr(vaddr)?.get(..len)
    }

    fn pointer(&self, vaddr: u64) -> Option<u64> {
        let value = self.elf.read_pointer(vaddr, &self.pointers)?;
        Some(if self.is_64() {
            value
        } else {
            value & 0xFFFF_FFFF
        })
    }

    fn data_ranges(&self) -> Vec<(u64, u64)> {
        self.elf
            .program_headers
            .iter()
            .filter(|p| {
                p.segment_type == SegmentType::Load && !p.flags.contains(SegmentFlags::EXECUTE)
            })
            .map(|p| (p.vaddr, p.vaddr + p.file_size))
            .collect()
    }
}

/// `Il2CppTypeEnum`, the same values as ECMA-335 element types.
pub mod element {
    pub const VOID: u8 = 0x01;
    pub const STRING: u8 = 0x0E;
    pub const PTR: u8 = 0x0F;
    pub const BYREF: u8 = 0x10;
    pub const VALUETYPE: u8 = 0x11;
    pub const CLASS: u8 = 0x12;
    pub const VAR: u8 = 0x13;
    pub const ARRAY: u8 = 0x14;
    pub const GENERICINST: u8 = 0x15;
    pub const TYPEDBYREF: u8 = 0x16;
    pub const FNPTR: u8 = 0x1B;
    pub const OBJECT: u8 = 0x1C;
    pub const SZARRAY: u8 = 0x1D;
    pub const MVAR: u8 = 0x1E;
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    /// `void`, `bool`, ..., `string`, `object`, `IntPtr`, `TypedReference`.
    Primitive(u8),
    Class(TypeDefinitionIndex),
    ValueType(TypeDefinitionIndex),
    SzArray(Box<Il2CppType>),
    Array {
        element: Box<Il2CppType>,
        rank: u8,
    },
    Ptr(Box<Il2CppType>),
    GenericInst {
        generic: Box<Il2CppType>,
        args: Vec<Il2CppType>,
    },
    /// Index into [`Metadata::generic_parameters`].
    Var(i32),
    MVar(i32),
    FnPtr,
    Unknown(u8),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Il2CppType {
    pub kind: TypeKind,
    pub attrs: u16,
    pub byref: bool,
}

/// `Il2CppMetadataRegistration` (v29).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataRegistration {
    pub address: u64,
    pub generic_classes: (u64, u64),
    pub generic_insts: (u64, u64),
    pub generic_method_table: (u64, u64),
    pub types: (u64, u64),
    pub method_specs: (u64, u64),
    pub field_offsets: (u64, u64),
    pub type_definition_sizes: (u64, u64),
}

impl MetadataRegistration {
    /// Finds the registration by its two counts that equal the number of type definitions
    /// (`fieldOffsetsCount` and `typeDefinitionsSizesCount`).
    pub fn find(image: &impl BinaryImage, metadata: &Metadata) -> Result<Self> {
        let p = image.pointer_size();
        let count = metadata.type_definitions.len() as u64;
        let word = |v: u64| image.pointer(v).unwrap_or(0);
        let pair = |v: u64| (word(v), word(v + p));
        let mut found = Vec::new();
        for (start, end) in image.data_ranges() {
            let mut vaddr = start;
            while vaddr + 4 * p <= end {
                if word(vaddr) == count
                    && word(vaddr + 2 * p) == count
                    && image.is_mapped(word(vaddr + p))
                    && image.is_mapped(word(vaddr + 3 * p))
                    && vaddr >= start + 10 * p
                {
                    let base = vaddr - 10 * p;
                    let types = pair(base + 6 * p);
                    if types.0 >= count && image.is_mapped(types.1) {
                        found.push(Self {
                            address: base,
                            generic_classes: pair(base),
                            generic_insts: pair(base + 2 * p),
                            generic_method_table: pair(base + 4 * p),
                            types,
                            method_specs: pair(base + 8 * p),
                            field_offsets: pair(base + 10 * p),
                            type_definition_sizes: pair(base + 12 * p),
                        });
                    }
                }
                vaddr += p;
            }
        }
        match found.len() {
            1 => Ok(found.remove(0)),
            0 => Err(Error::Binary("MetadataRegistration not found".into())),
            n => Err(Error::Binary(format!(
                "{} candidates for MetadataRegistration",
                n
            ))),
        }
    }
}

/// The game binary's type table, decoded.
pub struct Binary {
    pub metadata_registration: MetadataRegistration,
    pub types: Vec<Il2CppType>,
}

impl Binary {
    pub fn read(image: &impl BinaryImage, metadata: &Metadata) -> Result<Self> {
        let registration = MetadataRegistration::find(image, metadata)?;
        let p = image.pointer_size();
        let (count, array) = registration.types;
        let types = (0..count)
            .map(|i| {
                let address = image
                    .pointer(array + i * p)
                    .ok_or_else(|| Error::Binary(format!("type pointer {} is unmapped", i)))?;
                read_type(image, address, 0)
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            metadata_registration: registration,
            types,
        })
    }

    pub fn type_at(&self, index: i32) -> Option<&Il2CppType> {
        usize::try_from(index).ok().and_then(|i| self.types.get(i))
    }
}

fn read_type(image: &impl BinaryImage, address: u64, depth: usize) -> Result<Il2CppType> {
    if depth > 32 {
        return Err(Error::Binary("type nesting is too deep".into()));
    }
    let p = image.pointer_size();
    let unmapped = || Error::Binary(format!("Il2CppType at {:#x} is unmapped", address));
    let data = image.pointer(address).ok_or_else(unmapped)?;
    let bits = image.u32(address + p).ok_or_else(unmapped)?;
    let attrs = (bits & 0xFFFF) as u16;
    let element = ((bits >> 16) & 0xFF) as u8;
    let byref = (bits >> 29) & 1 == 1;
    let nested = |address: u64| read_type(image, address, depth + 1).map(Box::new);

    let kind = match element {
        element::CLASS => TypeKind::Class(data as i32),
        element::VALUETYPE => TypeKind::ValueType(data as i32),
        element::VAR => TypeKind::Var(data as i32),
        element::MVAR => TypeKind::MVar(data as i32),
        element::SZARRAY => TypeKind::SzArray(nested(data)?),
        element::PTR => TypeKind::Ptr(nested(data)?),
        element::ARRAY => TypeKind::Array {
            element: nested(image.pointer(data).ok_or_else(unmapped)?)?,
            rank: image.u8(data + p).ok_or_else(unmapped)?,
        },
        element::GENERICINST => {
            // Il2CppGenericClass { type, context { class_inst, method_inst }, cached_class }
            let generic = nested(image.pointer(data).ok_or_else(unmapped)?)?;
            let class_inst = image.pointer(data + p).ok_or_else(unmapped)?;
            // Il2CppGenericInst { type_argc, type_argv }
            let argc = image.pointer(class_inst).ok_or_else(unmapped)?;
            let argv = image.pointer(class_inst + p).ok_or_else(unmapped)?;
            if argc > 64 {
                return Err(Error::Binary(format!(
                    "generic instance at {:#x} has {} arguments",
                    data, argc
                )));
            }
            let args = (0..argc)
                .map(|i| {
                    let arg = image.pointer(argv + i * p).ok_or_else(unmapped)?;
                    read_type(image, arg, depth + 1)
                })
                .collect::<Result<_>>()?;
            TypeKind::GenericInst { generic, args }
        }
        element::FNPTR => TypeKind::FnPtr,
        0x01..=0x0E | element::TYPEDBYREF | 0x18 | 0x19 | element::OBJECT => {
            TypeKind::Primitive(element)
        }
        other => TypeKind::Unknown(other),
    };
    Ok(Il2CppType { kind, attrs, byref })
}

fn primitive_name(element: u8) -> &'static str {
    match element {
        0x01 => "void",
        0x02 => "bool",
        0x03 => "char",
        0x04 => "sbyte",
        0x05 => "byte",
        0x06 => "short",
        0x07 => "ushort",
        0x08 => "int",
        0x09 => "uint",
        0x0A => "long",
        0x0B => "ulong",
        0x0C => "float",
        0x0D => "double",
        0x0E => "string",
        0x16 => "TypedReference",
        0x18 => "nint",
        0x19 => "nuint",
        0x1C => "object",
        _ => "?",
    }
}

/// Removes the `` `N `` generic arity suffixes from every part of a type name.
fn strip_arity(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut chars = name.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '`' {
            while chars.peek().is_some_and(|d| d.is_ascii_digit()) {
                chars.next();
            }
        } else {
            out.push(c);
        }
    }
    out
}

impl Il2CppType {
    /// A C#-like name, e.g. `List<int>[]`.
    pub fn name(&self, metadata: &Metadata) -> String {
        let name = match &self.kind {
            TypeKind::Primitive(e) => primitive_name(*e).to_owned(),
            TypeKind::Class(t) | TypeKind::ValueType(t) => metadata.type_full_name(*t),
            TypeKind::SzArray(e) => format!("{}[]", e.name(metadata)),
            TypeKind::Array { element, rank } => {
                format!(
                    "{}[{}]",
                    element.name(metadata),
                    ",".repeat(rank.saturating_sub(1) as usize)
                )
            }
            TypeKind::Ptr(e) => format!("{}*", e.name(metadata)),
            TypeKind::GenericInst { generic, args } => {
                let base = strip_arity(&generic.name(metadata));
                let args: Vec<String> = args.iter().map(|a| a.name(metadata)).collect();
                format!("{}<{}>", base, args.join(", "))
            }
            TypeKind::Var(i) | TypeKind::MVar(i) => metadata
                .generic_parameters
                .get(*i as usize)
                .map(|g| metadata.string(g.name_index).to_owned())
                .unwrap_or_else(|| format!("T{}", i)),
            TypeKind::FnPtr => "fnptr".to_owned(),
            TypeKind::Unknown(e) => format!("<unknown {:#x}>", e),
        };
        if self.byref {
            format!("ref {}", name)
        } else {
            name
        }
    }
}
