//! `global-metadata.dat`: names and the shape of every type, method and field.
//!
//! Types of fields, parameters and return values are indices into the
//! `Il2CppType` table, which lives in the game binary, not in this file.
//!
//! Two metadata versions are supported:
//! - **v29** (Unity 2021.2 – 2023.x): header entries are `(offset, size)` pairs.
//! - **v39** (Unity 6): each header entry also carries a record count, so the
//!   entries are 12 bytes. Several records shrank: `type_definitions` dropped
//!   `element_type_index` and narrowed `generic_container_index`; `methods`
//!   narrowed its middle index fields (moving `parameter_start`); `images` and
//!   `assemblies` were reordered; `generic_parameters` narrowed `owner_index`.
//!   The v39 layout was reverse-engineered and validated against real Unity 6
//!   games (which ship Windows PE binaries, read via pewter's PE support).

use std::collections::HashMap;

use crate::error::{Error, Result};

const SANITY: u32 = 0xFAB1_1BAF;

/// Section names in header order, per metadata version.
fn section_names(version: u32) -> Option<&'static [&'static str]> {
    const V29: &[&str] = &[
        "string_literal",
        "string_literal_data",
        "string",
        "events",
        "properties",
        "methods",
        "parameter_default_values",
        "field_default_values",
        "field_and_parameter_default_value_data",
        "field_marshaled_sizes",
        "parameters",
        "fields",
        "generic_parameters",
        "generic_parameter_constraints",
        "generic_containers",
        "nested_types",
        "interfaces",
        "vtable_methods",
        "interface_offsets",
        "type_definitions",
        "images",
        "assemblies",
        "field_refs",
        "referenced_assemblies",
        "attribute_data",
        "attribute_data_range",
        "unresolved_virtual_call_parameter_types",
        "unresolved_virtual_call_parameter_ranges",
        "windows_runtime_type_names",
        "windows_runtime_strings",
        "exported_type_definitions",
    ];
    // v39 (Unity 6) keeps the same sections in the same order as v29; only the
    // per-entry header size and some record layouts differ (see `read_tables`).
    match version {
        29 | 39 => Some(V29),
        _ => None,
    }
}

/// Bytes per header section entry: `(offset, size)` for v29, plus a record-count
/// `u32` for v39.
fn header_entry_size(version: u32) -> usize {
    if version >= 39 {
        12
    } else {
        8
    }
}

struct Cursor<'a> {
    data: &'a [u8],
    position: usize,
}

impl Cursor<'_> {
    fn bytes<const N: usize>(&mut self) -> [u8; N] {
        let value = self.data[self.position..self.position + N]
            .try_into()
            .unwrap();
        self.position += N;
        value
    }
    fn u16(&mut self) -> u16 {
        u16::from_le_bytes(self.bytes())
    }
    fn i16(&mut self) -> i16 {
        i16::from_le_bytes(self.bytes())
    }
    fn u32(&mut self) -> u32 {
        u32::from_le_bytes(self.bytes())
    }
    fn i32(&mut self) -> i32 {
        i32::from_le_bytes(self.bytes())
    }
}

/// An index into the binary's `Il2CppType` table.
pub type TypeIndex = i32;
/// An index into [`Metadata::type_definitions`].
pub type TypeDefinitionIndex = i32;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "reflect", derive(facet::Facet))]
pub struct ImageDefinition {
    pub name_index: u32,
    pub assembly_index: i32,
    pub type_start: i32,
    pub type_count: u32,
    pub exported_type_start: i32,
    pub exported_type_count: u32,
    pub entry_point_index: i32,
    pub token: u32,
    pub custom_attribute_start: i32,
    pub custom_attribute_count: u32,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "reflect", derive(facet::Facet))]
pub struct AssemblyDefinition {
    pub image_index: i32,
    pub token: u32,
    pub referenced_assembly_start: i32,
    pub referenced_assembly_count: i32,
    pub name_index: u32,
    pub culture_index: u32,
    pub public_key_index: u32,
    pub hash_alg: u32,
    pub hash_len: i32,
    pub flags: u32,
    pub version: [i32; 4],
    pub public_key_token: [u8; 8],
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "reflect", derive(facet::Facet))]
pub struct TypeDefinition {
    pub name_index: u32,
    pub namespace_index: u32,
    pub byval_type_index: TypeIndex,
    pub declaring_type_index: TypeIndex,
    pub parent_index: TypeIndex,
    pub element_type_index: TypeIndex,
    pub generic_container_index: i32,
    pub flags: u32,
    pub field_start: i32,
    pub method_start: i32,
    pub event_start: i32,
    pub property_start: i32,
    pub nested_types_start: i32,
    pub interfaces_start: i32,
    pub vtable_start: i32,
    pub interface_offsets_start: i32,
    pub method_count: u16,
    pub property_count: u16,
    pub field_count: u16,
    pub event_count: u16,
    pub nested_type_count: u16,
    pub vtable_count: u16,
    pub interfaces_count: u16,
    pub interface_offsets_count: u16,
    pub bitfield: u32,
    pub token: u32,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "reflect", derive(facet::Facet))]
pub struct MethodDefinition {
    pub name_index: u32,
    pub declaring_type: TypeDefinitionIndex,
    pub return_type: TypeIndex,
    pub parameter_start: i32,
    pub generic_container_index: i32,
    pub token: u32,
    pub flags: u16,
    pub iflags: u16,
    pub slot: u16,
    pub parameter_count: u16,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "reflect", derive(facet::Facet))]
pub struct FieldDefinition {
    pub name_index: u32,
    pub type_index: TypeIndex,
    pub token: u32,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "reflect", derive(facet::Facet))]
pub struct ParameterDefinition {
    pub name_index: u32,
    pub token: u32,
    pub type_index: TypeIndex,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "reflect", derive(facet::Facet))]
pub struct PropertyDefinition {
    pub name_index: u32,
    /// Method index relative to the declaring type's `method_start`, or -1.
    pub get: i32,
    pub set: i32,
    pub attrs: u32,
    pub token: u32,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "reflect", derive(facet::Facet))]
pub struct EventDefinition {
    pub name_index: u32,
    pub type_index: TypeIndex,
    pub add: i32,
    pub remove: i32,
    pub raise: i32,
    pub token: u32,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "reflect", derive(facet::Facet))]
pub struct GenericContainer {
    /// A type definition index, or a method index when `is_method`.
    pub owner_index: i32,
    pub type_argc: i32,
    pub is_method: bool,
    pub generic_parameter_start: i32,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "reflect", derive(facet::Facet))]
pub struct GenericParameter {
    pub owner_index: i32,
    pub name_index: u32,
    pub constraints_start: i16,
    pub constraints_count: i16,
    pub num: u16,
    pub flags: u16,
}

#[cfg_attr(feature = "reflect", derive(facet::Facet))]
pub struct Metadata {
    pub version: u32,
    data: Vec<u8>,
    sections: HashMap<&'static str, (usize, usize)>,
    pub images: Vec<ImageDefinition>,
    pub assemblies: Vec<AssemblyDefinition>,
    pub type_definitions: Vec<TypeDefinition>,
    pub methods: Vec<MethodDefinition>,
    pub fields: Vec<FieldDefinition>,
    pub parameters: Vec<ParameterDefinition>,
    pub properties: Vec<PropertyDefinition>,
    pub events: Vec<EventDefinition>,
    pub generic_containers: Vec<GenericContainer>,
    pub generic_parameters: Vec<GenericParameter>,
    pub nested_types: Vec<TypeDefinitionIndex>,
    pub interfaces: Vec<TypeIndex>,
    pub generic_parameter_constraints: Vec<TypeIndex>,
    pub field_default_values: Vec<FieldDefaultValue>,
    declaring_types: Vec<Option<TypeDefinitionIndex>>,
}

/// A constant value for a field, stored in the default value data section.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "reflect", derive(facet::Facet))]
pub struct FieldDefaultValue {
    pub field_index: i32,
    pub type_index: TypeIndex,
    pub data_index: i32,
}

impl Metadata {
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Self> {
        Self::parse(std::fs::read(path)?)
    }

    pub fn parse(data: Vec<u8>) -> Result<Self> {
        if data.len() < 8 {
            return Err(Error::BadSection("header"));
        }
        let mut c = Cursor {
            data: &data,
            position: 0,
        };
        let sanity = c.u32();
        if sanity != SANITY {
            return Err(Error::BadSanity(sanity));
        }
        let version = c.u32();
        let names = section_names(version).ok_or(Error::UnsupportedVersion(version))?;
        let entry = header_entry_size(version);
        let header_end = 8 + names.len() * entry;
        if data.len() < header_end {
            return Err(Error::BadSection("header"));
        }
        let mut sections = HashMap::new();
        for name in names {
            let offset = c.u32() as usize;
            let size = c.i32().max(0) as usize;
            // v39 stores a record count after each (offset, size); skip it.
            c.position += entry - 8;
            if offset < header_end && size != 0 || offset + size > data.len() {
                return Err(Error::BadSection(name));
            }
            sections.insert(*name, (offset, size));
        }

        let mut metadata = Self {
            version,
            sections,
            images: Vec::new(),
            assemblies: Vec::new(),
            type_definitions: Vec::new(),
            methods: Vec::new(),
            fields: Vec::new(),
            parameters: Vec::new(),
            properties: Vec::new(),
            events: Vec::new(),
            generic_containers: Vec::new(),
            generic_parameters: Vec::new(),
            nested_types: Vec::new(),
            interfaces: Vec::new(),
            generic_parameter_constraints: Vec::new(),
            field_default_values: Vec::new(),
            declaring_types: Vec::new(),
            data: Vec::new(),
        };
        metadata.data = data;
        metadata.read_tables()?;
        Ok(metadata)
    }

    fn table<T>(
        &self,
        name: &'static str,
        size: usize,
        read: impl Fn(&mut Cursor) -> T,
    ) -> Result<Vec<T>> {
        let (offset, length) = self.sections[name];
        if length % size != 0 {
            return Err(Error::BadSection(name));
        }
        let mut c = Cursor {
            data: &self.data[offset..offset + length],
            position: 0,
        };
        Ok((0..length / size).map(|_| read(&mut c)).collect())
    }

    fn read_tables(&mut self) -> Result<()> {
        let v39 = self.version >= 39;
        if v39 {
            // Unity 6 packs the image record unaligned and no longer stores
            // type_start; type_count is at offset 10 and types are laid out in
            // image order, so type_start is filled in as a running total below.
            self.images = self.table("images", 36, |c| {
                let name_index = c.u32();
                let assembly_index = c.i32();
                let _pad = c.u16();
                let type_count = c.u32();
                let (token, entry_point_index, exported_type_start) = (c.u32(), c.i32(), c.i32());
                let (exported_type_count, custom_attribute_start) = (c.u32(), c.i32());
                let _tail = c.u16();
                ImageDefinition {
                    name_index,
                    assembly_index,
                    token,
                    entry_point_index,
                    exported_type_start,
                    exported_type_count,
                    custom_attribute_start,
                    custom_attribute_count: 0,
                    type_start: 0,
                    type_count,
                }
            })?;
            let mut start = 0;
            for image in &mut self.images {
                image.type_start = start;
                start += image.type_count as i32;
            }
            self.assemblies = self.table("assemblies", 68, |c| {
                let (image_index, token) = (c.i32(), c.u32());
                let (referenced_assembly_start, referenced_assembly_count) = (c.i32(), c.i32());
                let _custom_attribute_start = c.i32();
                AssemblyDefinition {
                    image_index,
                    token,
                    referenced_assembly_start,
                    referenced_assembly_count,
                    name_index: c.u32(),
                    culture_index: c.u32(),
                    public_key_index: c.u32(),
                    hash_alg: c.u32(),
                    hash_len: c.i32(),
                    flags: c.u32(),
                    version: [c.i32(), c.i32(), c.i32(), c.i32()],
                    public_key_token: c.bytes(),
                }
            })?;
        } else {
            self.images = self.table("images", 40, |c| ImageDefinition {
                name_index: c.u32(),
                assembly_index: c.i32(),
                type_start: c.i32(),
                type_count: c.u32(),
                exported_type_start: c.i32(),
                exported_type_count: c.u32(),
                entry_point_index: c.i32(),
                token: c.u32(),
                custom_attribute_start: c.i32(),
                custom_attribute_count: c.u32(),
            })?;
            self.assemblies = self.table("assemblies", 64, |c| AssemblyDefinition {
                image_index: c.i32(),
                token: c.u32(),
                referenced_assembly_start: c.i32(),
                referenced_assembly_count: c.i32(),
                name_index: c.u32(),
                culture_index: c.u32(),
                public_key_index: c.u32(),
                hash_alg: c.u32(),
                hash_len: c.i32(),
                flags: c.u32(),
                version: [c.i32(), c.i32(), c.i32(), c.i32()],
                public_key_token: c.bytes(),
            })?;
        }
        // v39 dropped `element_type_index` and shrank `generic_container_index`
        // to 16-bit; everything from `field_start` on is the same, 6 bytes earlier.
        let (type_size, read_type): (usize, fn(&mut Cursor) -> TypeDefinition) = if v39 {
            (82, |c| TypeDefinition {
                name_index: c.u32(),
                namespace_index: c.u32(),
                byval_type_index: c.i32(),
                declaring_type_index: c.i32(),
                parent_index: c.i32(),
                element_type_index: -1,
                generic_container_index: c.i16() as i32,
                flags: c.u32(),
                field_start: c.i32(),
                method_start: c.i32(),
                event_start: c.i32(),
                property_start: c.i32(),
                nested_types_start: c.i32(),
                interfaces_start: c.i32(),
                vtable_start: c.i32(),
                interface_offsets_start: c.i32(),
                method_count: c.u16(),
                property_count: c.u16(),
                field_count: c.u16(),
                event_count: c.u16(),
                nested_type_count: c.u16(),
                vtable_count: c.u16(),
                interfaces_count: c.u16(),
                interface_offsets_count: c.u16(),
                bitfield: c.u32(),
                token: c.u32(),
            })
        } else {
            (88, |c| TypeDefinition {
                name_index: c.u32(),
                namespace_index: c.u32(),
                byval_type_index: c.i32(),
                declaring_type_index: c.i32(),
                parent_index: c.i32(),
                element_type_index: c.i32(),
                generic_container_index: c.i32(),
                flags: c.u32(),
                field_start: c.i32(),
                method_start: c.i32(),
                event_start: c.i32(),
                property_start: c.i32(),
                nested_types_start: c.i32(),
                interfaces_start: c.i32(),
                vtable_start: c.i32(),
                interface_offsets_start: c.i32(),
                method_count: c.u16(),
                property_count: c.u16(),
                field_count: c.u16(),
                event_count: c.u16(),
                nested_type_count: c.u16(),
                vtable_count: c.u16(),
                interfaces_count: c.u16(),
                interface_offsets_count: c.u16(),
                bitfield: c.u32(),
                token: c.u32(),
            })
        };
        self.type_definitions = self.table("type_definitions", type_size, read_type)?;
        // v39 packs the 32-byte method record unaligned: declaring_type is 16-bit,
        // return_type sits at offset 6, and parameter_start at offset 14. token and
        // the trailing flags/slot/count u16s stay where v29 had them.
        self.methods = if v39 {
            self.table("methods", 32, |c| {
                let name_index = c.u32();
                let declaring_type = c.u16() as i32;
                let return_type = c.i32();
                let generic_container_index = c.i32();
                let parameter_start = c.i32();
                let _pad = c.u16();
                MethodDefinition {
                    name_index,
                    declaring_type,
                    return_type,
                    parameter_start,
                    generic_container_index,
                    token: c.u32(),
                    flags: c.u16(),
                    iflags: c.u16(),
                    slot: c.u16(),
                    parameter_count: c.u16(),
                }
            })?
        } else {
            self.table("methods", 32, |c| MethodDefinition {
                name_index: c.u32(),
                declaring_type: c.i32(),
                return_type: c.i32(),
                parameter_start: c.i32(),
                generic_container_index: c.i32(),
                token: c.u32(),
                flags: c.u16(),
                iflags: c.u16(),
                slot: c.u16(),
                parameter_count: c.u16(),
            })?
        };
        self.fields = self.table("fields", 12, |c| FieldDefinition {
            name_index: c.u32(),
            type_index: c.i32(),
            token: c.u32(),
        })?;
        self.parameters = self.table("parameters", 12, |c| ParameterDefinition {
            name_index: c.u32(),
            token: c.u32(),
            type_index: c.i32(),
        })?;
        self.properties = self.table("properties", 20, |c| PropertyDefinition {
            name_index: c.u32(),
            get: c.i32(),
            set: c.i32(),
            attrs: c.u32(),
            token: c.u32(),
        })?;
        self.events = self.table("events", 24, |c| EventDefinition {
            name_index: c.u32(),
            type_index: c.i32(),
            add: c.i32(),
            remove: c.i32(),
            raise: c.i32(),
            token: c.u32(),
        })?;
        self.generic_containers = self.table("generic_containers", 16, |c| GenericContainer {
            owner_index: c.i32(),
            type_argc: c.i32(),
            is_method: c.i32() != 0,
            generic_parameter_start: c.i32(),
        })?;
        // v39 shrank owner_index to 16-bit (name_index then sits at offset 2).
        let (gp_size, read_gp): (usize, fn(&mut Cursor) -> GenericParameter) = if v39 {
            (14, |c| GenericParameter {
                owner_index: c.u16() as i32,
                name_index: c.u32(),
                constraints_start: c.i16(),
                constraints_count: c.i16(),
                num: c.u16(),
                flags: c.u16(),
            })
        } else {
            (16, |c| GenericParameter {
                owner_index: c.i32(),
                name_index: c.u32(),
                constraints_start: c.i16(),
                constraints_count: c.i16(),
                num: c.u16(),
                flags: c.u16(),
            })
        };
        self.generic_parameters = self.table("generic_parameters", gp_size, read_gp)?;
        self.nested_types = self.table("nested_types", 4, |c| c.i32())?;
        self.interfaces = self.table("interfaces", 4, |c| c.i32())?;
        self.generic_parameter_constraints =
            self.table("generic_parameter_constraints", 4, |c| c.i32())?;
        self.field_default_values =
            self.table("field_default_values", 12, |c| FieldDefaultValue {
                field_index: c.i32(),
                type_index: c.i32(),
                data_index: c.i32(),
            })?;
        self.declaring_types = vec![None; self.type_definitions.len()];
        for (i, t) in self.type_definitions.iter().enumerate() {
            let start = t.nested_types_start.max(0) as usize;
            for nested in self
                .nested_types
                .get(start..start + t.nested_type_count as usize)
                .unwrap_or(&[])
            {
                if let Some(slot) = usize::try_from(*nested)
                    .ok()
                    .and_then(|n| self.declaring_types.get_mut(n))
                {
                    *slot = Some(i as TypeDefinitionIndex);
                }
            }
        }
        Ok(())
    }

    /// A string from the `#Strings`-like string section.
    pub fn string(&self, index: u32) -> &str {
        let (offset, size) = self.sections["string"];
        let heap = &self.data[offset..offset + size];
        let start = (index as usize).min(heap.len());
        let end = heap[start..]
            .iter()
            .position(|b| *b == 0)
            .map_or(heap.len(), |p| start + p);
        std::str::from_utf8(&heap[start..end]).unwrap_or("")
    }

    pub fn type_name(&self, index: TypeDefinitionIndex) -> &str {
        self.string(self.type_definitions[index as usize].name_index)
    }

    /// `Namespace.Name`, with `/` separating nested types.
    pub fn type_full_name(&self, index: TypeDefinitionIndex) -> String {
        let t = &self.type_definitions[index as usize];
        let name = self.string(t.name_index);
        if let Some(parent) = self.declaring_type_definition(index) {
            return format!("{}/{}", self.type_full_name(parent), name);
        }
        match self.string(t.namespace_index) {
            "" => name.to_owned(),
            ns => format!("{}.{}", ns, name),
        }
    }

    /// The enclosing type definition, found through the nested type lists.
    pub fn declaring_type_definition(
        &self,
        index: TypeDefinitionIndex,
    ) -> Option<TypeDefinitionIndex> {
        self.declaring_types.get(index as usize).copied().flatten()
    }

    pub fn image_name(&self, image: &ImageDefinition) -> &str {
        self.string(image.name_index)
    }

    /// Type definition indices belonging to `image`.
    pub fn image_types(&self, image: &ImageDefinition) -> std::ops::Range<TypeDefinitionIndex> {
        image.type_start..image.type_start + image.type_count as i32
    }

    pub fn type_methods(&self, t: &TypeDefinition) -> &[MethodDefinition] {
        let start = t.method_start.max(0) as usize;
        self.methods
            .get(start..start + t.method_count as usize)
            .unwrap_or(&[])
    }

    pub fn type_fields(&self, t: &TypeDefinition) -> &[FieldDefinition] {
        let start = t.field_start.max(0) as usize;
        self.fields
            .get(start..start + t.field_count as usize)
            .unwrap_or(&[])
    }

    /// Bytes of the default value data section from `index` on.
    pub fn default_value_data(&self, index: i32) -> &[u8] {
        let (offset, size) = self.sections["field_and_parameter_default_value_data"];
        let data = &self.data[offset..offset + size];
        usize::try_from(index)
            .ok()
            .and_then(|i| data.get(i..))
            .unwrap_or(&[])
    }

    pub fn generic_container_parameters(&self, container: i32) -> &[GenericParameter] {
        let Some(c) = usize::try_from(container)
            .ok()
            .and_then(|i| self.generic_containers.get(i))
        else {
            return &[];
        };
        let start = c.generic_parameter_start.max(0) as usize;
        self.generic_parameters
            .get(start..start + c.type_argc.max(0) as usize)
            .unwrap_or(&[])
    }

    pub fn method_parameters(&self, m: &MethodDefinition) -> &[ParameterDefinition] {
        let start = m.parameter_start.max(0) as usize;
        self.parameters
            .get(start..start + m.parameter_count as usize)
            .unwrap_or(&[])
    }
}
