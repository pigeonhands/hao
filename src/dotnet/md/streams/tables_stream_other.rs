use bitflags::bitflags;

use crate::{
    dotnet::md::MDStreamFlags,
    error::Result,
    io::{DataReader, ReadData, Readable},
};

use super::{
    coded_tokens::{CodedToken, CodedTokenSizes, ResolutionScopeToken, TypeDefOrRefToken},
    Version,
};


#[derive(Debug, Clone)]
pub struct TablesValues {
    pub module: ModulesTable,
    pub type_ref: Vec<TypeRefTableRow>,
    pub type_def: Vec<TypeDefTableRow>,
}

impl<'a> ReadData<TablesValues> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<TablesValues> {
        Ok(TablesValues {
            module: self.read()?,
            type_ref: self.read_rows(self.header.table_rows.type_ref)?,
            type_def: self.read_rows(self.header.table_rows.type_def)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct TablesStreams {
    pub header: TablesStreamsHeader,
    pub values: TablesValues,
}

impl TablesStreams {
    pub fn from_reader(mut reader: DataReader) -> Result<Self> {
        let header = reader.read()?;

        let values: TablesValues = {
            let mut tables_reader = TablesStreamReader::new(reader.remaning_slice(), &header);
            tables_reader.read()?
        };

        Ok(Self { header, values })
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct TableFlags: u64 {
        const Module	= 1<<0;
        const TypeRef	= 1<<1;
        const TypeDef	= 1<<2;
        const FieldPtr	= 1<<3;
        const Field	    = 1<<4;
        const MethodPtr	= 1<<5;
        const Method	= 1<<6;
        const ParamPtr	= 1<<7;
        const Param	    = 1<<8;
        const InterfaceImpl	= 1<<9;
        const MemberRef	    = 1<<10;
        const Constant	    = 1<<11;
        const CustomAttribute	= 1<<12;
        const FieldMarshal	= 1<<13;
        const DeclSecurity	= 1<<14;
        const ClassLayout	= 1<<15;
        const FieldLayout	= 1<<16;
        const StandAloneSig	= 1<<17;
        const EventMap	= 1<<18;
        const EventPtr	= 1<<19;
        const Event	= 1<<20;
        const PropertyMap	= 1<<21;
        const PropertyPtr	= 1<<22;
        const Property	= 1<<23;
        const MethodSemantics	= 1<<24;
        const MethodImpl	= 1<<25;
        const ModuleRef	= 1<<26;
        const TypeSpec	= 1<<27;
        const ImplMap	= 1<<28;
        const FieldRva	= 1<<29;
        const EncLog	= 1<<30;
        const EncMap	= 1<<31;
        const Assembly	= 1<<32;
        const AssemblyProcessor	= 1<<33;
        const AssemblyOs	= 1<<34;
        const AssemblyRef	= 1<<35;
        const AssemblyRefProcessor	= 1<<36;
        const AssemblyRefOs	= 1<<37;
        const File	= 1<<38;
        const ExportedType	= 1<<39;
        const ManifestResource	= 1<<40;
        const NestedClass	= 1<<41;
        const GenericParam	= 1<<42;
        const MethodSpec	= 1<<43;
        const GenericParamConstraint	= 1<<44;
        const Document	= 1<<45;
        const MethodDebugInformation	= 1<<46;
        const LocalScope	= 1<<47;
        const LocalVariable	= 1<<48;
        const LocalConstant	= 1<<49;
        const ImportScope	= 1<<50;
        const StateMachineMethod	= 1<<51;
        const CustomDebugInformation	= 1<<52;

    }
}

#[derive(Debug, Clone)]
pub struct TableRows {
    pub module: u32,
    pub type_ref: u32,
    pub type_def: u32,
    pub field_ptr: u32,
    pub field: u32,
    pub method_ptr: u32,
    pub method: u32,
    pub param_ptr: u32,
    pub param: u32,
    pub interface_impl: u32,
    pub member_ref: u32,
    pub constant: u32,
    pub custom_attribute: u32,
    pub field_marshal: u32,
    pub decl_security: u32,
    pub class_layout: u32,
    pub field_layout: u32,
    pub stand_alone_sig: u32,
    pub event_map: u32,
    pub event_ptr: u32,
    pub event: u32,
    pub property_map: u32,
    pub property_ptr: u32,
    pub property: u32,
    pub method_semantics: u32,
    pub method_impl: u32,
    pub module_ref: u32,
    pub type_spec: u32,
    pub impl_map: u32,
    pub field_rva: u32,
    pub enc_log: u32,
    pub enc_map: u32,
    pub assembly: u32,
    pub assembly_processor: u32,
    pub assembly_os: u32,
    pub assembly_ref: u32,
    pub assembly_ref_processor: u32,
    pub assembly_ref_os: u32,
    pub file: u32,
    pub exported_type: u32,
    pub manifest_resource: u32,
    pub nested_class: u32,
    pub generic_param: u32,
    pub method_spec: u32,
    pub generic_param_constraint: u32,
    pub document: u32,
    pub method_debug_information: u32,
    pub local_scope: u32,
    pub local_variable: u32,
    pub local_constant: u32,
    pub import_scope: u32,
    pub state_machine_method: u32,
    pub custom_debug_information: u32,
}

impl TableRows {
    pub fn from_reader(reader: &mut DataReader, valid_rows: TableFlags) -> Result<Self> {
        fn read_if_flag(
            reader: &mut DataReader,
            valid_rows: TableFlags,
            flag: TableFlags,
        ) -> Result<u32> {
            if valid_rows.contains(flag) {
                reader.read()
            } else {
                Ok(0)
            }
        }

        Ok(Self {
            module: read_if_flag(reader, valid_rows, TableFlags::Module)?,
            type_ref: read_if_flag(reader, valid_rows, TableFlags::TypeRef)?,
            type_def: read_if_flag(reader, valid_rows, TableFlags::TypeDef)?,
            field_ptr: read_if_flag(reader, valid_rows, TableFlags::FieldPtr)?,
            field: read_if_flag(reader, valid_rows, TableFlags::Field)?,
            method_ptr: read_if_flag(reader, valid_rows, TableFlags::MethodPtr)?,
            method: read_if_flag(reader, valid_rows, TableFlags::Method)?,
            param_ptr: read_if_flag(reader, valid_rows, TableFlags::ParamPtr)?,
            param: read_if_flag(reader, valid_rows, TableFlags::Param)?,
            interface_impl: read_if_flag(reader, valid_rows, TableFlags::InterfaceImpl)?,
            member_ref: read_if_flag(reader, valid_rows, TableFlags::MemberRef)?,
            constant: read_if_flag(reader, valid_rows, TableFlags::Constant)?,
            custom_attribute: read_if_flag(reader, valid_rows, TableFlags::CustomAttribute)?,
            field_marshal: read_if_flag(reader, valid_rows, TableFlags::FieldMarshal)?,
            decl_security: read_if_flag(reader, valid_rows, TableFlags::DeclSecurity)?,
            class_layout: read_if_flag(reader, valid_rows, TableFlags::ClassLayout)?,
            field_layout: read_if_flag(reader, valid_rows, TableFlags::FieldLayout)?,
            stand_alone_sig: read_if_flag(reader, valid_rows, TableFlags::StandAloneSig)?,
            event_map: read_if_flag(reader, valid_rows, TableFlags::EventMap)?,
            event_ptr: read_if_flag(reader, valid_rows, TableFlags::EventPtr)?,
            event: read_if_flag(reader, valid_rows, TableFlags::Event)?,
            property_map: read_if_flag(reader, valid_rows, TableFlags::PropertyMap)?,
            property_ptr: read_if_flag(reader, valid_rows, TableFlags::PropertyPtr)?,
            property: read_if_flag(reader, valid_rows, TableFlags::Property)?,
            method_semantics: read_if_flag(reader, valid_rows, TableFlags::MethodSemantics)?,
            method_impl: read_if_flag(reader, valid_rows, TableFlags::MethodImpl)?,
            module_ref: read_if_flag(reader, valid_rows, TableFlags::ModuleRef)?,
            type_spec: read_if_flag(reader, valid_rows, TableFlags::TypeSpec)?,
            impl_map: read_if_flag(reader, valid_rows, TableFlags::ImplMap)?,
            field_rva: read_if_flag(reader, valid_rows, TableFlags::FieldRva)?,
            enc_log: read_if_flag(reader, valid_rows, TableFlags::EncLog)?,
            enc_map: read_if_flag(reader, valid_rows, TableFlags::EncMap)?,
            assembly: read_if_flag(reader, valid_rows, TableFlags::Assembly)?,
            assembly_processor: read_if_flag(reader, valid_rows, TableFlags::AssemblyProcessor)?,
            assembly_os: read_if_flag(reader, valid_rows, TableFlags::AssemblyOs)?,
            assembly_ref: read_if_flag(reader, valid_rows, TableFlags::AssemblyRef)?,
            assembly_ref_processor: read_if_flag(
                reader,
                valid_rows,
                TableFlags::AssemblyRefProcessor,
            )?,
            assembly_ref_os: read_if_flag(reader, valid_rows, TableFlags::AssemblyRefOs)?,
            file: read_if_flag(reader, valid_rows, TableFlags::File)?,
            exported_type: read_if_flag(reader, valid_rows, TableFlags::ExportedType)?,
            manifest_resource: read_if_flag(reader, valid_rows, TableFlags::ManifestResource)?,
            nested_class: read_if_flag(reader, valid_rows, TableFlags::NestedClass)?,
            generic_param: read_if_flag(reader, valid_rows, TableFlags::GenericParam)?,
            method_spec: read_if_flag(reader, valid_rows, TableFlags::MethodSpec)?,
            generic_param_constraint: read_if_flag(
                reader,
                valid_rows,
                TableFlags::GenericParamConstraint,
            )?,
            document: read_if_flag(reader, valid_rows, TableFlags::Document)?,
            method_debug_information: read_if_flag(
                reader,
                valid_rows,
                TableFlags::MethodDebugInformation,
            )?,
            local_scope: read_if_flag(reader, valid_rows, TableFlags::LocalScope)?,
            local_variable: read_if_flag(reader, valid_rows, TableFlags::LocalVariable)?,
            local_constant: read_if_flag(reader, valid_rows, TableFlags::LocalConstant)?,
            import_scope: read_if_flag(reader, valid_rows, TableFlags::ImportScope)?,
            state_machine_method: read_if_flag(reader, valid_rows, TableFlags::StateMachineMethod)?,
            custom_debug_information: read_if_flag(
                reader,
                valid_rows,
                TableFlags::CustomDebugInformation,
            )?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct TablesStreamsHeader {
    pub valid_tables: TableFlags,
    pub table_rows: TableRows,
    pub flags: MDStreamFlags,
    pub version: Version,
    pub sorted_tables: TableFlags,
    pub coded_token_sizes: CodedTokenSizes,
}

impl Readable for TablesStreamsHeader {
    fn from_reader(reader: &mut DataReader) -> Result<Self> {
        let _reserved1: u32 = reader.read()?;
        let major_version: u8 = reader.read()?;
        let minor_version: u8 = reader.read()?;
        let flags = MDStreamFlags::from_bits_retain(reader.read()?);
        let _log_2_rid: u8 = reader.read()?;
        let valid_tables = TableFlags::from_bits_retain(reader.read()?);
        let sorted_tables = TableFlags::from_bits_retain(reader.read()?);

        if flags.contains(MDStreamFlags::ExtraData) {
            let _extra: u32 = reader.read()?;
        }

        let table_rows = TableRows::from_reader(reader, valid_tables)?;

        Ok(Self {
            valid_tables,
            coded_token_sizes: CodedTokenSizes::from_header(&table_rows),
            table_rows,
            flags,
            version: Version(major_version, minor_version),
            sorted_tables,
        })
    }
}

#[derive(Debug, Clone)]
pub struct StringsStreamOffset(u32);

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

#[derive(Debug, Clone)]
pub struct GuidStreamOffset(u32);

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
pub struct ModuleTableOffset(u32);

impl<'a> ReadData<ModuleTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ModuleTableOffset> {
        self.read_table_offset(self.header.table_rows.module)
            .map(ModuleTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TypeRefTableOffset(u32);

impl<'a> ReadData<TypeRefTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<TypeRefTableOffset> {
        self.read_table_offset(self.header.table_rows.type_ref)
            .map(TypeRefTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TypeDefTableOffset(u32);

impl<'a> ReadData<TypeDefTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<TypeDefTableOffset> {
        self.read_table_offset(self.header.table_rows.type_def)
            .map(TypeDefTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FieldPtrTableOffset(u32);

impl<'a> ReadData<FieldPtrTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<FieldPtrTableOffset> {
        self.read_table_offset(self.header.table_rows.field_ptr)
            .map(FieldPtrTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FieldTableOffset(u32);

impl<'a> ReadData<FieldTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<FieldTableOffset> {
        self.read_table_offset(self.header.table_rows.field)
            .map(FieldTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MethodPtrTableOffset(u32);

impl<'a> ReadData<MethodPtrTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<MethodPtrTableOffset> {
        self.read_table_offset(self.header.table_rows.method_ptr)
            .map(MethodPtrTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MethodTableOffset(u32);

impl<'a> ReadData<MethodTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<MethodTableOffset> {
        self.read_table_offset(self.header.table_rows.method)
            .map(MethodTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ParamPtrTableOffset(u32);

impl<'a> ReadData<ParamPtrTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ParamPtrTableOffset> {
        self.read_table_offset(self.header.table_rows.param_ptr)
            .map(ParamPtrTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ParamTableOffset(u32);

impl<'a> ReadData<ParamTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ParamTableOffset> {
        self.read_table_offset(self.header.table_rows.param)
            .map(ParamTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct InterfaceImplTableOffset(u32);

impl<'a> ReadData<InterfaceImplTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<InterfaceImplTableOffset> {
        self.read_table_offset(self.header.table_rows.interface_impl)
            .map(InterfaceImplTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MemberRefTableOffset(u32);

impl<'a> ReadData<MemberRefTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<MemberRefTableOffset> {
        self.read_table_offset(self.header.table_rows.member_ref)
            .map(MemberRefTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConstantTableOffset(u32);

impl<'a> ReadData<ConstantTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ConstantTableOffset> {
        self.read_table_offset(self.header.table_rows.constant)
            .map(ConstantTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CustomAttributeTableOffset(u32);

impl<'a> ReadData<CustomAttributeTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<CustomAttributeTableOffset> {
        self.read_table_offset(self.header.table_rows.custom_attribute)
            .map(CustomAttributeTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FieldMarshalTableOffset(u32);

impl<'a> ReadData<FieldMarshalTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<FieldMarshalTableOffset> {
        self.read_table_offset(self.header.table_rows.field_marshal)
            .map(FieldMarshalTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeclSecurityTableOffset(u32);

impl<'a> ReadData<DeclSecurityTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<DeclSecurityTableOffset> {
        self.read_table_offset(self.header.table_rows.decl_security)
            .map(DeclSecurityTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClassLayoutTableOffset(u32);

impl<'a> ReadData<ClassLayoutTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ClassLayoutTableOffset> {
        self.read_table_offset(self.header.table_rows.class_layout)
            .map(ClassLayoutTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FieldLayoutTableOffset(u32);

impl<'a> ReadData<FieldLayoutTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<FieldLayoutTableOffset> {
        self.read_table_offset(self.header.table_rows.field_layout)
            .map(FieldLayoutTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct StandAloneSigTableOffset(u32);

impl<'a> ReadData<StandAloneSigTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<StandAloneSigTableOffset> {
        self.read_table_offset(self.header.table_rows.stand_alone_sig)
            .map(StandAloneSigTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventMapTableOffset(u32);

impl<'a> ReadData<EventMapTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<EventMapTableOffset> {
        self.read_table_offset(self.header.table_rows.event_map)
            .map(EventMapTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventPtrTableOffset(u32);

impl<'a> ReadData<EventPtrTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<EventPtrTableOffset> {
        self.read_table_offset(self.header.table_rows.event_ptr)
            .map(EventPtrTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventTableOffset(u32);

impl<'a> ReadData<EventTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<EventTableOffset> {
        self.read_table_offset(self.header.table_rows.event)
            .map(EventTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PropertyMapTableOffset(u32);

impl<'a> ReadData<PropertyMapTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<PropertyMapTableOffset> {
        self.read_table_offset(self.header.table_rows.property_map)
            .map(PropertyMapTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PropertyPtrTableOffset(u32);

impl<'a> ReadData<PropertyPtrTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<PropertyPtrTableOffset> {
        self.read_table_offset(self.header.table_rows.property_ptr)
            .map(PropertyPtrTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PropertyTableOffset(u32);

impl<'a> ReadData<PropertyTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<PropertyTableOffset> {
        self.read_table_offset(self.header.table_rows.property)
            .map(PropertyTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MethodSemanticsTableOffset(u32);

impl<'a> ReadData<MethodSemanticsTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<MethodSemanticsTableOffset> {
        self.read_table_offset(self.header.table_rows.method_semantics)
            .map(MethodSemanticsTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MethodImplTableOffset(u32);

impl<'a> ReadData<MethodImplTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<MethodImplTableOffset> {
        self.read_table_offset(self.header.table_rows.method_impl)
            .map(MethodImplTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModuleRefTableOffset(u32);

impl<'a> ReadData<ModuleRefTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ModuleRefTableOffset> {
        self.read_table_offset(self.header.table_rows.module_ref)
            .map(ModuleRefTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TypeSpecTableOffset(u32);

impl<'a> ReadData<TypeSpecTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<TypeSpecTableOffset> {
        self.read_table_offset(self.header.table_rows.type_spec)
            .map(TypeSpecTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImplMapTableOffset(u32);

impl<'a> ReadData<ImplMapTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ImplMapTableOffset> {
        self.read_table_offset(self.header.table_rows.impl_map)
            .map(ImplMapTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FieldRVATableOffset(u32);

impl<'a> ReadData<FieldRVATableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<FieldRVATableOffset> {
        self.read_table_offset(self.header.table_rows.field_rva)
            .map(FieldRVATableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ENCLogTableOffset(u32);

impl<'a> ReadData<ENCLogTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ENCLogTableOffset> {
        self.read_table_offset(self.header.table_rows.enc_log)
            .map(ENCLogTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ENCMapTableOffset(u32);

impl<'a> ReadData<ENCMapTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ENCMapTableOffset> {
        self.read_table_offset(self.header.table_rows.enc_map)
            .map(ENCMapTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AssemblyTableOffset(u32);

impl<'a> ReadData<AssemblyTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<AssemblyTableOffset> {
        self.read_table_offset(self.header.table_rows.assembly)
            .map(AssemblyTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AssemblyProcessorTableOffset(u32);

impl<'a> ReadData<AssemblyProcessorTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<AssemblyProcessorTableOffset> {
        self.read_table_offset(self.header.table_rows.assembly_processor)
            .map(AssemblyProcessorTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AssemblyOSTableOffset(u32);

impl<'a> ReadData<AssemblyOSTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<AssemblyOSTableOffset> {
        self.read_table_offset(self.header.table_rows.assembly_os)
            .map(AssemblyOSTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AssemblyRefTableOffset(u32);

impl<'a> ReadData<AssemblyRefTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<AssemblyRefTableOffset> {
        self.read_table_offset(self.header.table_rows.assembly_ref)
            .map(AssemblyRefTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AssemblyRefProcessorTableOffset(u32);

impl<'a> ReadData<AssemblyRefProcessorTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<AssemblyRefProcessorTableOffset> {
        self.read_table_offset(self.header.table_rows.assembly_ref_processor)
            .map(AssemblyRefProcessorTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AssemblyRefOSTableOffset(u32);

impl<'a> ReadData<AssemblyRefOSTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<AssemblyRefOSTableOffset> {
        self.read_table_offset(self.header.table_rows.assembly_ref_os)
            .map(AssemblyRefOSTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileTableOffset(u32);

impl<'a> ReadData<FileTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<FileTableOffset> {
        self.read_table_offset(self.header.table_rows.file)
            .map(FileTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ExportedTypeTableOffset(u32);

impl<'a> ReadData<ExportedTypeTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ExportedTypeTableOffset> {
        self.read_table_offset(self.header.table_rows.exported_type)
            .map(ExportedTypeTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ManifestResourceTableOffset(u32);

impl<'a> ReadData<ManifestResourceTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ManifestResourceTableOffset> {
        self.read_table_offset(self.header.table_rows.manifest_resource)
            .map(ManifestResourceTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NestedClassTableOffset(u32);

impl<'a> ReadData<NestedClassTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<NestedClassTableOffset> {
        self.read_table_offset(self.header.table_rows.nested_class)
            .map(NestedClassTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GenericParamTableOffset(u32);

impl<'a> ReadData<GenericParamTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<GenericParamTableOffset> {
        self.read_table_offset(self.header.table_rows.generic_param)
            .map(GenericParamTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MethodSpecTableOffset(u32);

impl<'a> ReadData<MethodSpecTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<MethodSpecTableOffset> {
        self.read_table_offset(self.header.table_rows.method_spec)
            .map(MethodSpecTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GenericParamConstraintTableOffset(u32);

impl<'a> ReadData<GenericParamConstraintTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<GenericParamConstraintTableOffset> {
        self.read_table_offset(self.header.table_rows.generic_param_constraint)
            .map(GenericParamConstraintTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MethodDebugInformationTableOffset(u32);

impl<'a> ReadData<MethodDebugInformationTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<MethodDebugInformationTableOffset> {
        self.read_table_offset(self.header.table_rows.method_debug_information)
            .map(MethodDebugInformationTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LocalScopeTableOffset(u32);

impl<'a> ReadData<LocalScopeTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<LocalScopeTableOffset> {
        self.read_table_offset(self.header.table_rows.local_scope)
            .map(LocalScopeTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LocalVariableTableOffset(u32);

impl<'a> ReadData<LocalVariableTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<LocalVariableTableOffset> {
        self.read_table_offset(self.header.table_rows.local_variable)
            .map(LocalVariableTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LocalConstantTableOffset(u32);

impl<'a> ReadData<LocalConstantTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<LocalConstantTableOffset> {
        self.read_table_offset(self.header.table_rows.local_constant)
            .map(LocalConstantTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImportScopeTableOffset(u32);

impl<'a> ReadData<ImportScopeTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ImportScopeTableOffset> {
        self.read_table_offset(self.header.table_rows.import_scope)
            .map(ImportScopeTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct StateMachineMethodTableOffset(u32);

impl<'a> ReadData<StateMachineMethodTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<StateMachineMethodTableOffset> {
        self.read_table_offset(self.header.table_rows.state_machine_method)
            .map(StateMachineMethodTableOffset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CustomDebugInformationTableOffset(u32);

impl<'a> ReadData<CustomDebugInformationTableOffset> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<CustomDebugInformationTableOffset> {
        self.read_table_offset(self.header.table_rows.custom_debug_information)
            .map(CustomDebugInformationTableOffset)
    }
}

pub struct TablesStreamReader<'a> {
    pub reader: DataReader<'a>,
    pub header: &'a TablesStreamsHeader,
}

impl<'a> TablesStreamReader<'a> {
    pub fn new(tables_stream_data: &'a [u8], header: &'a TablesStreamsHeader) -> Self {
        Self {
            reader: DataReader::new(tables_stream_data),
            header,
        }
    }

    fn read_table_offset(&mut self, target_table_size: u32) -> Result<u32> {
        if target_table_size > u16::MAX {
            self.read()
        } else {
            let small_val: u16 = self.read()?;
            Ok(small_val as u32)
        }
    }

    pub fn read_rows<T>(&mut self, row_count: u32) -> Result<Vec<T>>
    where
        Self: ReadData<T>,
    {
        let mut data = Vec::with_capacity(row_count as usize);
        for _ in 0..row_count {
            data.push(self.read()?)
        }
        Ok(data)
    }
}

impl<'a, T> ReadData<T> for TablesStreamReader<'a>
where
    DataReader<'a>: ReadData<T>,
{
    fn read(&mut self) -> Result<T> {
        self.reader.read()
    }
}

#[derive(Debug, Clone)]
pub struct ModulesTable {
    pub generation: u16,
    pub name: StringsStreamOffset,
    pub mvid: GuidStreamOffset,
    pub enc_id: GuidStreamOffset,
    pub enc_base_id: GuidStreamOffset,
}

impl<'a> ReadData<ModulesTable> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ModulesTable> {
        if self.header.table_rows.module != 1 {
            return Err(crate::error::HaoError::BadImageFormat(
                "Module table should have exactly one entry",
            ));
        }
        Ok(ModulesTable {
            generation: self.read()?,
            name: self.read()?,
            mvid: self.read()?,
            enc_id: self.read()?,
            enc_base_id: self.read()?,
        })
    }
}


#[derive(Debug, Clone)]
pub struct TypeRefTableRow {
    pub resolution_scope: CodedToken<ResolutionScopeToken>,
    pub name: StringsStreamOffset,
    pub namespace: StringsStreamOffset,
}

impl<'a> ReadData<TypeRefTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<TypeRefTableRow> {
        Ok(TypeRefTableRow {
            resolution_scope: self.read()?,
            name: self.read()?,
            namespace: self.read()?,
        })
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct TypeDefFlags: u32 {
                 // Use this mask to retrieve the type visibility information.
        const     VisibilityMask        =   0x00000007;
        const     NotPublic             =   0x00000000;     // Class is not public scope.
        const     Public                =   0x00000001;     // Class is public scope.
        const     NestedPublic          =   0x00000002;     // Class is nested with public visibility.
        const     NestedPrivate         =   0x00000003;     // Class is nested with private visibility.
        const     NestedFamily          =   0x00000004;     // Class is nested with family visibility.
        const     NestedAssembly        =   0x00000005;     // Class is nested with assembly visibility.
        const     NestedFamANDAssem     =   0x00000006;     // Class is nested with family and assembly visibility.
        const     NestedFamORAssem      =   0x00000007;     // Class is nested with family or assembly visibility.

            // Use this mask to retrieve class layout information
        const     LayoutMask            =   0x00000018;
        const     AutoLayout            =   0x00000000;     // Class fields are auto-laid out
        const     SequentialLayout      =   0x00000008;     // Class fields are laid out sequentially
        const     ExplicitLayout        =   0x00000010;     // Layout is supplied explicitly
             // end layout mask

            // Use this mask to retrieve class semantics information.
        const     ClassSemanticsMask    =   0x00000060;
        const     Class                 =   0x00000000;     // Type is a class.
        const     Interface             =   0x00000020;     // Type is an interface.
             // end semantics mask

            // Special semantics in addition to class semantics.
        const     Abstract              =   0x00000080;     // Class is abstract
        const     Sealed                =   0x00000100;     // Class is concrete and may not be extended
        const     SpecialName           =   0x00000400;     // Class name is special. Name describes how.

            // Implementation attributes.
        const     Import                =   0x00001000;     // Class / interface is imported
        const     Serializable          =   0x00002000;     // The class is Serializable.

            // Use StringFormatMask to retrieve string information for native interop
        const     StringFormatMask      =   0x00030000;
        const     AnsiClass             =   0x00000000;     // LPTSTR is interpreted as ANSI in this class
        const     UnicodeClass          =   0x00010000;     // LPTSTR is interpreted as UNICODE
        const     AutoClass             =   0x00020000;     // LPTSTR is interpreted automatically
        const     CustomFormatClass     =   0x00030000;     // A non-standard encoding specified by CustomFormatMask
        const     CustomFormatMask      =   0x00C00000;     // Use this mask to retrieve non-standard encoding information for native interop. The meaning of the values of these 2 bits is unspecified.

            // end string format mask
        const     BeforeFieldInit       =   0x00100000;     // Initialize the class any time before first static field access.
        const     Forwarder             =   0x00200000;     // This ExportedType is a type forwarder.

            // Flags reserved for runtime use.
        const     ReservedMask          =   0x00040800;
        const     RTSpecialName         =   0x00000800;     // Runtime should check name encoding.
        const     HasSecurity           =   0x00040000;     // Class has security associate with it.
    }
}

#[derive(Debug, Clone)]
pub struct TypeDefTableRow {
    pub flags: TypeDefFlags,
    pub name: StringsStreamOffset,
    pub namespace: StringsStreamOffset,
    pub extends: CodedToken<TypeDefOrRefToken>,
    pub field_list: FieldTableOffset,
    pub method_list: MethodTableOffset,
}

impl<'a> ReadData<TypeDefTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<TypeDefTableRow> {
        Ok(TypeDefTableRow {
            flags: TypeDefFlags::from_bits_retain(self.read()?),
            name: self.read()?,
            namespace: self.read()?,
            extends: self.read()?,
            field_list: self.read()?,
            method_list: self.read()?,
        })
    }
}
