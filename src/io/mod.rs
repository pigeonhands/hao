mod data_reader;
mod peimage;

pub use data_reader::DataReader;
pub use peimage::PEImage;

use crate::error::{Result};


pub trait ReadData<T> {
    fn read(&mut self) -> Result<T>;
}

pub trait Readable: Sized {
    fn from_reader(reader: &mut DataReader) -> Result<Self>;
}

impl<'a, T> ReadData<T> for DataReader<'a>
where T: Readable {
    fn read(&mut self) -> Result<T> {
        T::from_reader(self)
    }
}