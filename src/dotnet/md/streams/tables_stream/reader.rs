use core::{fmt::Debug, marker::PhantomData};
use crate::alloc_containers::vec::Vec;
use super::{TableLocation, TablesStreamsHeader};
use crate::{
    error::{HaoError, Result},
    io::{DataReader, ReadData},
};

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

    pub fn read_table_offset(&mut self, target_table_size: TableLocation) -> Result<u32> {
        if target_table_size.rows.is_large() {
            self.read()
        } else {
            let small_val: u16 = self.read()?;
            Ok(small_val as u32)
        }
    }

    pub fn read_rows<T: Debug>(&mut self, table_location: TableLocation) -> Result<Vec<T>>
    where
        Self: ReadData<T>,
    {
        let row_count = table_location.rows.0 as usize;
        let mut data = Vec::with_capacity(row_count);
        for _ in 0..row_count {
            data.push(self.read()?);
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

pub struct TableRowsIterator<'a, T>
where
    TablesStreamReader<'a>: ReadData<T>,
{
    reader: TablesStreamReader<'a>,
    index: usize,
    rows: usize,
    _marker: PhantomData<T>,
}

impl<'a, T> TableRowsIterator<'a, T>
where
    TablesStreamReader<'a>: ReadData<T>,
{
    pub fn new(
        heap_data: &'a [u8],
        header: &'a TablesStreamsHeader,
        location: TableLocation,
    ) -> Result<Self> {
        let length = location.rows.0 as usize * location.row_size;
        let slice = heap_data
            .get(location.start_offset..location.start_offset + length)
            .ok_or_else(|| HaoError::InvalidStreamIndex("#~", location.start_offset + length))?;

        Ok(Self {
            reader: TablesStreamReader::new(slice, header),
            index: 0,
            rows: location.rows.0 as usize,
            _marker: PhantomData,
        })
    }
}

impl<'a, T> Iterator for TableRowsIterator<'a, T>
where
    TablesStreamReader<'a>: ReadData<T>,
{
    type Item = Result<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.rows {
            return None;
        }
        self.index += 1;
        Some(self.reader.read())
    }
}
