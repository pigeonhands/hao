mod blob_stream;
pub mod tables_stream;
mod values_streams;
pub use blob_stream::*;
pub use values_streams::*;

use super::metadata_header::StreamHeader;
use crate::{
    error::{HaoError, Result},
    io::PEImage,
};
use tables_stream::TablesStreams;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(u8, u8);
impl Version {
    pub fn new(major_version: u8, minor_version: u8) -> Self {
        Self(major_version, minor_version)
    }
}

#[derive(Debug, Clone)]
pub struct MetadataStreams<'a> {
    pub tables_stream: TablesStreams<'a>,
    pub strings_stream: StringsStream<'a>,
    pub guid_stream: GuidStream<'a>,
    pub blob_stream: BlobStream<'a>,
}

impl<'a> MetadataStreams<'a> {
    pub fn from_headers(
        pe_image: &PEImage<'a>,
        streams: &[StreamHeader],
        metadata_virtual_address: u32,
    ) -> Result<Self> {
        let mut strings_stream = None;
        let mut tables_stream = None;
        let mut blob_stream = None;
        let mut guid_stream = None;

        for stream_header in streams {
            let stream_reader = pe_image.create_reader(
                metadata_virtual_address + stream_header.offset,
                Some(stream_header.stream_size as usize),
            )?;

            match stream_header.name.as_ref() {
                "#Strings" => strings_stream = Some(StringsStream::from_reader(stream_reader)?),
                "#US" => continue,
                "#Blob" => blob_stream = Some(BlobStream::from_reader(stream_reader)?),
                "#GUID" => guid_stream = Some(GuidStream::from_reader(stream_reader)?),
                "#~" => tables_stream = Some(TablesStreams::from_reader(stream_reader)?),
                "#Pdb" => continue,
                _ => {}
            }
        }

        Ok(Self {
            tables_stream: tables_stream
                .ok_or_else(|| HaoError::BadImageFormat("No .net tables stream (#~)"))?,
            strings_stream: strings_stream
                .ok_or_else(|| HaoError::BadImageFormat("No strings stream (#Strings)"))?,
            guid_stream: guid_stream
                .ok_or_else(|| HaoError::BadImageFormat("No guid stream (#GUID)"))?,
            blob_stream: blob_stream
                .ok_or_else(|| HaoError::BadImageFormat("No blob stream (#Blob)"))?,
        })
    }
}
