use std::ops::Range;

use crate::{
    dotnet::{
        entries::MaybeUninitEntries,
        md::streams::tables_stream::coded_tokens::{CodedToken, CodedTokenTarget},
    },
    error::{HaoError, Result},
    io::{DataReader, ReadData},
};

#[derive(Debug, Clone)]
pub struct BlobStream<'a> {
    pub reader: DataReader<'a>,
    pub reader_heap_offset: usize,
}

impl<'a> BlobStream<'a> {
    pub fn from_reader(reader: DataReader<'a>) -> Result<Self> {
        Ok(Self {
            reader,
            reader_heap_offset: 0,
        })
    }

    pub fn heap_position(&self) -> usize {
        self.reader_heap_offset + self.reader.position()
    }

    pub fn slice(&self, range: Range<usize>) -> Result<Self> {
        let reader_heap_offset = self.reader_heap_offset + range.end;
        Ok(Self {
            reader: DataReader::new(&self.reader.remaning_slice()[range]),
            reader_heap_offset,
        })
    }

    pub fn split_from(&self, pos: usize) -> Result<Self> {
        Ok(Self {
            reader: DataReader::new(&self.reader.remaning_slice()[pos..]),
            reader_heap_offset: self.reader_heap_offset + pos,
        })
    }

    pub(crate) fn get_signature_reader(
        &self,
        offset: u32,
        entries: &'a MaybeUninitEntries,
    ) -> Result<SignatureReader> {
        let offset = offset as usize;
        let mut new_reader = self.split_from(offset)?;
        let len = new_reader
            .read_compressed_u32()
            .map_err(|_| HaoError::InvalidStreamIndex("#Blob", self.heap_position()))?
            as usize;

        if len > new_reader.reader.remaning() {
            return Err(HaoError::InvalidStreamIndex(
                "#Blob",
                self.reader_heap_offset + offset,
            ));
        }

        new_reader
            .slice(0..len)
            .map(|r| SignatureReader::new(r, entries))
    }

    pub fn read_compressed_u32(&mut self) -> Result<u32> {
        const U16_MASK: u8 = 0b10000000; // 0x80;
        const U32_MASK: u8 = 0b11000000; // 0xC0

        fn read_compressed(reader: &mut BlobStream) -> Result<u32> {
            let b: u8 = reader.read()?;

            let value = match b & U32_MASK {
                mask if (mask & U16_MASK) == 0 => b as u32,
                U16_MASK => {
                    let bytes = [(b & (!U16_MASK)), reader.read()?];
                    u16::from_be_bytes(bytes) as u32
                }
                _ => {
                    let bytes = [
                        (b & (!U32_MASK)), // assume 111x is 110x
                        reader.read()?,
                        reader.read()?,
                        reader.read()?,
                    ];
                    u32::from_be_bytes(bytes)
                }
            };
            Ok(value)
        }
        let last_pos = self.reader.position() + self.reader_heap_offset;
        read_compressed(self).map_err(|_| HaoError::InvalidStreamIndex("#Blob", last_pos))
    }
}

impl<'a, T> ReadData<T> for BlobStream<'a>
where
    DataReader<'a>: ReadData<T>,
{
    #[inline(always)]
    fn read(&mut self) -> Result<T> {
        self.reader.read()
    }
}

pub struct SignatureReader<'a> {
    pub reader: BlobStream<'a>,
    pub(crate) entries: &'a MaybeUninitEntries,
    pub recursion_count: usize,
}

impl<'a> SignatureReader<'a> {
    const RECURSTION_LIMIT: usize = 100;
    pub(crate) fn new(reader: BlobStream<'a>, entries: &'a MaybeUninitEntries) -> Self {
        Self {
            reader,
            entries,
            recursion_count: 0,
        }
    }

    pub(crate) fn recursion_inc(&mut self) -> Result<()> {
        self.recursion_count += 1;
        if self.recursion_count > Self::RECURSTION_LIMIT {
            return Err(HaoError::RecursionLimitReached);
        } else {
            Ok(())
        }
    }
    pub(crate) fn recursion_dec(&mut self) {
        self.recursion_count = self.recursion_count.saturating_sub(1);
    }
}

impl<'a, T> ReadData<T> for SignatureReader<'a>
where
    BlobStream<'a>: ReadData<T>,
{
    #[inline(always)]
    fn read(&mut self) -> Result<T> {
        self.reader.read()
    }
}

impl<'a, T: CodedTokenTarget> ReadData<CodedToken<T>> for SignatureReader<'a> {
    fn read(&mut self) -> Result<CodedToken<T>> {
        CodedToken::decode(self.reader.read_compressed_u32()?)
    }
}
