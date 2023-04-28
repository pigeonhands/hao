use std::borrow::Cow;

use crate::{
    error::{HaoError, Result},
    io::{DataReader, ReadData},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataType {
    Enc,
    Compressed,
}
pub struct MetadataHeader<'a> {
    pub signature: u32,
    pub major_version: u16,
    pub minor_version: u16,
    pub version: Cow<'a, str>,
    pub flags: u8,
    pub streams: Vec<StreamHeader<'a>>,
}

impl<'a> MetadataHeader<'a> {
    pub fn from_reader(r: &mut DataReader<'a>) -> Result<Self> {
        let signature = r.read()?;

        if signature != 0x424A5342 {
            return Err(crate::error::HaoError::BadImageFormat(
                "Invalid metadata header signature",
            ));
        }

        let major_version = r.read()?;
        let minor_version = r.read()?;
        let _reserved1: u32 = r.read()?;

        let version = {
            let str_len: u32 = r.read()?;
            let raw_data = r.read_slice(str_len as usize)?;
            let terminator = raw_data
                .iter()
                .position(|x| *x == 0)
                .unwrap_or(str_len as usize);
            String::from_utf8_lossy(&raw_data[..terminator])
        };

        // Offset2ndpart
        let flags: u8 = r.read()?;

        let _reserved2: u8 = r.read()?;

        let streams = {
            let stream_count: u16 = r.read()?;
            (0..stream_count)
                .map(|_| StreamHeader::from_reader(r))
                .collect::<Result<Vec<_>>>()?
        };

        Ok(Self {
            signature,
            major_version,
            minor_version,
            version,
            flags,
            streams,
        })
    }

    pub fn metadata_type(&self) -> Result<MetadataType> {
        self.streams
            .iter()
            .fold(None, |v, s| match (v, s.name.as_ref()) {
                (None, "#~") => Some(MetadataType::Compressed),
                (None, "#-") => Some(MetadataType::Enc),
                (_, "Schema") => Some(MetadataType::Compressed),
                _ => v,
            })
            .ok_or(HaoError::BadImageFormat("No #~ or #- stream found"))
    }
}

#[derive(Debug, Clone)]
pub struct StreamHeader<'a> {
    pub offset: u32,
    pub stream_size: u32,
    pub name: Cow<'a, str>,
}

impl<'a> StreamHeader<'a> {
    pub fn from_reader(r: &mut DataReader<'a>) -> Result<Self> {
        let offset: u32 = r.read()?;
        let stream_size: u32 = r.read()?;

        let name = {
            let str_view = r.view_slice(32)?;

            let stream_name = {
                let terminator = str_view
                    .iter()
                    .position(|c| *c == 0)
                    .unwrap_or(str_view.len());
                String::from_utf8_lossy(&str_view[..terminator])
            };

            let offset = if stream_name.len() != str_view.len() {
                (stream_name.len() + 1 + 3) & (!3)
            } else {
                str_view.len()
            };

            r.offset(offset)?;
            stream_name
        };

        Ok(Self {
            offset,
            stream_size,
            name,
        })
    }
}
