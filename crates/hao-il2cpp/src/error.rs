use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("not an il2cpp metadata file (bad sanity value {0:#x})")]
    BadSanity(u32),
    #[error("metadata version {0} is not supported")]
    UnsupportedVersion(u32),
    #[error("metadata section {0} is out of bounds or misaligned")]
    BadSection(&'static str),
    #[error("index {1} is out of range for {0}")]
    BadIndex(&'static str, i64),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
