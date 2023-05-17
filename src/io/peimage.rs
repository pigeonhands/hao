use crate::error::{HaoError, Result};
use crate::io::DataReader;

use pewter::pe::optional_header::OptionalHeader;
use pewter::pe::sections::cor20::ImageCor20Header;
use pewter::pe::sections::{SectionRow};
use pewter::PEFile;

pub struct PEImage<'a> {
    pub pe: PEFile<'a>,
}

impl<'a> PEImage<'a> {
    pub fn load_bytes(data: &'a [u8]) -> Result<Self> {
        let pe = PEFile::parse(data).map_err(|_| HaoError::BadPeFormat)?;

        Ok(Self { pe })
    }

    pub fn optional_header(&self) -> Result<&OptionalHeader> {
        self.pe.optional_header.as_ref().ok_or(HaoError::BadPeFormat)
    }

    pub fn rva_to_section(&self, rva: u32) -> Option<&SectionRow> {
        self.pe.sections.find_rva(rva as usize)
    }

    pub fn rva_to_file_offset_(&self, _rva: u32) -> Result<usize> {
        todo!()
    }

    pub fn create_reader(&self, rva: u32, size: Option<usize>) -> Result<DataReader<'a>> {
        let data = self
            .pe
            .sections
            .find_rva_data(rva as usize)
            .ok_or_else(|| HaoError::BadRva(rva))?;

        let data = if let Some(size) = size {
            &data[..size]
        } else {
            data
        };

        Ok(DataReader::new(data))
    }

    pub fn read_clr_rt_header(&self) -> Result<ImageCor20Header> {
        let clr = self
            .pe
            .read_clr_runtime_header()
            .map_err(|_| HaoError::BadImageFormat("Failed to read image cor20 header"))?;

        let header = clr.ok_or(HaoError::NotDotNetBinary)?;

        Ok(header)
    }
}
