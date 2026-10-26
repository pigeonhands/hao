//! Builds the thunk assembly: native callable static wrappers around the bound
//! methods, laid out as `hao_runtime::bindings` documents.

use hao::{
    dotnet::{
        entries::{CustomAttributeCtor, Entry, Method, TypeDef, TypeDefOrRef},
        il::{Emitter, MemberToken, OpCode},
        module::{ModuleKind, TargetRuntime},
        signature::{MethodSig, TypeSig},
    },
    error::Result,
    Module,
};

use crate::{
    model::{BoundMethod, MethodKind, Model},
    ty::Ty,
};

/// The helper thunks before the bound methods, `hao_runtime::bindings::helper::COUNT`.
pub const HELPER_COUNT: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    CoreClr,
    Mono,
}

/// Core library members the thunks use.
struct Corlib {
    exception: TypeDefOrRef,
    string: TypeDefOrRef,
    gchandle: TypeSig,
    alloc: MemberToken,
    to_int_ptr: MemberToken,
    from_int_ptr: MemberToken,
    get_target: MemberToken,
    free: MemberToken,
    string_ctor: MemberToken,
    string_length: MemberToken,
    to_char_array: MemberToken,
    marshal_copy: MemberToken,
    to_string: MemberToken,
}

impl Corlib {
    fn import(module: &mut Module) -> Result<Self> {
        const GC_HANDLE: &str = "System.Runtime.InteropServices.GCHandle";
        let gchandle = TypeSig::ValueType(module.corlib_type(GC_HANDLE)?);
        let chars = TypeSig::sz_array(TypeSig::CHAR);
        let (int, nint) = (TypeSig::INT32, TypeSig::NATIVE_INT);
        let r#static = MethodSig::new_static;
        let instance = MethodSig::new_instance;
        let mut import = |ty: &str, name: &str, sig| module.import_corlib_method(ty, name, sig);
        Ok(Self {
            alloc: import(
                GC_HANDLE,
                "Alloc",
                r#static(gchandle.clone(), vec![TypeSig::OBJECT]),
            )?,
            to_int_ptr: import(
                GC_HANDLE,
                "ToIntPtr",
                r#static(nint.clone(), vec![gchandle.clone()]),
            )?,
            from_int_ptr: import(
                GC_HANDLE,
                "FromIntPtr",
                r#static(gchandle.clone(), vec![nint.clone()]),
            )?,
            get_target: import(GC_HANDLE, "get_Target", instance(TypeSig::OBJECT, vec![]))?,
            free: import(GC_HANDLE, "Free", instance(TypeSig::VOID, vec![]))?,
            string_ctor: import(
                "System.String",
                ".ctor",
                instance(
                    TypeSig::VOID,
                    vec![TypeSig::ptr(TypeSig::CHAR), int.clone(), int.clone()],
                ),
            )?,
            string_length: import("System.String", "get_Length", instance(int.clone(), vec![]))?,
            to_char_array: import(
                "System.String",
                "ToCharArray",
                instance(chars.clone(), vec![]),
            )?,
            marshal_copy: import(
                "System.Runtime.InteropServices.Marshal",
                "Copy",
                r#static(TypeSig::VOID, vec![chars, int.clone(), nint, int]),
            )?,
            to_string: import(
                "System.Object",
                "ToString",
                instance(TypeSig::STRING, vec![]),
            )?,
            exception: module.corlib_type("System.Exception")?,
            string: module.corlib_type("System.String")?,
            gchandle,
        })
    }
}

/// Code generation inside one thunk.
struct Thunk<'a> {
    corlib: &'a Corlib,
    e: &'a mut Emitter,
    /// A `GCHandle` local for unwrapping handles.
    gch: u16,
}

impl Thunk<'_> {
    /// Pushes the object behind the handle in argument `arg`, `null` for 0.
    fn load_object(&mut self, arg: u16, cast: Option<&TypeDefOrRef>) {
        let (c, gch) = (self.corlib, self.gch);
        self.e.ldarg(arg).if_else(
            |e| {
                e.ldarg(arg)
                    .call(&c.from_int_ptr)
                    .stloc(gch)
                    .ldloca(gch)
                    .call(&c.get_target);
                if let Some(cast) = cast {
                    e.castclass(cast.clone());
                }
            },
            |e| {
                e.ldnull();
            },
        );
    }

    /// Pushes a new string from the UTF-16 pointer and length in `arg` and `arg + 1`, `null` for a null pointer.
    fn load_string(&mut self, arg: u16) {
        let c = self.corlib;
        self.e.ldarg(arg).if_else(
            |e| {
                e.ldarg(arg).ldc_i4(0).ldarg(arg + 1).newobj(&c.string_ctor);
            },
            |e| {
                e.ldnull();
            },
        );
    }

    /// Replaces the object on the stack with a new handle to it, 0 for `null`.
    fn store_object(&mut self) {
        let c = self.corlib;
        self.e.dup().if_else(
            |e| {
                e.call(&c.alloc).call(&c.to_int_ptr);
            },
            |e| {
                e.pop().ldc_i4(0).op(OpCode::Conv_I);
            },
        );
    }

    /// Loads a parameter of type `ty` starting at argument `arg`, returning how many arguments it used.
    fn load_param(&mut self, ty: &Ty, arg: u16, cast: Option<&TypeDefOrRef>) -> u16 {
        match ty {
            Ty::String => self.load_string(arg),
            Ty::Object { .. } => self.load_object(arg, cast),
            _ => {
                self.e.ldarg(arg);
            }
        }
        ty.native().len() as u16
    }

    /// Writes the result on the stack through the pointer pushed before the call.
    fn store_result(&mut self, ty: &Ty) {
        if ty.returned_as_handle() {
            self.store_object();
        }
        self.e.op(ty.store_op());
    }
}

struct Builder {
    module: Module,
    corlib: Corlib,
    class: Entry<TypeDef>,
    /// `[UnmanagedCallersOnly]` on CoreCLR.
    attribute: Option<CustomAttributeCtor>,
    count: usize,
}

impl Builder {
    fn new(name: &str, target: Target) -> Result<Self> {
        let runtime = match target {
            Target::CoreClr => TargetRuntime::NetCoreImplementation(8),
            Target::Mono => TargetRuntime::NetFramework,
        };
        let mut module = Module::new(name, ModuleKind::Dll, runtime);
        let attribute = match target {
            Target::CoreClr => Some(module.corlib_attribute(
                "System.Runtime.InteropServices.UnmanagedCallersOnlyAttribute",
            )?),
            Target::Mono => None,
        };
        Ok(Self {
            corlib: Corlib::import(&mut module)?,
            class: module.add_static_class("HaoThunks", "Thunks")?,
            module,
            attribute,
            count: 0,
        })
    }

    /// Adds the next thunk, `f{n}`. `body` runs inside a try block; a thrown
    /// exception is returned as a handle through an extra last parameter.
    fn add(&mut self, params: Vec<TypeSig>, body: impl FnOnce(&mut Thunk)) -> Result<()> {
        let exception_arg = params.len() as u16;
        let mut e = Emitter::new();
        let gch = e.declare_local(self.corlib.gchandle.clone());
        let exception = e.declare_local(TypeSig::OBJECT);
        let corlib = &self.corlib;
        e.try_catch(
            corlib.exception.clone(),
            |e| body(&mut Thunk { corlib, e, gch }),
            |e| {
                e.stloc(exception).ldarg(exception_arg).ldloc(exception);
                e.call(&corlib.alloc)
                    .call(&corlib.to_int_ptr)
                    .op(OpCode::Stind_I);
            },
        );
        e.ret();

        let params = params.into_iter().chain([TypeSig::NATIVE_INT]).collect();
        let mut method = Method::public_static(
            format!("f{}", self.count),
            MethodSig::new_static(TypeSig::VOID, params),
        )
        .with_body(e.finish());
        if let Some(attribute) = &self.attribute {
            method = method.with_attribute(attribute.clone());
        }
        TypeDef::add_method(&self.class, method);
        self.count += 1;
        Ok(())
    }

    /// The helpers listed in `hao_runtime::bindings::helper`.
    fn add_helpers(&mut self) -> Result<()> {
        let nint = TypeSig::NATIVE_INT;
        let string = self.corlib.string.clone();
        // free(handle)
        self.add(vec![nint.clone()], |t| {
            let c = t.corlib;
            t.e.ldarg(0)
                .call(&c.from_int_ptr)
                .stloc(t.gch)
                .ldloca(t.gch)
                .call(&c.free);
        })?;
        // string_length(handle, *mut i32)
        self.add(vec![nint.clone(), nint.clone()], |t| {
            t.e.ldarg(1);
            t.load_object(0, Some(&string));
            t.e.callvirt(&t.corlib.string_length).op(OpCode::Stind_I4);
        })?;
        // string_copy(handle, *mut u16)
        self.add(vec![nint.clone(), nint.clone()], |t| {
            t.load_object(0, Some(&string));
            let length = t.e.declare_local(TypeSig::INT32);
            let c = t.corlib;
            t.e.callvirt(&c.to_char_array)
                .dup()
                .op(OpCode::Ldlen)
                .op(OpCode::Conv_I4)
                .stloc(length);
            t.e.ldc_i4(0).ldarg(1).ldloc(length).call(&c.marshal_copy);
        })?;
        // to_string(handle, *mut handle)
        self.add(vec![nint.clone(), nint], |t| {
            t.e.ldarg(1);
            t.load_object(0, None);
            t.e.callvirt(&t.corlib.to_string);
            t.store_result(&Ty::String);
        })?;
        assert_eq!(self.count, HELPER_COUNT);
        Ok(())
    }

    fn add_method(
        &mut self,
        library: &Module,
        declaring: &TypeDefOrRef,
        method: &BoundMethod,
    ) -> Result<()> {
        let target = self.module.import_foreign_method(&method.method, library)?;
        let mut casts = Vec::new();
        for (_, ty) in &method.params {
            casts.push(match ty {
                Ty::Object { ty: Some(t), .. } if !t.full_name_is("System", "Object") => {
                    Some(self.module.import_foreign_type(t, library)?)
                }
                _ => None,
            });
        }

        let instance = method.kind == MethodKind::Instance;
        let mut native = Vec::new();
        if instance {
            native.push(TypeSig::NATIVE_INT);
        }
        native.extend(method.params.iter().flat_map(|(_, ty)| ty.native()));
        let result_arg = native.len() as u16;
        if method.returns() {
            native.push(TypeSig::NATIVE_INT);
        }

        self.add(native, |t| {
            if method.returns() {
                t.e.ldarg(result_arg);
            }
            let mut arg = 0;
            if instance {
                t.load_object(0, Some(declaring));
                arg = 1;
            }
            for ((_, ty), cast) in method.params.iter().zip(&casts) {
                arg += t.load_param(ty, arg, cast.as_ref());
            }
            match method.kind {
                MethodKind::Constructor => {
                    t.e.newobj(&target);
                    t.store_result(&Ty::Object {
                        ty: None,
                        bound: None,
                    });
                }
                MethodKind::Static => {
                    t.e.call(&target);
                }
                MethodKind::Instance => {
                    t.e.callvirt(&target);
                }
            }
            if method.kind != MethodKind::Constructor && method.ret != Ty::Void {
                t.store_result(&method.ret);
            }
        })
    }
}

pub fn build(library: &Module, model: &Model, name: &str, target: Target) -> Result<Vec<u8>> {
    let mut builder = Builder::new(name, target)?;
    builder.add_helpers()?;
    for bound in &model.types {
        let declaring = builder
            .module
            .import_foreign_type(&TypeDefOrRef::TypeDef(bound.ty.clone()), library)?;
        for method in &bound.methods {
            builder.add_method(library, &declaring, method)?;
        }
    }
    builder.module.write()
}
