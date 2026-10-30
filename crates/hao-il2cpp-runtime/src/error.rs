use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("could not load the game binary: {0}")]
    Load(libloading::Error),
    #[error("the game binary does not export {0}: {1}")]
    MissingExport(&'static str, libloading::Error),
    #[error("{0}")]
    UnsupportedBinary(String),
    #[error("signature mismatch, {0}")]
    Signature(String),
    #[error("il2cpp_init failed")]
    Init,
    #[error("could not find {0}")]
    NotFound(String),
    #[error("{0} contains a nul byte")]
    Nul(String),
    #[error("hooking failed: {0:?}")]
    Hook(hooking::error::HookingError),
    #[error("{0}")]
    Exception(String),
}
