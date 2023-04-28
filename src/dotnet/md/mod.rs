pub mod image_headers;
pub mod metadata_header;
pub mod streams;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MDToken(u32);

use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MDStreamFlags: u8 {
        const BigStrings    = 1;
        const BigGUID       = 2;
        const BigBlob       = 4;
        const Padding       = 8;
        const DeltaOnly     = 0x20;
        const ExtraData     = 0x40;
        const HasDelete     = 0x80;
    }
}
