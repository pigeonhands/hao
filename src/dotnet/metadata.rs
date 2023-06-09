use super::md::streams::MetadataStreams;
use crate::dotnet::md::metadata_header::{MetadataHeader, MetadataType};
use crate::error::{HaoError, Result};
use crate::io::{PEImage};

pub struct Metadata<'a> {
    pub metadata_streams: MetadataStreams<'a>,
}

impl<'a> Metadata<'a> {
    pub fn parse(buffer: &'a [u8]) -> Result<Self> {

        let pe_image = PEImage::load_bytes(buffer)?;
       
        let cor20_header = pe_image.read_clr_rt_header()?;
    
        if cor20_header.metadata.virtual_address == 0 {
            return Err(HaoError::BadImageFormat(".NET metadata RVA is 0"));
        }

        let metadata_header = {
            let mut md_header_reader =
                pe_image.create_reader(cor20_header.metadata.virtual_address, None)?;
            MetadataHeader::from_reader(&mut md_header_reader)?
        };

        match metadata_header.metadata_type()? {
            MetadataType::Compressed => {}
            MetadataType::Enc => panic!("Only compressed streams are currently supported"),
        };

        let metadata_streams = MetadataStreams::from_headers(
            &pe_image,
            &metadata_header.streams,
            cor20_header.metadata.virtual_address,
        )?;

        Ok(Self { metadata_streams })
    }
}
