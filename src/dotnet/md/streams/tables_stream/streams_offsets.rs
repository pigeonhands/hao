use crate::{
    dotnet::md::MDStreamFlags,
    error::Result,
    io::{ReadData},
};
use super::TablesStreamReader;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StringsStreamOffset(pub u32);


impl<'a> ReadData<StringsStreamOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<StringsStreamOffset> {
        if self.header.flags.contains(MDStreamFlags::BigStrings) {
            Ok(StringsStreamOffset(self.read()?))
        } else {
            let small_str: u16 = self.read()?;
            Ok(StringsStreamOffset(small_str as u32))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlobStreamOffset(pub u32);

impl<'a> ReadData<BlobStreamOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<BlobStreamOffset> {
        if self.header.flags.contains(MDStreamFlags::BigBlob) {
            Ok(BlobStreamOffset(self.read()?))
        } else {
            let small_str: u16 = self.read()?;
            Ok(BlobStreamOffset(small_str as u32))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuidStreamOffset(pub u32);

impl<'a> ReadData<GuidStreamOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<GuidStreamOffset> {
        if self.header.flags.contains(MDStreamFlags::BigGUID) {
            Ok(GuidStreamOffset(self.read()?))
        } else {
            let small_str: u16 = self.read()?;
            Ok(GuidStreamOffset(small_str as u32))
        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModuleTableOffset(pub u32);

impl<'a> ReadData<ModuleTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ModuleTableOffset> {
        self.read_table_offset(self.header.table_rows.module)
            .map(ModuleTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TypeRefTableOffset(pub u32);

impl<'a> ReadData<TypeRefTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<TypeRefTableOffset> {
        self.read_table_offset(self.header.table_rows.type_ref)
            .map(TypeRefTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TypeDefTableOffset(pub u32);

impl<'a> ReadData<TypeDefTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<TypeDefTableOffset> {
        self.read_table_offset(self.header.table_rows.type_def)
            .map(TypeDefTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FieldPtrTableOffset(pub u32);

impl<'a> ReadData<FieldPtrTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<FieldPtrTableOffset> {
        self.read_table_offset(self.header.table_rows.field_ptr)
            .map(FieldPtrTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FieldTableOffset(pub u32);

impl<'a> ReadData<FieldTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<FieldTableOffset> {
        self.read_table_offset(self.header.table_rows.field)
            .map(FieldTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MethodPtrTableOffset(pub u32);

impl<'a> ReadData<MethodPtrTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<MethodPtrTableOffset> {
        self.read_table_offset(self.header.table_rows.method_ptr)
            .map(MethodPtrTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MethodTableOffset(pub u32);

impl<'a> ReadData<MethodTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<MethodTableOffset> {
        self.read_table_offset(self.header.table_rows.method)
            .map(MethodTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ParamPtrTableOffset(pub u32);

impl<'a> ReadData<ParamPtrTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ParamPtrTableOffset> {
        self.read_table_offset(self.header.table_rows.param_ptr)
            .map(ParamPtrTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ParamTableOffset(pub u32);

impl<'a> ReadData<ParamTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ParamTableOffset> {
        self.read_table_offset(self.header.table_rows.param)
            .map(ParamTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct InterfaceImplTableOffset(pub u32);

impl<'a> ReadData<InterfaceImplTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<InterfaceImplTableOffset> {
        self.read_table_offset(self.header.table_rows.interface_impl)
            .map(InterfaceImplTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MemberRefTableOffset(pub u32);

impl<'a> ReadData<MemberRefTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<MemberRefTableOffset> {
        self.read_table_offset(self.header.table_rows.member_ref)
            .map(MemberRefTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConstantTableOffset(pub u32);

impl<'a> ReadData<ConstantTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ConstantTableOffset> {
        self.read_table_offset(self.header.table_rows.constant)
            .map(ConstantTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CustomAttributeTableOffset(pub u32);

impl<'a> ReadData<CustomAttributeTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<CustomAttributeTableOffset> {
        self.read_table_offset(self.header.table_rows.custom_attribute)
            .map(CustomAttributeTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FieldMarshalTableOffset(pub u32);

impl<'a> ReadData<FieldMarshalTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<FieldMarshalTableOffset> {
        self.read_table_offset(self.header.table_rows.field_marshal)
            .map(FieldMarshalTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeclSecurityTableOffset(pub u32);

impl<'a> ReadData<DeclSecurityTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<DeclSecurityTableOffset> {
        self.read_table_offset(self.header.table_rows.decl_security)
            .map(DeclSecurityTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClassLayoutTableOffset(pub u32);

impl<'a> ReadData<ClassLayoutTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ClassLayoutTableOffset> {
        self.read_table_offset(self.header.table_rows.class_layout)
            .map(ClassLayoutTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FieldLayoutTableOffset(pub u32);

impl<'a> ReadData<FieldLayoutTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<FieldLayoutTableOffset> {
        self.read_table_offset(self.header.table_rows.field_layout)
            .map(FieldLayoutTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct StandAloneSigTableOffset(pub u32);

impl<'a> ReadData<StandAloneSigTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<StandAloneSigTableOffset> {
        self.read_table_offset(self.header.table_rows.stand_alone_sig)
            .map(StandAloneSigTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventMapTableOffset(pub u32);

impl<'a> ReadData<EventMapTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<EventMapTableOffset> {
        self.read_table_offset(self.header.table_rows.event_map)
            .map(EventMapTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventPtrTableOffset(pub u32);

impl<'a> ReadData<EventPtrTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<EventPtrTableOffset> {
        self.read_table_offset(self.header.table_rows.event_ptr)
            .map(EventPtrTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventTableOffset(pub u32);

impl<'a> ReadData<EventTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<EventTableOffset> {
        self.read_table_offset(self.header.table_rows.event)
            .map(EventTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PropertyMapTableOffset(pub u32);

impl<'a> ReadData<PropertyMapTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<PropertyMapTableOffset> {
        self.read_table_offset(self.header.table_rows.property_map)
            .map(PropertyMapTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PropertyPtrTableOffset(pub u32);

impl<'a> ReadData<PropertyPtrTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<PropertyPtrTableOffset> {
        self.read_table_offset(self.header.table_rows.property_ptr)
            .map(PropertyPtrTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PropertyTableOffset(pub u32);

impl<'a> ReadData<PropertyTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<PropertyTableOffset> {
        self.read_table_offset(self.header.table_rows.property)
            .map(PropertyTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MethodSemanticsTableOffset(pub u32);

impl<'a> ReadData<MethodSemanticsTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<MethodSemanticsTableOffset> {
        self.read_table_offset(self.header.table_rows.method_semantics)
            .map(MethodSemanticsTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MethodImplTableOffset(pub u32);

impl<'a> ReadData<MethodImplTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<MethodImplTableOffset> {
        self.read_table_offset(self.header.table_rows.method_impl)
            .map(MethodImplTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModuleRefTableOffset(pub u32);

impl<'a> ReadData<ModuleRefTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ModuleRefTableOffset> {
        self.read_table_offset(self.header.table_rows.module_ref)
            .map(ModuleRefTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TypeSpecTableOffset(pub u32);

impl<'a> ReadData<TypeSpecTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<TypeSpecTableOffset> {
        self.read_table_offset(self.header.table_rows.type_spec)
            .map(TypeSpecTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImplMapTableOffset(pub u32);

impl<'a> ReadData<ImplMapTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ImplMapTableOffset> {
        self.read_table_offset(self.header.table_rows.impl_map)
            .map(ImplMapTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FieldRVATableOffset(pub u32);

impl<'a> ReadData<FieldRVATableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<FieldRVATableOffset> {
        self.read_table_offset(self.header.table_rows.field_rva)
            .map(FieldRVATableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ENCLogTableOffset(pub u32);

impl<'a> ReadData<ENCLogTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ENCLogTableOffset> {
        self.read_table_offset(self.header.table_rows.enc_log)
            .map(ENCLogTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ENCMapTableOffset(pub u32);

impl<'a> ReadData<ENCMapTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ENCMapTableOffset> {
        self.read_table_offset(self.header.table_rows.enc_map)
            .map(ENCMapTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AssemblyTableOffset(pub u32);

impl<'a> ReadData<AssemblyTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<AssemblyTableOffset> {
        self.read_table_offset(self.header.table_rows.assembly)
            .map(AssemblyTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AssemblyProcessorTableOffset(pub u32);

impl<'a> ReadData<AssemblyProcessorTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<AssemblyProcessorTableOffset> {
        self.read_table_offset(self.header.table_rows.assembly_processor)
            .map(AssemblyProcessorTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AssemblyOSTableOffset(pub u32);

impl<'a> ReadData<AssemblyOSTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<AssemblyOSTableOffset> {
        self.read_table_offset(self.header.table_rows.assembly_os)
            .map(AssemblyOSTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AssemblyRefTableOffset(pub u32);

impl<'a> ReadData<AssemblyRefTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<AssemblyRefTableOffset> {
        self.read_table_offset(self.header.table_rows.assembly_ref)
            .map(AssemblyRefTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AssemblyRefProcessorTableOffset(pub u32);

impl<'a> ReadData<AssemblyRefProcessorTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<AssemblyRefProcessorTableOffset> {
        self.read_table_offset(self.header.table_rows.assembly_ref_processor)
            .map(AssemblyRefProcessorTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AssemblyRefOSTableOffset(pub u32);

impl<'a> ReadData<AssemblyRefOSTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<AssemblyRefOSTableOffset> {
        self.read_table_offset(self.header.table_rows.assembly_ref_os)
            .map(AssemblyRefOSTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileTableOffset(pub u32);

impl<'a> ReadData<FileTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<FileTableOffset> {
        self.read_table_offset(self.header.table_rows.file)
            .map(FileTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ExportedTypeTableOffset(pub u32);

impl<'a> ReadData<ExportedTypeTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ExportedTypeTableOffset> {
        self.read_table_offset(self.header.table_rows.exported_type)
            .map(ExportedTypeTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ManifestResourceTableOffset(pub u32);

impl<'a> ReadData<ManifestResourceTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ManifestResourceTableOffset> {
        self.read_table_offset(self.header.table_rows.manifest_resource)
            .map(ManifestResourceTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NestedClassTableOffset(pub u32);

impl<'a> ReadData<NestedClassTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<NestedClassTableOffset> {
        self.read_table_offset(self.header.table_rows.nested_class)
            .map(NestedClassTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GenericParamTableOffset(pub u32);

impl<'a> ReadData<GenericParamTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<GenericParamTableOffset> {
        self.read_table_offset(self.header.table_rows.generic_param)
            .map(GenericParamTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MethodSpecTableOffset(pub u32);

impl<'a> ReadData<MethodSpecTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<MethodSpecTableOffset> {
        self.read_table_offset(self.header.table_rows.method_spec)
            .map(MethodSpecTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GenericParamConstraintTableOffset(pub u32);

impl<'a> ReadData<GenericParamConstraintTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<GenericParamConstraintTableOffset> {
        self.read_table_offset(self.header.table_rows.generic_param_constraint)
            .map(GenericParamConstraintTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MethodDebugInformationTableOffset(pub u32);

impl<'a> ReadData<MethodDebugInformationTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<MethodDebugInformationTableOffset> {
        self.read_table_offset(self.header.table_rows.method_debug_information)
            .map(MethodDebugInformationTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LocalScopeTableOffset(pub u32);

impl<'a> ReadData<LocalScopeTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<LocalScopeTableOffset> {
        self.read_table_offset(self.header.table_rows.local_scope)
            .map(LocalScopeTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LocalVariableTableOffset(pub u32);

impl<'a> ReadData<LocalVariableTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<LocalVariableTableOffset> {
        self.read_table_offset(self.header.table_rows.local_variable)
            .map(LocalVariableTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LocalConstantTableOffset(pub u32);

impl<'a> ReadData<LocalConstantTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<LocalConstantTableOffset> {
        self.read_table_offset(self.header.table_rows.local_constant)
            .map(LocalConstantTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImportScopeTableOffset(pub u32);

impl<'a> ReadData<ImportScopeTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ImportScopeTableOffset> {
        self.read_table_offset(self.header.table_rows.import_scope)
            .map(ImportScopeTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct StateMachineMethodTableOffset(pub u32);

impl<'a> ReadData<StateMachineMethodTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<StateMachineMethodTableOffset> {
        self.read_table_offset(self.header.table_rows.state_machine_method)
            .map(StateMachineMethodTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CustomDebugInformationTableOffset(pub u32);

impl<'a> ReadData<CustomDebugInformationTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<CustomDebugInformationTableOffset> {
        self.read_table_offset(self.header.table_rows.custom_debug_information)
            .map(CustomDebugInformationTableOffset)
    }
}