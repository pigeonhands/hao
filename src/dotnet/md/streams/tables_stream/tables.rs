use crate::{
    dotnet::md::{
        calculator::{CalculateTableSize, SizeCalculator},
        streams::Version,
    },
    error::Result,
    io::ReadData,
};

use super::{coded_tokens::*, reader::TablesStreamReader, streams_offsets::*};
use bitflags::bitflags;

#[derive(Debug, Clone)]
pub struct ModulesTableRow {
    pub generation: u16,
    pub name: StringsStreamOffset,
    pub mvid: GuidStreamOffset,
    pub enc_id: GuidStreamOffset,
    pub enc_base_id: GuidStreamOffset,
}

impl<'a> CalculateTableSize<ModulesTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        self.size_of_prim::<u16>()
            + StringsStreamOffset::streams_offset_size(self.flags).byte_size()
            + GuidStreamOffset::streams_offset_size(self.flags).byte_size()
            + GuidStreamOffset::streams_offset_size(self.flags).byte_size()
            + GuidStreamOffset::streams_offset_size(self.flags).byte_size()
    }
}

impl<'a> ReadData<ModulesTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ModulesTableRow> {
        if self.header.table_locations.module.rows != 1 {
            return Err(crate::error::HaoError::BadImageFormat(
                "Module table should have exactly one entry",
            ));
        }
        Ok(ModulesTableRow {
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

impl<'a> CalculateTableSize<TypeRefTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        ResolutionScopeToken::token_size(self.coded_tokens_sizes).byte_size()
            + StringsStreamOffset::streams_offset_size(self.flags).byte_size()
            + StringsStreamOffset::streams_offset_size(self.flags).byte_size()
    }
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
    pub struct TypeAttributes: u32 {
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

#[derive(Debug, Clone, PartialEq)]
pub struct TypeDefTableRow {
    pub flags: TypeAttributes,
    pub name: StringsStreamOffset,
    pub namespace: StringsStreamOffset,
    pub extends: CodedToken<TypeDefOrRefToken>,
    pub field_list: FieldTableOffset,
    pub method_list: MethodTableOffset,
}

impl<'a> CalculateTableSize<TypeDefTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        self.size_of_prim::<TypeAttributes>()
            + StringsStreamOffset::streams_offset_size(self.flags).byte_size()
            + StringsStreamOffset::streams_offset_size(self.flags).byte_size()
            + TypeDefOrRefToken::token_size(self.coded_tokens_sizes).byte_size()
            + FieldTableOffset::table_offset_size(self.rows).byte_size()
            + MethodTableOffset::table_offset_size(self.rows).byte_size()
    }
}

impl<'a> ReadData<TypeDefTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<TypeDefTableRow> {
        Ok(TypeDefTableRow {
            flags: TypeAttributes::from_bits_retain(self.read()?),
            name: self.read()?,
            namespace: self.read()?,
            extends: self.read()?,
            field_list: self.read()?,
            method_list: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct FieldPtrTableRow {
    pub field: FieldTableOffset,
}

impl<'a> CalculateTableSize<FieldPtrTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        FieldTableOffset::table_offset_size(self.rows).byte_size()
    }
}

impl<'a> ReadData<FieldPtrTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<FieldPtrTableRow> {
        Ok(FieldPtrTableRow {
            field: self.read()?,
        })
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct FieldFlags: u16 {
        // member access mask - Use this mask to retrieve accessibility information.
        const FieldAccessMask           =   0x0007;
        const PrivateScope              =   0x0000;     // Member not referenceable.
        const Private                   =   0x0001;     // Accessible only by the parent type.
        const FamANDAssem               =   0x0002;     // Accessible by sub-types only in this Assembly.
        const Assembly                  =   0x0003;     // Accessibly by anyone in the Assembly.
        const Family                    =   0x0004;     // Accessible only by type and sub-types.
        const FamORAssem                =   0x0005;     // Accessibly by sub-types anywhere; plus anyone in assembly.
        const Public                    =   0x0006;     // Accessibly by anyone who has visibility to this scope.
        // end member access mask

        // field contract attributes.
        const Static                    =   0x0010;     // Defined on type; else per instance.
        const InitOnly                  =   0x0020;     // Field may only be initialized; not written to after init.
        const Literal                   =   0x0040;     // Value is compile time constant.
        const NotSerialized             =   0x0080;     // Field does not have to be serialized when type is remoted.

        const SpecialName               =   0x0200;     // field is special. Name describes how.

        // interop attributes
        const PinvokeImpl               =   0x2000;     // Implementation is forwarded through pinvoke.

        // Reserved flags for runtime use only.
        const ReservedMask              =   0x9500;
        const RTSpecialName             =   0x0400;     // Runtime(metadata internal APIs) should check name encoding.
        const HasFieldMarshal           =   0x1000;     // Field has marshalling information.
        const HasDefault                =   0x8000;     // Field has default.
        const HasFieldRVA               =   0x0100;     // Field has RVA.
    }
}

#[derive(Debug, Clone)]
pub struct FieldTableRow {
    pub flags: FieldFlags,
    pub name: StringsStreamOffset,
    pub signature: BlobStreamOffset,
}

impl<'a> CalculateTableSize<FieldTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        self.size_of_prim::<FieldFlags>()
            + StringsStreamOffset::streams_offset_size(self.flags).byte_size()
            + BlobStreamOffset::streams_offset_size(self.flags).byte_size()
    }
}

impl<'a> ReadData<FieldTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<FieldTableRow> {
        Ok(FieldTableRow {
            flags: FieldFlags::from_bits_retain(self.read()?),
            name: self.read()?,
            signature: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct MethodPtrTableRow {
    pub method: MethodTableOffset,
}

impl<'a> CalculateTableSize<MethodPtrTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        MethodTableOffset::table_offset_size(self.rows).byte_size()
    }
}

impl<'a> ReadData<MethodPtrTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<MethodPtrTableRow> {
        Ok(MethodPtrTableRow {
            method: self.read()?,
        })
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MethodImplFlags: u16 {
           // code impl mask
        const CodeTypeMask      =   0x0003;   // Flags about code type.
        const IL                =   0x0000;   // Method impl is IL.
        const Native            =   0x0001;   // Method impl is native.
        const OPTIL             =   0x0002;   // Method impl is OPTIL
        const Runtime           =   0x0003;   // Method impl is provided by the runtime.
        // end code impl mask

        // managed mask
        const ManagedMask       =   0x0004;   // Flags specifying whether the code is managed or unmanaged.
        const Unmanaged         =   0x0004;   // Method impl is unmanaged; otherwise managed.
        const Managed           =   0x0000;   // Method impl is managed.
        // end managed mask

        // implementation info and interop
        const ForwardRef        =   0x0010;   // Indicates method is defined; used primarily in merge scenarios.
        const PreserveSig       =   0x0080;   // Indicates method sig is not to be mangled to do HRESULT conversion.

        const InternalCall      =   0x1000;   // Reserved for internal use.

        const Synchronized      =   0x0020;   // Method is single threaded through the body.
        const NoInlining        =   0x0008;   // Method may not be inlined.
        const MaxMethodImplVal  =   0xffff;   // Range check value
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MethodFlags: u16 {
        // member access mask - Use this mask to retrieve accessibility information.
        const MemberAccessMask          =   0x0007;
        const PrivateScope              =   0x0000;     // Member not referenceable.
        const Private                   =   0x0001;     // Accessible only by the parent type.
        const FamANDAssem               =   0x0002;     // Accessible by sub-types only in this Assembly.
        const Assem                     =   0x0003;     // Accessibly by anyone in the Assembly.
        const Family                    =   0x0004;     // Accessible only by type and sub-types.
        const FamORAssem                =   0x0005;     // Accessibly by sub-types anywhere; plus anyone in assembly.
        const Public                    =   0x0006;     // Accessibly by anyone who has visibility to this scope.
        // end member access mask

        // method contract attributes.
        const Static                    =   0x0010;     // Defined on type; else per instance.
        const Final                     =   0x0020;     // Method may not be overridden.
        const Virtual                   =   0x0040;     // Method virtual.
        const HideBySig                 =   0x0080;     // Method hides by name+sig; else just by name.

        // vtable layout mask - Use this mask to retrieve vtable attributes.
        const VtableLayoutMask          =   0x0100;
        const ReuseSlot                 =   0x0000;     // The default.
        const NewSlot                   =   0x0100;     // Method always gets a new slot in the vtable.
        // end vtable layout mask

        // method implementation attributes.
        const CheckAccessOnOverride     =   0x0200;     // Overridability is the same as the visibility.
        const Abstract                  =   0x0400;     // Method does not provide an implementation.
        const SpecialName               =   0x0800;     // Method is special. Name describes how.

        // interop attributes
        const PinvokeImpl               =   0x2000;     // Implementation is forwarded through pinvoke.
        const UnmanagedExport           =   0x0008;     // Managed method exported via thunk to unmanaged code.

        // Reserved flags for runtime use only.
        const ReservedMask              =   0xd000;
        const RTSpecialName             =   0x1000;     // Runtime should check name encoding.
        const HasSecurity               =   0x4000;     // Method has security associate with it.
        const RequireSecObject          =   0x8000;     // Method calls another method containing security code.
    }
}

#[derive(Debug, Clone)]
pub struct MethodTableRow {
    pub rva: u32,
    pub impl_flags: MethodImplFlags,
    pub flags: MethodFlags,
    pub name: StringsStreamOffset,
    pub signature: BlobStreamOffset,
    pub param_list: ParamTableOffset,
}

impl<'a> CalculateTableSize<MethodTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        self.size_of_prim::<u32>()
            + self.size_of_prim::<MethodImplFlags>()
            + self.size_of_prim::<MethodFlags>()
            + StringsStreamOffset::streams_offset_size(self.flags).byte_size()
            + BlobStreamOffset::streams_offset_size(self.flags).byte_size()
            + ParamTableOffset::table_offset_size(self.rows).byte_size()
    }
}

impl<'a> ReadData<MethodTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<MethodTableRow> {
        Ok(MethodTableRow {
            rva: self.read()?,
            impl_flags: MethodImplFlags::from_bits_retain(self.read()?),
            flags: MethodFlags::from_bits_retain(self.read()?),
            name: self.read()?,
            signature: self.read()?,
            param_list: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ParamPtrTableRow {
    pub param: ParamPtrTableOffset,
}

impl<'a> CalculateTableSize<ParamPtrTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        ParamPtrTableOffset::table_offset_size(self.rows).byte_size()
    }
}

impl<'a> ReadData<ParamPtrTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ParamPtrTableRow> {
        Ok(ParamPtrTableRow {
            param: self.read()?,
        })
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ParamFlags: u16 {
        const In                        =   0x0001;     // Param is [In]
        const Out                       =   0x0002;     // Param is [out]
        const Optional                  =   0x0010;     // Param is optional

        // Reserved flags for Runtime use only.
        const ReservedMask              =   0xf000;
        const HasDefault                =   0x1000;     // Param has default value.
        const HasFieldMarshal           =   0x2000;     // Param has FieldMarshal.

        const Unused                    =   0xcfe0;
    }
}

#[derive(Debug, Clone)]
pub struct ParamTableRow {
    pub flags: ParamFlags,
    pub sequence: u16,
    pub name: StringsStreamOffset,
}

impl<'a> CalculateTableSize<ParamTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        self.size_of_prim::<ParamFlags>()
            + self.size_of_prim::<u16>()
            + StringsStreamOffset::streams_offset_size(self.flags).byte_size()
    }
}

impl<'a> ReadData<ParamTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ParamTableRow> {
        Ok(ParamTableRow {
            flags: ParamFlags::from_bits_retain(self.read()?),
            sequence: self.read()?,
            name: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct InterfaceImplTableRow {
    pub class: TypeDefTableOffset,
    pub interface: CodedToken<TypeDefOrRefToken>,
}

impl<'a> CalculateTableSize<InterfaceImplTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        TypeDefTableOffset::table_offset_size(self.rows).byte_size()
            + TypeDefOrRefToken::token_size(self.coded_tokens_sizes).byte_size()
    }
}

impl<'a> ReadData<InterfaceImplTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<InterfaceImplTableRow> {
        Ok(InterfaceImplTableRow {
            class: self.read()?,
            interface: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct MemberRefTableRow {
    pub class: CodedToken<MemberRefParentToken>,
    pub name: StringsStreamOffset,
    pub signature: BlobStreamOffset,
}

impl<'a> CalculateTableSize<MemberRefTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        MemberRefParentToken::token_size(self.coded_tokens_sizes).byte_size()
            + StringsStreamOffset::streams_offset_size(self.flags).byte_size()
            + BlobStreamOffset::streams_offset_size(self.flags).byte_size()
    }
}

impl<'a> ReadData<MemberRefTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<MemberRefTableRow> {
        Ok(MemberRefTableRow {
            class: self.read()?,
            name: self.read()?,
            signature: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ConstantTableRow {
    pub ty: u8,
    pub padding: u8,
    pub parent: CodedToken<HasConstantToken>,
    pub value: BlobStreamOffset,
}

impl<'a> CalculateTableSize<ConstantTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        self.size_of_prim::<u8>()
            + self.size_of_prim::<u8>()
            + HasConstantToken::token_size(self.coded_tokens_sizes).byte_size()
            + BlobStreamOffset::streams_offset_size(self.flags).byte_size()
    }
}

impl<'a> ReadData<ConstantTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ConstantTableRow> {
        Ok(ConstantTableRow {
            ty: self.read()?,
            padding: self.read()?,
            parent: self.read()?,
            value: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct CustomAttributeTableRow {
    pub parent: CodedToken<HasCustomAttributeToken>,
    pub ty: CodedToken<CustomAttributeTypeToken>,
    pub value: BlobStreamOffset,
}

impl<'a> CalculateTableSize<CustomAttributeTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        HasCustomAttributeToken::token_size(self.coded_tokens_sizes).byte_size()
            + CustomAttributeTypeToken::token_size(self.coded_tokens_sizes).byte_size()
            + BlobStreamOffset::streams_offset_size(self.flags).byte_size()
    }
}

impl<'a> ReadData<CustomAttributeTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<CustomAttributeTableRow> {
        Ok(CustomAttributeTableRow {
            parent: self.read()?,
            ty: self.read()?,
            value: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct FieldMarshalTableRow {
    pub parent: CodedToken<HasFieldMarshalToken>,
    pub native_type: BlobStreamOffset,
}

impl<'a> CalculateTableSize<FieldMarshalTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        HasFieldMarshalToken::token_size(self.coded_tokens_sizes).byte_size()
            + BlobStreamOffset::streams_offset_size(self.flags).byte_size()
    }
}

impl<'a> ReadData<FieldMarshalTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<FieldMarshalTableRow> {
        Ok(FieldMarshalTableRow {
            parent: self.read()?,
            native_type: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct DeclSecurityTableRow {
    pub action: u16,
    pub parent: CodedToken<HasDeclSecurityToken>,
    pub permission_set: BlobStreamOffset,
}

impl<'a> CalculateTableSize<DeclSecurityTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        self.size_of_prim::<u16>()
            + HasDeclSecurityToken::token_size(self.coded_tokens_sizes).byte_size()
            + BlobStreamOffset::streams_offset_size(self.flags).byte_size()
    }
}

impl<'a> ReadData<DeclSecurityTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<DeclSecurityTableRow> {
        Ok(DeclSecurityTableRow {
            action: self.read()?,
            parent: self.read()?,
            permission_set: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ClassLayoutTableRow {
    pub packing_size: u16,
    pub class_size: u32,
    pub parent: TypeDefTableOffset,
}

impl<'a> CalculateTableSize<ClassLayoutTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        self.size_of_prim::<u16>()
            + self.size_of_prim::<u32>()
            + TypeDefTableOffset::table_offset_size(self.rows).byte_size()
    }
}

impl<'a> ReadData<ClassLayoutTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ClassLayoutTableRow> {
        Ok(ClassLayoutTableRow {
            packing_size: self.read()?,
            class_size: self.read()?,
            parent: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct FieldLayoutTableRow {
    pub offset: u32,
    pub field: FieldTableOffset,
}

impl<'a> CalculateTableSize<FieldLayoutTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        self.size_of_prim::<u32>() + FieldTableOffset::table_offset_size(self.rows).byte_size()
    }
}

impl<'a> ReadData<FieldLayoutTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<FieldLayoutTableRow> {
        Ok(FieldLayoutTableRow {
            offset: self.read()?,
            field: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct StandAloneSigTableRow {
    pub signature: BlobStreamOffset,
}

impl<'a> CalculateTableSize<StandAloneSigTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        BlobStreamOffset::streams_offset_size(self.flags).byte_size()
    }
}

impl<'a> ReadData<StandAloneSigTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<StandAloneSigTableRow> {
        Ok(StandAloneSigTableRow {
            signature: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct EventMapTableRow {
    pub parent: TypeDefTableOffset,
    pub event_list: EventTableOffset,
}

impl<'a> CalculateTableSize<EventMapTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        TypeDefTableOffset::table_offset_size(self.rows).byte_size()
            + EventTableOffset::table_offset_size(self.rows).byte_size()
    }
}

impl<'a> ReadData<EventMapTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<EventMapTableRow> {
        Ok(EventMapTableRow {
            parent: self.read()?,
            event_list: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct EventPtrTableRow {
    pub event: EventTableOffset,
}

impl<'a> CalculateTableSize<EventPtrTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        EventTableOffset::table_offset_size(self.rows).byte_size()
    }
}

impl<'a> ReadData<EventPtrTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<EventPtrTableRow> {
        Ok(EventPtrTableRow {
            event: self.read()?,
        })
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct EventFlags: u16 {
        const SpecialName           =   0x0200;     // const ent is special. Name describes how.

        // Reserved flags for Runtime use only.
        const ReservedMask          =   0x0400;
        const RTSpecialName         =   0x0400;     // Runtime(metadata internal APIs) should check name encoding.
    }
}

#[derive(Debug, Clone)]
pub struct EventTableRow {
    pub event_flags: EventFlags,
    pub name: StringsStreamOffset,
    pub event_type: CodedToken<TypeDefOrRefToken>,
}

impl<'a> CalculateTableSize<EventTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        self.size_of_prim::<EventFlags>()
            + StringsStreamOffset::streams_offset_size(self.flags).byte_size()
            + TypeDefOrRefToken::token_size(self.coded_tokens_sizes).byte_size()
    }
}

impl<'a> ReadData<EventTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<EventTableRow> {
        Ok(EventTableRow {
            event_flags: EventFlags::from_bits_retain(self.read()?),
            name: self.read()?,
            event_type: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct PropertyMapTableRow {
    pub parent: TypeDefTableOffset,
    pub property_list: PropertyTableOffset,
}

impl<'a> CalculateTableSize<PropertyMapTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        TypeDefTableOffset::table_offset_size(self.rows).byte_size()
            + PropertyTableOffset::table_offset_size(self.rows).byte_size()
    }
}

impl<'a> ReadData<PropertyMapTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<PropertyMapTableRow> {
        Ok(PropertyMapTableRow {
            parent: self.read()?,
            property_list: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct PropertyPtrTableRow {
    pub property: PropertyTableOffset,
}

impl<'a> CalculateTableSize<PropertyPtrTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        PropertyTableOffset::table_offset_size(self.rows).byte_size()
    }
}

impl<'a> ReadData<PropertyPtrTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<PropertyPtrTableRow> {
        Ok(PropertyPtrTableRow {
            property: self.read()?,
        })
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct PropertyFlags: u16 {
        const SpecialName           =   0x0200;     // const operty is special. Name describes how.

        // Reserved flags for Runtime use only.
        const ReservedMask          =   0xf400;
        const RTSpecialName         =   0x0400;     // Runtime(metadata internal APIs) should check name encoding.
        const HasDefault            =   0x1000;     // const operty has default

        const Unused                =   0xe9ff;
    }
}

#[derive(Debug, Clone)]
pub struct PropertyTableRow {
    pub prop_flags: PropertyFlags,
    pub name: StringsStreamOffset,
    // indexes the signature in the Blob heap of the Property
    pub ty: BlobStreamOffset,
}

impl<'a> CalculateTableSize<PropertyTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        self.size_of_prim::<PropertyFlags>()
            + StringsStreamOffset::streams_offset_size(self.flags).byte_size()
            + BlobStreamOffset::streams_offset_size(self.flags).byte_size()
    }
}

impl<'a> ReadData<PropertyTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<PropertyTableRow> {
        Ok(PropertyTableRow {
            prop_flags: PropertyFlags::from_bits_retain(self.read()?),
            name: self.read()?,
            ty: self.read()?,
        })
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MethodSemanticsFlags: u16 {
        const Setter    =   0x0001;     // Setter for property
        const Getter    =   0x0002;     // Getter for property
        const Other     =   0x0004;     // other method for property or event
        const AddOn     =   0x0008;     // AddOn method for event
        const RemoveOn  =   0x0010;     // RemoveOn method for event
        const Fire      =   0x0020;     // Fire method for event
    }
}

#[derive(Debug, Clone)]
pub struct MethodSemanticsTableRow {
    pub semantic: MethodSemanticsFlags,
    pub method: MethodTableOffset,
    pub association: CodedToken<HasSemanticToken>,
}

impl<'a> CalculateTableSize<MethodSemanticsTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        self.size_of_prim::<MethodSemanticsFlags>()
            + MethodTableOffset::table_offset_size(self.rows).byte_size()
            + HasSemanticToken::token_size(self.coded_tokens_sizes).byte_size()
    }
}

impl<'a> ReadData<MethodSemanticsTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<MethodSemanticsTableRow> {
        Ok(MethodSemanticsTableRow {
            semantic: MethodSemanticsFlags::from_bits_retain(self.read()?),
            method: self.read()?,
            association: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct MethodImplTableRow {
    pub class: TypeDefTableOffset,
    pub method_body: CodedToken<MethodDefOrRefToken>,
    pub method_declaration: CodedToken<MethodDefOrRefToken>,
}

impl<'a> CalculateTableSize<MethodImplTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        TypeDefTableOffset::table_offset_size(self.rows).byte_size()
            + MethodDefOrRefToken::token_size(self.coded_tokens_sizes).byte_size()
            + MethodDefOrRefToken::token_size(self.coded_tokens_sizes).byte_size()
    }
}

impl<'a> ReadData<MethodImplTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<MethodImplTableRow> {
        Ok(MethodImplTableRow {
            class: self.read()?,
            method_body: self.read()?,
            method_declaration: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ModuleRefTableRow {
    pub name: StringsStreamOffset,
}

impl<'a> CalculateTableSize<ModuleRefTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        StringsStreamOffset::streams_offset_size(self.flags).byte_size()
    }
}

impl<'a> ReadData<ModuleRefTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ModuleRefTableRow> {
        Ok(ModuleRefTableRow { name: self.read()? })
    }
}

#[derive(Debug, Clone)]
pub struct TypeSpecTableRow {
    pub signature: BlobStreamOffsetTypeSpec,
}

impl<'a> CalculateTableSize<TypeSpecTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        BlobStreamOffsetTypeSpec::streams_offset_size(self.flags).byte_size()
    }
}

impl<'a> ReadData<TypeSpecTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<TypeSpecTableRow> {
        Ok(TypeSpecTableRow {
            signature: self.read()?,
        })
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct PInvokeMapFlags: u16 {
        const NoMangle          = 0x0001;   // Pinvoke is to use the member name as specified.

        // Use this mask to retrieve the CharSet information.
        const CharSetMask       = 0x0006;
        const CharSetNotSpec    = 0x0000;
        const CharSetAnsi       = 0x0002;
        const CharSetUnicode    = 0x0004;
        const CharSetAuto       = 0x0006;


        const BestFitUseAssem   = 0x0000;
        const BestFitEnabled    = 0x0010;
        const BestFitDisabled   = 0x0020;
        const BestFitMask       = 0x0030;

        const ThrowOnUnmappableCharUseAssem   = 0x0000;
        const ThrowOnUnmappableCharEnabled    = 0x1000;
        const ThrowOnUnmappableCharDisabled   = 0x2000;
        const ThrowOnUnmappableCharMask       = 0x3000;

        const SupportsLastError = 0x0040;   // Information about target function. Not relevant for fields.

        // None of the calling convention flags is relevant for fields.
        const CallConvMask      = 0x0700;
        const CallConvWinapi    = 0x0100;   // Pinvoke will use native callconv appropriate to target windows platform.
        const CallConvCdecl     = 0x0200;
        const CallConvStdcall   = 0x0300;
        const CallConvThiscall  = 0x0400;   // In M9; pinvoke will raise exception.
        const CallConvFastcall  = 0x0500;

        const MaxValue          = 0xFFFF;
    }
}

#[derive(Debug, Clone)]
pub struct ImplMapTableRow {
    pub mapping_flags: PInvokeMapFlags,
    pub member_forwarded: CodedToken<MemberForwardedToken>,
    pub import_name: StringsStreamOffset,
    pub import_scope: ModuleRefTableOffset,
}

impl<'a> CalculateTableSize<ImplMapTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        self.size_of_prim::<PInvokeMapFlags>()
            + MemberForwardedToken::token_size(self.coded_tokens_sizes).byte_size()
            + StringsStreamOffset::streams_offset_size(self.flags).byte_size()
            + ModuleRefTableOffset::table_offset_size(self.rows).byte_size()
    }
}

impl<'a> ReadData<ImplMapTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ImplMapTableRow> {
        Ok(ImplMapTableRow {
            mapping_flags: PInvokeMapFlags::from_bits_retain(self.read()?),
            member_forwarded: self.read()?,
            import_name: self.read()?,
            import_scope: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct FieldRVATableRow {
    pub rva: u32,
    pub field: FieldTableOffset,
}

impl<'a> CalculateTableSize<FieldRVATableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        self.size_of_prim::<u32>() + FieldTableOffset::table_offset_size(self.rows).byte_size()
    }
}

impl<'a> ReadData<FieldRVATableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<FieldRVATableRow> {
        Ok(FieldRVATableRow {
            rva: self.read()?,
            field: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ENCLogTableRow {
    pub token: u32,
    pub func_code: u32,
}

impl<'a> CalculateTableSize<ENCLogTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        self.size_of_prim::<u32>() + self.size_of_prim::<u32>()
    }
}

impl<'a> ReadData<ENCLogTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ENCLogTableRow> {
        Ok(ENCLogTableRow {
            token: self.read()?,
            func_code: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ENCMapTableRow {
    pub token: u32,
}

impl<'a> CalculateTableSize<ENCMapTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        self.size_of_prim::<u32>()
    }
}

impl<'a> ReadData<ENCMapTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ENCMapTableRow> {
        Ok(ENCMapTableRow {
            token: self.read()?,
        })
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct AssemblyFlags: u32 {
        const PublicKey             =   0x0001;     // The assembly ref holds the full (unhashed) public key.

        const PA_None               =   0x0000;     // Processor Architecture unspecified
        const PA_MSIL               =   0x0010;     // Processor Architecture: neutral (PE32)
        const PA_x86                =   0x0020;     // Processor Architecture: x86 (PE32)
        const PA_IA64               =   0x0030;     // Processor Architecture: Itanium (PE32+)
        const PA_AMD64              =   0x0040;     // Processor Architecture: AMD X64 (PE32+)
        const PA_Specified          =   0x0080;     // Propagate PA flags to AssemblyRef record
        const PA_Mask               =   0x0070;     // Bits describing the processor architecture
        const PA_FullMask           =   0x00F0;     // Bits describing the PA incl. Specified
        const PA_Shift              =   0x0004;     // NOT A FLAG; shift count in PA flags <--> index conversion

        const EnableJITcompileTracking  =   0x8000; // From "DebuggableAttribute".
        const DisableJITcompileOptimizer=   0x4000; // From "DebuggableAttribute".

        const Retargetable          =   0x0100;     // The assembly can be retargeted (at runtime) to an
                                                // assembly from a different publisher.
    }
}

#[derive(Debug, Clone)]
pub struct AssemblyTableRow {
    pub hash_alg_id: u32,
    pub major_version: u16,
    pub minor_version: u16,
    pub build_number: u16,
    pub revision_number: u16,
    pub flags: AssemblyFlags,
    pub public_key: BlobStreamOffset,
    pub name: StringsStreamOffset,
    pub locale: StringsStreamOffset,
}

impl<'a> CalculateTableSize<AssemblyTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        self.size_of_prim::<u32>()
            + self.size_of_prim::<u16>()
            + self.size_of_prim::<u16>()
            + self.size_of_prim::<u16>()
            + self.size_of_prim::<u16>()
            + self.size_of_prim::<AssemblyFlags>()
            + BlobStreamOffset::streams_offset_size(self.flags).byte_size()
            + StringsStreamOffset::streams_offset_size(self.flags).byte_size()
            + StringsStreamOffset::streams_offset_size(self.flags).byte_size()
    }
}

impl<'a> ReadData<AssemblyTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<AssemblyTableRow> {
        if self.header.table_locations.assembly.rows != 1 {
            return Err(crate::error::HaoError::BadImageFormat(
                "Assembly table should have exactly one entry",
            ));
        }

        Ok(AssemblyTableRow {
            hash_alg_id: self.read()?,
            major_version: self.read()?,
            minor_version: self.read()?,
            build_number: self.read()?,
            revision_number: self.read()?,
            flags: AssemblyFlags::from_bits_retain(self.read()?),
            public_key: self.read()?,
            name: self.read()?,
            locale: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AssemblyProcessorTableRow {
    pub processor: u32,
}

impl<'a> CalculateTableSize<AssemblyProcessorTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        self.size_of_prim::<u32>()
    }
}

impl<'a> ReadData<AssemblyProcessorTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<AssemblyProcessorTableRow> {
        Ok(AssemblyProcessorTableRow {
            processor: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AssemblyOSTableRow {
    pub os_platform_id: u32,
    pub os_major_version: u32,
    pub os_minor_version: u32,
}

impl<'a> CalculateTableSize<AssemblyOSTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        self.size_of_prim::<u32>() + self.size_of_prim::<u32>() + self.size_of_prim::<u32>()
    }
}

impl<'a> ReadData<AssemblyOSTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<AssemblyOSTableRow> {
        Ok(AssemblyOSTableRow {
            os_platform_id: self.read()?,
            os_major_version: self.read()?,
            os_minor_version: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AssemblyRefTableRow {
    pub major_version: u16,
    pub minor_version: u16,
    pub build_number: u16,
    pub revision_number: u16,
    pub flags: AssemblyFlags,
    pub public_key_or_token: BlobStreamOffset,
    pub name: StringsStreamOffset,
    pub locale: StringsStreamOffset,
    pub hash_value: BlobStreamOffset,
}

impl<'a> CalculateTableSize<AssemblyRefTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        self.size_of_prim::<u16>()
            + self.size_of_prim::<u16>()
            + self.size_of_prim::<u16>()
            + self.size_of_prim::<u16>()
            + self.size_of_prim::<AssemblyFlags>()
            + BlobStreamOffset::streams_offset_size(self.flags).byte_size()
            + StringsStreamOffset::streams_offset_size(self.flags).byte_size()
            + StringsStreamOffset::streams_offset_size(self.flags).byte_size()
            + BlobStreamOffset::streams_offset_size(self.flags).byte_size()
    }
}

impl<'a> ReadData<AssemblyRefTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<AssemblyRefTableRow> {
        Ok(AssemblyRefTableRow {
            major_version: self.read()?,
            minor_version: self.read()?,
            build_number: self.read()?,
            revision_number: self.read()?,
            flags: AssemblyFlags::from_bits_retain(self.read()?),
            public_key_or_token: self.read()?,
            name: self.read()?,
            locale: self.read()?,
            hash_value: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AssemblyRefProcessorTableRow {
    pub processor: u32,
    pub assembly_ref: AssemblyRefTableOffset,
}

impl<'a> CalculateTableSize<AssemblyRefProcessorTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        self.size_of_prim::<u32>()
            + AssemblyRefTableOffset::table_offset_size(self.rows).byte_size()
    }
}

impl<'a> ReadData<AssemblyRefProcessorTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<AssemblyRefProcessorTableRow> {
        Ok(AssemblyRefProcessorTableRow {
            processor: self.read()?,
            assembly_ref: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AssemblyRefOSTableRow {
    pub os_platform_id: u32,
    pub os_major_version: u32,
    pub os_minor_version: u32,
    pub assembly_ref: AssemblyRefTableOffset,
}

impl<'a> CalculateTableSize<AssemblyRefOSTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        self.size_of_prim::<u32>()
            + self.size_of_prim::<u32>()
            + self.size_of_prim::<u32>()
            + AssemblyRefTableOffset::table_offset_size(self.rows).byte_size()
    }
}

impl<'a> ReadData<AssemblyRefOSTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<AssemblyRefOSTableRow> {
        Ok(AssemblyRefOSTableRow {
            os_platform_id: self.read()?,
            os_major_version: self.read()?,
            os_minor_version: self.read()?,
            assembly_ref: self.read()?,
        })
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct FileFlags: u16 {
        const ContainsMetaData      =   0x0000;     // This is not a resource file
        const ContainsNoMetaData    =   0x0001;     // This is a resource file or other non-metadata-containing file                                      // assembly from a different publisher.
    }
}

#[derive(Debug, Clone)]
pub struct FileTableRow {
    pub flags: FileFlags,
    pub name: StringsStreamOffset,
    pub hash_value: BlobStreamOffset,
}

impl<'a> CalculateTableSize<FileTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        self.size_of_prim::<FileFlags>()
            + StringsStreamOffset::streams_offset_size(self.flags).byte_size()
            + BlobStreamOffset::streams_offset_size(self.flags).byte_size()
    }
}

impl<'a> ReadData<FileTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<FileTableRow> {
        Ok(FileTableRow {
            flags: FileFlags::from_bits_retain(self.read()?),
            name: self.read()?,
            hash_value: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ExportedTypeTableRow {
    pub flags: TypeAttributes,
    pub type_def_id: TypeDefTableOffset,
    pub type_name: StringsStreamOffset,
    pub type_namespace: StringsStreamOffset,
    pub implementation: CodedToken<ImplementationToken>,
}

impl<'a> CalculateTableSize<ExportedTypeTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        self.size_of_prim::<TypeAttributes>()
            + TypeDefTableOffset::table_offset_size(self.rows).byte_size()
            + StringsStreamOffset::streams_offset_size(self.flags).byte_size()
            + StringsStreamOffset::streams_offset_size(self.flags).byte_size()
            + ImplementationToken::token_size(self.coded_tokens_sizes).byte_size()
    }
}

impl<'a> ReadData<ExportedTypeTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ExportedTypeTableRow> {
        Ok(ExportedTypeTableRow {
            flags: TypeAttributes::from_bits_retain(self.read()?),
            type_def_id: self.read()?,
            type_name: self.read()?,
            type_namespace: self.read()?,
            implementation: self.read()?,
        })
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ManifestResourceFlags: u32 {
        const VisibilityMask        =   0x0007;
        const Public                =   0x0001;     // The Resource is exported from the Assembly.
        const Private               =   0x0002;     // The Resource is private to the Assembly.
    }
}

#[derive(Debug, Clone)]
pub struct ManifestResourceTableRow {
    pub offset: u32,
    pub flags: ManifestResourceFlags,
    pub name: StringsStreamOffset,
    pub implementation: CodedToken<ImplementationToken>,
}

impl<'a> CalculateTableSize<ManifestResourceTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        self.size_of_prim::<u32>()
            + self.size_of_prim::<ManifestResourceFlags>()
            + StringsStreamOffset::streams_offset_size(self.flags).byte_size()
            + ImplementationToken::token_size(self.coded_tokens_sizes).byte_size()
    }
}

impl<'a> ReadData<ManifestResourceTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ManifestResourceTableRow> {
        Ok(ManifestResourceTableRow {
            offset: self.read()?,
            flags: ManifestResourceFlags::from_bits_truncate(self.read()?),
            name: self.read()?,
            implementation: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct NestedClassTableRow {
    pub nested_class: TypeDefTableOffset,
    pub enclosing_class: TypeDefTableOffset,
}

impl<'a> CalculateTableSize<NestedClassTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        TypeDefTableOffset::table_offset_size(self.rows).byte_size()
            + TypeDefTableOffset::table_offset_size(self.rows).byte_size()
    }
}

impl<'a> ReadData<NestedClassTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<NestedClassTableRow> {
        Ok(NestedClassTableRow {
            nested_class: self.read()?,
            enclosing_class: self.read()?,
        })
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct GenericParamFlags: u16 {
        // Variance of type parameters; only applicable to generic parameters
        // for generic interfaces and delegates
        const VarianceMask          =   0x0003;
        const NonVariant            =   0x0000;
        const Covariant             =   0x0001;
        const Contravariant         =   0x0002;

        // Special constraints; applicable to any type parameters
        const SpecialConstraintMask =  0x001C;
        const NoSpecialConstraint   =   0x0000;
        const ReferenceTypeConstraint = 0x0004;      // type argument must be a reference type
        const NotNullableValueTypeConstraint   =   0x0008; // type argument must be a value type but not Nullable
        const DefaultConstructorConstraint = 0x0010; // type argument must have a public default constructor
    }
}

#[derive(Debug, Clone)]
pub struct GenericParamTableRow {
    pub number: u16,
    pub flags: GenericParamFlags,
    pub owner: CodedToken<TypeOrMethodDefToken>,
    pub name: StringsStreamOffset,
    // Kind is only in assemblies of version 1.1
    pub kind: Option<CodedToken<TypeDefOrRefToken>>,
}

impl<'a> CalculateTableSize<GenericParamTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        self.size_of_prim::<u16>()
            + self.size_of_prim::<GenericParamFlags>()
            + TypeOrMethodDefToken::token_size(self.coded_tokens_sizes).byte_size()
            + StringsStreamOffset::streams_offset_size(self.flags).byte_size()
            + if self.version == Version(1, 1) {
                TypeDefOrRefToken::token_size(self.coded_tokens_sizes).byte_size()
            } else {
                0
            }
    }
}

impl<'a> ReadData<GenericParamTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<GenericParamTableRow> {
        Ok(GenericParamTableRow {
            number: self.read()?,
            flags: GenericParamFlags::from_bits_retain(self.read()?),
            owner: self.read()?,
            name: self.read()?,
            kind: (self.header.version == Version(1, 1))
                .then(|| self.read())
                .transpose()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct MethodSpecTableRow {
    pub method: CodedToken<MethodDefOrRefToken>,
    pub instantiation: BlobStreamOffset,
}

impl<'a> CalculateTableSize<MethodSpecTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        MethodDefOrRefToken::token_size(self.coded_tokens_sizes).byte_size()
            + BlobStreamOffset::streams_offset_size(self.flags).byte_size()
    }
}

impl<'a> ReadData<MethodSpecTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<MethodSpecTableRow> {
        Ok(MethodSpecTableRow {
            method: self.read()?,
            instantiation: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct GenericParamConstraintTableRow {
    pub owner: GenericParamTableOffset,
    pub constraint: CodedToken<TypeDefOrRefToken>,
}

impl<'a> CalculateTableSize<GenericParamConstraintTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        GenericParamTableOffset::table_offset_size(self.rows).byte_size()
            + TypeDefOrRefToken::token_size(self.coded_tokens_sizes).byte_size()
    }
}

impl<'a> ReadData<GenericParamConstraintTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<GenericParamConstraintTableRow> {
        Ok(GenericParamConstraintTableRow {
            owner: self.read()?,
            constraint: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct DocumentTableRow {
    pub name: BlobStreamOffset,
    pub hash_algorithm: GuidStreamOffset,
    pub hash: BlobStreamOffset,
    pub language: GuidStreamOffset,
}

impl<'a> CalculateTableSize<DocumentTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        BlobStreamOffset::streams_offset_size(self.flags).byte_size()
            + GuidStreamOffset::streams_offset_size(self.flags).byte_size()
            + BlobStreamOffset::streams_offset_size(self.flags).byte_size()
            + GuidStreamOffset::streams_offset_size(self.flags).byte_size()
    }
}

impl<'a> ReadData<DocumentTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<DocumentTableRow> {
        Ok(DocumentTableRow {
            name: self.read()?,
            hash_algorithm: self.read()?,
            hash: self.read()?,
            language: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct MethodDebugInformationTableRow {
    pub document: DocumentTableRowOffset,
    pub sequence_points: BlobStreamOffset,
}

impl<'a> CalculateTableSize<MethodDebugInformationTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        DocumentTableRowOffset::table_offset_size(self.rows).byte_size()
            + BlobStreamOffset::streams_offset_size(self.flags).byte_size()
    }
}

impl<'a> ReadData<MethodDebugInformationTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<MethodDebugInformationTableRow> {
        Ok(MethodDebugInformationTableRow {
            document: self.read()?,
            sequence_points: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct LocalScopeTableRow {
    pub method: MethodTableOffset,
    pub import_scope: ImportScopeTableOffset,
    pub variable_list: LocalVariableTableOffset,
    pub constant_list: LocalConstantTableOffset,
    pub start_offset: u32,
    pub length: u32,
}

impl<'a> CalculateTableSize<LocalScopeTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        MethodTableOffset::table_offset_size(self.rows).byte_size()
            + ImportScopeTableOffset::table_offset_size(self.rows).byte_size()
            + LocalConstantTableOffset::table_offset_size(self.rows).byte_size()
            + self.size_of_prim::<u32>()
            + self.size_of_prim::<u32>()
    }
}

impl<'a> ReadData<LocalScopeTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<LocalScopeTableRow> {
        Ok(LocalScopeTableRow {
            method: self.read()?,
            import_scope: self.read()?,
            variable_list: self.read()?,
            constant_list: self.read()?,
            start_offset: self.read()?,
            length: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct LocalVariableTableRow {
    pub attributes: (),
    pub index: (),
    pub name: (),
}

impl<'a> CalculateTableSize<LocalVariableTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        0
    }
}

impl<'a> ReadData<LocalVariableTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<LocalVariableTableRow> {
        Ok(LocalVariableTableRow {
            attributes: self.read()?,
            index: self.read()?,
            name: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct LocalConstantTableRow {
    pub name: (),
    pub signature: (),
}

impl<'a> CalculateTableSize<LocalConstantTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        0
    }
}

impl<'a> ReadData<LocalConstantTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<LocalConstantTableRow> {
        Ok(LocalConstantTableRow {
            name: self.read()?,
            signature: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ImportScopeTableRow {
    pub parent: (),
    pub imports: (),
}

impl<'a> CalculateTableSize<ImportScopeTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        0
    }
}

impl<'a> ReadData<ImportScopeTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ImportScopeTableRow> {
        Ok(ImportScopeTableRow {
            parent: self.read()?,
            imports: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct StateMachineMethodTableRow {
    pub move_next_method: (),
    pub kickoff_method: (),
}

impl<'a> CalculateTableSize<StateMachineMethodTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        0
    }
}

impl<'a> ReadData<StateMachineMethodTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<StateMachineMethodTableRow> {
        Ok(StateMachineMethodTableRow {
            move_next_method: self.read()?,
            kickoff_method: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct CustomDebugInformationTableRow {
    pub parent: (),
    pub kind: (),
    pub value: (),
}

impl<'a> CalculateTableSize<CustomDebugInformationTableRow> for SizeCalculator<'a> {
    fn calculate_table_size_bytes(&self) -> usize {
        0
    }
}

impl<'a> ReadData<CustomDebugInformationTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<CustomDebugInformationTableRow> {
        Ok(CustomDebugInformationTableRow {
            parent: self.read()?,
            kind: self.read()?,
            value: self.read()?,
        })
    }
}
