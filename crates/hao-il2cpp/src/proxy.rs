//! Generates "runtime proxy" assemblies from Il2Cpp metadata.
//!
//! A declaration assembly (see [`crate::generate`](mod@crate::generate)) rebuilds every type and
//! method *signature* of an Il2Cpp game, but its method bodies just `throw`. A
//! *proxy* assembly is the same declaration assembly with eligible method bodies
//! rewritten to route through a per-module invoke seam keyed by each method's
//! `(image, token)` identity.
//!
//! These are **reference artifacts** (browse in ILSpy, compile against), not run
//! in a managed host. Calling the game at runtime is done from Rust:
//! `hao-il2cpp-runtime` resolves the same `(image, token)` to a live `MethodInfo`
//! with `Il2Cpp::method` and calls or hooks it. A managed C#-consumer stack à la
//! Il2CppInterop is intentionally out of scope.
//!
//! # The il2cpp method ABI (research notes)
//!
//! il2cpp compiles each managed method to a native C function with an extra,
//! trailing hidden parameter: `const MethodInfo*`. For an instance method the
//! first parameter is `this` (an `Il2CppObject*`); value types are passed and
//! returned unboxed, reference types (and `string`) as pointers. So the raw
//! native signature of `int Foo(string s)` on a class is roughly
//! `int32_t Foo(Il2CppObject* this, Il2CppString* s, const MethodInfo* method)`.
//!
//! There are two common ways to invoke a method:
//!
//! 1. **Raw native pointer** — call the method's compiled code directly (its
//!    address is [`crate::game::MethodView::code_address`]), passing `this`, the
//!    arguments, and the `MethodInfo*`. Fast, but the C signature and the
//!    `MethodInfo*` must both be exactly right.
//! 2. **`il2cpp_runtime_invoke`** — `Il2CppObject* il2cpp_runtime_invoke(const
//!    MethodInfo* method, void* obj, void** params, Il2CppException** exc)`. It
//!    boxes/unboxes arguments and the return value for you; slower but forgiving.
//!
//! Crucially, the `MethodInfo*` (and the relocated code address under ASLR) are
//! only known at **runtime**, inside the loaded game. A proxy body therefore
//! cannot bake them in; it must resolve the method at runtime from stable
//! identity data. The stable identity we have at generation time is the method's
//! metadata **token** and its **image** (assembly) name, which
//! [`crate::binary::Binary::method_pointer`] already uses to find native code.
//!
//! # Invocation strategy emitted here
//!
//! Every proxy body funnels through one synthetic seam method per module:
//!
//! ```text
//! namespace Hao {
//!     public static class <Il2CppProxyRuntime> {
//!         // A throwing stub by design: these are reference artifacts, not run
//!         // in a managed host. Execution is the Rust runtime's job (see below).
//!         public static object Invoke(string image, uint token,
//!                                     object instance, object[] args);
//!     }
//! }
//! ```
//!
//! A proxy body boxes its value-type arguments into an `object[]`, loads `this`
//! (or `null` for a static method), and calls `Invoke(image, token, this, args)`,
//! then casts/unboxes the returned `object` back to the method's return type. The
//! `(image, token)` pair is carried as plain IL data — exactly the identity the
//! runtime needs to find the `MethodInfo*`.
//!
//! # Status and scope
//!
//! Generation produces **structurally valid IL** that round-trips (generate ->
//! write PE -> reload with hao), tested below. In scope are static and instance
//! methods whose parameters and return type are primitives, `string`, or
//! reference objects; left as throwing stubs: generics, by-value structs/enums,
//! `ref`/`out`, pointers.
//!
//! The seam's `Invoke` body is a throwing stub **by design** — these assemblies
//! are reference artifacts, not executed in a managed host. To call the game,
//! take a method's `(image, token)` and resolve it to a live `MethodInfo` with
//! `hao-il2cpp-runtime`'s `Il2Cpp::method(image, token)`, then `.call(..)` or
//! `.hook(..)`. A managed host à la Il2CppInterop is intentionally out of scope.

use hao::dotnet::{
    entries::{Entry, Method, TypeDef, TypeDefOrRef, TypeSpec},
    il::{Emitter, Instruction, MethodBody, OpCode, Operand},
    md::streams::tables_stream::MethodFlags,
    signature::{MethodSig, PrimitiveType, TypeSig},
};

use crate::{
    binary::Binary,
    error::Result,
    generate::{generate_inner, GeneratedAssembly, Generator},
    metadata::Metadata,
};

/// The namespace, type and method names of the per-module invoke seam. The
/// runtime (`hao-il2cpp-runtime`) locates `Invoke` by these names to patch in the
/// real native dispatch.
pub const SEAM_NAMESPACE: &str = "Hao";
pub const SEAM_TYPE: &str = "<Il2CppProxyRuntime>";
pub const SEAM_METHOD: &str = "Invoke";

/// The ECMA element byte for `System.Object`.
const ELEMENT_OBJECT: u8 = 0x1C;
/// The ECMA element byte for `System.String`.
const ELEMENT_STRING: u8 = 0x0E;

/// Builds a proxy assembly for every image in the game: a declaration assembly
/// whose eligible method bodies call into the game's native code through a
/// runtime invoke seam instead of throwing. See the module docs for the ABI and
/// the (partial) status.
pub fn generate_proxies(metadata: &Metadata, binary: &Binary) -> Result<Vec<GeneratedAssembly>> {
    generate_inner(metadata, binary, true)
}

/// How a single argument or return value crosses the `object`-typed seam.
enum Slot {
    /// A reference-typed value (`string`, `object`, a class, or an array): already
    /// compatible with `object`, so pushed and stored as-is.
    Reference,
    /// A value-type primitive: boxed into `object` on the way in, unboxed on the
    /// way out, using this primitive's type token.
    Primitive(PrimitiveType),
}

/// What the proxy does with the `object` the seam returns.
enum ReturnPlan {
    /// `void`: discard the returned `object`.
    Void,
    /// A value-type primitive: `unbox.any` to this type.
    Primitive(PrimitiveType),
    /// A reference type: `castclass` to this type, or no cast when it is `object`.
    Reference(Option<TypeDefOrRef>),
}

/// Whether a primitive is a value type carried through `object` by boxing
/// (everything except `void`, `string`, `object` and `TypedReference`).
fn is_value_primitive(p: PrimitiveType) -> bool {
    use PrimitiveType::*;
    matches!(
        p,
        Boolean | Char | I1 | U1 | I2 | U2 | I4 | U4 | I8 | U8 | R4 | R8 | I | U
    )
}

/// Classifies an argument or (non-void) return type, or `None` when it is out of
/// scope for proxying (structs/enums by value, `ref`/`out`, pointers, generics).
fn classify(sig: &TypeSig) -> Option<Slot> {
    match sig.unmodified() {
        TypeSig::Primitive(p) if is_value_primitive(*p) => Some(Slot::Primitive(*p)),
        TypeSig::Primitive(PrimitiveType::String | PrimitiveType::Object) => Some(Slot::Reference),
        TypeSig::Class(_) | TypeSig::SzArray(_) | TypeSig::Array { .. } => Some(Slot::Reference),
        _ => None,
    }
}

impl<'a> Generator<'a> {
    /// A reference to `System.Object` from `module`, or `None` if the game has no
    /// `System.Object` (then the module is left as declarations).
    fn object_ref(&mut self, module: usize) -> Option<TypeDefOrRef> {
        let index = *self.primitive_types.get(&ELEMENT_OBJECT)?;
        Some(self.type_ref(module, index))
    }

    /// A reference to the type backing ECMA `element` (a primitive), from `module`.
    fn primitive_ref(&mut self, module: usize, element: u8) -> Option<TypeDefOrRef> {
        let index = *self.primitive_types.get(&element)?;
        Some(self.type_ref(module, index))
    }

    /// The `castclass` target for a reference return type: `None` means `object`
    /// (no cast). Returns `None` for the whole option only if the type is not a
    /// supported reference type.
    fn return_cast(&mut self, module: usize, sig: &TypeSig) -> Option<Option<TypeDefOrRef>> {
        match sig.unmodified() {
            TypeSig::Primitive(PrimitiveType::Object) => Some(None),
            TypeSig::Primitive(PrimitiveType::String) => {
                Some(Some(self.primitive_ref(module, ELEMENT_STRING)?))
            }
            TypeSig::Class(t) => Some(Some(t.clone())),
            TypeSig::SzArray(_) | TypeSig::Array { .. } => Some(Some(TypeDefOrRef::TypeSpec(
                Entry::new(TypeSpec {
                    signature: sig.clone(),
                    custom_attributes: Vec::new(),
                }),
            ))),
            _ => None,
        }
    }

    /// Adds the `Hao.<Il2CppProxyRuntime>` seam type to `module` and returns its
    /// `Invoke` method. The body is a throwing stub for now; the runtime replaces
    /// it with native dispatch.
    fn add_seam(&mut self, module: usize) -> Option<Entry<Method>> {
        let object = self.object_ref(module)?;
        let ty = Entry::new(TypeDef::new_static_class(SEAM_NAMESPACE, SEAM_TYPE, object));
        let signature = MethodSig::new_static(
            TypeSig::OBJECT,
            vec![
                TypeSig::STRING,                           // image name
                TypeSig::Primitive(PrimitiveType::U4),     // metadata token
                TypeSig::OBJECT,                           // instance (null if static)
                TypeSig::sz_array(TypeSig::OBJECT),        // boxed arguments
            ],
        );
        let mut method = Method::public_static(SEAM_METHOD, signature);
        method.body = Some(MethodBody {
            instructions: vec![
                Instruction::simple(OpCode::Ldnull),
                Instruction::simple(OpCode::Throw),
            ],
            ..Default::default()
        });
        let method = TypeDef::add_method(&ty, method);
        self.modules[module].types.push(ty);
        Some(method)
    }

    /// Builds the proxy body for one method, or `None` to leave its throwing stub
    /// (abstract/internal-call/generic methods, out-of-scope signatures, or
    /// methods with no native code).
    fn proxy_body(
        &mut self,
        module: usize,
        method: &Entry<Method>,
        image_name: &str,
        token: u32,
        seam: &Entry<Method>,
    ) -> Option<MethodBody> {
        let (signature, is_static, has_body, is_generic) = {
            let m = method.get();
            (
                m.signature.clone(),
                m.flags.contains(MethodFlags::Static),
                m.body.is_some(),
                !m.generic_params.is_empty(),
            )
        };
        // Only rewrite methods that already have a (throwing) body, and skip
        // generic methods (open generics have no single native entry point).
        if !has_body || is_generic {
            return None;
        }
        // The method must have real native code to call into.
        self.binary.method_pointer(image_name, token)?;

        // Plan the return first, so unsupported returns bail before we allocate.
        let return_plan = match signature.ret.unmodified() {
            TypeSig::Primitive(PrimitiveType::Void) => ReturnPlan::Void,
            TypeSig::Primitive(p) if is_value_primitive(*p) => ReturnPlan::Primitive(*p),
            _ => ReturnPlan::Reference(self.return_cast(module, &signature.ret)?),
        };

        // Plan each argument: `None` for a reference, or the box type token.
        let mut arg_box: Vec<Option<TypeDefOrRef>> = Vec::with_capacity(signature.params.len());
        for param in &signature.params {
            match classify(param)? {
                Slot::Reference => arg_box.push(None),
                Slot::Primitive(p) => arg_box.push(Some(self.primitive_ref(module, p as u8)?)),
            }
        }
        let object_ref = self.object_ref(module)?;

        let mut e = Emitter::new();
        // Invoke(image, token, instance, args)
        e.ldstr(image_name);
        e.ldc_i4(token as i32);
        if is_static {
            e.ldnull();
        } else {
            e.ldarg(0);
        }
        // new object[arg_box.len()]
        e.ldc_i4(arg_box.len() as i32);
        e.emit(Instruction::new(
            OpCode::Newarr,
            Operand::Token(object_ref.into()),
        ));
        for (i, box_ty) in arg_box.iter().enumerate() {
            e.dup();
            e.ldc_i4(i as i32);
            let arg_index = if is_static { i } else { i + 1 } as u16;
            e.ldarg(arg_index);
            if let Some(box_ty) = box_ty {
                e.emit(Instruction::new(
                    OpCode::Box,
                    Operand::Token(box_ty.clone().into()),
                ));
            }
            e.emit(Instruction::simple(OpCode::Stelem_Ref));
        }
        e.call(seam.clone());
        match return_plan {
            ReturnPlan::Void => {
                e.pop();
            }
            ReturnPlan::Primitive(p) => {
                let ty = self.primitive_ref(module, p as u8)?;
                e.emit(Instruction::new(OpCode::Unbox_Any, Operand::Token(ty.into())));
            }
            ReturnPlan::Reference(None) => {}
            ReturnPlan::Reference(Some(ty)) => {
                e.castclass(ty);
            }
        }
        e.ret();
        Some(e.finish())
    }
}

/// Rewrites eligible method bodies of every module in `gen` to call into the
/// game's native code through the per-module invoke seam. Called from
/// [`generate_inner`] after the declaration assemblies are fully built.
pub(crate) fn fill_proxy_bodies(gen: &mut Generator) {
    // One seam method per module (the module may have no System.Object, in which
    // case it keeps its declaration bodies).
    let seams: Vec<Option<Entry<Method>>> = (0..gen.modules.len())
        .map(|module| gen.add_seam(module))
        .collect();

    for i in 0..gen.metadata.type_definitions.len() {
        let def = gen.metadata.type_definitions[i].clone();
        // Generic type definitions have no single native entry point per method.
        if def.generic_container_index >= 0 {
            continue;
        }
        let module = gen.type_image[i];
        let Some(seam) = seams[module].clone() else {
            continue;
        };
        let image_name = gen.metadata.image_name(&gen.metadata.images[module]).to_owned();

        let methods = gen.types[i].get().methods.clone();
        let tokens: Vec<u32> = gen
            .metadata
            .type_methods(&def)
            .iter()
            .map(|m| m.token)
            .collect();
        for (method, token) in methods.iter().zip(tokens) {
            if let Some(body) = gen.proxy_body(module, method, &image_name, token, &seam) {
                method.get_mut().body = Some(body);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs against a real game. Set `HAO_IL2CPP_GAME` to an Il2Cpp game folder
    /// with a v29 `GameAssembly.so` (tested against Megabonk). Skipped otherwise.
    ///
    /// Generates proxy assemblies, writes every one to a PE image and reloads it
    /// with hao (proving the IL and metadata are structurally valid), and checks
    /// the expected proxy call shape is present: a seam `Invoke` method and method
    /// bodies that call it.
    #[test]
    fn generates_valid_proxy_assemblies() {
        let Some(dir) = std::env::var_os("HAO_IL2CPP_GAME") else {
            eprintln!("skipping: set HAO_IL2CPP_GAME to an Il2Cpp game folder");
            return;
        };
        let game = crate::game::Game::open(dir).unwrap();
        let assemblies = generate_proxies(game.metadata(), game.binary()).unwrap();

        let mut seam_methods = 0usize;
        let mut proxy_calls = 0usize;
        let mut reloaded = 0usize;

        for assembly in &assemblies {
            // Every generated assembly must write to a valid PE and reload.
            let bytes = assembly.module.write().expect("proxy module should write");
            let module = hao::Module::from_bytes(&bytes).expect("proxy module should reload");
            reloaded += 1;

            // The reloaded module carries the seam and real proxy call sites.
            for ty in module.all_types() {
                let t = ty.get();
                let is_seam_type = t.name == SEAM_TYPE && t.namespace == SEAM_NAMESPACE;
                for method in &t.methods {
                    let m = method.get();
                    if is_seam_type && m.name == SEAM_METHOD {
                        seam_methods += 1;
                    }
                    let Some(body) = &m.body else { continue };
                    // A proxy body calls the seam: an Invoke call carrying the
                    // (image, token) identity. Count call sites that target a
                    // method named Invoke on the seam type.
                    for instruction in &body.instructions {
                        if instruction.opcode == OpCode::Call && calls_seam(&instruction.operand) {
                            proxy_calls += 1;
                        }
                    }
                }
            }
        }

        assert!(reloaded > 0, "expected to generate at least one assembly");
        assert!(
            seam_methods > 0,
            "expected a Hao.<Il2CppProxyRuntime>.Invoke seam method"
        );
        assert!(
            proxy_calls > 0,
            "expected method bodies that call the invoke seam"
        );
    }

    /// Whether a `call` operand targets the seam `Invoke` method, by name.
    fn calls_seam(operand: &Operand) -> bool {
        use hao::dotnet::il::MemberToken;
        let Operand::Token(token) = operand else {
            return false;
        };
        match token {
            MemberToken::Method(m) => m.get().name == SEAM_METHOD,
            MemberToken::MemberRef(m) => m.get().name == SEAM_METHOD,
            _ => false,
        }
    }
}
