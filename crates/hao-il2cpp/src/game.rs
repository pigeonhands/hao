//! A high level view of an Il2Cpp game: types, fields and methods by name,
//! with their types already resolved.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

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
    pub fn from_bytes(metadata: Vec<u8>, binary: &[u8]) -> Result<Self> {
        let metadata = Metadata::parse(metadata)?;
        // pewter detects ELF (`GameAssembly.so`) or PE (`GameAssembly.dll`).
        let image =
            pewter::image::AnyImage::load(binary).map_err(|e| Error::Binary(format!("{:?}", e)))?;
        let binary = Binary::read(&image, &metadata)?;
        Ok(Self::new(metadata, binary))
    }

    pub fn new(metadata: Metadata, binary: Binary) -> Self {
        let by_name = (0..metadata.type_definitions.len() as TypeDefinitionIndex)
            .map(|i| (metadata.type_full_name(i), i))
            .collect();
        Self {
            metadata,
            binary,
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
        game.metadata
            .type_fields(self.definition())
            .iter()
            .map(move |field| FieldView { game, field })
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
}

impl std::fmt::Display for FieldView<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_static() {
            write!(f, "static ")?;
        }
        write!(f, "{} {}", self.type_name(), self.name())
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
