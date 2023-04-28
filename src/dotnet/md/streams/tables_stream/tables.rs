use crate::{dotnet::md::streams::Version, error::Result, io::ReadData};

use super::{coded_tokens::*, reader::TablesStreamReader, streams_offsets::*};

use bitflags::bitflags;

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

#[derive(Debug, Clone)]
pub struct FieldPtrTableRow {
    pub field: FieldTableOffset,
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
    pub class: (),
    pub name: (),
    pub signature: (),
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
    pub ty: (),
    pub padding: (),
    pub parent: (),
    pub value: (),
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
    pub parent: (),
    pub ty: (),
    pub value: (),
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
    pub parent: (),
    pub native_type: (),
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
    pub action: (),
    pub parent: (),
    pub permission_set: (),
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
    pub packing_size: (),
    pub class_size: (),
    pub parent: (),
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
    pub off_set: (),
    pub field: (),
}

impl<'a> ReadData<FieldLayoutTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<FieldLayoutTableRow> {
        Ok(FieldLayoutTableRow {
            off_set: self.read()?,
            field: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct StandAloneSigTableRow {
    pub signature: (),
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
    pub parent: (),
    pub event_list: (),
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
    pub event: (),
}

impl<'a> ReadData<EventPtrTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<EventPtrTableRow> {
        Ok(EventPtrTableRow {
            event: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct EventTableRow {
    pub event_flags: (),
    pub name: (),
    pub event_type: (),
}

impl<'a> ReadData<EventTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<EventTableRow> {
        Ok(EventTableRow {
            event_flags: self.read()?,
            name: self.read()?,
            event_type: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct PropertyMapTableRow {
    pub parent: (),
    pub property_list: (),
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
    pub property: (),
}

impl<'a> ReadData<PropertyPtrTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<PropertyPtrTableRow> {
        Ok(PropertyPtrTableRow {
            property: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct PropertyTableRow {
    pub prop_flags: (),
    pub name: (),
    pub ty: (),
}

impl<'a> ReadData<PropertyTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<PropertyTableRow> {
        Ok(PropertyTableRow {
            prop_flags: self.read()?,
            name: self.read()?,
            ty: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct MethodSemanticsTableRow {
    pub semantic: (),
    pub method: (),
    pub association: (),
}

impl<'a> ReadData<MethodSemanticsTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<MethodSemanticsTableRow> {
        Ok(MethodSemanticsTableRow {
            semantic: self.read()?,
            method: self.read()?,
            association: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct MethodImplTableRow {
    pub class: (),
    pub method_body: (),
    pub method_declaration: (),
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
    pub name: (),
}

impl<'a> ReadData<ModuleRefTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ModuleRefTableRow> {
        Ok(ModuleRefTableRow { name: self.read()? })
    }
}

#[derive(Debug, Clone)]
pub struct TypeSpecTableRow {
    pub signature: (),
}

impl<'a> ReadData<TypeSpecTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<TypeSpecTableRow> {
        Ok(TypeSpecTableRow {
            signature: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ImplMapTableRow {
    pub mapping_flags: (),
    pub member_forwarded: (),
    pub import_name: (),
    pub import_scope: (),
}

impl<'a> ReadData<ImplMapTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ImplMapTableRow> {
        Ok(ImplMapTableRow {
            mapping_flags: self.read()?,
            member_forwarded: self.read()?,
            import_name: self.read()?,
            import_scope: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct FieldRVATableRow {
    pub rva: (),
    pub field: (),
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
    pub token: (),
    pub func_code: (),
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
    pub token: (),
}

impl<'a> ReadData<ENCMapTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ENCMapTableRow> {
        Ok(ENCMapTableRow {
            token: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AssemblyTableRow {
    pub hash_alg_id: (),
    pub major_version: (),
    pub minor_version: (),
    pub build_number: (),
    pub revision_number: (),
    pub flags: (),
    pub public_key: (),
    pub name: (),
    pub locale: (),
}

impl<'a> ReadData<AssemblyTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<AssemblyTableRow> {
        Ok(AssemblyTableRow {
            hash_alg_id: self.read()?,
            major_version: self.read()?,
            minor_version: self.read()?,
            build_number: self.read()?,
            revision_number: self.read()?,
            flags: self.read()?,
            public_key: self.read()?,
            name: self.read()?,
            locale: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AssemblyProcessorTableRow {
    pub processor: (),
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
    pub os_platform_id: (),
    pub os_major_version: (),
    pub os_minor_version: (),
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
    pub major_version: (),
    pub minor_version: (),
    pub build_number: (),
    pub revision_number: (),
    pub flags: (),
    pub public_key_or_token: (),
    pub name: (),
    pub locale: (),
    pub hash_value: (),
}

impl<'a> ReadData<AssemblyRefTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<AssemblyRefTableRow> {
        Ok(AssemblyRefTableRow {
            major_version: self.read()?,
            minor_version: self.read()?,
            build_number: self.read()?,
            revision_number: self.read()?,
            flags: self.read()?,
            public_key_or_token: self.read()?,
            name: self.read()?,
            locale: self.read()?,
            hash_value: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AssemblyRefProcessorTableRow {
    pub processor: (),
    pub assembly_ref: (),
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
    pub os_platform_id: (),
    pub os_major_version: (),
    pub os_minor_version: (),
    pub assembly_ref: (),
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

#[derive(Debug, Clone)]
pub struct FileTableRow {
    pub flags: (),
    pub name: (),
    pub hash_value: (),
}

impl<'a> ReadData<FileTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<FileTableRow> {
        Ok(FileTableRow {
            flags: self.read()?,
            name: self.read()?,
            hash_value: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ExportedTypeTableRow {
    pub flags: (),
    pub type_def_id: (),
    pub type_name: (),
    pub type_namespace: (),
    pub implementation: (),
}

impl<'a> ReadData<ExportedTypeTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ExportedTypeTableRow> {
        Ok(ExportedTypeTableRow {
            flags: self.read()?,
            type_def_id: self.read()?,
            type_name: self.read()?,
            type_namespace: self.read()?,
            implementation: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ManifestResourceTableRow {
    pub offset: (),
    pub flags: (),
    pub name: (),
    pub implementation: (),
}

impl<'a> ReadData<ManifestResourceTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<ManifestResourceTableRow> {
        Ok(ManifestResourceTableRow {
            offset: self.read()?,
            flags: self.read()?,
            name: self.read()?,
            implementation: self.read()?,
        })
    }
}



#[derive(Debug, Clone)]
pub struct NestedClassTableRow {
    pub nested_class: (),
    pub enclosing_class: (),
}

impl<'a> ReadData<NestedClassTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<NestedClassTableRow> {
        Ok(NestedClassTableRow {
            nested_class: self.read()?,
            enclosing_class: self.read()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct GenericParamTableRow {
    pub number: (),
    pub flags: (),
    pub owner: (),
    pub name: (),
    pub kind: Option<()>,
}

impl<'a> ReadData<GenericParamTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<GenericParamTableRow> {
        Ok(GenericParamTableRow {
            number: self.read()?,
            flags: self.read()?,
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
    pub method: (),
    pub instantiation: (),
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
    pub owner: (),
    pub constraint: (),
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
    pub name: (),
    pub hash_algorithm: (),
    pub hash: (),
    pub language: (),
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
    pub document: (),
    pub sequence_points: (),
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
    pub method: (),
    pub import_scope: (),
    pub variable_list: (),
    pub constant_list: (),
    pub start_offset: (),
    pub length: (),
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

impl<'a> ReadData<CustomDebugInformationTableRow> for TablesStreamReader<'a> {
    fn read(&mut self) -> Result<CustomDebugInformationTableRow> {
        Ok(CustomDebugInformationTableRow {
            parent: self.read()?,
            kind: self.read()?,
            value: self.read()?,
        })
    }
}
