use super::{
    streams::{
        tables_stream::{
            coded_tokens::CodedTokenSizes,
            metadata::{TableRowCount, TableRows},
            TableLocation,
        },
        Version,
    },
    MDStreamFlags,
};

pub trait CalculateTableSize<T> {
    fn calculate_table_size_bytes(&self) -> usize;
}

pub struct SizeCalculator<'a> {
    pub rows: &'a TableRows,
    pub flags: MDStreamFlags,
    pub coded_tokens_sizes: &'a CodedTokenSizes,
    pub version: Version,
}

impl<'a> SizeCalculator<'a> {
    pub fn size_of_prim<T: Sized>(&self) -> usize {
        core::mem::size_of::<T>()
    }

    pub fn size_of<T>(&self) -> usize
    where
        Self: CalculateTableSize<T>,
    {
        self.calculate_table_size_bytes()
    }
}

pub struct TablePositionCalculator<'a> {
    pub size_calculator: &'a SizeCalculator<'a>,
    index: usize,
}

impl<'a> TablePositionCalculator<'a> {
    pub fn new(size_calculator: &'a SizeCalculator) -> Self {
        Self {
            size_calculator,
            index: 0,
        }
    }

    pub fn calculate_location<T>(&mut self, rows: TableRowCount) -> TableLocation
    where
        SizeCalculator<'a>: CalculateTableSize<T>,
    {
        let start_offset = self.index;
        let row_size = self.size_calculator.size_of::<T>();
        self.index += row_size * rows.0 as usize;

        TableLocation {
            start_offset,
            rows,
            row_size,
        }
    }
}
