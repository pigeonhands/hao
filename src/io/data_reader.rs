use std::{convert::TryInto, ops::Range};

use crate::error::{HaoError, Result};

pub trait ReadData<T> {
    fn read(&mut self) -> Result<T>;
}

pub trait Readable: Sized {
    fn from_reader(reader: &mut DataReader) -> Result<Self>;
}

impl<'a, T> ReadData<T> for DataReader<'a>
where
    T: Readable,
{
    fn read(&mut self) -> Result<T> {
        T::from_reader(self)
    }
}

#[derive(Debug, Clone)]
pub struct DataReader<'a> {
    buffer: &'a [u8],
    index: usize,
}

impl<'a> DataReader<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self { buffer, index: 0 }
    }

    pub fn position(&self) -> usize {
        self.index
    }

    pub fn remaning(&self) -> usize {
        self.buffer.len() - self.position()
    }

    pub fn offset(&mut self, amount: usize) -> Result<()> {
        if self.buffer.len() < self.index + amount {
            return Err(HaoError::NotEnoughDataLeft(
                (self.index + amount) - self.buffer.len(),
            ));
        }

        self.index += amount;
        Ok(())
    }

    pub fn view_slice(&self, len: usize) -> Result<&'a [u8]> {
        if self.buffer.len() < self.index + len {
            return Err(HaoError::NotEnoughDataLeft(
                (self.index + len) - self.buffer.len(),
            ));
        }
        Ok(&self.buffer[self.index..self.index + len])
    }

    pub fn view_range(&self, range: Range<usize>) -> Result<&'a [u8]> {
        if self.buffer.len() < range.end {
            return Err(HaoError::NotEnoughDataLeft(range.end - self.buffer.len()));
        }

        Ok(&self.buffer[range])
    }

    pub fn read_slice(&mut self, len: usize) -> Result<&'a [u8]> {
        let s = self.view_slice(len)?;
        self.index += s.len();
        Ok(s)
    }

    pub fn remaning_slice(&self) -> &'a [u8] {
        &self.buffer[self.index..]
    }
}

impl ReadData<()> for DataReader<'_> {
    fn read(&mut self) -> Result<()> {
        Ok(())
    }
}

macro_rules! impl_read_data {
    ($($t:ty),+) => {
        $(
            impl ReadData<$t> for DataReader<'_> {
                fn read(&mut self) -> Result<$t> {
                    let data = self.read();
                    data.map(<$t>::from_le_bytes)
                }
            }
         )*
    };
}

impl_read_data! {
    u16,
    u32,
    u64
}

impl ReadData<u8> for DataReader<'_> {
    fn read(&mut self) -> Result<u8> {
        if self.buffer.len() < self.index + 1 {
            return Err(HaoError::NotEnoughDataLeft(1));
        }
        let b = self.buffer[self.index];
        self.index += 1;
        Ok(b)
    }
}

impl<const N: usize> ReadData<[u8; N]> for DataReader<'_> {
    fn read(&mut self) -> Result<[u8; N]> {
        let e = self
            .read_slice(N)?
            .try_into()
            .map_err(|_| HaoError::NotEnoughDataLeft(N))?;
        Ok(e)
    }
}
