use std::collections::HashMap;

use crate::dotnet::signature::encode_compressed_u32;

#[derive(Debug)]
pub(crate) struct StringsHeap {
    pub data: Vec<u8>,
    map: HashMap<String, u32>,
}

impl Default for StringsHeap {
    fn default() -> Self {
        Self {
            data: vec![0],
            map: HashMap::new(),
        }
    }
}

impl StringsHeap {
    pub fn add(&mut self, value: &str) -> u32 {
        if value.is_empty() {
            return 0;
        }
        if let Some(offset) = self.map.get(value) {
            return *offset;
        }
        let offset = self.data.len() as u32;
        self.data.extend_from_slice(value.as_bytes());
        self.data.push(0);
        self.map.insert(value.to_owned(), offset);
        offset
    }
}

#[derive(Debug)]
pub(crate) struct BlobHeap {
    pub data: Vec<u8>,
    map: HashMap<Vec<u8>, u32>,
}

impl Default for BlobHeap {
    fn default() -> Self {
        Self {
            data: vec![0],
            map: HashMap::new(),
        }
    }
}

impl BlobHeap {
    pub fn add(&mut self, value: &[u8]) -> u32 {
        if value.is_empty() {
            return 0;
        }
        if let Some(offset) = self.map.get(value) {
            return *offset;
        }
        let offset = self.data.len() as u32;
        encode_compressed_u32(&mut self.data, value.len() as u32).expect("blob too large");
        self.data.extend_from_slice(value);
        self.map.insert(value.to_vec(), offset);
        offset
    }
}

#[derive(Debug, Default)]
pub(crate) struct GuidHeap {
    pub data: Vec<u8>,
}

impl GuidHeap {
    /// Returns the 1 based index of the guid, 0 for nil.
    pub fn add(&mut self, guid: &uuid::Uuid) -> u32 {
        if guid.is_nil() {
            return 0;
        }
        let bytes = guid.to_bytes_le();
        if let Some(i) = self.data.chunks(16).position(|c| c == bytes) {
            return i as u32 + 1;
        }
        self.data.extend_from_slice(&bytes);
        (self.data.len() / 16) as u32
    }
}

#[derive(Debug)]
pub(crate) struct UserStringHeap {
    pub data: Vec<u8>,
    map: HashMap<String, u32>,
}

impl Default for UserStringHeap {
    fn default() -> Self {
        Self {
            data: vec![0],
            map: HashMap::new(),
        }
    }
}

impl UserStringHeap {
    pub fn add(&mut self, value: &str) -> u32 {
        if let Some(offset) = self.map.get(value) {
            return *offset;
        }
        let offset = self.data.len() as u32;
        let units: Vec<u16> = value.encode_utf16().collect();
        encode_compressed_u32(&mut self.data, units.len() as u32 * 2 + 1)
            .expect("string too large");
        let mut special = 0;
        for unit in units {
            self.data.extend_from_slice(&unit.to_le_bytes());
            if unit > 0xFF || matches!(unit, 0x01..=0x08 | 0x0E..=0x1F | 0x27 | 0x2D | 0x7F) {
                special = 1;
            }
        }
        self.data.push(special);
        self.map.insert(value.to_owned(), offset);
        offset
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }
}
