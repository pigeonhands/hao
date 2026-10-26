//! Writes entries as C#-like source. Only declarations are produced; method
//! bodies can optionally be listed as IL comments.

mod il;

use std::fmt::{self, Write};

use crate::{
    dotnet::{
        entries::*,
        md::streams::tables_stream::{
            FieldFlags, GenericParamFlags, MethodFlags, ParamFlags, TypeAttributes,
        },
        signature::{ElementType, MethodSig, TypeSig},
    },
    Module,
};

#[derive(Debug, Clone, Default)]
pub struct CSharpOptions {
    /// Write type names with their namespace.
    pub full_names: bool,
    /// List method bodies as IL comments.
    pub il_bodies: bool,
}

pub struct CSharpWriter<W: Write> {
    out: W,
    indent: usize,
    at_line_start: bool,
    pub options: CSharpOptions,
    type_generics: Vec<String>,
    method_generics: Vec<String>,
}

impl<W: Write> CSharpWriter<W> {
    pub fn new(out: W) -> Self {
        Self::with_options(out, CSharpOptions::default())
    }

    pub fn with_options(out: W, options: CSharpOptions) -> Self {
        Self {
            out,
            indent: 0,
            at_line_start: true,
            options,
            type_generics: Vec::new(),
            method_generics: Vec::new(),
        }
    }

    pub fn into_inner(self) -> W {
        self.out
    }

    pub fn indent(&mut self) {
        self.indent += 1;
    }

    pub fn dedent(&mut self) {
        self.indent = self.indent.saturating_sub(1);
    }

    pub fn newline(&mut self) -> fmt::Result {
        self.write_str("\n")
    }

    pub fn open_block(&mut self) -> fmt::Result {
        self.write_str("\n{\n")?;
        self.indent();
        Ok(())
    }

    pub fn close_block(&mut self) -> fmt::Result {
        self.dedent();
        self.write_str("}\n")
    }

    pub fn write<T: WriteCSharp + ?Sized>(&mut self, value: &T) -> fmt::Result {
        value.write_csharp(self)
    }

    fn separated<T>(
        &mut self,
        items: impl IntoIterator<Item = T>,
        separator: &str,
        mut f: impl FnMut(&mut Self, T) -> fmt::Result,
    ) -> fmt::Result {
        for (i, item) in items.into_iter().enumerate() {
            if i > 0 {
                self.write_str(separator)?;
            }
            f(self, item)?;
        }
        Ok(())
    }

    fn generic_name(&self, method: bool, index: u32) -> String {
        let names = if method {
            &self.method_generics
        } else {
            &self.type_generics
        };
        names
            .get(index as usize)
            .cloned()
            .unwrap_or_else(|| format!("{}{}", if method { "M" } else { "T" }, index))
    }

    fn type_name(&mut self, namespace: &str, name: &str) -> fmt::Result {
        if let Some(alias) = keyword_alias(namespace, name) {
            return self.write_str(alias);
        }
        if self.options.full_names && !namespace.is_empty() {
            write!(self, "{}.", namespace)?;
        }
        self.write_str(strip_arity(name))
    }
}

impl<W: Write> Write for CSharpWriter<W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for (i, line) in s.split('\n').enumerate() {
            if i > 0 {
                self.out.write_char('\n')?;
                self.at_line_start = true;
            }
            if !line.is_empty() {
                if self.at_line_start {
                    for _ in 0..self.indent {
                        self.out.write_str("    ")?;
                    }
                    self.at_line_start = false;
                }
                self.out.write_str(line)?;
            }
        }
        Ok(())
    }
}

pub trait WriteCSharp {
    fn write_csharp<W: Write>(&self, w: &mut CSharpWriter<W>) -> fmt::Result;

    fn to_csharp(&self) -> String {
        let mut w = CSharpWriter::new(String::new());
        let _ = self.write_csharp(&mut w);
        w.into_inner()
    }

    /// A [`Display`](fmt::Display) adapter.
    fn csharp(&self) -> CSharp<'_, Self> {
        CSharp(self)
    }
}

pub struct CSharp<'a, T: ?Sized>(&'a T);

impl<T: WriteCSharp + ?Sized> fmt::Display for CSharp<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.write_csharp(&mut CSharpWriter::new(f))
    }
}

impl<T: WriteCSharp> WriteCSharp for Entry<T> {
    fn write_csharp<W: Write>(&self, w: &mut CSharpWriter<W>) -> fmt::Result {
        self.get().write_csharp(w)
    }
}

fn strip_arity(name: &str) -> &str {
    name.split_once('`').map_or(name, |(n, _)| n)
}

fn keyword_alias(namespace: &str, name: &str) -> Option<&'static str> {
    if namespace != "System" {
        return None;
    }
    Some(match name {
        "Void" => "void",
        "Boolean" => "bool",
        "Char" => "char",
        "SByte" => "sbyte",
        "Byte" => "byte",
        "Int16" => "short",
        "UInt16" => "ushort",
        "Int32" => "int",
        "UInt32" => "uint",
        "Int64" => "long",
        "UInt64" => "ulong",
        "Single" => "float",
        "Double" => "double",
        "Decimal" => "decimal",
        "String" => "string",
        "Object" => "object",
        "IntPtr" => "nint",
        "UIntPtr" => "nuint",
        _ => return None,
    })
}

impl WriteCSharp for TypeDefOrRef {
    fn write_csharp<W: Write>(&self, w: &mut CSharpWriter<W>) -> fmt::Result {
        match self {
            Self::TypeDef(t) => {
                let t = t.get();
                w.type_name(&t.namespace, &t.name)
            }
            Self::TypeRef(t) => {
                let t = t.get();
                w.type_name(&t.namespace, &t.name)
            }
            Self::TypeSpec(t) => t.get().signature.write_csharp(w),
        }
    }
}

impl WriteCSharp for TypeSig {
    fn write_csharp<W: Write>(&self, w: &mut CSharpWriter<W>) -> fmt::Result {
        match self {
            Self::Primitive(p) => w.write_str(p.keyword()),
            Self::ValueType(t) | Self::Class(t) => t.write_csharp(w),
            Self::Ptr(t) => {
                t.write_csharp(w)?;
                w.write_str("*")
            }
            Self::ByRef(t) => {
                w.write_str("ref ")?;
                t.write_csharp(w)
            }
            Self::Pinned(t) => t.write_csharp(w),
            Self::Modified { ty, .. } => ty.write_csharp(w),
            Self::SzArray(t) => {
                t.write_csharp(w)?;
                w.write_str("[]")
            }
            Self::Array { element, shape } => {
                element.write_csharp(w)?;
                write!(w, "[{}]", ",".repeat(shape.rank.saturating_sub(1) as usize))
            }
            Self::GenericInst { generic, args } => {
                if generic
                    .type_def_or_ref()
                    .is_some_and(|t| t.full_name_is("System", "Nullable`1"))
                    && args.len() == 1
                {
                    args[0].write_csharp(w)?;
                    return w.write_str("?");
                }
                generic.write_csharp(w)?;
                w.write_str("<")?;
                w.separated(args, ", ", |w, a| a.write_csharp(w))?;
                w.write_str(">")
            }
            Self::Var(i) => {
                let name = w.generic_name(false, *i);
                w.write_str(&name)
            }
            Self::MVar(i) => {
                let name = w.generic_name(true, *i);
                w.write_str(&name)
            }
            Self::FnPtr(sig) => {
                w.write_str("delegate*<")?;
                for p in &sig.params {
                    p.write_csharp(w)?;
                    w.write_str(", ")?;
                }
                sig.ret.write_csharp(w)?;
                w.write_str(">")
            }
        }
    }
}

impl WriteCSharp for MethodSig {
    fn write_csharp<W: Write>(&self, w: &mut CSharpWriter<W>) -> fmt::Result {
        self.ret.write_csharp(w)?;
        w.write_str(" (")?;
        w.separated(&self.params, ", ", |w, p| p.write_csharp(w))?;
        if !self.varargs.is_empty() {
            w.write_str(", __arglist")?;
        }
        w.write_str(")")
    }
}

fn type_visibility(flags: TypeAttributes) -> &'static str {
    match flags & TypeAttributes::VisibilityMask {
        TypeAttributes::Public | TypeAttributes::NestedPublic => "public",
        TypeAttributes::NestedPrivate => "private",
        TypeAttributes::NestedFamily => "protected",
        TypeAttributes::NestedFamANDAssem => "private protected",
        TypeAttributes::NestedFamORAssem => "protected internal",
        _ => "internal",
    }
}

fn member_visibility(access: u16) -> &'static str {
    match access & 0x7 {
        0x1 => "private",
        0x2 => "private protected",
        0x3 => "internal",
        0x4 => "protected",
        0x5 => "protected internal",
        0x6 => "public",
        _ => "private",
    }
}

fn write_attributes<W: Write>(
    w: &mut CSharpWriter<W>,
    attributes: &[CustomAttribute],
) -> fmt::Result {
    for attribute in attributes {
        let name = attribute.type_name();
        let short = name.rsplit(['.', '/']).next().unwrap_or(&name);
        let short = short.strip_suffix("Attribute").unwrap_or(short);
        if matches!(
            short,
            "CompilerGenerated"
                | "Nullable"
                | "NullableContext"
                | "IsReadOnly"
                | "DebuggerBrowsable"
        ) {
            continue;
        }
        writeln!(w, "[{}]", short)?;
    }
    Ok(())
}

fn write_constant<W: Write>(w: &mut CSharpWriter<W>, constant: &Constant) -> fmt::Result {
    if let Some(s) = constant.as_string() {
        return write!(w, "{:?}", s);
    }
    match constant.element_type {
        ElementType::Boolean => w.write_str(if constant.as_i64() == Some(0) {
            "false"
        } else {
            "true"
        }),
        ElementType::Char => match constant.as_i64().and_then(|c| char::from_u32(c as u32)) {
            Some(c) => write!(w, "{:?}", c),
            None => w.write_str("'\\0'"),
        },
        ElementType::R4 => match constant.value.get(..4) {
            Some(b) => write!(w, "{}f", f32::from_le_bytes(b.try_into().unwrap())),
            None => w.write_str("0f"),
        },
        ElementType::R8 => match constant.value.get(..8) {
            Some(b) => write!(w, "{}", f64::from_le_bytes(b.try_into().unwrap())),
            None => w.write_str("0"),
        },
        ElementType::Class => w.write_str("null"),
        ElementType::U8 => match constant.value.get(..8) {
            Some(b) => write!(w, "{}", u64::from_le_bytes(b.try_into().unwrap())),
            None => w.write_str("0"),
        },
        _ => write!(w, "{}", constant.as_i64().unwrap_or_default()),
    }
}

fn write_generic_params<W: Write>(
    w: &mut CSharpWriter<W>,
    params: &[Entry<GenericParam>],
) -> fmt::Result {
    if params.is_empty() {
        return Ok(());
    }
    w.write_str("<")?;
    w.separated(params, ", ", |w, p| {
        let p = p.get();
        let variance = p.flags & GenericParamFlags::VarianceMask;
        if variance == GenericParamFlags::Covariant {
            w.write_str("out ")?;
        } else if variance == GenericParamFlags::Contravariant {
            w.write_str("in ")?;
        }
        w.write_str(&p.name)
    })?;
    w.write_str(">")
}

fn write_constraints<W: Write>(
    w: &mut CSharpWriter<W>,
    params: &[Entry<GenericParam>],
) -> fmt::Result {
    for p in params {
        let p = p.get();
        let mut parts = Vec::new();
        if p.flags
            .contains(GenericParamFlags::NotNullableValueTypeConstraint)
        {
            parts.push("struct".to_owned());
        } else if p.flags.contains(GenericParamFlags::ReferenceTypeConstraint) {
            parts.push("class".to_owned());
        }
        for c in &p.constraints {
            if c.constraint
                .is_system_type(well_known::SystemType::ValueType)
            {
                continue;
            }
            parts.push(c.constraint.to_csharp_with(w));
        }
        if p.flags
            .contains(GenericParamFlags::DefaultConstructorConstraint)
            && !p
                .flags
                .contains(GenericParamFlags::NotNullableValueTypeConstraint)
        {
            parts.push("new()".to_owned());
        }
        if !parts.is_empty() {
            write!(w, " where {} : {}", p.name, parts.join(", "))?;
        }
    }
    Ok(())
}

trait ToCSharpWith {
    fn to_csharp_with<W: Write>(&self, w: &CSharpWriter<W>) -> String;
}

impl<T: WriteCSharp> ToCSharpWith for T {
    /// Writes into a new string using the generic context of `w`.
    fn to_csharp_with<W: Write>(&self, w: &CSharpWriter<W>) -> String {
        let mut inner = CSharpWriter::with_options(String::new(), w.options.clone());
        inner.type_generics = w.type_generics.clone();
        inner.method_generics = w.method_generics.clone();
        let _ = self.write_csharp(&mut inner);
        inner.into_inner()
    }
}

impl WriteCSharp for Field {
    fn write_csharp<W: Write>(&self, w: &mut CSharpWriter<W>) -> fmt::Result {
        write_attributes(w, &self.custom_attributes)?;
        w.write_str(member_visibility(self.flags.bits()))?;
        if self.flags.contains(FieldFlags::Literal) {
            w.write_str(" const")?;
        } else {
            if self.is_static() {
                w.write_str(" static")?;
            }
            if self.flags.contains(FieldFlags::InitOnly) {
                w.write_str(" readonly")?;
            }
        }
        w.write_str(" ")?;
        self.signature.ty.write_csharp(w)?;
        write!(w, " {}", self.name)?;
        if let Some(constant) = &self.constant {
            w.write_str(" = ")?;
            write_constant(w, constant)?;
        }
        w.write_str(";")
    }
}

impl WriteCSharp for Method {
    fn write_csharp<W: Write>(&self, w: &mut CSharpWriter<W>) -> fmt::Result {
        w.method_generics = self
            .generic_params
            .iter()
            .map(|g| g.get().name.clone())
            .collect();
        let result = write_method(w, self);
        w.method_generics.clear();
        result
    }
}

fn write_method<W: Write>(w: &mut CSharpWriter<W>, m: &Method) -> fmt::Result {
    let declaring = m.declaring_type.upgrade();
    let in_interface = declaring.as_ref().is_some_and(|t| t.get().is_interface());
    write_attributes(w, &m.custom_attributes)?;

    if m.is_cctor() {
        w.write_str("static ")?;
    } else if !in_interface {
        w.write_str(member_visibility(m.flags.bits()))?;
        w.write_str(" ")?;
        if m.is_static() {
            w.write_str("static ")?;
        }
        if m.impl_map.is_some() || m.flags.contains(MethodFlags::PinvokeImpl) {
            w.write_str("extern ")?;
        }
        let virt = m.flags.contains(MethodFlags::Virtual);
        let new_slot = m.flags.contains(MethodFlags::NewSlot);
        let is_final = m.flags.contains(MethodFlags::Final);
        if m.flags.contains(MethodFlags::Abstract) {
            w.write_str(if new_slot {
                "abstract "
            } else {
                "abstract override "
            })?;
        } else if virt && !new_slot {
            w.write_str(if is_final {
                "sealed override "
            } else {
                "override "
            })?;
        } else if virt && !is_final {
            w.write_str("virtual ")?;
        }
    }

    if m.is_ctor() || m.is_cctor() {
        let name = declaring.map(|t| t.get().name.clone()).unwrap_or_default();
        w.write_str(strip_arity(&name))?;
    } else {
        m.signature.ret.write_csharp(w)?;
        write!(w, " {}", m.name)?;
        write_generic_params(w, &m.generic_params)?;
    }

    w.write_str("(")?;
    w.separated(m.signature.params.iter().enumerate(), ", ", |w, (i, ty)| {
        let flags = m
            .param(i as u16 + 1)
            .map(|p| p.get().flags)
            .unwrap_or_default();
        match ty {
            TypeSig::ByRef(inner) => {
                w.write_str(if flags.contains(ParamFlags::Out) {
                    "out "
                } else if flags.contains(ParamFlags::In) {
                    "in "
                } else {
                    "ref "
                })?;
                inner.write_csharp(w)?;
            }
            ty => ty.write_csharp(w)?,
        }
        write!(w, " {}", m.param_name(i))
    })?;
    if !m.signature.varargs.is_empty() {
        w.write_str(", __arglist")?;
    }
    w.write_str(")")?;
    write_constraints(w, &m.generic_params)?;

    match &m.body {
        Some(body) if w.options.il_bodies => {
            w.open_block()?;
            body.write_csharp(w)?;
            w.close_block()
        }
        _ => w.write_str(";\n"),
    }
}

impl WriteCSharp for Property {
    fn write_csharp<W: Write>(&self, w: &mut CSharpWriter<W>) -> fmt::Result {
        write_attributes(w, &self.custom_attributes)?;
        let accessor = self.getter.as_ref().or(self.setter.as_ref());
        if let Some(accessor) = accessor {
            let accessor = accessor.get();
            let in_interface = accessor
                .declaring_type
                .upgrade()
                .is_some_and(|t| t.get().is_interface());
            if !in_interface {
                write!(w, "{} ", member_visibility(accessor.flags.bits()))?;
            }
            if accessor.is_static() {
                w.write_str("static ")?;
            }
        }
        self.signature.ty.write_csharp(w)?;
        if self.signature.params.is_empty() {
            write!(w, " {} {{", self.name)?;
        } else {
            w.write_str(" this[")?;
            w.separated(&self.signature.params, ", ", |w, p| p.write_csharp(w))?;
            w.write_str("] {")?;
        }
        if self.getter.is_some() {
            w.write_str(" get;")?;
        }
        if self.setter.is_some() {
            w.write_str(" set;")?;
        }
        w.write_str(" }")
    }
}

impl WriteCSharp for Event {
    fn write_csharp<W: Write>(&self, w: &mut CSharpWriter<W>) -> fmt::Result {
        write_attributes(w, &self.custom_attributes)?;
        if let Some(add) = &self.add {
            let add = add.get();
            write!(w, "{} ", member_visibility(add.flags.bits()))?;
            if add.is_static() {
                w.write_str("static ")?;
            }
        }
        w.write_str("event ")?;
        if let Some(ty) = &self.event_type {
            ty.write_csharp(w)?;
        }
        write!(w, " {};", self.name)
    }
}

fn accessor_methods(t: &TypeDef) -> Vec<Entry<Method>> {
    let mut out = Vec::new();
    for p in &t.properties {
        let p = p.get();
        out.extend(p.getter.clone());
        out.extend(p.setter.clone());
        out.extend(p.other_methods.iter().cloned());
    }
    for e in &t.events {
        let e = e.get();
        out.extend(e.add.clone());
        out.extend(e.remove.clone());
        out.extend(e.fire.clone());
        out.extend(e.other_methods.iter().cloned());
    }
    out
}

impl WriteCSharp for TypeDef {
    fn write_csharp<W: Write>(&self, w: &mut CSharpWriter<W>) -> fmt::Result {
        let saved = std::mem::replace(
            &mut w.type_generics,
            self.generic_params
                .iter()
                .map(|g| g.get().name.clone())
                .collect(),
        );
        let result = write_type(w, self);
        w.type_generics = saved;
        result
    }
}

fn write_type<W: Write>(w: &mut CSharpWriter<W>, t: &TypeDef) -> fmt::Result {
    write_attributes(w, &t.custom_attributes)?;
    write!(w, "{} ", type_visibility(t.flags))?;

    if t.is_delegate() {
        if let Some(invoke) = t.find_method("Invoke") {
            let invoke = invoke.get();
            w.write_str("delegate ")?;
            invoke.signature.ret.write_csharp(w)?;
            write!(w, " {}", strip_arity(&t.name))?;
            write_generic_params(w, &t.generic_params)?;
            w.write_str("(")?;
            w.separated(
                invoke.signature.params.iter().enumerate(),
                ", ",
                |w, (i, p)| {
                    p.write_csharp(w)?;
                    write!(w, " {}", invoke.param_name(i))
                },
            )?;
            return w.write_str(");\n");
        }
    }

    let kind = if t.is_enum() {
        "enum"
    } else if t.is_interface() {
        "interface"
    } else if t.is_value_type() {
        if t.custom_attributes
            .iter()
            .any(|a| a.type_name().ends_with("IsReadOnlyAttribute"))
        {
            "readonly struct"
        } else {
            "struct"
        }
    } else if t.is_static() {
        "static class"
    } else if t.flags.contains(TypeAttributes::Abstract) {
        "abstract class"
    } else if t.flags.contains(TypeAttributes::Sealed) {
        "sealed class"
    } else {
        "class"
    };
    write!(w, "{} {}", kind, strip_arity(&t.name))?;
    let inherited = t
        .declaring_type
        .upgrade()
        .map_or(0, |p| p.get().generic_params.len());
    let own_generics = t.generic_params.get(inherited..).unwrap_or_default();
    write_generic_params(w, own_generics)?;

    let mut bases = Vec::new();
    if t.is_enum() {
        if let Some(field) = t.enum_underlying_field() {
            let ty = field.get().signature.ty.clone();
            if ty != TypeSig::Primitive(crate::dotnet::signature::PrimitiveType::I4) {
                bases.push(ty.to_csharp_with(w));
            }
        }
    } else if let Some(extends) = &t.extends {
        let implicit = [
            well_known::SystemType::Object,
            well_known::SystemType::ValueType,
        ]
        .into_iter()
        .any(|s| extends.is_system_type(s));
        if !implicit {
            bases.push(extends.to_csharp_with(w));
        }
    }
    bases.extend(t.interfaces.iter().map(|i| i.interface.to_csharp_with(w)));
    if !bases.is_empty() {
        write!(w, " : {}", bases.join(", "))?;
    }
    write_constraints(w, own_generics)?;
    w.open_block()?;

    if t.is_enum() {
        for field in &t.fields {
            let field = field.get();
            if !field.is_static() {
                continue;
            }
            w.write_str(&field.name)?;
            if let Some(constant) = &field.constant {
                w.write_str(" = ")?;
                write_constant(w, constant)?;
            }
            w.write_str(",\n")?;
        }
        return w.close_block();
    }

    let event_names: Vec<String> = t.events.iter().map(|e| e.get().name.clone()).collect();
    for field in &t.fields {
        let name = &field.get().name;
        if name.contains('<') || event_names.contains(name) {
            continue;
        }
        field.write_csharp(w)?;
        w.newline()?;
    }
    for property in &t.properties {
        property.write_csharp(w)?;
        w.newline()?;
    }
    for event in &t.events {
        event.write_csharp(w)?;
        w.newline()?;
    }
    let accessors = accessor_methods(t);
    for method in &t.methods {
        if accessors.contains(method) {
            continue;
        }
        method.write_csharp(w)?;
    }
    for nested in &t.nested_types {
        if nested.get().name.starts_with('<') {
            continue;
        }
        w.newline()?;
        nested.write_csharp(w)?;
    }
    w.close_block()
}

impl WriteCSharp for Module {
    fn write_csharp<W: Write>(&self, w: &mut CSharpWriter<W>) -> fmt::Result {
        let mut namespaces: Vec<(String, Vec<Entry<TypeDef>>)> = Vec::new();
        for ty in self.types.iter().skip(1) {
            if ty.get().name.starts_with('<') {
                continue;
            }
            let ns = ty.get().namespace.clone();
            match namespaces.iter_mut().find(|(n, _)| *n == ns) {
                Some((_, types)) => types.push(ty.clone()),
                None => namespaces.push((ns, vec![ty.clone()])),
            }
        }

        writeln!(w, "// {}", self.name())?;
        for (ns, types) in namespaces {
            w.newline()?;
            if !ns.is_empty() {
                write!(w, "namespace {}", ns)?;
                w.open_block()?;
            }
            for (i, ty) in types.iter().enumerate() {
                if i > 0 {
                    w.newline()?;
                }
                ty.write_csharp(w)?;
            }
            if !ns.is_empty() {
                w.close_block()?;
            }
        }
        Ok(())
    }
}

impl Module {
    pub fn to_csharp(&self) -> String {
        WriteCSharp::to_csharp(self)
    }

    pub fn to_csharp_with(&self, options: CSharpOptions) -> String {
        let mut w = CSharpWriter::with_options(String::new(), options);
        let _ = self.write_csharp(&mut w);
        w.into_inner()
    }
}
