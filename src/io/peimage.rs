use crate::error::{HaoError, Result};
use crate::io::DataReader;

use goblin::pe::{
    data_directories::DataDirectory, import::Bitfield, optional_header::OptionalHeader,
    section_table::SectionTable, PE,
};

pub struct PEImage<'a> {
    pe: PE<'a>,
    data: &'a [u8],
}

impl<'a> PEImage<'a> {
    pub fn load_bytes(data: &'a [u8]) -> Result<Self> {
        let pe = PE::parse(data).map_err(|_| HaoError::BadPeFormat)?;

        Ok(Self { pe, data })
    }

    pub fn optional_header(&self) -> Result<OptionalHeader> {
        self.pe.header.optional_header.ok_or(HaoError::BadPeFormat)
    }

    pub fn rva_to_section(&self, rva: u32) -> Option<&SectionTable> {
        fn align_up(v: u32, allignment: u32) -> u32 {
            (v + allignment - 1) & !(allignment - 1)
        }
        let alignment = self
            .optional_header()
            .ok()?
            .windows_fields
            .section_alignment;

        self.pe.sections.iter().find(|section| {
            rva >= section.virtual_address
                && rva < (section.virtual_address + align_up(section.virtual_size, alignment))
        })
    }

    pub fn rva_to_file_offset(&self, rva: u32) -> Result<usize> {
        let optional_header = self.optional_header()?;

        if rva >= optional_header.windows_fields.size_of_image {
            return Err(HaoError::BadRva(rva));
        }

        let image_section_header = self.rva_to_section(rva);

        if let Some(section) = image_section_header {
            let offset = rva - section.virtual_address;
            if offset >= section.size_of_raw_data {
                return Err(HaoError::BadRva(rva));
            }
            Ok((offset + section.pointer_to_raw_data) as usize)
        } else {
            Ok(rva as usize)
        }
    }

    pub fn create_reader(&self, rva: u32, size: Option<usize>) -> Result<DataReader<'a>> {
        let file_offset = self.rva_to_file_offset(rva)?;

        let data = if let Some(size) = size {
            &self.data[file_offset..file_offset + size]
        } else {
            &self.data[file_offset..]
        };

        Ok(DataReader::new(data))
    }

    pub fn read_clr_rt_header(&self) -> Result<DataDirectory> {
        let header = self
            .optional_header()?
            .data_directories
            .get_clr_runtime_header()
            .ok_or(HaoError::NotDotNetBinary)?;

        if header.virtual_address.is_zero() {
            Err(HaoError::BadImageFormat(".NET data directory RVA is 0"))
        } else {
            Ok(header)
        }
    }
}
