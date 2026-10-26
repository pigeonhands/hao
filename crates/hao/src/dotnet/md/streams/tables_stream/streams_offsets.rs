use super::{TableRows, TablesStreamReader, ValueSize};
use crate::{dotnet::md::MDStreamFlags, error::Result, io::ReadData};

pub trait TableOffsetSize {
    fn table_offset_size(rows: &TableRows) -> ValueSize;
}

pub trait StreamsOffsetSize {
    fn streams_offset_size(rows: MDStreamFlags) -> ValueSize;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct StringsStreamOffset(pub u32);

impl StreamsOffsetSize for StringsStreamOffset {
    fn streams_offset_size(flags: MDStreamFlags) -> ValueSize {
        if flags.contains(MDStreamFlags::BigStrings) {
            ValueSize::Big
        } else {
            ValueSize::Small
        }
    }
}

impl<'a> ReadData<StringsStreamOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<StringsStreamOffset> {
        if StringsStreamOffset::streams_offset_size(self.header.flags) == ValueSize::Big {
            Ok(StringsStreamOffset(self.read()?))
        } else {
            let small_str: u16 = self.read()?;
            Ok(StringsStreamOffset(small_str as u32))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct BlobStreamOffset(pub u32);

impl StreamsOffsetSize for BlobStreamOffset {
    fn streams_offset_size(flags: MDStreamFlags) -> ValueSize {
        if flags.contains(MDStreamFlags::BigBlob) {
            ValueSize::Big
        } else {
            ValueSize::Small
        }
    }
}

impl<'a> ReadData<BlobStreamOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<BlobStreamOffset> {
        if BlobStreamOffset::streams_offset_size(self.header.flags) == ValueSize::Big {
            Ok(BlobStreamOffset(self.read()?))
        } else {
            let small_str: u16 = self.read()?;
            Ok(BlobStreamOffset(small_str as u32))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct BlobStreamOffsetTypeSpec(pub u32);

impl StreamsOffsetSize for BlobStreamOffsetTypeSpec {
    fn streams_offset_size(flags: MDStreamFlags) -> ValueSize {
        if flags.contains(MDStreamFlags::BigBlob) {
            ValueSize::Big
        } else {
            ValueSize::Small
        }
    }
}

impl<'a> ReadData<BlobStreamOffsetTypeSpec> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<BlobStreamOffsetTypeSpec> {
        if BlobStreamOffsetTypeSpec::streams_offset_size(self.header.flags) == ValueSize::Big {
            Ok(BlobStreamOffsetTypeSpec(self.read()?))
        } else {
            let small_str: u16 = self.read()?;
            Ok(BlobStreamOffsetTypeSpec(small_str as u32))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct GuidStreamOffset(pub u32);

impl StreamsOffsetSize for GuidStreamOffset {
    fn streams_offset_size(flags: MDStreamFlags) -> ValueSize {
        if flags.contains(MDStreamFlags::BigGUID) {
            ValueSize::Big
        } else {
            ValueSize::Small
        }
    }
}

impl<'a> ReadData<GuidStreamOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<GuidStreamOffset> {
        if GuidStreamOffset::streams_offset_size(self.header.flags) == ValueSize::Big {
            Ok(GuidStreamOffset(self.read()?))
        } else {
            let small_str: u16 = self.read()?;
            Ok(GuidStreamOffset(small_str as u32))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ModuleTableOffset(pub u32);

impl<'a> ReadData<ModuleTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ModuleTableOffset> {
        self.read_table_offset(self.header.table_locations.module)
            .map(ModuleTableOffset)
    }
}

impl TableOffsetSize for ModuleTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.module.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct TypeRefTableOffset(pub u32);

impl<'a> ReadData<TypeRefTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<TypeRefTableOffset> {
        self.read_table_offset(self.header.table_locations.type_ref)
            .map(TypeRefTableOffset)
    }
}

impl TableOffsetSize for TypeRefTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.type_ref.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct TypeDefTableOffset(pub u32);

impl<'a> ReadData<TypeDefTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<TypeDefTableOffset> {
        self.read_table_offset(self.header.table_locations.type_def)
            .map(TypeDefTableOffset)
    }
}

impl TableOffsetSize for TypeDefTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.type_def.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct FieldPtrTableOffset(pub u32);

impl<'a> ReadData<FieldPtrTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<FieldPtrTableOffset> {
        self.read_table_offset(self.header.table_locations.field_ptr)
            .map(FieldPtrTableOffset)
    }
}

impl TableOffsetSize for FieldPtrTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.field_ptr.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct FieldTableOffset(pub u32);

impl<'a> ReadData<FieldTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<FieldTableOffset> {
        self.read_table_offset(self.header.table_locations.field)
            .map(FieldTableOffset)
    }
}

impl TableOffsetSize for FieldTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.field.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct MethodPtrTableOffset(pub u32);

impl<'a> ReadData<MethodPtrTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<MethodPtrTableOffset> {
        self.read_table_offset(self.header.table_locations.method_ptr)
            .map(MethodPtrTableOffset)
    }
}

impl TableOffsetSize for MethodPtrTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.method_ptr.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct MethodTableOffset(pub u32);

impl<'a> ReadData<MethodTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<MethodTableOffset> {
        self.read_table_offset(self.header.table_locations.method)
            .map(MethodTableOffset)
    }
}

impl TableOffsetSize for MethodTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.method.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ParamPtrTableOffset(pub u32);

impl<'a> ReadData<ParamPtrTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ParamPtrTableOffset> {
        self.read_table_offset(self.header.table_locations.param_ptr)
            .map(ParamPtrTableOffset)
    }
}

impl TableOffsetSize for ParamPtrTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.param_ptr.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ParamTableOffset(pub u32);

impl<'a> ReadData<ParamTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ParamTableOffset> {
        self.read_table_offset(self.header.table_locations.param)
            .map(ParamTableOffset)
    }
}

impl TableOffsetSize for ParamTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.param.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct InterfaceImplTableOffset(pub u32);

impl<'a> ReadData<InterfaceImplTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<InterfaceImplTableOffset> {
        self.read_table_offset(self.header.table_locations.interface_impl)
            .map(InterfaceImplTableOffset)
    }
}

impl TableOffsetSize for InterfaceImplTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.interface_impl.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct MemberRefTableOffset(pub u32);

impl<'a> ReadData<MemberRefTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<MemberRefTableOffset> {
        self.read_table_offset(self.header.table_locations.member_ref)
            .map(MemberRefTableOffset)
    }
}

impl TableOffsetSize for MemberRefTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.member_ref.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ConstantTableOffset(pub u32);

impl<'a> ReadData<ConstantTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ConstantTableOffset> {
        self.read_table_offset(self.header.table_locations.constant)
            .map(ConstantTableOffset)
    }
}

impl TableOffsetSize for ConstantTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.constant.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct CustomAttributeTableOffset(pub u32);

impl<'a> ReadData<CustomAttributeTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<CustomAttributeTableOffset> {
        self.read_table_offset(self.header.table_locations.custom_attribute)
            .map(CustomAttributeTableOffset)
    }
}

impl TableOffsetSize for CustomAttributeTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.custom_attribute.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct FieldMarshalTableOffset(pub u32);

impl<'a> ReadData<FieldMarshalTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<FieldMarshalTableOffset> {
        self.read_table_offset(self.header.table_locations.field_marshal)
            .map(FieldMarshalTableOffset)
    }
}

impl TableOffsetSize for FieldMarshalTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.field_marshal.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct DeclSecurityTableOffset(pub u32);

impl<'a> ReadData<DeclSecurityTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<DeclSecurityTableOffset> {
        self.read_table_offset(self.header.table_locations.decl_security)
            .map(DeclSecurityTableOffset)
    }
}

impl TableOffsetSize for DeclSecurityTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.decl_security.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ClassLayoutTableOffset(pub u32);

impl<'a> ReadData<ClassLayoutTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ClassLayoutTableOffset> {
        self.read_table_offset(self.header.table_locations.class_layout)
            .map(ClassLayoutTableOffset)
    }
}

impl TableOffsetSize for ClassLayoutTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.class_layout.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct FieldLayoutTableOffset(pub u32);

impl<'a> ReadData<FieldLayoutTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<FieldLayoutTableOffset> {
        self.read_table_offset(self.header.table_locations.field_layout)
            .map(FieldLayoutTableOffset)
    }
}

impl TableOffsetSize for FieldLayoutTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.field_layout.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct StandAloneSigTableOffset(pub u32);

impl<'a> ReadData<StandAloneSigTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<StandAloneSigTableOffset> {
        self.read_table_offset(self.header.table_locations.stand_alone_sig)
            .map(StandAloneSigTableOffset)
    }
}

impl TableOffsetSize for StandAloneSigTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.stand_alone_sig.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct EventMapTableOffset(pub u32);

impl<'a> ReadData<EventMapTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<EventMapTableOffset> {
        self.read_table_offset(self.header.table_locations.event_map)
            .map(EventMapTableOffset)
    }
}

impl TableOffsetSize for EventMapTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.event_map.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct EventPtrTableOffset(pub u32);

impl<'a> ReadData<EventPtrTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<EventPtrTableOffset> {
        self.read_table_offset(self.header.table_locations.event_ptr)
            .map(EventPtrTableOffset)
    }
}

impl TableOffsetSize for EventPtrTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.event_ptr.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct EventTableOffset(pub u32);

impl<'a> ReadData<EventTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<EventTableOffset> {
        self.read_table_offset(self.header.table_locations.event)
            .map(EventTableOffset)
    }
}

impl TableOffsetSize for EventTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.event.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PropertyMapTableOffset(pub u32);

impl<'a> ReadData<PropertyMapTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<PropertyMapTableOffset> {
        self.read_table_offset(self.header.table_locations.property_map)
            .map(PropertyMapTableOffset)
    }
}

impl TableOffsetSize for PropertyMapTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.property_map.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PropertyPtrTableOffset(pub u32);

impl<'a> ReadData<PropertyPtrTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<PropertyPtrTableOffset> {
        self.read_table_offset(self.header.table_locations.property_ptr)
            .map(PropertyPtrTableOffset)
    }
}

impl TableOffsetSize for PropertyPtrTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.property_ptr.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PropertyTableOffset(pub u32);

impl<'a> ReadData<PropertyTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<PropertyTableOffset> {
        self.read_table_offset(self.header.table_locations.property)
            .map(PropertyTableOffset)
    }
}

impl TableOffsetSize for PropertyTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.property.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct MethodSemanticsTableOffset(pub u32);

impl<'a> ReadData<MethodSemanticsTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<MethodSemanticsTableOffset> {
        self.read_table_offset(self.header.table_locations.method_semantics)
            .map(MethodSemanticsTableOffset)
    }
}

impl TableOffsetSize for MethodSemanticsTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.method_semantics.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct MethodImplTableOffset(pub u32);

impl<'a> ReadData<MethodImplTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<MethodImplTableOffset> {
        self.read_table_offset(self.header.table_locations.method_impl)
            .map(MethodImplTableOffset)
    }
}

impl TableOffsetSize for MethodImplTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.method_impl.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ModuleRefTableOffset(pub u32);

impl<'a> ReadData<ModuleRefTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ModuleRefTableOffset> {
        self.read_table_offset(self.header.table_locations.module_ref)
            .map(ModuleRefTableOffset)
    }
}

impl TableOffsetSize for ModuleRefTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.module_ref.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct TypeSpecTableOffset(pub u32);

impl<'a> ReadData<TypeSpecTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<TypeSpecTableOffset> {
        self.read_table_offset(self.header.table_locations.type_spec)
            .map(TypeSpecTableOffset)
    }
}

impl TableOffsetSize for TypeSpecTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.type_spec.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ImplMapTableOffset(pub u32);

impl<'a> ReadData<ImplMapTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ImplMapTableOffset> {
        self.read_table_offset(self.header.table_locations.impl_map)
            .map(ImplMapTableOffset)
    }
}

impl TableOffsetSize for ImplMapTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.impl_map.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct FieldRVATableOffset(pub u32);

impl<'a> ReadData<FieldRVATableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<FieldRVATableOffset> {
        self.read_table_offset(self.header.table_locations.field_rva)
            .map(FieldRVATableOffset)
    }
}

impl TableOffsetSize for FieldRVATableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.field_rva.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ENCLogTableOffset(pub u32);

impl<'a> ReadData<ENCLogTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ENCLogTableOffset> {
        self.read_table_offset(self.header.table_locations.enc_log)
            .map(ENCLogTableOffset)
    }
}

impl TableOffsetSize for ENCLogTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.enc_log.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ENCMapTableOffset(pub u32);

impl<'a> ReadData<ENCMapTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ENCMapTableOffset> {
        self.read_table_offset(self.header.table_locations.enc_map)
            .map(ENCMapTableOffset)
    }
}

impl TableOffsetSize for ENCMapTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.enc_map.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct AssemblyTableOffset(pub u32);

impl<'a> ReadData<AssemblyTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<AssemblyTableOffset> {
        self.read_table_offset(self.header.table_locations.assembly)
            .map(AssemblyTableOffset)
    }
}

impl TableOffsetSize for AssemblyTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.assembly.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct AssemblyProcessorTableOffset(pub u32);

impl<'a> ReadData<AssemblyProcessorTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<AssemblyProcessorTableOffset> {
        self.read_table_offset(self.header.table_locations.assembly_processor)
            .map(AssemblyProcessorTableOffset)
    }
}

impl TableOffsetSize for AssemblyProcessorTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.assembly_processor.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct AssemblyOSTableOffset(pub u32);

impl<'a> ReadData<AssemblyOSTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<AssemblyOSTableOffset> {
        self.read_table_offset(self.header.table_locations.assembly_os)
            .map(AssemblyOSTableOffset)
    }
}

impl TableOffsetSize for AssemblyOSTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.assembly_os.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct AssemblyRefTableOffset(pub u32);

impl<'a> ReadData<AssemblyRefTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<AssemblyRefTableOffset> {
        self.read_table_offset(self.header.table_locations.assembly_ref)
            .map(AssemblyRefTableOffset)
    }
}

impl TableOffsetSize for AssemblyRefTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.assembly_ref.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct AssemblyRefProcessorTableOffset(pub u32);

impl<'a> ReadData<AssemblyRefProcessorTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<AssemblyRefProcessorTableOffset> {
        self.read_table_offset(self.header.table_locations.assembly_ref_processor)
            .map(AssemblyRefProcessorTableOffset)
    }
}

impl TableOffsetSize for AssemblyRefProcessorTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.assembly_ref_processor.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct AssemblyRefOSTableOffset(pub u32);

impl<'a> ReadData<AssemblyRefOSTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<AssemblyRefOSTableOffset> {
        self.read_table_offset(self.header.table_locations.assembly_ref_os)
            .map(AssemblyRefOSTableOffset)
    }
}

impl TableOffsetSize for AssemblyRefOSTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.assembly_ref_os.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct FileTableOffset(pub u32);

impl<'a> ReadData<FileTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<FileTableOffset> {
        self.read_table_offset(self.header.table_locations.file)
            .map(FileTableOffset)
    }
}

impl TableOffsetSize for FileTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.file.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ExportedTypeTableOffset(pub u32);

impl<'a> ReadData<ExportedTypeTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ExportedTypeTableOffset> {
        self.read_table_offset(self.header.table_locations.exported_type)
            .map(ExportedTypeTableOffset)
    }
}

impl TableOffsetSize for ExportedTypeTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.exported_type.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ManifestResourceTableOffset(pub u32);

impl<'a> ReadData<ManifestResourceTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ManifestResourceTableOffset> {
        self.read_table_offset(self.header.table_locations.manifest_resource)
            .map(ManifestResourceTableOffset)
    }
}

impl TableOffsetSize for ManifestResourceTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.manifest_resource.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct NestedClassTableOffset(pub u32);

impl<'a> ReadData<NestedClassTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<NestedClassTableOffset> {
        self.read_table_offset(self.header.table_locations.nested_class)
            .map(NestedClassTableOffset)
    }
}

impl TableOffsetSize for NestedClassTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.nested_class.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct GenericParamTableOffset(pub u32);

impl<'a> ReadData<GenericParamTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<GenericParamTableOffset> {
        self.read_table_offset(self.header.table_locations.generic_param)
            .map(GenericParamTableOffset)
    }
}

impl TableOffsetSize for GenericParamTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.generic_param.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct MethodSpecTableOffset(pub u32);

impl<'a> ReadData<MethodSpecTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<MethodSpecTableOffset> {
        self.read_table_offset(self.header.table_locations.method_spec)
            .map(MethodSpecTableOffset)
    }
}

impl TableOffsetSize for MethodSpecTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.method_spec.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct GenericParamConstraintTableOffset(pub u32);

impl<'a> ReadData<GenericParamConstraintTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<GenericParamConstraintTableOffset> {
        self.read_table_offset(self.header.table_locations.generic_param_constraint)
            .map(GenericParamConstraintTableOffset)
    }
}

impl TableOffsetSize for GenericParamConstraintTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.generic_param_constraint.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct DocumentTableRowOffset(pub u32);

impl<'a> ReadData<DocumentTableRowOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<DocumentTableRowOffset> {
        self.read_table_offset(self.header.table_locations.generic_param)
            .map(DocumentTableRowOffset)
    }
}

impl TableOffsetSize for DocumentTableRowOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.document.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct MethodDebugInformationTableOffset(pub u32);

impl<'a> ReadData<MethodDebugInformationTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<MethodDebugInformationTableOffset> {
        self.read_table_offset(self.header.table_locations.method_debug_information)
            .map(MethodDebugInformationTableOffset)
    }
}

impl TableOffsetSize for MethodDebugInformationTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.method_debug_information.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct LocalScopeTableOffset(pub u32);

impl<'a> ReadData<LocalScopeTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<LocalScopeTableOffset> {
        self.read_table_offset(self.header.table_locations.local_scope)
            .map(LocalScopeTableOffset)
    }
}

impl TableOffsetSize for LocalScopeTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.local_scope.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct LocalVariableTableOffset(pub u32);

impl<'a> ReadData<LocalVariableTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<LocalVariableTableOffset> {
        self.read_table_offset(self.header.table_locations.local_variable)
            .map(LocalVariableTableOffset)
    }
}

impl TableOffsetSize for LocalVariableTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.local_variable.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct LocalConstantTableOffset(pub u32);

impl<'a> ReadData<LocalConstantTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<LocalConstantTableOffset> {
        self.read_table_offset(self.header.table_locations.local_constant)
            .map(LocalConstantTableOffset)
    }
}

impl TableOffsetSize for LocalConstantTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.local_constant.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ImportScopeTableOffset(pub u32);

impl<'a> ReadData<ImportScopeTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ImportScopeTableOffset> {
        self.read_table_offset(self.header.table_locations.import_scope)
            .map(ImportScopeTableOffset)
    }
}

impl TableOffsetSize for ImportScopeTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.import_scope.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct StateMachineMethodTableOffset(pub u32);

impl<'a> ReadData<StateMachineMethodTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<StateMachineMethodTableOffset> {
        self.read_table_offset(self.header.table_locations.state_machine_method)
            .map(StateMachineMethodTableOffset)
    }
}

impl TableOffsetSize for StateMachineMethodTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.state_machine_method.row_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct CustomDebugInformationTableOffset(pub u32);

impl<'a> ReadData<CustomDebugInformationTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<CustomDebugInformationTableOffset> {
        self.read_table_offset(self.header.table_locations.custom_debug_information)
            .map(CustomDebugInformationTableOffset)
    }
}

impl TableOffsetSize for CustomDebugInformationTableOffset {
    fn table_offset_size(rows: &TableRows) -> ValueSize {
        rows.custom_debug_information.row_size()
    }
}
