mod streams_offsets;
mod reader;
mod tables;
pub mod coded_tokens;

pub use streams_offsets::*;
pub use reader::*;
pub use tables::*;

use bitflags::bitflags;

use crate::{
    dotnet::md::MDStreamFlags,
    error::Result,
    io::{DataReader, ReadData, Readable},
};

use self::coded_tokens::CodedTokenSizes;

use super::{
    Version,
};


#[derive(Debug, Clone)]
pub struct TablesValues {
    pub module: ModulesTable,
    pub type_ref: Vec<TypeRefTableRow>,
    pub type_def: Vec<TypeDefTableRow>,
    pub field_ptr: Vec<FieldPtrTableRow>,
    pub field: Vec<FieldTableRow>,
    pub method_ptr: Vec<MethodPtrTableRow>,
    pub method: Vec<MethodTableRow>,
    pub param_ptr: Vec<ParamPtrTableRow>,
    pub param: Vec<ParamTableRow>,
    pub interface_impl: Vec<InterfaceImplTableRow>,
    pub member_ref: Vec<MemberRefTableRow>,
    pub constant: Vec<ConstantTableRow>,
    pub custom_attribute: Vec<CustomAttributeTableRow>,
    pub field_marshal: Vec<FieldMarshalTableRow>,
    pub decl_security: Vec<DeclSecurityTableRow>,
    pub class_layout: Vec<ClassLayoutTableRow>,
    pub field_layout: Vec<FieldLayoutTableRow>,
    pub stand_alone_sig: Vec<StandAloneSigTableRow>,
    pub event_map: Vec<EventMapTableRow>,
    pub event_ptr: Vec<EventPtrTableRow>,
    pub event: Vec<EventTableRow>,
    pub property_map: Vec<PropertyMapTableRow>,
    pub property_ptr: Vec<PropertyPtrTableRow>,
    pub property: Vec<PropertyTableRow>,
    pub method_semantics: Vec<MethodSemanticsTableRow>,
    pub method_impl: Vec<MethodImplTableRow>,
    pub module_ref: Vec<ModuleRefTableRow>,
    pub type_spec: Vec<TypeSpecTableRow>,
    pub impl_map: Vec<ImplMapTableRow>,
    pub field_rva: Vec<FieldRVATableRow>,
    pub enc_log: Vec<ENCLogTableRow>,
    pub enc_map: Vec<ENCMapTableRow>,
    pub assembly: AssemblyTableRow,
    pub assembly_processor: Vec<AssemblyProcessorTableRow>,
    pub assembly_os: Vec<AssemblyOSTableRow>,
    pub assembly_ref: Vec<AssemblyRefTableRow>,
    pub assembly_ref_processor: Vec<AssemblyRefProcessorTableRow>,
    pub assembly_ref_os: Vec<AssemblyRefOSTableRow>,
    pub file: Vec<FileTableRow>,
    pub exported_type: Vec<ExportedTypeTableRow>,
    pub manifest_resource: Vec<ManifestResourceTableRow>,
    pub nested_class: Vec<NestedClassTableRow>,
    pub generic_param: Vec<GenericParamTableRow>,
    pub method_spec: Vec<MethodSpecTableRow>,
    pub generic_param_constraint: Vec<GenericParamConstraintTableRow>,
    pub method_debug_information: Vec<MethodDebugInformationTableRow>,
    pub local_scope: Vec<LocalScopeTableRow>,
    pub local_variable: Vec<LocalVariableTableRow>,
    pub local_constant: Vec<LocalConstantTableRow>,
    pub import_scope: Vec<ImportScopeTableRow>,
    pub state_machine_method: Vec<StateMachineMethodTableRow>,
    pub custom_debug_information: Vec<CustomDebugInformationTableRow>,

}

impl<'a> ReadData<TablesValues> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<TablesValues> {
        Ok(TablesValues {
            module: self.read()?,
            type_ref: self.read_rows(self.header.table_rows.type_ref)?,
            type_def: self.read_rows(self.header.table_rows.type_def)?,
            field_ptr: self.read_rows(self.header.table_rows.field_ptr)?,
            field: self.read_rows(self.header.table_rows.field)?,
            method_ptr: self.read_rows(self.header.table_rows.method_ptr)?,
            method: self.read_rows(self.header.table_rows.method)?,
            param_ptr: self.read_rows(self.header.table_rows.param_ptr)?,
            param: self.read_rows(self.header.table_rows.param)?,
            interface_impl: self.read_rows(self.header.table_rows.interface_impl)?,
            member_ref: self.read_rows(self.header.table_rows.member_ref)?,
            constant: self.read_rows(self.header.table_rows.constant)?,
            custom_attribute: self.read_rows(self.header.table_rows.custom_attribute)?,
            field_marshal: self.read_rows(self.header.table_rows.field_marshal)?,
            decl_security: self.read_rows(self.header.table_rows.decl_security)?,
            class_layout: self.read_rows(self.header.table_rows.class_layout)?,
            field_layout: self.read_rows(self.header.table_rows.field_layout)?,
            stand_alone_sig: self.read_rows(self.header.table_rows.stand_alone_sig)?,
            event_map: self.read_rows(self.header.table_rows.event_map)?,
            event_ptr: self.read_rows(self.header.table_rows.event_ptr)?,
            event: self.read_rows(self.header.table_rows.event)?,
            property_map: self.read_rows(self.header.table_rows.property_map)?,
            property_ptr: self.read_rows(self.header.table_rows.property_ptr)?,
            property: self.read_rows(self.header.table_rows.property)?,
            method_semantics: self.read_rows(self.header.table_rows.method_semantics)?,
            method_impl: self.read_rows(self.header.table_rows.method_impl)?,
            module_ref: self.read_rows(self.header.table_rows.module_ref)?,
            type_spec: self.read_rows(self.header.table_rows.type_spec)?,
            impl_map: self.read_rows(self.header.table_rows.impl_map)?,
            field_rva: self.read_rows(self.header.table_rows.field_rva)?,
            enc_log: self.read_rows(self.header.table_rows.enc_log)?,
            enc_map: self.read_rows(self.header.table_rows.enc_map)?,
            assembly: self.read()?,
            assembly_processor: self.read_rows(self.header.table_rows.assembly_processor)?,
            assembly_os: self.read_rows(self.header.table_rows.assembly_os)?,
            assembly_ref: self.read_rows(self.header.table_rows.assembly_ref)?,
            assembly_ref_processor: self.read_rows(self.header.table_rows.assembly_ref_processor)?,
            assembly_ref_os: self.read_rows(self.header.table_rows.assembly_ref_os)?,
            file: self.read_rows(self.header.table_rows.file)?,
            exported_type: self.read_rows(self.header.table_rows.exported_type)?,
            manifest_resource: self.read_rows(self.header.table_rows.manifest_resource)?,
            nested_class: self.read_rows(self.header.table_rows.nested_class)?,
            generic_param: self.read_rows(self.header.table_rows.generic_param)?,
            method_spec: self.read_rows(self.header.table_rows.method_spec)?,
            generic_param_constraint: self.read_rows(self.header.table_rows.generic_param_constraint)?,
            method_debug_information: self.read_rows(self.header.table_rows.method_debug_information)?,
            local_scope: self.read_rows(self.header.table_rows.local_scope)?,
            local_variable: self.read_rows(self.header.table_rows.local_variable)?,
            local_constant: self.read_rows(self.header.table_rows.local_constant)?,
            import_scope: self.read_rows(self.header.table_rows.import_scope)?,
            state_machine_method: self.read_rows(self.header.table_rows.state_machine_method)?,
            custom_debug_information: self.read_rows(self.header.table_rows.custom_debug_information)?,

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