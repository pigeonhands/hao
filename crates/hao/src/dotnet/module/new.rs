use pewter::pe::sections::cor20::{Cor20Flags, ImageCor20Header};

use super::Module;
use crate::dotnet::{
    entries::*,
    md::streams::tables_stream::{AssemblyFlags, TypeAttributes},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "reflect", derive(facet::Facet), repr(C))]
pub enum ModuleKind {
    #[default]
    Dll,
    Console,
    Windows,
}

/// The core library new modules reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetRuntime {
    /// .NET (Core) 5+, referencing `System.Runtime` with the given major version.
    NetCore(u16),
    /// .NET Framework 4.x and Mono (including Unity), referencing `mscorlib`.
    NetFramework,
    /// .NET (Core) referencing the implementation assembly `System.Private.CoreLib`,
    /// where every core type lives. Only suitable for code that runs on the
    /// runtime it is generated for, as the reference ties it to that version.
    NetCoreImplementation(u16),
}

impl TargetRuntime {
    fn corlib(self) -> AssemblyRef {
        let (name, version, token) = match self {
            Self::NetCore(major) => (
                "System.Runtime",
                Version {
                    major,
                    ..Default::default()
                },
                [0xb0, 0x3f, 0x5f, 0x7f, 0x11, 0xd5, 0x0a, 0x3a],
            ),
            Self::NetCoreImplementation(major) => (
                "System.Private.CoreLib",
                Version {
                    major,
                    ..Default::default()
                },
                [0x7c, 0xec, 0x85, 0xd7, 0xbe, 0xa7, 0x79, 0x8e],
            ),
            Self::NetFramework => (
                "mscorlib",
                Version {
                    major: 4,
                    ..Default::default()
                },
                [0xb7, 0x7a, 0x5c, 0x56, 0x19, 0x34, 0xe0, 0x89],
            ),
        };
        AssemblyRef {
            name: name.into(),
            version,
            public_key_or_token: token.to_vec(),
            ..Default::default()
        }
    }

    /// The public key token framework assemblies other than the core library use.
    pub fn framework_token(self) -> [u8; 8] {
        [0xb0, 0x3f, 0x5f, 0x7f, 0x11, 0xd5, 0x0a, 0x3a]
    }
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
fn new_mvid(_name: &str) -> uuid::Uuid {
    uuid::Uuid::new_v4()
}

/// Browser wasm has no randomness, so derive the id from the name (FNV-1a).
/// Set `module.module.get_mut().mvid` if it has to be unique.
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
fn new_mvid(name: &str) -> uuid::Uuid {
    let mut hash: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
    for byte in name.bytes() {
        hash ^= byte as u128;
        hash = hash.wrapping_mul(0x0000_0000_0100_0000_0000_0000_0000_013B);
    }
    uuid::Builder::from_random_bytes(hash.to_le_bytes()).into_uuid()
}

impl Module {
    /// A new, empty assembly. `name` is the assembly name without an extension.
    ///
    /// ```
    /// use hao::{Module, dotnet::module::{ModuleKind, TargetRuntime}};
    ///
    /// let module = Module::new("Generated", ModuleKind::Dll, TargetRuntime::NetCore(9));
    /// assert_eq!(module.name(), "Generated.dll");
    /// ```
    pub fn new(name: &str, kind: ModuleKind, runtime: TargetRuntime) -> Self {
        let extension = if kind == ModuleKind::Dll {
            "dll"
        } else {
            "exe"
        };
        let global_type = TypeDef {
            flags: TypeAttributes::empty(),
            name: "<Module>".into(),
            ..Default::default()
        };
        let mut module = Self::default();
        module.module = Entry::new(ModuleDef {
            name: format!("{}.{}", name, extension),
            mvid: new_mvid(name),
            ..Default::default()
        });
        module.assembly = Some(Entry::new(Assembly {
            hash_alg_id: 0x8004,
            version: Version {
                major: 1,
                ..Default::default()
            },
            flags: AssemblyFlags::empty(),
            name: name.into(),
            ..Default::default()
        }));
        module.types = vec![Entry::new(global_type)];
        module.assembly_refs = vec![Entry::new(runtime.corlib())];
        module.runtime_version = "v4.0.30319".into();
        module.kind = kind;
        module.cor20 = ImageCor20Header {
            major_rt_version: 2,
            minor_rt_version: 5,
            flags: Cor20Flags::IL_ONLY,
            ..Default::default()
        };
        module
    }

    /// The reference to the core library (`System.Runtime`, `mscorlib`, ...).
    pub fn corlib(&self) -> Option<Entry<AssemblyRef>> {
        self.assembly_refs
            .iter()
            .find(|a| a.get().is_corlib())
            .cloned()
    }

    /// A type from the core library, e.g. `System.Object`.
    pub fn corlib_type(&mut self, full_name: &str) -> crate::error::Result<TypeDefOrRef> {
        let corlib = self.corlib().ok_or(crate::error::HaoError::BadImageFormat(
            "module has no core library reference",
        ))?;
        let name = corlib.get().name.clone();
        self.import_type_from(&name, full_name)
    }

    /// A method of a core library type, e.g. `System.Math::Max`.
    pub fn import_corlib_method(
        &mut self,
        type_full_name: &str,
        name: &str,
        signature: crate::dotnet::signature::MethodSig,
    ) -> crate::error::Result<crate::dotnet::il::MemberToken> {
        let ty = self.corlib_type(type_full_name)?;
        self.import_method_on(&ty, name, signature)
    }

    /// The parameterless constructor of a core library attribute, for
    /// [`Method::with_attribute`](crate::dotnet::entries::Method::with_attribute), e.g.
    /// `System.Runtime.InteropServices.UnmanagedCallersOnlyAttribute`.
    pub fn corlib_attribute(
        &mut self,
        full_name: &str,
    ) -> crate::error::Result<crate::dotnet::entries::CustomAttributeCtor> {
        use crate::dotnet::signature::{MethodSig, TypeSig};
        let ctor = self.import_corlib_method(
            full_name,
            ".ctor",
            MethodSig::new_instance(TypeSig::VOID, vec![]),
        )?;
        ctor.try_into().map_err(|_| {
            crate::error::HaoError::BadImageFormat("attribute constructor is not a method")
        })
    }

    /// Adds a new top level type.
    pub fn add_type(&mut self, ty: TypeDef) -> Entry<TypeDef> {
        let ty = Entry::new(ty);
        self.types.push(ty.clone());
        ty
    }

    /// Adds a `public static class`.
    pub fn add_static_class(
        &mut self,
        namespace: &str,
        name: &str,
    ) -> crate::error::Result<Entry<TypeDef>> {
        let object = self.corlib_type("System.Object")?;
        Ok(self.add_type(TypeDef::new_static_class(namespace, name, object)))
    }

    /// Renames the assembly, e.g. to load an edited copy next to the original.
    pub fn set_assembly_name(&mut self, name: &str) {
        if let Some(assembly) = &self.assembly {
            assembly.get_mut().name = name.to_string();
        }
        self.module.get_mut().name = format!("{}.dll", name);
    }
}
