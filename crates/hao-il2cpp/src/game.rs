//! A high level view of an Il2Cpp game: types, fields and methods by name,
//! with their types already resolved.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use pewter::image::LoadedImage;

use crate::{
    binary::{Binary, Il2CppType},
    error::{Error, Result},
    generate::{generate, GeneratedAssembly},
    metadata::{
        FieldDefinition, Metadata, MethodDefinition, ParameterDefinition, TypeDefinition,
        TypeDefinitionIndex,
    },
};

const FIELD_STATIC: u16 = 0x10;
const METHOD_STATIC: u16 = 0x10;

/// A Unity Il2Cpp game: its metadata and the type table from its binary.
#[cfg_attr(feature = "reflect", derive(facet::Facet))]
pub struct Game {
    metadata: Metadata,
    binary: Binary,
    image_bytes: Vec<u8>,
    by_name: HashMap<String, TypeDefinitionIndex>,
}

impl Game {
    /// Opens a game folder: the one holding `GameAssembly.so` and `<Game>_Data`.
    pub fn open(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref();
        let metadata = std::fs::read_dir(dir)?
            .filter_map(|e| {
                e.ok()
                    .map(|e| e.path().join("il2cpp_data/Metadata/global-metadata.dat"))
            })
            .find(|p| p.exists())
            .ok_or_else(|| {
                Error::NotFound(format!(
                    "*_Data/il2cpp_data/Metadata/global-metadata.dat in {}",
                    dir.display()
                ))
            })?;
        let binary = ["GameAssembly.so", "GameAssembly.dll"]
            .iter()
            .map(|name| dir.join(name))
            .find(|p| p.exists())
            .ok_or_else(|| Error::NotFound(format!("GameAssembly in {}", dir.display())))?;
        Self::from_files(metadata, binary)
    }

    /// Opens a `global-metadata.dat` and its `GameAssembly` binary.
    pub fn from_files(metadata: impl AsRef<Path>, binary: impl AsRef<Path>) -> Result<Self> {
        Self::from_bytes(std::fs::read(metadata)?, &std::fs::read(binary)?)
    }

    /// Reads a game from the contents of its `global-metadata.dat` and `GameAssembly`,
    /// e.g. files uploaded to a web page.
    pub fn from_bytes(metadata: Vec<u8>, binary_bytes: &[u8]) -> Result<Self> {
        let metadata = Metadata::parse(metadata)?;
        // pewter detects ELF (`GameAssembly.so`) or PE (`GameAssembly.dll`).
        let image = pewter::image::AnyImage::load(binary_bytes)
            .map_err(|e| Error::Binary(format!("{:?}", e)))?;
        let binary = Binary::read(&image, &metadata)?;
        Ok(Self::new_with_image(metadata, binary, binary_bytes.to_vec()))
    }

    pub fn new(metadata: Metadata, binary: Binary) -> Self {
        Self::new_with_image(metadata, binary, Vec::new())
    }

    pub fn new_with_image(metadata: Metadata, binary: Binary, image_bytes: Vec<u8>) -> Self {
        let by_name = (0..metadata.type_definitions.len() as TypeDefinitionIndex)
            .map(|i| (metadata.type_full_name(i), i))
            .collect();
        Self {
            metadata,
            binary,
            image_bytes,
            by_name,
        }
    }

    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    pub fn binary(&self) -> &Binary {
        &self.binary
    }

    /// Every type definition in the game.
    pub fn types(&self) -> impl Iterator<Item = TypeView<'_>> {
        (0..self.metadata.type_definitions.len() as TypeDefinitionIndex)
            .map(|index| TypeView { game: self, index })
    }

    /// Finds a type by its full name (`Namespace.Name`, `Outer/Nested`, `List`1`).
    pub fn find_type(&self, full_name: &str) -> Option<TypeView<'_>> {
        self.by_name.get(full_name).map(|index| TypeView {
            game: self,
            index: *index,
        })
    }

    /// The assembly names in the game, e.g. `Assembly-CSharp.dll`.
    pub fn image_names(&self) -> impl Iterator<Item = &str> {
        self.metadata
            .images
            .iter()
            .map(|i| self.metadata.image_name(i))
    }

    /// Builds a declaration assembly for every image.
    pub fn generate(&self) -> Result<Vec<GeneratedAssembly>> {
        generate(&self.metadata, &self.binary)
    }

    /// Generates the declaration assemblies and writes them to `dir`, returning their paths.
    pub fn write_assemblies(&self, dir: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir)?;
        self.generate()?
            .into_iter()
            .map(|assembly| {
                let path = dir.join(&assembly.file_name);
                assembly.module.write_to_path(&path)?;
                Ok(path)
            })
            .collect()
    }

    fn type_name(&self, index: i32) -> String {
        self.binary
            .type_at(index)
            .map(|t| t.name(&self.metadata))
            .unwrap_or_else(|| format!("<type {}>", index))
    }

    /// Read raw bytes from the game binary at a virtual address.
    pub fn bytes_at(&self, vaddr: u64, len: usize) -> Option<Vec<u8>> {
        if self.image_bytes.is_empty() {
            return None;
        }
        let image = pewter::image::AnyImage::load(&self.image_bytes).ok()?;
        LoadedImage::bytes(&image, vaddr, len).map(|b| b.to_vec())
    }

    /// Look up the native code address for a method definition.
    pub fn method_pointer(&self, method: &MethodDefinition) -> Option<u64> {
        let image = self.metadata.images.iter().find(|img| {
            let start = img.type_start as i32;
            let end = start + img.type_count as i32;
            method.declaring_type >= start && method.declaring_type < end
        })?;
        let name = self.metadata.image_name(image);
        self.binary.method_pointer(name, method.token)
    }
}

#[derive(Clone, Copy)]
#[cfg_attr(feature = "reflect", derive(facet::Facet))]
pub struct TypeView<'g> {
    game: &'g Game,
    index: TypeDefinitionIndex,
}

impl<'g> TypeView<'g> {
    pub fn index(&self) -> TypeDefinitionIndex {
        self.index
    }

    pub fn definition(&self) -> &'g TypeDefinition {
        &self.game.metadata.type_definitions[self.index as usize]
    }

    pub fn name(&self) -> &'g str {
        self.game.metadata.string(self.definition().name_index)
    }

    pub fn namespace(&self) -> &'g str {
        self.game.metadata.string(self.definition().namespace_index)
    }

    /// `Namespace.Name`, with `/` separating nested types.
    pub fn full_name(&self) -> String {
        self.game.metadata.type_full_name(self.index)
    }

    /// The assembly the type is in, e.g. `Assembly-CSharp.dll`.
    pub fn image_name(&self) -> &'g str {
        let metadata = &self.game.metadata;
        metadata
            .images
            .iter()
            .find(|i| metadata.image_types(i).contains(&self.index))
            .map_or("", |i| metadata.image_name(i))
    }

    /// `TypeAttributes` flags.
    pub fn flags(&self) -> u32 {
        self.definition().flags
    }

    /// The base type, e.g. `UnityEngine.MonoBehaviour`.
    pub fn base_type_name(&self) -> Option<String> {
        let parent = self.definition().parent_index;
        (parent >= 0).then(|| self.game.type_name(parent))
    }

    pub fn declaring_type(&self) -> Option<TypeView<'g>> {
        self.game
            .metadata
            .declaring_type_definition(self.index)
            .map(|index| TypeView {
                game: self.game,
                index,
            })
    }

    pub fn nested_types(&self) -> impl Iterator<Item = TypeView<'g>> + 'g {
        let def = self.definition();
        let start = def.nested_types_start.max(0) as usize;
        let game = self.game;
        game.metadata
            .nested_types
            .get(start..start + def.nested_type_count as usize)
            .unwrap_or(&[])
            .iter()
            .map(move |index| TypeView {
                game,
                index: *index,
            })
    }

    pub fn fields(&self) -> impl Iterator<Item = FieldView<'g>> + 'g {
        let game = self.game;
        let start = self.definition().field_start;
        game.metadata
            .type_fields(self.definition())
            .iter()
            .enumerate()
            .map(move |(i, field)| FieldView {
                game,
                index: start + i as i32,
                field,
            })
    }

    /// For an enum type, each member's `(name, value)`; empty otherwise. The
    /// synthetic `value__` field is skipped.
    pub fn enum_values(&self) -> Vec<(&'g str, i64)> {
        self.fields()
            .filter(|f| f.name() != "value__")
            .filter_map(|f| Some((f.name(), f.constant()?.as_i64()?)))
            .collect()
    }

    pub fn methods(&self) -> impl Iterator<Item = MethodView<'g>> + 'g {
        let game = self.game;
        game.metadata
            .type_methods(self.definition())
            .iter()
            .map(move |method| MethodView { game, method })
    }

    pub fn find_field(&self, name: &str) -> Option<FieldView<'g>> {
        self.fields().find(|f| f.name() == name)
    }

    /// The first method called `name`. Use [`TypeView::methods`] to see all overloads.
    pub fn find_method(&self, name: &str) -> Option<MethodView<'g>> {
        self.methods().find(|m| m.name() == name)
    }
}

impl std::fmt::Debug for TypeView<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TypeView({})", self.full_name())
    }
}

#[derive(Clone, Copy)]
#[cfg_attr(feature = "reflect", derive(facet::Facet))]
pub struct FieldView<'g> {
    game: &'g Game,
    /// The field's global index, matched against the constant-value table.
    index: i32,
    field: &'g FieldDefinition,
}

impl<'g> FieldView<'g> {
    pub fn name(&self) -> &'g str {
        self.game.metadata.string(self.field.name_index)
    }

    pub fn ty(&self) -> Option<&'g Il2CppType> {
        self.game.binary.type_at(self.field.type_index)
    }

    /// A C#-like type name, e.g. `List<int>`.
    pub fn type_name(&self) -> String {
        self.game.type_name(self.field.type_index)
    }

    pub fn is_static(&self) -> bool {
        self.ty().is_some_and(|t| t.attrs & FIELD_STATIC != 0)
    }

    /// The field's compile-time constant, if it has one (enum members and
    /// `const` fields). The common case is an enum member's integer value.
    pub fn constant(&self) -> Option<ConstValue> {
        let value = self
            .game
            .metadata
            .field_default_values
            .iter()
            .find(|v| v.field_index == self.index)?;
        let code = match &self.game.binary.type_at(value.type_index)?.kind {
            crate::binary::TypeKind::Primitive(code) => *code,
            _ => return None,
        };
        ConstValue::decode(code, self.game.metadata.default_value_data(value.data_index))
    }
}

/// A field's compile-time constant value (an enum member or a `const`).
#[derive(Debug, Clone, PartialEq)]
pub enum ConstValue {
    Bool(bool),
    Char(u16),
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    F32(f32),
    F64(f64),
    String(String),
}

impl ConstValue {
    /// Decodes a constant of ECMA element type `code` from the front of `bytes`.
    fn decode(code: u8, bytes: &[u8]) -> Option<Self> {
        use crate::binary::element::*;
        fn le<const N: usize>(bytes: &[u8]) -> Option<[u8; N]> {
            bytes.get(..N)?.try_into().ok()
        }
        Some(match code {
            BOOLEAN => Self::Bool(*bytes.first()? != 0),
            CHAR => Self::Char(u16::from_le_bytes(le(bytes)?)),
            I1 => Self::I8(*bytes.first()? as i8),
            U1 => Self::U8(*bytes.first()?),
            I2 => Self::I16(i16::from_le_bytes(le(bytes)?)),
            U2 => Self::U16(u16::from_le_bytes(le(bytes)?)),
            I4 => Self::I32(i32::from_le_bytes(le(bytes)?)),
            U4 => Self::U32(u32::from_le_bytes(le(bytes)?)),
            I8 => Self::I64(i64::from_le_bytes(le(bytes)?)),
            U8 => Self::U64(u64::from_le_bytes(le(bytes)?)),
            R4 => Self::F32(f32::from_le_bytes(le(bytes)?)),
            R8 => Self::F64(f64::from_le_bytes(le(bytes)?)),
            STRING => {
                let len = usize::try_from(i32::from_le_bytes(le(bytes)?)).ok()?;
                Self::String(String::from_utf8_lossy(bytes.get(4..4 + len)?).into_owned())
            }
            _ => return None,
        })
    }

    /// The value as a signed integer, for bool/char/integer constants.
    pub fn as_i64(&self) -> Option<i64> {
        Some(match self {
            Self::Bool(b) => *b as i64,
            Self::Char(c) => *c as i64,
            Self::I8(v) => *v as i64,
            Self::U8(v) => *v as i64,
            Self::I16(v) => *v as i64,
            Self::U16(v) => *v as i64,
            Self::I32(v) => *v as i64,
            Self::U32(v) => *v as i64,
            Self::I64(v) => *v,
            Self::U64(v) => *v as i64,
            _ => return None,
        })
    }
}

impl std::fmt::Display for ConstValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bool(v) => write!(f, "{v}"),
            Self::Char(v) => write!(f, "{v}"),
            Self::I8(v) => write!(f, "{v}"),
            Self::U8(v) => write!(f, "{v}"),
            Self::I16(v) => write!(f, "{v}"),
            Self::U16(v) => write!(f, "{v}"),
            Self::I32(v) => write!(f, "{v}"),
            Self::U32(v) => write!(f, "{v}"),
            Self::I64(v) => write!(f, "{v}"),
            Self::U64(v) => write!(f, "{v}"),
            Self::F32(v) => write!(f, "{v}"),
            Self::F64(v) => write!(f, "{v}"),
            Self::String(v) => write!(f, "{v:?}"),
        }
    }
}

impl std::fmt::Display for FieldView<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_static() {
            write!(f, "static ")?;
        }
        write!(f, "{} {}", self.type_name(), self.name())?;
        if let Some(constant) = self.constant() {
            write!(f, " = {constant}")?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
#[cfg_attr(feature = "reflect", derive(facet::Facet))]
pub struct MethodView<'g> {
    game: &'g Game,
    method: &'g MethodDefinition,
}

#[derive(Clone, Copy)]
#[cfg_attr(feature = "reflect", derive(facet::Facet))]
pub struct ParameterView<'g> {
    game: &'g Game,
    parameter: &'g ParameterDefinition,
}

impl<'g> ParameterView<'g> {
    pub fn name(&self) -> &'g str {
        self.game.metadata.string(self.parameter.name_index)
    }

    pub fn ty(&self) -> Option<&'g Il2CppType> {
        self.game.binary.type_at(self.parameter.type_index)
    }

    pub fn type_name(&self) -> String {
        self.game.type_name(self.parameter.type_index)
    }
}

impl<'g> MethodView<'g> {
    pub fn definition(&self) -> &'g MethodDefinition {
        self.method
    }

    pub fn name(&self) -> &'g str {
        self.game.metadata.string(self.method.name_index)
    }

    pub fn is_static(&self) -> bool {
        self.method.flags & METHOD_STATIC != 0
    }

    pub fn return_type(&self) -> Option<&'g Il2CppType> {
        self.game.binary.type_at(self.method.return_type)
    }

    pub fn return_type_name(&self) -> String {
        self.game.type_name(self.method.return_type)
    }

    pub fn parameters(&self) -> impl Iterator<Item = ParameterView<'g>> + 'g {
        let game = self.game;
        game.metadata
            .method_parameters(self.method)
            .iter()
            .map(move |parameter| ParameterView { game, parameter })
    }

    /// The types of the parameters, e.g. `["string", "string"]`.
    pub fn parameter_type_names(&self) -> Vec<String> {
        self.parameters().map(|p| p.type_name()).collect()
    }

    /// The native code address of this method in the game binary, if known.
    pub fn code_address(&self) -> Option<u64> {
        self.game.method_pointer(self.method)
    }
}

impl std::fmt::Display for MethodView<'_> {
    /// `static string Concat(string str0, string str1)`
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_static() {
            write!(f, "static ")?;
        }
        let params: Vec<String> = self
            .parameters()
            .map(|p| format!("{} {}", p.type_name(), p.name()))
            .collect();
        write!(
            f,
            "{} {}({})",
            self.return_type_name(),
            self.name(),
            params.join(", ")
        )
    }
}
