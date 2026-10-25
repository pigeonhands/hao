pub mod csharp;
pub mod entries;
pub mod il;
pub mod md;
pub mod metadata;
pub(crate) mod module;
pub mod signature;

pub use metadata::Metadata;
pub use module::Module;
