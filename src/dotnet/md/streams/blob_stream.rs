use crate::{
    dotnet::signature::decode_compressed_u32,
    error::{HaoError, Result},
    io::DataReader,
};

#[derive(Debug, Clone)]
pub struct BlobStream<'a> {
    pub heap_data: &'a [u8],
}

impl<'a> BlobStream<'a> {
    pub fn from_reader(reader: DataReader<'a>) -> Result<Self> {
        Ok(Self {
            heap_data: reader.remaning_slice(),
        })
    }

    /// Reads the length prefixed blob at `offset`.
    pub fn read_blob(&self, offset: u32) -> Result<&'a [u8]> {
        read_length_prefixed(self.heap_data, offset, "#Blob")
    }
}

/// Heap with UTF-16 strings used by `ldstr`.
#[derive(Debug, Clone)]
pub struct UserStringsStream<'a> {
    pub heap_data: &'a [u8],
}

impl<'a> UserStringsStream<'a> {
    pub fn from_reader(reader: DataReader<'a>) -> Result<Self> {
        Ok(Self {
            heap_data: reader.remaning_slice(),
        })
    }

    pub fn read_string(&self, offset: u32) -> Result<String> {
        let data = read_length_prefixed(self.heap_data, offset, "#US")?;
        let units: Vec<u16> = data
            .as_chunks::<2>().0.iter()
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        Ok(String::from_utf16_lossy(&units))
    }
}

fn read_length_prefixed<'a>(heap: &'a [u8], offset: u32, name: &'static str) -> Result<&'a [u8]> {
    let offset = offset as usize;
    let data = heap
        .get(offset..)
        .ok_or(HaoError::InvalidStreamIndex(name, offset))?;
    let (len, header) =
        decode_compressed_u32(data).ok_or(HaoError::InvalidStreamIndex(name, offset))?;
    data.get(header..header + len as usize)
        .ok_or(HaoError::InvalidStreamIndex(name, offset))
}
