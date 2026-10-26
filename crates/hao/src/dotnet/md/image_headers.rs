use crate::{
    error::Result,
    io::{DataReader, ReadData},
};

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct ImageDataDirectory {
    pub virtual_address: u32,
    pub data_size: u32,
}

impl ImageDataDirectory {
    pub fn from_reader(r: &mut DataReader) -> Result<Self> {
        Ok(Self {
            virtual_address: r.read()?,
            data_size: r.read()?,
        })
    }
}