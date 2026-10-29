use pewter::pe::sections::cor20::ImageCor20Header;

use super::md::streams::MetadataStreams;
use crate::dotnet::md::metadata_header::{MetadataHeader, MetadataType};
use crate::error::{HaoError, Result};
use crate::io::PEImage;

pub struct Metadata<'a> {
    pub data: &'a [u8],
    pub pe_image: PEImage<'a>,
    pub cor20_header: ImageCor20Header,
    pub version: String,
    pub metadata_type: MetadataType,
    pub metadata_streams: MetadataStreams<'a>,
}

impl<'a> Metadata<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self> {
        Self::from_image(PEImage::load_bytes(data)?)
    }

    /// Reads .NET metadata from an already-parsed PE image, instead of parsing
    /// bytes.
    pub fn from_image(pe_image: PEImage<'a>) -> Result<Self> {
        let data = pe_image.pe.file_bytes();
        let cor20_header = pe_image.read_clr_rt_header()?;

        if cor20_header.metadata.virtual_address == 0 {
            return Err(HaoError::BadImageFormat(".NET metadata RVA is 0"));
        }

        let metadata_header = {
            let mut md_header_reader =
                pe_image.create_reader(cor20_header.metadata.virtual_address, None)?;
            MetadataHeader::from_reader(&mut md_header_reader)?
        };

        let metadata_type = metadata_header.metadata_type()?;

        let metadata_streams = MetadataStreams::from_headers(
            &pe_image,
            &metadata_header.streams,
            cor20_header.metadata.virtual_address,
        )?;

        Ok(Self {
            data,
            version: metadata_header.version.into_owned(),
            metadata_type,
            pe_image,
            cor20_header,
            metadata_streams,
        })
    }
}
