//! Reads Unity Il2Cpp games, to generate .NET proxy assemblies with hao.
//!
//! Work in progress, for metadata v29 and ELF game binaries:
//! - [`Metadata`] reads `global-metadata.dat`: the names and shape of every type.
//! - [`Binary`] reads the `Il2CppType` table from `GameAssembly`, which gives the
//!   types of fields, parameters and return values.
//! - [`generate()`] builds a declaration assembly per image with hao.
//!
//! ```no_run
//! use hao_il2cpp::Metadata;
//!
//! let metadata = Metadata::from_path("Game_Data/il2cpp_data/Metadata/global-metadata.dat").unwrap();
//! for image in &metadata.images {
//!     println!("{} ({} types)", metadata.image_name(image), image.type_count);
//! }
//! ```

pub mod binary;
mod error;
pub mod generate;
pub mod metadata;

pub use error::{Error, Result};
pub use binary::{Binary, BinaryImage, ElfImage};
pub use generate::{generate, GeneratedAssembly};
pub use metadata::Metadata;
