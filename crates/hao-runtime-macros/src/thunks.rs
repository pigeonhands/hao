//! Builds the thunk assembly: static native callable wrappers around the bound
//! methods, using the layout documented in `hao_runtime::bindings`.

use hao::{
    dotnet::{
        entries::{CustomAttribute, CustomAttributeCtor, CustomAttributeValue, Method, TypeDef, TypeDefOrRef},
        il::{Emitter, Instruction, MemberToken, OpCode, Operand},
        md::streams::tables_stream::{MethodFlags, TypeAttributes},
        module::{ModuleKind, TargetRuntime},
        signature::{MethodSig, PrimitiveType, TypeSig},
    },
    error::Result,
    Module,
};

use crate::model::{MethodKind, Model, Ty};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    CoreClr,
    Mono,
}

struct Ctx {
    module: Module,
    target: Target,
    uco: Option<MemberToken>,
    gchandle: TypeSig,
    exception: TypeDefOrRef,
    string: TypeDefOrRef,
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

const NATIVE_INT: TypeSig = TypeSig::Primitive(PrimitiveType::I);

fn native_ty(ty: &Ty) -> Vec<TypeSig> {
    match ty {
        Ty::Void => vec![],
        Ty::Bool => vec![TypeSig::Primitive(PrimitiveType::U1)],
        Ty::Primitive(PrimitiveType::Char) => vec![TypeSig::Primitive(PrimitiveType::U2)],
        Ty::Primitive(p) => vec![TypeSig::Primitive(*p)],
        Ty::String => vec![NATIVE_INT, TypeSig::INT32],
        Ty::Object { .. } => vec![NATIVE_INT],
    }
}

fn store_op(ty: &Ty) -> OpCode {
    match ty {
        Ty::Bool => OpCode::Stind_I1,
        Ty::Primitive(p) => match p {
            PrimitiveType::I1 | PrimitiveType::U1 => OpCode::Stind_I1,
            PrimitiveType::I2 | PrimitiveType::U2 | PrimitiveType::Char => OpCode::Stind_I2,
            PrimitiveType::I4 | PrimitiveType::U4 => OpCode::Stind_I4,
            PrimitiveType::I8 | PrimitiveType::U8 => OpCode::Stind_I8,
            PrimitiveType::R4 => OpCode::Stind_R4,
            PrimitiveType::R8 => OpCode::Stind_R8,
            _ => OpCode::Stind_I,
        },
        _ => OpCode::Stind_I,
    }
}

impl Ctx {
    fn new(name: &str, target: Target) -> Result<Self> {
        let runtime = match target {
            Target::CoreClr => TargetRuntime::NetCoreImplementation(8),
            Target::Mono => TargetRuntime::NetFramework,
        };
        let mut module = Module::new(name, ModuleKind::Dll, runtime);
        let instance = |ret, params| MethodSig::new_instance(ret, params);
        let r#static = |ret, params| MethodSig::new_static(ret, params);

        let gchandle_type = module.corlib_type("System.Runtime.InteropServices.GCHandle")?;
        let gchandle = TypeSig::ValueType(gchandle_type.clone());
        let marshal = module.corlib_type("System.Runtime.InteropServices.Marshal")?;
        let string = module.corlib_type("System.String")?;
        let object = module.corlib_type("System.Object")?;
        let exception = module.corlib_type("System.Exception")?;
        let chars = TypeSig::sz_array(TypeSig::CHAR);

        let uco = match target {
            Target::CoreClr => {
                let attribute = module.corlib_type("System.Runtime.InteropServices.UnmanagedCallersOnlyAttribute")?;
                Some(module.import_method_on(&attribute, ".ctor", instance(TypeSig::VOID, vec![]))?)
            }
            Target::Mono => None,
        };

        Ok(Self {
            alloc: module.import_method_on(&gchandle_type, "Alloc", r#static(gchandle.clone(), vec![TypeSig::OBJECT]))?,
            to_int_ptr: module.import_method_on(&gchandle_type, "ToIntPtr", r#static(NATIVE_INT, vec![gchandle.clone()]))?,
            from_int_ptr: module.import_method_on(&gchandle_type, "FromIntPtr", r#static(gchandle.clone(), vec![NATIVE_INT]))?,
            get_target: module.import_method_on(&gchandle_type, "get_Target", instance(TypeSig::OBJECT, vec![]))?,
            free: module.import_method_on(&gchandle_type, "Free", instance(TypeSig::VOID, vec![]))?,
            string_ctor: module.import_method_on(
                &string,
                ".ctor",
                instance(TypeSig::VOID, vec![TypeSig::Ptr(Box::new(TypeSig::CHAR)), TypeSig::INT32, TypeSig::INT32]),
            )?,
            string_length: module.import_method_on(&string, "get_Length", instance(TypeSig::INT32, vec![]))?,
            to_char_array: module.import_method_on(&string, "ToCharArray", instance(chars.clone(), vec![]))?,
            marshal_copy: module.import_method_on(
                &marshal,
                "Copy",
                r#static(TypeSig::VOID, vec![chars, TypeSig::INT32, NATIVE_INT, TypeSig::INT32]),
            )?,
            to_string: module.import_method_on(&object, "ToString", instance(TypeSig::STRING, vec![]))?,
            uco,
            gchandle,
            exception,
            string,
            module,
            target,
        })
    }

    /// Converts the handle in argument `arg` to its object, or `null` for 0.
    fn load_object(&self, e: &mut Emitter, arg: u16, gch: u16, cast: Option<&TypeDefOrRef>) {
        let not_null = e.label();
        let done = e.label();
        e.emit(Instruction::ldarg(arg));
        e.branch(OpCode::Brtrue, not_null);
        e.op(OpCode::Ldnull);
        e.branch(OpCode::Br, done);
        e.mark(not_null);
        e.emit(Instruction::ldarg(arg));
        e.emit(Instruction::call(self.from_int_ptr.clone()));
        e.emit(Instruction::stloc(gch));
        e.emit(Instruction::new(OpCode::Ldloca_S, Operand::Local(gch)));
        e.emit(Instruction::call(self.get_target.clone()));
        if let Some(cast) = cast {
            e.emit(Instruction::new(OpCode::Castclass, Operand::Token(cast.clone().into())));
        }
        e.mark(done);
    }

    /// Turns the object on the stack into a handle, 0 for `null`.
    fn store_object(&self, e: &mut Emitter) {
        let not_null = e.label();
        let done = e.label();
        e.op(OpCode::Dup);
        e.branch(OpCode::Brtrue, not_null);
        e.op(OpCode::Pop);
        e.emit(Instruction::ldc_i4(0));
        e.op(OpCode::Conv_I);
        e.branch(OpCode::Br, done);
        e.mark(not_null);
        e.emit(Instruction::call(self.alloc.clone()));
        e.emit(Instruction::call(self.to_int_ptr.clone()));
        e.mark(done);
    }

    /// Adds `f{index}` to the thunk type. `body` emits the code inside the try
    /// block; exceptions are stored through the last argument.
    fn add_thunk(
        &mut self,
        thunks: &hao::dotnet::entries::Entry<TypeDef>,
        index: usize,
        params: Vec<TypeSig>,
        body: impl FnOnce(&Self, &mut Emitter, u16),
    ) -> Result<()> {
        let exception_arg = params.len() as u16;
        let mut all = params;
        all.push(NATIVE_INT);

        let mut e = Emitter::new();
        let gch = e.declare_local(self.gchandle.clone());
        let ex = e.declare_local(TypeSig::OBJECT);
        let (try_start, try_end, handler_end) = (e.label(), e.label(), e.label());
        e.mark(try_start);
        body(self, &mut e, gch);
        e.branch(OpCode::Leave, handler_end);
        e.mark(try_end);
        e.emit(Instruction::stloc(ex));
        e.emit(Instruction::ldarg(exception_arg));
        e.emit(Instruction::ldloc(ex));
        e.emit(Instruction::call(self.alloc.clone()));
        e.emit(Instruction::call(self.to_int_ptr.clone()));
        e.op(OpCode::Stind_I);
        e.branch(OpCode::Leave, handler_end);
        e.mark(handler_end);
        e.emit(Instruction::ret());
        e.exception_handler(Some(self.exception.clone()), try_start, try_end, try_end, handler_end);

        let mut method = Method::new(
            MethodFlags::Public | MethodFlags::Static | MethodFlags::HideBySig,
            format!("f{}", index),
            MethodSig::new_static(TypeSig::VOID, all),
        );
        method.body = Some(e.finish());
        if let Some(MemberToken::MemberRef(ctor)) = &self.uco {
            method.custom_attributes.push(CustomAttribute::new(
                CustomAttributeCtor::MemberRef(ctor.clone()),
                &CustomAttributeValue::default(),
            )?);
        }
        TypeDef::add_method(thunks, method);
        Ok(())
    }
}

pub fn build(library: &Module, model: &Model, name: &str, target: Target) -> Result<Vec<u8>> {
    let mut ctx = Ctx::new(name, target)?;
    let object = ctx.module.corlib_type("System.Object")?;
    let thunks = ctx.module.add_type(TypeDef::new(
        TypeAttributes::Public | TypeAttributes::Abstract | TypeAttributes::Sealed,
        "HaoThunks",
        "Thunks",
        Some(object),
    ));
    let string = ctx.string.clone();

    ctx.add_thunk(&thunks, 0, vec![NATIVE_INT], |c, e, gch| {
        e.emit(Instruction::ldarg(0));
        e.emit(Instruction::call(c.from_int_ptr.clone()));
        e.emit(Instruction::stloc(gch));
        e.emit(Instruction::new(OpCode::Ldloca_S, Operand::Local(gch)));
        e.emit(Instruction::call(c.free.clone()));
    })?;
    ctx.add_thunk(&thunks, 1, vec![NATIVE_INT, NATIVE_INT], |c, e, gch| {
        e.emit(Instruction::ldarg(1));
        c.load_object(e, 0, gch, Some(&string));
        e.emit(Instruction::callvirt(c.string_length.clone()));
        e.op(OpCode::Stind_I4);
    })?;
    ctx.add_thunk(&thunks, 2, vec![NATIVE_INT, NATIVE_INT], |c, e, gch| {
        c.load_object(e, 0, gch, Some(&string));
        e.emit(Instruction::callvirt(c.to_char_array.clone()));
        e.op(OpCode::Dup);
        e.op(OpCode::Ldlen);
        e.op(OpCode::Conv_I4);
        let length = e.declare_local(TypeSig::INT32);
        e.emit(Instruction::stloc(length));
        e.emit(Instruction::ldc_i4(0));
        e.emit(Instruction::ldarg(1));
        e.emit(Instruction::ldloc(length));
        e.emit(Instruction::call(c.marshal_copy.clone()));
    })?;
    ctx.add_thunk(&thunks, 3, vec![NATIVE_INT, NATIVE_INT], |c, e, gch| {
        e.emit(Instruction::ldarg(1));
        c.load_object(e, 0, gch, None);
        e.emit(Instruction::callvirt(c.to_string.clone()));
        c.store_object(e);
        e.op(OpCode::Stind_I);
    })?;

    for bound_type in &model.types {
        let declaring = ctx
            .module
            .import_foreign_type(&TypeDefOrRef::TypeDef(bound_type.ty.clone()), library)?;
        for method in &bound_type.methods {
            let target_method = ctx.module.import_foreign_method(&method.method, library)?;
            let casts = method
                .params
                .iter()
                .map(|(_, ty)| match ty {
                    Ty::Object { ty: Some(t), .. } if !t.full_name_is("System", "Object") => {
                        ctx.module.import_foreign_type(t, library).map(Some)
                    }
                    _ => Ok(None),
                })
                .collect::<Result<Vec<_>>>()?;

            let mut native = Vec::new();
            if method.kind == MethodKind::Instance {
                native.push(NATIVE_INT);
            }
            for (_, ty) in &method.params {
                native.extend(native_ty(ty));
            }
            let returns = method.kind == MethodKind::Constructor || method.ret != Ty::Void;
            let ret_arg = native.len() as u16;
            if returns {
                native.push(NATIVE_INT);
            }

            let declaring = declaring.clone();
            ctx.add_thunk(&thunks, method.thunk, native, |c, e, gch| {
                if returns {
                    e.emit(Instruction::ldarg(ret_arg));
                }
                let mut arg = 0;
                if method.kind == MethodKind::Instance {
                    c.load_object(e, 0, gch, Some(&declaring));
                    arg = 1;
                }
                for ((_, ty), cast) in method.params.iter().zip(&casts) {
                    match ty {
                        Ty::String => {
                            let not_null = e.label();
                            let done = e.label();
                            e.emit(Instruction::ldarg(arg));
                            e.branch(OpCode::Brtrue, not_null);
                            e.op(OpCode::Ldnull);
                            e.branch(OpCode::Br, done);
                            e.mark(not_null);
                            e.emit(Instruction::ldarg(arg));
                            e.emit(Instruction::ldc_i4(0));
                            e.emit(Instruction::ldarg(arg + 1));
                            e.emit(Instruction::newobj(c.string_ctor.clone()));
                            e.mark(done);
                            arg += 2;
                        }
                        Ty::Object { .. } => {
                            c.load_object(e, arg, gch, cast.as_ref());
                            arg += 1;
                        }
                        _ => {
                            e.emit(Instruction::ldarg(arg));
                            arg += 1;
                        }
                    }
                }
                match method.kind {
                    MethodKind::Constructor => {
                        e.emit(Instruction::newobj(target_method.clone()));
                        c.store_object(e);
                        e.op(OpCode::Stind_I);
                    }
                    MethodKind::Static => {
                        e.emit(Instruction::call(target_method.clone()));
                    }
                    MethodKind::Instance => {
                        e.emit(Instruction::callvirt(target_method.clone()));
                    }
                }
                if method.kind != MethodKind::Constructor && returns {
                    match &method.ret {
                        Ty::String | Ty::Object { .. } => {
                            c.store_object(e);
                            e.op(OpCode::Stind_I);
                        }
                        ty => {
                            e.op(store_op(ty));
                        }
                    }
                }
            })?;
        }
    }

    let _ = ctx.target;
    ctx.module.write()
}
