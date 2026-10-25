use super::Module;
use crate::{
    dotnet::{
        entries::*,
        il::MemberToken,
        signature::{FieldSig, MemberRefSig, MethodSig},
    },
    error::{HaoError, Result},
};

fn split_full_name(full_name: &str) -> (&str, &str) {
    full_name.rsplit_once('.').unwrap_or(("", full_name))
}

fn not_found(what: String) -> HaoError {
    HaoError::IoError(std::io::Error::new(std::io::ErrorKind::NotFound, what))
}

impl Module {
    /// Finds a method by the full name of its type (`Ns.Type`, `Ns.Outer/Inner`) and its name.
    pub fn find_method(&self, type_full_name: &str, name: &str) -> Option<Entry<Method>> {
        self.find_type_by_full_name(type_full_name)?.get().find_method(name)
    }

    /// Finds a field by the full name of its type and its name.
    pub fn find_field(&self, type_full_name: &str, name: &str) -> Option<Entry<Field>> {
        self.find_type_by_full_name(type_full_name)?.get().find_field(name)
    }

    pub fn find_assembly_ref(&self, name: &str) -> Option<Entry<AssemblyRef>> {
        self.assembly_refs
            .iter()
            .find(|a| a.get().name.eq_ignore_ascii_case(name))
            .cloned()
    }

    /// Gets a reference to a type for use in signatures and instructions.
    ///
    /// Uses the type definition if it is in this module, then an existing type
    /// reference, then any loaded dependency that defines the type.
    pub fn import_type(&mut self, full_name: &str) -> Result<TypeDefOrRef> {
        if let Some(def) = self.find_type_by_full_name(full_name) {
            return Ok(TypeDefOrRef::TypeDef(def));
        }
        let (namespace, name) = split_full_name(full_name);
        if let Some(existing) = self
            .type_refs
            .iter()
            .find(|t| t.get().full_name_is(namespace, name))
        {
            return Ok(TypeDefOrRef::TypeRef(existing.clone()));
        }
        let assembly = self
            .assembly_refs
            .iter()
            .find(|a| {
                a.get()
                    .resolved
                    .as_ref()
                    .is_some_and(|m| m.resolve_type(namespace, name).is_some())
            })
            .cloned()
            .ok_or_else(|| {
                not_found(format!(
                    "{} is not referenced by this module; use import_type_from",
                    full_name
                ))
            })?;
        Ok(self.add_type_ref(assembly, namespace, name))
    }

    /// Gets a reference to `full_name` in the referenced assembly `assembly`.
    pub fn import_type_from(&mut self, assembly: &str, full_name: &str) -> Result<TypeDefOrRef> {
        let (namespace, name) = split_full_name(full_name);
        if let Some(existing) = self.type_refs.iter().find(|t| {
            let t = t.get();
            t.full_name_is(namespace, name)
                && matches!(&t.scope, Some(ResolutionScope::AssemblyRef(a)) if a.get().name.eq_ignore_ascii_case(assembly))
        }) {
            return Ok(TypeDefOrRef::TypeRef(existing.clone()));
        }
        let assembly = self
            .find_assembly_ref(assembly)
            .ok_or_else(|| not_found(format!("assembly {} is not referenced", assembly)))?;
        Ok(self.add_type_ref(assembly, namespace, name))
    }

    fn add_type_ref(&mut self, assembly: Entry<AssemblyRef>, namespace: &str, name: &str) -> TypeDefOrRef {
        let type_ref = Entry::new(TypeRef::new(
            ResolutionScope::AssemblyRef(assembly),
            namespace,
            name,
        ));
        self.type_refs.push(type_ref.clone());
        TypeDefOrRef::TypeRef(type_ref)
    }

    /// Gets a token for a method, ready to use as a `call` operand.
    ///
    /// ```no_run
    /// # use hao::{Module, dotnet::{il::Instruction, signature::{MethodSig, TypeSig}}};
    /// # let mut module = Module::default();
    /// let write_line = module.import_method(
    ///     "System.Console",
    ///     "WriteLine",
    ///     MethodSig::new_static(TypeSig::VOID, vec![TypeSig::STRING]),
    /// ).unwrap();
    /// let call = Instruction::call(write_line);
    /// ```
    pub fn import_method(&mut self, type_full_name: &str, name: &str, signature: MethodSig) -> Result<MemberToken> {
        match self.import_type(type_full_name)? {
            TypeDefOrRef::TypeDef(def) => def
                .get()
                .methods
                .iter()
                .find(|m| m.get().name == name && m.get().signature == signature)
                .cloned()
                .map(MemberToken::Method)
                .ok_or_else(|| not_found(format!("{}::{} with that signature", type_full_name, name))),
            parent => Ok(MemberToken::MemberRef(self.import_member_ref(
                parent,
                name,
                MemberRefSig::Method(signature),
            ))),
        }
    }

    /// Gets a token for a field, ready to use as a `ldfld`/`stsfld` operand.
    pub fn import_field(&mut self, type_full_name: &str, name: &str, signature: FieldSig) -> Result<MemberToken> {
        match self.import_type(type_full_name)? {
            TypeDefOrRef::TypeDef(def) => def
                .get()
                .find_field(name)
                .map(MemberToken::Field)
                .ok_or_else(|| not_found(format!("{}::{}", type_full_name, name))),
            parent => Ok(MemberToken::MemberRef(self.import_member_ref(
                parent,
                name,
                MemberRefSig::Field(signature),
            ))),
        }
    }

    fn import_member_ref(&mut self, parent: TypeDefOrRef, name: &str, signature: MemberRefSig) -> Entry<MemberRef> {
        let parent = match parent {
            TypeDefOrRef::TypeDef(t) => MemberRefParent::TypeDef(t),
            TypeDefOrRef::TypeRef(t) => MemberRefParent::TypeRef(t),
            TypeDefOrRef::TypeSpec(t) => MemberRefParent::TypeSpec(t),
        };
        if let Some(existing) = self.member_refs.iter().find(|m| {
            let m = m.get();
            m.parent == parent && m.name == name && m.signature == signature
        }) {
            return existing.clone();
        }
        let member_ref = Entry::new(MemberRef {
            parent,
            name: name.into(),
            signature,
            custom_attributes: Vec::new(),
        });
        self.member_refs.push(member_ref.clone());
        member_ref
    }
}

impl Entry<Method> {
    /// Runs `f` on the method body. Returns `None` if the method has no body.
    pub fn edit_body<R>(&self, f: impl FnOnce(&mut crate::dotnet::il::MethodBody) -> R) -> Option<R> {
        self.get_mut().body.as_mut().map(f)
    }
}
