use super::Module;
use crate::{
    dotnet::md::streams::tables_stream::AssemblyFlags,
    dotnet::{
        entries::*,
        il::MemberToken,
        signature::{FieldSig, MemberRefSig, MethodSig, TypeSig},
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

    /// Adds a reference to the assembly `name`, or returns the existing one.
    ///
    /// `public_key_token` is the 8 byte token, e.g. `b03f5f7f11d50a3a` for
    /// most framework assemblies. Pass an empty slice for unsigned assemblies.
    pub fn add_assembly_ref(&mut self, name: &str, version: Version, public_key_token: &[u8]) -> Entry<AssemblyRef> {
        if let Some(existing) = self.find_assembly_ref(name) {
            return existing;
        }
        let assembly_ref = Entry::new(AssemblyRef {
            version,
            name: name.into(),
            public_key_or_token: public_key_token.to_vec(),
            ..Default::default()
        });
        self.assembly_refs.push(assembly_ref.clone());
        assembly_ref
    }

    /// Adds a reference to the assembly `other` was loaded from, e.g. a mod
    /// referencing its loader. Returns the existing reference if there is one.
    pub fn add_reference_to(&mut self, other: &Module) -> Result<Entry<AssemblyRef>> {
        let assembly = other
            .assembly
            .as_ref()
            .ok_or_else(|| not_found(format!("{} has no assembly manifest", other.name())))?
            .get();
        if let Some(existing) = self.find_assembly_ref(&assembly.name) {
            return Ok(existing);
        }
        let flags = if assembly.public_key.is_empty() {
            AssemblyFlags::empty()
        } else {
            AssemblyFlags::PublicKey
        };
        let assembly_ref = Entry::new(AssemblyRef {
            version: assembly.version,
            flags,
            public_key_or_token: assembly.public_key.clone(),
            name: assembly.name.clone(),
            culture: assembly.culture.clone(),
            ..Default::default()
        });
        self.assembly_refs.push(assembly_ref.clone());
        Ok(assembly_ref)
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
        let ty = self.import_type(type_full_name)?;
        self.import_method_on(&ty, name, signature)
    }

    /// Like [`Module::import_method`] for a type you already have, such as one from
    /// [`Module::import_type_from`] or a generic instance [`TypeSpec`].
    pub fn import_method_on(&mut self, ty: &TypeDefOrRef, name: &str, signature: MethodSig) -> Result<MemberToken> {
        match ty {
            TypeDefOrRef::TypeDef(def) => def
                .get()
                .methods
                .iter()
                .find(|m| m.get().name == name && m.get().signature == signature)
                .cloned()
                .map(MemberToken::Method)
                .ok_or_else(|| not_found(format!("{}::{} with that signature", ty.full_name(), name))),
            parent => Ok(MemberToken::MemberRef(self.import_member_ref(
                parent.clone(),
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

impl Module {
    fn import_assembly_ref(&mut self, assembly: &Entry<AssemblyRef>) -> Entry<AssemblyRef> {
        let name = assembly.get().name.clone();
        if let Some(existing) = self.find_assembly_ref(&name) {
            return existing;
        }
        let copy = Entry::new(AssemblyRef {
            custom_attributes: Vec::new(),
            ..assembly.get().clone()
        });
        self.assembly_refs.push(copy.clone());
        copy
    }

    fn find_or_add_type_ref(&mut self, scope: ResolutionScope, namespace: &str, name: &str) -> Entry<TypeRef> {
        if let Some(existing) = self.type_refs.iter().find(|t| {
            let t = t.get();
            t.full_name_is(namespace, name) && t.scope.as_ref() == Some(&scope)
        }) {
            return existing.clone();
        }
        let type_ref = Entry::new(TypeRef::new(scope, namespace, name));
        self.type_refs.push(type_ref.clone());
        type_ref
    }

    fn import_foreign_type_ref(&mut self, ty: &TypeDefOrRef, source: &Module) -> Result<Entry<TypeRef>> {
        let unsupported = || HaoError::WriteError(format!("can't import {}", ty.full_name()));
        let (scope, namespace, name) = match ty {
            TypeDefOrRef::TypeDef(def) => {
                let (parent, namespace, name) = {
                    let d = def.get();
                    (d.declaring_type.upgrade(), d.namespace.clone(), d.name.clone())
                };
                let scope = match parent {
                    Some(parent) => ResolutionScope::TypeRef(
                        self.import_foreign_type_ref(&TypeDefOrRef::TypeDef(parent), source)?,
                    ),
                    None => ResolutionScope::AssemblyRef(self.add_reference_to(source)?),
                };
                (scope, namespace, name)
            }
            TypeDefOrRef::TypeRef(r) => {
                let (scope, namespace, name) = {
                    let r = r.get();
                    (r.scope.clone(), r.namespace.clone(), r.name.clone())
                };
                let scope = match scope.ok_or_else(unsupported)? {
                    ResolutionScope::AssemblyRef(a) => ResolutionScope::AssemblyRef(self.import_assembly_ref(&a)),
                    ResolutionScope::TypeRef(parent) => ResolutionScope::TypeRef(
                        self.import_foreign_type_ref(&TypeDefOrRef::TypeRef(parent), source)?,
                    ),
                    ResolutionScope::Module(_) => ResolutionScope::AssemblyRef(self.add_reference_to(source)?),
                    ResolutionScope::ModuleRef(_) => return Err(unsupported()),
                };
                (scope, namespace, name)
            }
            TypeDefOrRef::TypeSpec(_) => return Err(unsupported()),
        };
        Ok(self.find_or_add_type_ref(scope, &namespace, &name))
    }

    /// Imports a type that belongs to `source` so it can be used in this module.
    pub fn import_foreign_type(&mut self, ty: &TypeDefOrRef, source: &Module) -> Result<TypeDefOrRef> {
        match ty {
            TypeDefOrRef::TypeSpec(spec) => {
                let signature = spec.get().signature.clone();
                let spec = Entry::new(TypeSpec {
                    signature: self.import_foreign_signature(&signature, source)?,
                    custom_attributes: Vec::new(),
                });
                self.type_specs.push(spec.clone());
                Ok(TypeDefOrRef::TypeSpec(spec))
            }
            _ => self.import_foreign_type_ref(ty, source).map(TypeDefOrRef::TypeRef),
        }
    }

    /// Imports every type in a signature from `source`.
    pub fn import_foreign_signature(&mut self, sig: &TypeSig, source: &Module) -> Result<TypeSig> {
        let boxed = |t: &TypeSig, this: &mut Self| this.import_foreign_signature(t, source).map(Box::new);
        Ok(match sig {
            TypeSig::Primitive(p) => TypeSig::Primitive(*p),
            TypeSig::ValueType(t) => TypeSig::ValueType(self.import_foreign_type(t, source)?),
            TypeSig::Class(t) => TypeSig::Class(self.import_foreign_type(t, source)?),
            TypeSig::Ptr(t) => TypeSig::Ptr(boxed(t, self)?),
            TypeSig::ByRef(t) => TypeSig::ByRef(boxed(t, self)?),
            TypeSig::Pinned(t) => TypeSig::Pinned(boxed(t, self)?),
            TypeSig::SzArray(t) => TypeSig::SzArray(boxed(t, self)?),
            TypeSig::Array { element, shape } => TypeSig::Array {
                element: boxed(element, self)?,
                shape: shape.clone(),
            },
            TypeSig::GenericInst { generic, args } => TypeSig::GenericInst {
                generic: boxed(generic, self)?,
                args: args
                    .iter()
                    .map(|a| self.import_foreign_signature(a, source))
                    .collect::<Result<_>>()?,
            },
            TypeSig::Var(i) => TypeSig::Var(*i),
            TypeSig::MVar(i) => TypeSig::MVar(*i),
            TypeSig::FnPtr(m) => TypeSig::FnPtr(Box::new(self.import_foreign_method_sig(m, source)?)),
            TypeSig::Modified { required, modifier, ty } => TypeSig::Modified {
                required: *required,
                modifier: self.import_foreign_type(modifier, source)?,
                ty: boxed(ty, self)?,
            },
        })
    }

    pub fn import_foreign_method_sig(&mut self, sig: &MethodSig, source: &Module) -> Result<MethodSig> {
        Ok(MethodSig {
            ret: self.import_foreign_signature(&sig.ret, source)?,
            params: sig
                .params
                .iter()
                .map(|p| self.import_foreign_signature(p, source))
                .collect::<Result<_>>()?,
            varargs: sig
                .varargs
                .iter()
                .map(|p| self.import_foreign_signature(p, source))
                .collect::<Result<_>>()?,
            ..sig.clone()
        })
    }

    /// A reference to `method`, which is defined in `source`.
    pub fn import_foreign_method(&mut self, method: &Entry<Method>, source: &Module) -> Result<MemberToken> {
        let (declaring, name, signature) = {
            let m = method.get();
            (m.declaring_type.upgrade(), m.name.clone(), m.signature.clone())
        };
        let declaring = declaring.ok_or_else(|| HaoError::WriteError(format!("{} has no declaring type", name)))?;
        if !declaring.get().generic_params.is_empty() {
            return Err(HaoError::WriteError(format!(
                "importing methods of generic types is not supported ({})",
                declaring.get().full_name()
            )));
        }
        let parent = self.import_foreign_type(&TypeDefOrRef::TypeDef(declaring), source)?;
        let signature = self.import_foreign_method_sig(&signature, source)?;
        self.import_method_on(&parent, &name, signature)
    }

    /// A reference to `field`, which is defined in `source`.
    pub fn import_foreign_field(&mut self, field: &Entry<Field>, source: &Module) -> Result<MemberToken> {
        let (declaring, name, signature) = {
            let f = field.get();
            (f.declaring_type.upgrade(), f.name.clone(), f.signature.clone())
        };
        let declaring = declaring.ok_or_else(|| HaoError::WriteError(format!("{} has no declaring type", name)))?;
        let parent = self.import_foreign_type(&TypeDefOrRef::TypeDef(declaring), source)?;
        let ty = self.import_foreign_signature(&signature.ty, source)?;
        Ok(MemberToken::MemberRef(self.import_member_ref(
            parent,
            &name,
            MemberRefSig::Field(FieldSig { ty }),
        )))
    }
}
