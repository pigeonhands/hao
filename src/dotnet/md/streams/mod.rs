pub mod tables_stream;
use tables_stream::TablesStreams;
use crate::{io::{PEImage}, error::{Result, HaoError}};
use super::metadata_header::StreamHeader;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(u8,u8);
impl Version {
    pub fn new(major_version: u8, minor_version: u8) -> Self{
        Self(major_version, minor_version)
    }
}

#[derive(Debug, Clone)]
pub struct MetadataStreams {
    pub tables_stream: TablesStreams
}

impl MetadataStreams {
    pub fn from_headers(
        pe_image: &PEImage,
        streams: &[StreamHeader],
        metadata_virtual_address: u32,
    ) -> Result<Self> {

        let mut tables_stream = None;

        for stream_header in streams {
            let stream_reader = pe_image.create_reader(
                metadata_virtual_address + stream_header.offset,
                Some(stream_header.stream_size as usize),
            )?;

            match stream_header.name.as_ref() {
                "#Strings" => continue,
                "#US" => continue,
                "#Blob" => continue,
                "#GUID" => continue,
                "#~" => tables_stream = Some(TablesStreams::from_reader(stream_reader)?),
                "#Pdb" => continue,
                _ => {}
            }
        }

        Ok(Self{
            tables_stream: tables_stream.ok_or_else(|| {
                HaoError::BadImageFormat("No .net tables stream (#~)")
            })?
        })
    }
}
