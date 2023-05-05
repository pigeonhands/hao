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
    #[error("Invalid offset ({0}) for coded token \"{1}\".")]
    InvalidCodedTokenOffset(u32, &'static str),
    #[error("Invalid index ({0}) for stream \"{1}\".")]
    InvalidStreamIndex(&'static str, usize),
    #[error("Invalid UTF8 string at position {0} in #Strings stream. ({1:?})")]
    InvalidUTF8String(usize, Vec<u8>),
    #[error("Invalid signature calling convention. {0}.")]
    InvalidSignatureCallingConvention(u8),
    #[error("Invalid signature element type ({0}) at position ({0}).")]
    InvalidSignatureElementType(u8, usize),
    #[error("Recursion limit reached.")]
    RecursionLimitReached,
    #[error("unknown error")]
    Unknown,
}
