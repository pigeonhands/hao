

use std::fmt::Debug;

use crate::{
    error::Result,
    io::{DataReader, ReadData},
};
use super::TablesStreamsHeader;

pub struct TablesStreamReader<'a> {
    pub reader: DataReader<'a>,
    pub header: &'a TablesStreamsHeader,
}

impl<'a> TablesStreamReader<'a> {
    pub fn new(tables_stream_data: &'a [u8], header: &'a TablesStreamsHeader) -> Self {
        Self {
            reader: DataReader::new(tables_stream_data),
            header,
        }
    }

    pub fn read_table_offset(&mut self, target_table_size: u32) -> Result<u32> {
        if target_table_size > u16::MAX as u32 {
            self.read()
        } else {
            let small_val: u16 = self.read()?;
            Ok(small_val as u32)
        }
    }

    pub fn read_rows<T: Debug>(&mut self, row_count: u32) -> Result<Vec<T>>
    where
        Self: ReadData<T>,
    {
        let mut data = Vec::with_capacity(row_count as usize);
        for _ in 0..row_count {
            data.push(self.read()?)
        }
        Ok(data)
    }
}

impl<'a, T> ReadData<T> for TablesStreamReader<'a>
where
    DataReader<'a>: ReadData<T>,
{
    fn read(&mut self) -> Result<T> {
        self.reader.read()
    }
}