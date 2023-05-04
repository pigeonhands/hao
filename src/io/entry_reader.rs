use crate::{
    dotnet::{
        entries::{Field, Method, Ptr, RowRange},
        md::streams::{
            tables_stream::{
                BlobStreamOffset, FieldTableOffset, GuidStreamOffset,
                MethodTableOffset, StringsStreamOffset,
            },
            MetadataStreams, Signature,
        },
        module::{GetEntry, MaybeUninitEntries},
    },
    error::Result,
};

pub trait ValueReadable<T> {
    type EntryValue;
    fn read(&self, identifier: T) -> Result<Self::EntryValue>;
}

pub struct EntryReader<'a> {
    pub(crate) streams: &'a MetadataStreams<'a>,
    pub(crate) entries: &'a MaybeUninitEntries,
}

impl<'a> EntryReader<'a> {
    pub(crate) fn from_metadata(
        streams: &'a MetadataStreams<'a>,
        entries: &'a MaybeUninitEntries,
    ) -> Self {
        Self { streams , entries  }
    }
}

impl<'a, T> ValueReadable<T> for EntryReader<'a>
where
    MaybeUninitEntries: GetEntry<T>,
{
    type EntryValue = <MaybeUninitEntries as GetEntry<T>>::EntryValue;
    fn read(&self, identifier: T) -> Result<Self::EntryValue> {
        self.entries.get_entry(identifier)
    }
}
 
impl<'a> ValueReadable<u16> for EntryReader<'a> {
    type EntryValue = u16;

    fn read(&self, identifier: u16) -> Result<Self::EntryValue> {
        Ok(identifier)
    }
}

impl<'a> ValueReadable<StringsStreamOffset> for EntryReader<'a> {
    type EntryValue = String;

    fn read(&self, identifier: StringsStreamOffset) -> Result<Self::EntryValue> {
        self.streams
            .strings_stream
            .read_string(identifier.0)
            .map(String::from)
    }
}

impl<'a> ValueReadable<GuidStreamOffset> for EntryReader<'a> {
    type EntryValue = uuid::Uuid;

    fn read(&self, identifier: GuidStreamOffset) -> Result<Self::EntryValue> {
        self.streams.guid_stream.read_guid(identifier.0)
    }
}

impl<'a> ValueReadable<BlobStreamOffset> for EntryReader<'a> {
    type EntryValue = Signature;

    fn read(&self, identifier: BlobStreamOffset) -> Result<Self::EntryValue> {
        let reader = self
            .streams
            .blob_stream
            .get_signature_reader(identifier.0, self.entries)?;
        Signature::from_reader(reader)
    }
}

impl<'a> ValueReadable<RowRange<FieldTableOffset>> for EntryReader<'a> {
    type EntryValue = Vec<Ptr<Field>>;

    fn read(&self, identifier: RowRange<FieldTableOffset>) -> Result<Self::EntryValue> {
        let target_rows = &self.entries.fields;

        let start = identifier.start.0 as usize;
        let end = identifier
            .end
            .map(|c| c.0 as usize)
            .unwrap_or(target_rows.len());

        let slice = target_rows.get(start..end).unwrap_or(&target_rows[start..]);
        Ok(slice.to_vec())
    }
}

impl<'a> ValueReadable<RowRange<MethodTableOffset>> for EntryReader<'a> {
    type EntryValue = Vec<Ptr<Method>>;

    fn read(&self, identifier: RowRange<MethodTableOffset>) -> Result<Self::EntryValue> {
        let target_rows = &self.entries.methods;

        let start = identifier.start.0 as usize;
        let end = identifier
            .end
            .map(|c| c.0 as usize)
            .unwrap_or(target_rows.len());

        let slice = target_rows.get(start..end).unwrap_or(&target_rows[start..]);
        Ok(slice.to_vec())
    }
}
