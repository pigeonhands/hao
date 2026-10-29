mod data_reader;
mod peimage;

pub use data_reader::*;
pub use peimage::PEImage;

/// Re-exported so a [`PEImage`] or [`Module`](crate::Module) can be built from a
/// PE parsed with pewter, without a direct pewter dependency.
pub use pewter::PEFile;
