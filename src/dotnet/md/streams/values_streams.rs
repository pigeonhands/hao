
use crate::{
    error::{HaoError, Result},
    io::{DataReader},
};

#[derive(Debug, Clone)]
pub struct StringsStream<'a> {
    pub heap_data: &'a [u8],
}

impl<'a> StringsStream<'a> {
    pub fn from_reader(reader: DataReader<'a>) -> Result<Self> {
        Ok(Self {
            heap_data: reader.remaning_slice(),
        })
    }

    pub fn read_string(&self, offset: u32) -> Result<&str> {
        let offset = offset as usize;

        if offset > self.heap_data.len() {
            return Err(HaoError::InvalidStreamIndex("#Strings", offset));
        }

        let slice = &self.heap_data[offset..];
        let terminator = slice
            .iter()
            .position(|c| *c == 0)
            .ok_or_else(|| HaoError::InvalidStreamIndex("#Strings", offset))?;

        std::str::from_utf8(&slice[..terminator])
            .map_err(|_| HaoError::InvalidUTF8String(offset, Vec::from(&slice[..terminator])))
    }
}

#[derive(Debug, Clone)]
pub struct GuidStream<'a> {
    pub heap_data: &'a [u8],
}

impl<'a> GuidStream<'a> {
    pub fn from_reader(reader: DataReader<'a>) -> Result<Self> {
        Ok(Self {
            heap_data: reader.remaning_slice(),
        })
    }

    pub fn read_guid(&self, offset: u32) -> Result<uuid::Uuid> {
        const GUID_SIZE: usize = 16;

        if offset == 0 {
            return Ok(uuid::Uuid::nil());
        }

        let position = (offset - 1) as usize * GUID_SIZE;

        uuid::Uuid::from_slice(
            &self.heap_data[position..],
        ).map_err(|_| {
            HaoError::InvalidStreamIndex("#GUID", position)
        })

    }
}

