//! Reads Unity Il2Cpp games, to generate .NET proxy assemblies with hao.
//!
//! Work in progress: [`Metadata`] reads `global-metadata.dat` (metadata v29),
//! which has the names and shape of every type. The `GameAssembly` binary,
//! which holds the `Il2CppType` table that field, parameter and return types
//! index into, isn't read yet.
//!
//! ```no_run
//! use hao_il2cpp::Metadata;
//!
//! let metadata = Metadata::from_path("Game_Data/il2cpp_data/Metadata/global-metadata.dat").unwrap();
//! for image in &metadata.images {
//!     println!("{} ({} types)", metadata.image_name(image), image.type_count);
//! }
//! ```

mod error;
pub mod metadata;

pub use error::{Error, Result};
pub use metadata::Metadata;
