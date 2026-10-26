use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("could not find {0}")]
    NotFound(String),
    #[error("could not load {0}: {1}")]
    Library(String, libloading::Error),
    #[error("{function} failed with {code:#010x}")]
    Host { function: &'static str, code: u32 },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}
