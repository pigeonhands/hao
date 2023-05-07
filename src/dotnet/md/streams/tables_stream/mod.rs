pub mod coded_tokens;
pub mod metadata;
mod reader;
mod streams_offsets;
mod tables;
pub mod values;

use self::{
    coded_tokens::CodedTokenSizes,
    metadata::{TableExistsFlags, TableLocations, TableRowCount, TableRows},
};
use super::Version;
use crate::{
    dotnet::md::MDStreamFlags,
    error::Result,
    io::{DataReader, ReadData, Readable},
};
pub use reader::*;
pub use streams_offsets::*;
pub use tables::*;

#[derive(Debug, Clone, Copy)]
pub struct TableLocation {
    pub start_offset: usize,
    pub rows: TableRowCount,
    pub row_size: usize,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ValueSize {
    Small,
    Big,
}

impl ValueSize {
    #[inline(always)]
    pub fn byte_size(&self) -> usize {
        match self {
            ValueSize::Big => std::mem::size_of::<u32>(),
            ValueSize::Small => std::mem::size_of::<u16>(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TablesStreamsHeader {
    pub valid_tables: TableExistsFlags,
    pub table_locations: TableLocations,
    pub flags: MDStreamFlags,
    pub version: Version,
    pub sorted_tables: TableExistsFlags,
    pub coded_token_sizes: CodedTokenSizes,
}

impl Readable for TablesStreamsHeader {
    fn from_reader(reader: &mut DataReader) -> Result<Self> {
        let _reserved1: u32 = reader.read()?;
        let major_version: u8 = reader.read()?;
        let minor_version: u8 = reader.read()?;
        let flags = MDStreamFlags::from_bits_retain(reader.read()?);
        let _log_2_rid: u8 = reader.read()?;
        let valid_tables = TableExistsFlags::from_bits_retain(reader.read()?);
        let sorted_tables = TableExistsFlags::from_bits_retain(reader.read()?);

        if flags.contains(MDStreamFlags::ExtraData) {
            let _extra: u32 = reader.read()?;
        }
        let version = Version(major_version, minor_version);
        let table_rows = TableRows::from_reader(reader, valid_tables)?;
        let coded_token_sizes = CodedTokenSizes::from_header(&table_rows);

        let table_lcoations =
            TableLocations::from_metadata(&table_rows, &coded_token_sizes, flags, version);

        Ok(Self {
            valid_tables,
            coded_token_sizes,
            table_locations: table_lcoations,
            flags,
            version,
            sorted_tables,
        })
    }
}

#[derive(Debug, Clone)]
pub struct TablesStreams<'a> {
    pub header: TablesStreamsHeader,
    //pub values: TablesValues,
    heap_data: &'a [u8],
}

impl<'a> TablesStreams<'a> {
    pub fn from_reader(mut reader: DataReader<'a>) -> Result<Self> {
        let header = reader.read()?;

        let heap_data = reader.remaning_slice();

        Ok(Self { header, heap_data })
    }

    pub(crate) fn row_iter<T>(&'a self, location: TableLocation) -> Result<TableRowsIterator<'a, T>>
    where
        TablesStreamReader<'a>: ReadData<T>,
    {
        TableRowsIterator::new(self.heap_data, &self.header, location)
    }

    pub fn modules(&'a self) -> Result<impl Iterator<Item = Result<ModulesTableRow>> + 'a> {
        self.row_iter(self.header.table_locations.module)
    }
}
