use crate::{
    error::Result,
    io::{DataReader, ReadData},
};

pub struct ImageDataDirectory {
    pub virtual_address: u32,
    pub data_size: u32,
}

impl ImageDataDirectory {
    pub fn from_reader(r: &mut DataReader) -> Result<Self> {
        Ok(Self {
            virtual_address: r.read()?,
            data_size: r.read()?,
        })
    }
}

pub struct ImageCor20Header {
    pub major_rt_version: u16,
    pub minor_rt_version: u16,
    pub metadata: ImageDataDirectory,
    pub flags: u32,
    pub entrypoint_token_or_rva: u32,
    pub resources: ImageDataDirectory,
    pub strong_name_signature: ImageDataDirectory,
    pub code_manager_table: ImageDataDirectory,
    pub vtable_fixups: ImageDataDirectory,
    pub export_address_table_jumps: ImageDataDirectory,
    pub managed_native_header: ImageDataDirectory,
}

impl ImageCor20Header {
    pub fn from_reader(r: &mut DataReader) -> Result<Self> {
        let cb: u32 = r.read()?;
        if cb < 0x48 {
            return Err(crate::error::HaoError::BadImageFormat(
                "Invalid IMAGE_COR20_HEADER.cb value",
            ));
        }

        Ok(Self {
            major_rt_version: r.read()?,
            minor_rt_version: r.read()?,
            metadata: ImageDataDirectory::from_reader(r)?,
            flags: r.read()?,
            entrypoint_token_or_rva: r.read()?,
            resources: ImageDataDirectory::from_reader(r)?,
            strong_name_signature: ImageDataDirectory::from_reader(r)?,
            code_manager_table: ImageDataDirectory::from_reader(r)?,
            vtable_fixups: ImageDataDirectory::from_reader(r)?,
            export_address_table_jumps: ImageDataDirectory::from_reader(r)?,
            managed_native_header: ImageDataDirectory::from_reader(r)?,
        })
    }
}
