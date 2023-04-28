use thiserror::Error;

pub type Result<T> = std::result::Result<T, HaoError>;

#[derive(Error, Debug)]
pub enum HaoError {
    #[error("failed to parse PE file")]
    BadPeFormat,
    #[error("file is not a .net binary")]
    NotDotNetBinary,
    #[error("file is badly formatted. {0}")]
    BadImageFormat(&'static str),
    #[error("RVA {0} is too large for this image")]
    BadRva(u32),
    #[error("Not enough data to complete read of {0} bytes")]
    NotEnoughDataLeft(usize),
    #[error("Invalid column size: {0}")]
    InvalidColumnSize(i32),
    #[error("Invalid value ({0}) for coded token \"{1}\".")]
    InvalidCodedToken(u32, &'static str),
    #[error("unknown error")]
    Unknown,
}
