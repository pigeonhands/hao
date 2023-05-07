use crate::{
    dotnet::{
        entries::{values::TypeDef, GetEntryField, MaybeUninitEntries},
        entries::{
            values::{Field, Method, Param},
            Ptr, RowRange,
        },
        md::streams::{
            tables_stream::{
                coded_tokens::{CodedToken, CodedTokenTarget},
                BlobStreamOffset, BlobStreamOffsetTypeSpec, FieldTableOffset, GuidStreamOffset,
                MethodTableOffset, ParamTableOffset, StringsStreamOffset, TypeDefTableOffset,
            },
            MetadataStreams, SignatureDef, TypeSigDef,
        },
    },
    error::{HaoError, Result},
};

use super::ReadData;

pub trait ValueReadable<T> {
    type EntryValue: Clone;
    fn read(&self, identifier: T) -> Result<Self::EntryValue>;
}

pub(crate) trait GetTableForRead<T> {
    type TablevalueType: Clone;
    fn to_index(&self, offset: T) -> Result<usize>;
    fn get_table(&self) -> &[Ptr<Self::TablevalueType>];
}

pub(crate) struct EntryReader<'a> {
    pub(crate) streams: &'a MetadataStreams<'a>,
    pub(crate) entries: &'a MaybeUninitEntries,
}

impl<'a> EntryReader<'a> {
    pub(crate) fn from_metadata(
        streams: &'a MetadataStreams<'a>,
        entries: &'a MaybeUninitEntries,
    ) -> Self {
        Self { streams, entries }
    }
}

impl<'a, T: CodedTokenTarget> ValueReadable<CodedToken<T>> for EntryReader<'a>
where
    MaybeUninitEntries: GetEntryField<CodedToken<T>>,
{
    type EntryValue = <MaybeUninitEntries as GetEntryField<CodedToken<T>>>::EntryFieldValue;
    fn read(&self, identifier: CodedToken<T>) -> Result<Self::EntryValue> {
        self.entries.get_entry_field(identifier)
    }
}

impl<'a, T> ValueReadable<T> for EntryReader<'a>
where
    EntryReader<'a>: GetTableForRead<T>,
{
    type EntryValue = Ptr<<EntryReader<'a> as GetTableForRead<T>>::TablevalueType>;

    fn read(&self, identifier: T) -> Result<Self::EntryValue> {
        let index = self.to_index(identifier)?;
        let table = self.get_table();

        table
            .get(index)
            .ok_or_else(|| {
                HaoError::InvalidEntryRefrence(std::any::type_name::<Self::EntryValue>(), index)
            })
            .cloned()
    }
}

impl<'a, T> ValueReadable<RowRange<T>> for EntryReader<'a>
where
    EntryReader<'a>: GetTableForRead<T>,
{
    //type EntryValue = Vec<Ptr<EntryReader<'a> as GetTableForRead<T>>::TablevalueType>>
    type EntryValue = Vec<Ptr<<EntryReader<'a> as GetTableForRead<T>>::TablevalueType>>;

    fn read(&self, identifier: RowRange<T>) -> Result<Self::EntryValue> {
        let target_rows = self.get_table();

        let start = self.to_index(identifier.start)?;
        let end = identifier.end.map(|v| self.to_index(v)).transpose()?;

        let end = end.unwrap_or(target_rows.len());

        if start == end {
            return Ok(Vec::new());
        } else {
            if start >= target_rows.len() {
                return Err(HaoError::InvalidEntryRefrence(
                    std::any::type_name::<Self::EntryValue>(),
                    start,
                ));
            }

            let slice = target_rows.get(start..end).unwrap_or(&target_rows[start..]);
            Ok(slice.to_vec())
        }
    }
}

impl<'a> GetTableForRead<FieldTableOffset> for EntryReader<'a> {
    type TablevalueType = Field;
    fn to_index(&self, offset: FieldTableOffset) -> Result<usize> {
        (offset.0 as usize).checked_sub(1).ok_or_else(|| {
            HaoError::InvalidEntryRefrence(std::any::type_name::<Self::TablevalueType>(), 0)
        })
    }
    fn get_table(&self) -> &[Ptr<Self::TablevalueType>] {
        self.entries.fields.as_slice()
    }
}

impl<'a> GetTableForRead<TypeDefTableOffset> for EntryReader<'a> {
    type TablevalueType = TypeDef;
    fn to_index(&self, offset: TypeDefTableOffset) -> Result<usize> {
        (offset.0 as usize).checked_sub(1).ok_or_else(|| {
            HaoError::InvalidEntryRefrence(std::any::type_name::<Self::TablevalueType>(), 0)
        })
    }
    fn get_table(&self) -> &[Ptr<Self::TablevalueType>] {
        self.entries.type_defs.as_slice()
    }
}

impl<'a> GetTableForRead<ParamTableOffset> for EntryReader<'a> {
    type TablevalueType = Param;
    fn to_index(&self, offset: ParamTableOffset) -> Result<usize> {
        (offset.0 as usize).checked_sub(1).ok_or_else(|| {
            HaoError::InvalidEntryRefrence(std::any::type_name::<Self::TablevalueType>(), 0)
        })
    }
    fn get_table(&self) -> &[Ptr<Self::TablevalueType>] {
        self.entries.params.as_slice()
    }
}

impl<'a> GetTableForRead<MethodTableOffset> for EntryReader<'a> {
    type TablevalueType = Method;
    fn to_index(&self, offset: MethodTableOffset) -> Result<usize> {
        (offset.0 as usize).checked_sub(1).ok_or_else(|| {
            HaoError::InvalidEntryRefrence(std::any::type_name::<Self::TablevalueType>(), 0)
        })
    }
    fn get_table(&self) -> &[Ptr<Self::TablevalueType>] {
        self.entries.methods.as_slice()
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
    type EntryValue = SignatureDef;

    fn read(&self, identifier: BlobStreamOffset) -> Result<Self::EntryValue> {
        let mut reader = self
            .streams
            .blob_stream
            .get_signature_reader(identifier.0, self.entries)?;
        reader.read()
    }
}

impl<'a> ValueReadable<BlobStreamOffsetTypeSpec> for EntryReader<'a> {
    type EntryValue = TypeSigDef;

    fn read(&self, identifier: BlobStreamOffsetTypeSpec) -> Result<Self::EntryValue> {
        let mut reader = self
            .streams
            .blob_stream
            .get_signature_reader(identifier.0, self.entries)?;
        reader.read()
    }
}
