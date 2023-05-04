use super::coded_tokens::CodedTokenSizes;
use super::{tables::*, TableLocation, ValueSize};
use crate::dotnet::md::calculator::{SizeCalculator, TablePositionCalculator};
use crate::dotnet::md::streams::Version;
use crate::dotnet::md::MDStreamFlags;
use crate::io::ReadData;
use crate::{error::Result, io::DataReader};
use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct TableExistsFlags: u64 {
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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct TableRowCount(pub u32);

impl TableRowCount {
    pub fn is_large(&self) -> bool {
        self.0 > u16::MAX as u32
    }

    pub fn row_size(&self) -> ValueSize {
        if self.is_large() {
            ValueSize::Big
        } else {
            ValueSize::Small
        }
    }
}

impl PartialEq<usize> for TableRowCount {
    fn eq(&self, other: &usize) -> bool {
        self.0 as usize == *other
    }
}

#[derive(Debug, Clone)]
pub struct TableRows {
    pub module: TableRowCount,
    pub type_ref: TableRowCount,
    pub type_def: TableRowCount,
    pub field_ptr: TableRowCount,
    pub field: TableRowCount,
    pub method_ptr: TableRowCount,
    pub method: TableRowCount,
    pub param_ptr: TableRowCount,
    pub param: TableRowCount,
    pub interface_impl: TableRowCount,
    pub member_ref: TableRowCount,
    pub constant: TableRowCount,
    pub custom_attribute: TableRowCount,
    pub field_marshal: TableRowCount,
    pub decl_security: TableRowCount,
    pub class_layout: TableRowCount,
    pub field_layout: TableRowCount,
    pub stand_alone_sig: TableRowCount,
    pub event_map: TableRowCount,
    pub event_ptr: TableRowCount,
    pub event: TableRowCount,
    pub property_map: TableRowCount,
    pub property_ptr: TableRowCount,
    pub property: TableRowCount,
    pub method_semantics: TableRowCount,
    pub method_impl: TableRowCount,
    pub module_ref: TableRowCount,
    pub type_spec: TableRowCount,
    pub impl_map: TableRowCount,
    pub field_rva: TableRowCount,
    pub enc_log: TableRowCount,
    pub enc_map: TableRowCount,
    pub assembly: TableRowCount,
    pub assembly_processor: TableRowCount,
    pub assembly_os: TableRowCount,
    pub assembly_ref: TableRowCount,
    pub assembly_ref_processor: TableRowCount,
    pub assembly_ref_os: TableRowCount,
    pub file: TableRowCount,
    pub exported_type: TableRowCount,
    pub manifest_resource: TableRowCount,
    pub nested_class: TableRowCount,
    pub generic_param: TableRowCount,
    pub method_spec: TableRowCount,
    pub generic_param_constraint: TableRowCount,
    pub document: TableRowCount,
    pub method_debug_information: TableRowCount,
    pub local_scope: TableRowCount,
    pub local_variable: TableRowCount,
    pub local_constant: TableRowCount,
    pub import_scope: TableRowCount,
    pub state_machine_method: TableRowCount,
    pub custom_debug_information: TableRowCount,
}

impl TableRows {
    pub fn from_reader(reader: &mut DataReader, valid_rows: TableExistsFlags) -> Result<Self> {
        fn read_if_flag(
            reader: &mut DataReader,
            valid_rows: TableExistsFlags,
            flag: TableExistsFlags,
        ) -> Result<TableRowCount> {
            if valid_rows.contains(flag) {
                reader.read().map(TableRowCount)
            } else {
                Ok(TableRowCount(0))
            }
        }

        Ok(Self {
            module: read_if_flag(reader, valid_rows, TableExistsFlags::Module)?,
            type_ref: read_if_flag(reader, valid_rows, TableExistsFlags::TypeRef)?,
            type_def: read_if_flag(reader, valid_rows, TableExistsFlags::TypeDef)?,
            field_ptr: read_if_flag(reader, valid_rows, TableExistsFlags::FieldPtr)?,
            field: read_if_flag(reader, valid_rows, TableExistsFlags::Field)?,
            method_ptr: read_if_flag(reader, valid_rows, TableExistsFlags::MethodPtr)?,
            method: read_if_flag(reader, valid_rows, TableExistsFlags::Method)?,
            param_ptr: read_if_flag(reader, valid_rows, TableExistsFlags::ParamPtr)?,
            param: read_if_flag(reader, valid_rows, TableExistsFlags::Param)?,
            interface_impl: read_if_flag(reader, valid_rows, TableExistsFlags::InterfaceImpl)?,
            member_ref: read_if_flag(reader, valid_rows, TableExistsFlags::MemberRef)?,
            constant: read_if_flag(reader, valid_rows, TableExistsFlags::Constant)?,
            custom_attribute: read_if_flag(reader, valid_rows, TableExistsFlags::CustomAttribute)?,
            field_marshal: read_if_flag(reader, valid_rows, TableExistsFlags::FieldMarshal)?,
            decl_security: read_if_flag(reader, valid_rows, TableExistsFlags::DeclSecurity)?,
            class_layout: read_if_flag(reader, valid_rows, TableExistsFlags::ClassLayout)?,
            field_layout: read_if_flag(reader, valid_rows, TableExistsFlags::FieldLayout)?,
            stand_alone_sig: read_if_flag(reader, valid_rows, TableExistsFlags::StandAloneSig)?,
            event_map: read_if_flag(reader, valid_rows, TableExistsFlags::EventMap)?,
            event_ptr: read_if_flag(reader, valid_rows, TableExistsFlags::EventPtr)?,
            event: read_if_flag(reader, valid_rows, TableExistsFlags::Event)?,
            property_map: read_if_flag(reader, valid_rows, TableExistsFlags::PropertyMap)?,
            property_ptr: read_if_flag(reader, valid_rows, TableExistsFlags::PropertyPtr)?,
            property: read_if_flag(reader, valid_rows, TableExistsFlags::Property)?,
            method_semantics: read_if_flag(reader, valid_rows, TableExistsFlags::MethodSemantics)?,
            method_impl: read_if_flag(reader, valid_rows, TableExistsFlags::MethodImpl)?,
            module_ref: read_if_flag(reader, valid_rows, TableExistsFlags::ModuleRef)?,
            type_spec: read_if_flag(reader, valid_rows, TableExistsFlags::TypeSpec)?,
            impl_map: read_if_flag(reader, valid_rows, TableExistsFlags::ImplMap)?,
            field_rva: read_if_flag(reader, valid_rows, TableExistsFlags::FieldRva)?,
            enc_log: read_if_flag(reader, valid_rows, TableExistsFlags::EncLog)?,
            enc_map: read_if_flag(reader, valid_rows, TableExistsFlags::EncMap)?,
            assembly: read_if_flag(reader, valid_rows, TableExistsFlags::Assembly)?,
            assembly_processor: read_if_flag(
                reader,
                valid_rows,
                TableExistsFlags::AssemblyProcessor,
            )?,
            assembly_os: read_if_flag(reader, valid_rows, TableExistsFlags::AssemblyOs)?,
            assembly_ref: read_if_flag(reader, valid_rows, TableExistsFlags::AssemblyRef)?,
            assembly_ref_processor: read_if_flag(
                reader,
                valid_rows,
                TableExistsFlags::AssemblyRefProcessor,
            )?,
            assembly_ref_os: read_if_flag(reader, valid_rows, TableExistsFlags::AssemblyRefOs)?,
            file: read_if_flag(reader, valid_rows, TableExistsFlags::File)?,
            exported_type: read_if_flag(reader, valid_rows, TableExistsFlags::ExportedType)?,
            manifest_resource: read_if_flag(
                reader,
                valid_rows,
                TableExistsFlags::ManifestResource,
            )?,
            nested_class: read_if_flag(reader, valid_rows, TableExistsFlags::NestedClass)?,
            generic_param: read_if_flag(reader, valid_rows, TableExistsFlags::GenericParam)?,
            method_spec: read_if_flag(reader, valid_rows, TableExistsFlags::MethodSpec)?,
            generic_param_constraint: read_if_flag(
                reader,
                valid_rows,
                TableExistsFlags::GenericParamConstraint,
            )?,
            document: read_if_flag(reader, valid_rows, TableExistsFlags::Document)?,
            method_debug_information: read_if_flag(
                reader,
                valid_rows,
                TableExistsFlags::MethodDebugInformation,
            )?,
            local_scope: read_if_flag(reader, valid_rows, TableExistsFlags::LocalScope)?,
            local_variable: read_if_flag(reader, valid_rows, TableExistsFlags::LocalVariable)?,
            local_constant: read_if_flag(reader, valid_rows, TableExistsFlags::LocalConstant)?,
            import_scope: read_if_flag(reader, valid_rows, TableExistsFlags::ImportScope)?,
            state_machine_method: read_if_flag(
                reader,
                valid_rows,
                TableExistsFlags::StateMachineMethod,
            )?,
            custom_debug_information: read_if_flag(
                reader,
                valid_rows,
                TableExistsFlags::CustomDebugInformation,
            )?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct TableLocations {
    pub module: TableLocation,
    pub type_ref: TableLocation,
    pub type_def: TableLocation,
    pub field_ptr: TableLocation,
    pub field: TableLocation,
    pub method_ptr: TableLocation,
    pub method: TableLocation,
    pub param_ptr: TableLocation,
    pub param: TableLocation,
    pub interface_impl: TableLocation,
    pub member_ref: TableLocation,
    pub constant: TableLocation,
    pub custom_attribute: TableLocation,
    pub field_marshal: TableLocation,
    pub decl_security: TableLocation,
    pub class_layout: TableLocation,
    pub field_layout: TableLocation,
    pub stand_alone_sig: TableLocation,
    pub event_map: TableLocation,
    pub event_ptr: TableLocation,
    pub event: TableLocation,
    pub property_map: TableLocation,
    pub property_ptr: TableLocation,
    pub property: TableLocation,
    pub method_semantics: TableLocation,
    pub method_impl: TableLocation,
    pub module_ref: TableLocation,
    pub type_spec: TableLocation,
    pub impl_map: TableLocation,
    pub field_rva: TableLocation,
    pub enc_log: TableLocation,
    pub enc_map: TableLocation,
    pub assembly: TableLocation,
    pub assembly_processor: TableLocation,
    pub assembly_os: TableLocation,
    pub assembly_ref: TableLocation,
    pub assembly_ref_processor: TableLocation,
    pub assembly_ref_os: TableLocation,
    pub file: TableLocation,
    pub exported_type: TableLocation,
    pub manifest_resource: TableLocation,
    pub nested_class: TableLocation,
    pub generic_param: TableLocation,
    pub method_spec: TableLocation,
    pub generic_param_constraint: TableLocation,
    pub document: TableLocation,
    pub method_debug_information: TableLocation,
    pub local_scope: TableLocation,
    pub local_variable: TableLocation,
    pub local_constant: TableLocation,
    pub import_scope: TableLocation,
    pub state_machine_method: TableLocation,
    pub custom_debug_information: TableLocation,
}

impl TableLocations {
    pub fn from_metadata(
        rows: &TableRows,
        coded_tokens_sizes: &CodedTokenSizes,
        flags: MDStreamFlags,
        version: Version,
    ) -> Self {
        let size_calculator = SizeCalculator {
            coded_tokens_sizes,
            flags,
            rows,
            version,
        };
        let mut calc = TablePositionCalculator::new(&size_calculator);
        Self {
            module: calc.calculate_location::<ModulesTableRow>(rows.module),
            type_ref: calc.calculate_location::<TypeRefTableRow>(rows.type_ref),
            type_def: calc.calculate_location::<TypeDefTableRow>(rows.type_def),
            field_ptr: calc.calculate_location::<FieldPtrTableRow>(rows.field_ptr),
            field: calc.calculate_location::<FieldTableRow>(rows.field),
            method_ptr: calc.calculate_location::<MethodPtrTableRow>(rows.method_ptr),
            method: calc.calculate_location::<MethodTableRow>(rows.method),
            param_ptr: calc.calculate_location::<ParamPtrTableRow>(rows.param_ptr),
            param: calc.calculate_location::<ParamTableRow>(rows.param),
            interface_impl: calc.calculate_location::<InterfaceImplTableRow>(rows.interface_impl),
            member_ref: calc.calculate_location::<MemberRefTableRow>(rows.member_ref),
            constant: calc.calculate_location::<ConstantTableRow>(rows.constant),
            custom_attribute: calc
                .calculate_location::<CustomAttributeTableRow>(rows.custom_attribute),
            field_marshal: calc.calculate_location::<FieldMarshalTableRow>(rows.field_marshal),
            decl_security: calc.calculate_location::<DeclSecurityTableRow>(rows.decl_security),
            class_layout: calc.calculate_location::<ClassLayoutTableRow>(rows.class_layout),
            field_layout: calc.calculate_location::<FieldLayoutTableRow>(rows.field_layout),
            stand_alone_sig: calc.calculate_location::<StandAloneSigTableRow>(rows.stand_alone_sig),
            event_map: calc.calculate_location::<EventMapTableRow>(rows.event_map),
            event_ptr: calc.calculate_location::<EventPtrTableRow>(rows.event_ptr),
            event: calc.calculate_location::<EventTableRow>(rows.event),
            property_map: calc.calculate_location::<PropertyMapTableRow>(rows.property_map),
            property_ptr: calc.calculate_location::<PropertyPtrTableRow>(rows.property_ptr),
            property: calc.calculate_location::<PropertyTableRow>(rows.property),
            method_semantics: calc
                .calculate_location::<MethodSemanticsTableRow>(rows.method_semantics),
            method_impl: calc.calculate_location::<MethodImplTableRow>(rows.method_impl),
            module_ref: calc.calculate_location::<ModuleRefTableRow>(rows.module_ref),
            type_spec: calc.calculate_location::<TypeSpecTableRow>(rows.type_spec),
            impl_map: calc.calculate_location::<ImplMapTableRow>(rows.impl_map),
            field_rva: calc.calculate_location::<FieldRVATableRow>(rows.field_rva),
            enc_log: calc.calculate_location::<ENCLogTableRow>(rows.enc_log),
            enc_map: calc.calculate_location::<ENCMapTableRow>(rows.enc_map),
            assembly: calc.calculate_location::<AssemblyTableRow>(rows.assembly),
            assembly_processor: calc
                .calculate_location::<AssemblyProcessorTableRow>(rows.assembly_processor),
            assembly_os: calc.calculate_location::<AssemblyOSTableRow>(rows.assembly_os),
            assembly_ref: calc.calculate_location::<AssemblyRefTableRow>(rows.assembly_ref),
            assembly_ref_processor: calc
                .calculate_location::<AssemblyRefProcessorTableRow>(rows.assembly_ref_processor),
            assembly_ref_os: calc.calculate_location::<AssemblyRefOSTableRow>(rows.assembly_ref_os),
            file: calc.calculate_location::<FileTableRow>(rows.file),
            exported_type: calc.calculate_location::<ExportedTypeTableRow>(rows.exported_type),
            manifest_resource: calc
                .calculate_location::<ManifestResourceTableRow>(rows.manifest_resource),
            nested_class: calc.calculate_location::<NestedClassTableRow>(rows.nested_class),
            generic_param: calc.calculate_location::<GenericParamTableRow>(rows.generic_param),
            method_spec: calc.calculate_location::<MethodSpecTableRow>(rows.method_spec),
            generic_param_constraint: calc.calculate_location::<GenericParamConstraintTableRow>(
                rows.generic_param_constraint,
            ),
            document: calc.calculate_location::<DocumentTableRow>(rows.document),
            method_debug_information: calc.calculate_location::<MethodDebugInformationTableRow>(
                rows.method_debug_information,
            ),
            local_scope: calc.calculate_location::<LocalScopeTableRow>(rows.local_scope),
            local_variable: calc.calculate_location::<LocalVariableTableRow>(rows.local_variable),
            local_constant: calc.calculate_location::<LocalConstantTableRow>(rows.local_constant),
            import_scope: calc.calculate_location::<ImportScopeTableRow>(rows.import_scope),
            state_machine_method: calc
                .calculate_location::<StateMachineMethodTableRow>(rows.state_machine_method),
            custom_debug_information: calc.calculate_location::<CustomDebugInformationTableRow>(
                rows.custom_debug_information,
            ),
        }
    }
}
