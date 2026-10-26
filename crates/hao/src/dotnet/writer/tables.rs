use crate::dotnet::md::{
    streams::tables_stream::{
        coded_tokens::{CodedToken, CodedTokenSizes, CodedTokenTarget},
        metadata::TableRows,
        *,
    },
    MDStreamFlags,
};

pub(crate) struct RowWriter<'a> {
    pub buf: Vec<u8>,
    pub rows: &'a TableRows,
    pub flags: MDStreamFlags,
    pub coded: &'a CodedTokenSizes,
}

impl RowWriter<'_> {
    fn sized(&mut self, size: ValueSize, value: u32) {
        match size {
            ValueSize::Small => self.buf.extend_from_slice(&(value as u16).to_le_bytes()),
            ValueSize::Big => self.buf.extend_from_slice(&value.to_le_bytes()),
        }
    }
}

pub(crate) trait Column {
    fn write_column(&self, w: &mut RowWriter);
}

impl Column for u8 {
    fn write_column(&self, w: &mut RowWriter) {
        w.buf.push(*self);
    }
}

impl Column for u16 {
    fn write_column(&self, w: &mut RowWriter) {
        w.buf.extend_from_slice(&self.to_le_bytes());
    }
}

impl Column for u32 {
    fn write_column(&self, w: &mut RowWriter) {
        w.buf.extend_from_slice(&self.to_le_bytes());
    }
}

impl<T: CodedTokenTarget> Column for CodedToken<T> {
    fn write_column(&self, w: &mut RowWriter) {
        w.sized(T::token_size(w.coded), self.encode());
    }
}

macro_rules! heap_columns {
    ($($t:ty),*) => {$(
        impl Column for $t {
            fn write_column(&self, w: &mut RowWriter) {
                w.sized(<$t>::streams_offset_size(w.flags), self.0);
            }
        }
    )*};
}

macro_rules! table_columns {
    ($($t:ty),*) => {$(
        impl Column for $t {
            fn write_column(&self, w: &mut RowWriter) {
                w.sized(<$t>::table_offset_size(w.rows), self.0);
            }
        }
    )*};
}

macro_rules! flag_columns {
    ($($t:ty),*) => {$(
        impl Column for $t {
            fn write_column(&self, w: &mut RowWriter) {
                self.bits().write_column(w);
            }
        }
    )*};
}

heap_columns!(
    StringsStreamOffset,
    BlobStreamOffset,
    BlobStreamOffsetTypeSpec,
    GuidStreamOffset
);
table_columns!(
    TypeDefTableOffset,
    FieldTableOffset,
    MethodTableOffset,
    ParamTableOffset,
    EventTableOffset,
    PropertyTableOffset,
    ModuleRefTableOffset,
    GenericParamTableOffset
);
flag_columns!(
    TypeAttributes,
    FieldFlags,
    MethodImplFlags,
    MethodFlags,
    ParamFlags,
    EventFlags,
    PropertyFlags,
    MethodSemanticsFlags,
    PInvokeMapFlags,
    AssemblyFlags,
    FileFlags,
    ManifestResourceFlags,
    GenericParamFlags
);

pub(crate) trait WriteRow {
    fn write_row(&self, w: &mut RowWriter);
}

macro_rules! rows {
    ($($row:ty { $($field:ident),* })*) => {$(
        impl WriteRow for $row {
            fn write_row(&self, w: &mut RowWriter) {
                $(self.$field.write_column(w);)*
            }
        }
    )*};
}

rows! {
    ModulesTableRow { generation, name, mvid, enc_id, enc_base_id }
    TypeRefTableRow { resolution_scope, name, namespace }
    TypeDefTableRow { flags, name, namespace, extends, field_list, method_list }
    FieldTableRow { flags, name, signature }
    MethodTableRow { rva, impl_flags, flags, name, signature, param_list }
    ParamTableRow { flags, sequence, name }
    InterfaceImplTableRow { class, interface }
    MemberRefTableRow { class, name, signature }
    ConstantTableRow { ty, padding, parent, value }
    CustomAttributeTableRow { parent, ty, value }
    FieldMarshalTableRow { parent, native_type }
    DeclSecurityTableRow { action, parent, permission_set }
    ClassLayoutTableRow { packing_size, class_size, parent }
    FieldLayoutTableRow { offset, field }
    StandAloneSigTableRow { signature }
    EventMapTableRow { parent, event_list }
    EventTableRow { event_flags, name, event_type }
    PropertyMapTableRow { parent, property_list }
    PropertyTableRow { prop_flags, name, ty }
    MethodSemanticsTableRow { semantic, method, association }
    MethodImplTableRow { class, method_body, method_declaration }
    ModuleRefTableRow { name }
    TypeSpecTableRow { signature }
    ImplMapTableRow { mapping_flags, member_forwarded, import_name, import_scope }
    FieldRVATableRow { rva, field }
    AssemblyTableRow { hash_alg_id, major_version, minor_version, build_number, revision_number, flags, public_key, name, locale }
    AssemblyRefTableRow { major_version, minor_version, build_number, revision_number, flags, public_key_or_token, name, locale, hash_value }
    FileTableRow { flags, name, hash_value }
    ExportedTypeTableRow { flags, type_def_id, type_name, type_namespace, implementation }
    ManifestResourceTableRow { offset, flags, name, implementation }
    NestedClassTableRow { nested_class, enclosing_class }
    GenericParamTableRow { number, flags, owner, name }
    MethodSpecTableRow { method, instantiation }
    GenericParamConstraintTableRow { owner, constraint }
}
