//! Reads Unity Il2Cpp games: `global-metadata.dat` and (later) the
//! `GameAssembly` binary, to generate .NET proxy assemblies with hao.

mod error;
pub mod metadata;

pub use error::{Error, Result};
pub use metadata::Metadata;
