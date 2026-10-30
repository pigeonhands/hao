//! Typed values, hooks and calls, checked against the game's method signatures.

use std::{any::Any, ffi::c_void, marker::PhantomData, sync::RwLock};

use crate::{Error, Il2Cpp, Method, MethodInfo, Ptr, Result};

/// Managed element types (`Il2CppTypeEnum`).
pub mod element {
    pub const VOID: i32 = 0x01;
    pub const BOOLEAN: i32 = 0x02;
    pub const CHAR: i32 = 0x03;
    pub const I1: i32 = 0x04;
    pub const U1: i32 = 0x05;
    pub const I2: i32 = 0x06;
    pub const U2: i32 = 0x07;
    pub const I4: i32 = 0x08;
    pub const U4: i32 = 0x09;
    pub const I8: i32 = 0x0A;
    pub const U8: i32 = 0x0B;
    pub const R4: i32 = 0x0C;
    pub const R8: i32 = 0x0D;
    pub const STRING: i32 = 0x0E;
    pub const PTR: i32 = 0x0F;
    pub const VALUETYPE: i32 = 0x11;
    pub const CLASS: i32 = 0x12;
    pub const VAR: i32 = 0x13;
    pub const ARRAY: i32 = 0x14;
    pub const GENERICINST: i32 = 0x15;
    pub const I: i32 = 0x18;
    pub const U: i32 = 0x19;
    pub const OBJECT: i32 = 0x1C;
    pub const SZARRAY: i32 = 0x1D;
    pub const MVAR: i32 = 0x1E;
}

/// The managed element type of a field, parameter, or return value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ElementType {
    Void,
    Boolean,
    Char,
    I1,
    U1,
    I2,
    U2,
    I4,
    U4,
    I8,
    U8,
    R4,
    R8,
    String,
    Ptr,
    ValueType,
    Class,
    Var,
    Array,
    GenericInst,
    I,
    U,
    Object,
    SzArray,
    MVar,
    Unknown(i32),
}

impl ElementType {
    pub fn is_float(self) -> bool {
        matches!(self, Self::R4 | Self::R8)
    }

    pub fn is_integer(self) -> bool {
        matches!(
            self,
            Self::Boolean | Self::Char | Self::I1 | Self::U1 | Self::I2 | Self::U2
                | Self::I4 | Self::U4 | Self::I8 | Self::U8 | Self::I | Self::U
        )
    }

    pub fn is_reference(self) -> bool {
        matches!(
            self,
            Self::String | Self::Class | Self::Object | Self::GenericInst
                | Self::Array | Self::SzArray
        )
    }

    pub fn raw(self) -> i32 {
        match self {
            Self::Void => element::VOID,
            Self::Boolean => element::BOOLEAN,
            Self::Char => element::CHAR,
            Self::I1 => element::I1,
            Self::U1 => element::U1,
            Self::I2 => element::I2,
            Self::U2 => element::U2,
            Self::I4 => element::I4,
            Self::U4 => element::U4,
            Self::I8 => element::I8,
            Self::U8 => element::U8,
            Self::R4 => element::R4,
            Self::R8 => element::R8,
            Self::String => element::STRING,
            Self::Ptr => element::PTR,
            Self::ValueType => element::VALUETYPE,
            Self::Class => element::CLASS,
            Self::Var => element::VAR,
            Self::Array => element::ARRAY,
            Self::GenericInst => element::GENERICINST,
            Self::I => element::I,
            Self::U => element::U,
            Self::Object => element::OBJECT,
            Self::SzArray => element::SZARRAY,
            Self::MVar => element::MVAR,
            Self::Unknown(v) => v,
        }
    }
}

impl From<i32> for ElementType {
    fn from(v: i32) -> Self {
        match v {
            element::VOID => Self::Void,
            element::BOOLEAN => Self::Boolean,
            element::CHAR => Self::Char,
            element::I1 => Self::I1,
            element::U1 => Self::U1,
            element::I2 => Self::I2,
            element::U2 => Self::U2,
            element::I4 => Self::I4,
            element::U4 => Self::U4,
            element::I8 => Self::I8,
            element::U8 => Self::U8,
            element::R4 => Self::R4,
            element::R8 => Self::R8,
            element::STRING => Self::String,
            element::PTR => Self::Ptr,
            element::VALUETYPE => Self::ValueType,
            element::CLASS => Self::Class,
            element::VAR => Self::Var,
            element::ARRAY => Self::Array,
            element::GENERICINST => Self::GenericInst,
            element::I => Self::I,
            element::U => Self::U,
            element::OBJECT => Self::Object,
            element::SZARRAY => Self::SzArray,
            element::MVAR => Self::MVar,
            other => Self::Unknown(other),
        }
    }
}

impl std::fmt::Display for ElementType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Void => f.write_str("void"),
            Self::Boolean => f.write_str("bool"),
            Self::Char => f.write_str("char"),
            Self::I1 => f.write_str("i8"),
            Self::U1 => f.write_str("u8"),
            Self::I2 => f.write_str("i16"),
            Self::U2 => f.write_str("u16"),
            Self::I4 => f.write_str("i32"),
            Self::U4 => f.write_str("u32"),
            Self::I8 => f.write_str("i64"),
            Self::U8 => f.write_str("u64"),
            Self::R4 => f.write_str("f32"),
            Self::R8 => f.write_str("f64"),
            Self::String => f.write_str("string"),
            Self::Ptr => f.write_str("ptr"),
            Self::ValueType => f.write_str("valuetype"),
            Self::Class => f.write_str("class"),
            Self::Var => f.write_str("var"),
            Self::Array => f.write_str("array"),
            Self::GenericInst => f.write_str("generic"),
            Self::I => f.write_str("nint"),
            Self::U => f.write_str("nuint"),
            Self::Object => f.write_str("object"),
            Self::SzArray => f.write_str("szarray"),
            Self::MVar => f.write_str("mvar"),
            Self::Unknown(v) => write!(f, "unknown(0x{v:02X})"),
        }
    }
}

/// The managed type of a parameter or return value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "reflect", derive(facet::Facet))]
pub struct TypeInfo {
    /// The `Il2CppTypeEnum` value. For enums, the element type of the underlying integer.
    pub element: i32,
    pub value_type: bool,
    pub is_enum: bool,
    pub byref: bool,
    pub namespace: &'static str,
    pub name: &'static str,
}

impl TypeInfo {
    /// Whether this is the value type `namespace.name`, passed by value. For
    /// [`Il2CppValue`] impls of `#[repr(C)]` structs mirroring a managed struct.
    pub fn is_value_type(&self, namespace: &str, name: &str) -> bool {
        !self.byref
            && self.value_type
            && !self.is_enum
            && self.namespace == namespace
            && self.name == name
    }
}

/// A method parameter's name and managed type name, from [`Method::parameters`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "reflect", derive(facet::Facet))]
pub struct Parameter {
    /// The declared name, e.g. `amount` (often `arg0` in release builds).
    pub name: String,
    /// The managed type name, e.g. `System.Single`.
    pub type_name: String,
}

/// A managed object reference (`Il2CppObject*`), possibly null.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Object(pub Ptr);

unsafe impl Send for Object {}
unsafe impl Sync for Object {}

impl Object {
    pub const NULL: Object = Object(std::ptr::null_mut());

    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }

    pub fn as_ptr(&self) -> Ptr {
        self.0
    }
}

/// A managed string (`Il2CppString*`), possibly null.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Il2CppString(pub Object);

impl Il2CppString {
    pub fn new(il2cpp: &Il2Cpp, value: &str) -> Result<Self> {
        il2cpp.string(value)
    }

    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }

    /// The string's contents, or `None` for null.
    pub fn to_rust(&self) -> Option<String> {
        let il2cpp = Il2Cpp::get()?;
        // Values of this type only come from string typed arguments and returns.
        unsafe { il2cpp.read_string(self.0) }
    }
}

/// A Rust type that has the same representation as some managed types when
/// passed to and from compiled Il2Cpp code.
///
/// # Safety
/// `matches` may only accept managed types whose values are passed exactly like `Self`.
pub unsafe trait Il2CppValue: Copy + 'static {
    fn matches(ty: TypeInfo) -> bool;

    fn describe() -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Whether this is a pointer to an object, so it can be the instance (`this`).
    fn is_reference() -> bool {
        false
    }
}

macro_rules! primitive {
    ($($t:ty => [$($e:ident),+];)*) => {$(
        unsafe impl Il2CppValue for $t {
            fn matches(ty: TypeInfo) -> bool {
                !ty.byref && [$(element::$e),+].contains(&ty.element)
            }
        }
    )*};
}

primitive! {
    () => [VOID];
    bool => [BOOLEAN];
    u16 => [CHAR, U2];
    i8 => [I1];
    u8 => [U1];
    i16 => [I2];
    i32 => [I4];
    u32 => [U4];
    i64 => [I8];
    u64 => [U8];
    f32 => [R4];
    f64 => [R8];
    isize => [I];
    usize => [U];
}

unsafe impl Il2CppValue for Object {
    fn is_reference() -> bool {
        true
    }

    fn matches(ty: TypeInfo) -> bool {
        !ty.byref
            && !ty.value_type
            && [
                element::STRING,
                element::CLASS,
                element::OBJECT,
                element::SZARRAY,
                element::ARRAY,
                element::GENERICINST,
                element::VAR,
                element::MVAR,
            ]
            .contains(&ty.element)
    }
}

unsafe impl Il2CppValue for Il2CppString {
    fn is_reference() -> bool {
        true
    }

    fn matches(ty: TypeInfo) -> bool {
        !ty.byref && ty.element == element::STRING
    }
}

/// `ref`/`out` parameters and pointers.
unsafe impl Il2CppValue for *mut c_void {
    fn is_reference() -> bool {
        true
    }

    fn matches(ty: TypeInfo) -> bool {
        ty.byref || ty.element == element::PTR
    }
}

/// Calls the original code of a hooked method.
pub struct Original<Args, R> {
    code: *const c_void,
    method: MethodInfo,
    _marker: PhantomData<fn(Args) -> R>,
}

/// How a Rust type is checked against a managed parameter or return type.
#[derive(Clone, Copy)]
pub struct ValueCheck {
    pub matches: fn(TypeInfo) -> bool,
    pub name: &'static str,
    pub reference: bool,
}

impl ValueCheck {
    pub fn of<T: Il2CppValue>() -> Self {
        Self {
            matches: T::matches,
            name: T::describe(),
            reference: T::is_reference(),
        }
    }
}

/// The Rust types of a hook or call, checked against the method before use.
pub struct Signature {
    pub parameters: Vec<ValueCheck>,
    pub ret: ValueCheck,
}

impl<Args: MethodArgs, R: Il2CppValue> Original<Args, R> {
    /// Calls the original code with `args`, a tuple like the callback's arguments.
    pub fn call(&self, args: Args) -> R {
        unsafe { args.invoke(self.code, self.method) }
    }
}

/// A closure usable as a hook for a method with parameters `Args` returning `R`.
/// Implemented for `Fn(args..., Original<(args...), R>) -> R`.
pub trait HookCallback<Args, R>: Send + Sync + 'static {
    #[doc(hidden)]
    fn trampoline() -> *mut u8;
    #[doc(hidden)]
    fn signature() -> Signature;
}

/// Argument tuples for [`Method::call`].
pub trait MethodArgs {
    #[doc(hidden)]
    fn parameters() -> Vec<ValueCheck>;
    #[doc(hidden)]
    unsafe fn invoke<R: Il2CppValue>(self, code: *const c_void, method: MethodInfo) -> R;
}

type Callbacks = RwLock<Vec<(usize, &'static (dyn Any + Send + Sync))>>;

/// Hook closures by the address of their hook's original-function stub.
static CALLBACKS: Callbacks = RwLock::new(Vec::new());

pub(crate) fn register<F: Any + Send + Sync>(stub: usize, callback: F) {
    let callback: &'static (dyn Any + Send + Sync) = Box::leak(Box::new(callback));
    CALLBACKS
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .push((stub, callback));
}

fn callback<F: Any>(stub: usize) -> &'static F {
    let callbacks = CALLBACKS.read().unwrap_or_else(|e| e.into_inner());
    callbacks
        .iter()
        .rev()
        .find(|(s, _)| *s == stub)
        .and_then(|(_, c)| c.downcast_ref::<F>())
        .unwrap_or_else(|| {
            tracing::error!("no callback registered for hook {:#x}, aborting", stub);
            eprintln!("[hao] no callback registered for hook {:#x}", stub);
            std::process::abort()
        })
}

macro_rules! arity {
    ($trampoline:ident; $($t:ident $v:ident),*) => {
        impl<F, $($t: Il2CppValue,)* R: Il2CppValue> HookCallback<($($t,)*), R> for F
        where
            F: Fn($($t,)* Original<($($t,)*), R>) -> R + Send + Sync + 'static,
        {
            fn trampoline() -> *mut u8 {
                $trampoline::<F, $($t,)* R> as *mut u8
            }

            fn signature() -> Signature {
                Signature {
                    parameters: vec![$(ValueCheck::of::<$t>(),)*],
                    ret: ValueCheck::of::<R>(),
                }
            }
        }

        #[allow(clippy::too_many_arguments)]
        extern "C-unwind" fn $trampoline<F, $($t,)* R>($($v: $t,)* method: MethodInfo) -> R
        where
            F: Fn($($t,)* Original<($($t,)*), R>) -> R + Send + Sync + 'static,
        {
            // Must come first: the hook passes the original code in registers.
            let code = hooking::original_function_ptr().map_or(std::ptr::null_mut(), |p| p.as_ptr());
            let callback = callback::<F>(code as usize);
            callback($($v,)* Original { code, method, _marker: PhantomData })
        }

        impl<$($t: Il2CppValue,)*> MethodArgs for ($($t,)*) {
            fn parameters() -> Vec<ValueCheck> {
                vec![$(ValueCheck::of::<$t>(),)*]
            }

            #[allow(clippy::unused_unit)]
            unsafe fn invoke<R: Il2CppValue>(self, code: *const c_void, method: MethodInfo) -> R {
                let ($($v,)*) = self;
                let f: extern "C-unwind" fn($($t,)* MethodInfo) -> R = unsafe { std::mem::transmute(code) };
                f($($v,)* method)
            }
        }
    };
}

arity!(trampoline0;);
arity!(trampoline1; A a);
arity!(trampoline2; A a, B b);
arity!(trampoline3; A a, B b, C c);
arity!(trampoline4; A a, B b, C c, D d);
arity!(trampoline5; A a, B b, C c, D d, E e);
arity!(trampoline6; A a, B b, C c, D d, E e, G g);
arity!(trampoline7; A a, B b, C c, D d, E e, G g, H h);
arity!(trampoline8; A a, B b, C c, D d, E e, G g, H h, I i);

impl Method {
    fn type_info(&self, ty: Ptr) -> TypeInfo {
        let api = &self.il2cpp.api;
        unsafe {
            let mut element = (api.il2cpp_type_get_type)(ty);
            let byref = (api.il2cpp_type_is_byref)(ty);
            let class = (api.il2cpp_type_get_class_or_element_class)(ty);
            let value_type = match element {
                element::VALUETYPE => true,
                element::GENERICINST => !class.is_null() && (api.il2cpp_class_is_valuetype)(class),
                _ => false,
            };
            let is_enum = element == element::VALUETYPE
                && !class.is_null()
                && (api.il2cpp_class_is_enum)(class);
            if is_enum {
                element = (api.il2cpp_type_get_type)((api.il2cpp_class_enum_basetype)(class));
            }
            let (namespace, name) = if class.is_null() {
                ("", "")
            } else {
                (
                    crate::c_str((api.il2cpp_class_get_namespace)(class)),
                    crate::c_str((api.il2cpp_class_get_name)(class)),
                )
            };
            TypeInfo {
                element,
                value_type,
                is_enum,
                byref,
                namespace,
                name,
            }
        }
    }

    fn type_name(&self, ty: Ptr) -> String {
        unsafe {
            let name = (self.il2cpp.api.il2cpp_type_get_name)(ty);
            if name.is_null() {
                return "?".into();
            }
            std::ffi::CStr::from_ptr(name)
                .to_string_lossy()
                .into_owned()
        }
    }

    pub fn is_instance(&self) -> bool {
        unsafe { (self.il2cpp.api.il2cpp_method_is_instance)(self.ptr) }
    }

    pub fn parameter_count(&self) -> u32 {
        unsafe { (self.il2cpp.api.il2cpp_method_get_param_count)(self.ptr) }
    }

    /// The managed name of the return type, e.g. `System.Int32`.
    pub fn return_type_name(&self) -> String {
        self.type_name(unsafe { (self.il2cpp.api.il2cpp_method_get_return_type)(self.ptr) })
    }

    /// The managed type name of parameter `index`, e.g. `System.Single`.
    pub fn parameter_type_name(&self, index: u32) -> String {
        self.type_name(unsafe { (self.il2cpp.api.il2cpp_method_get_param)(self.ptr, index) })
    }

    /// The declared name of parameter `index`, e.g. `amount`. Release builds
    /// often strip these to `arg0`, `arg1`, ...
    pub fn parameter_name(&self, index: u32) -> String {
        let name = unsafe { (self.il2cpp.api.il2cpp_method_get_param_name)(self.ptr, index) };
        unsafe { crate::c_str(name) }.to_owned()
    }

    /// The parameters, each with its name and type name.
    pub fn parameters(&self) -> impl Iterator<Item = Parameter> + '_ {
        (0..self.parameter_count()).map(move |i| Parameter {
            name: self.parameter_name(i),
            type_name: self.parameter_type_name(i),
        })
    }

    /// The managed signature, e.g. `System.Int32 Heal(System.Single, System.Boolean)`,
    /// built from [`Method::return_type_name`], [`Method::name`] and [`Method::parameters`].
    pub fn signature(&self) -> String {
        let params: Vec<String> = self.parameters().map(|p| p.type_name).collect();
        format!(
            "{}{} {}({})",
            if self.is_instance() { "" } else { "static " },
            self.return_type_name(),
            self.name(),
            params.join(", ")
        )
    }

    /// Checks `params` (with the instance first for instance methods) and `ret` against the method.
    fn check(&self, params: &[ValueCheck], ret: ValueCheck) -> Result<()> {
        let api = &self.il2cpp.api;
        let instance = self.is_instance() as usize;
        let count = self.parameter_count() as usize;
        let mismatch =
            |detail: String| Error::Signature(format!("{}: {}", self.signature(), detail));
        if params.len() != count + instance {
            return Err(mismatch(format!(
                "expected {} arguments{}, got {}",
                count + instance,
                if instance == 1 {
                    " including the instance"
                } else {
                    ""
                },
                params.len()
            )));
        }
        if instance == 1 && !params[0].reference {
            return Err(mismatch(format!(
                "the instance can't be passed as {}",
                params[0].name
            )));
        }
        for (i, check) in params[instance..].iter().enumerate() {
            let ty = unsafe { (api.il2cpp_method_get_param)(self.ptr, i as u32) };
            if !(check.matches)(self.type_info(ty)) {
                return Err(mismatch(format!(
                    "parameter {} is {}, not {}",
                    i,
                    self.type_name(ty),
                    check.name
                )));
            }
        }
        let ty = unsafe { (api.il2cpp_method_get_return_type)(self.ptr) };
        if !(ret.matches)(self.type_info(ty)) {
            return Err(mismatch(format!(
                "returns {}, not {}",
                self.type_name(ty),
                ret.name
            )));
        }
        Ok(())
    }

    /// Checks argument types `Args` (a tuple, with the instance first for
    /// instance methods) and return type `R` against the method's signature, as
    /// [`Method::hook`] and [`Method::call`] do.
    pub fn check_types<Args: MethodArgs, R: Il2CppValue>(&self) -> Result<()> {
        self.check(&Args::parameters(), ValueCheck::of::<R>())
    }

    /// Redirects calls of this method to `callback`, after checking its types
    /// against the method's signature. The callback gets the arguments (the
    /// instance first for instance methods) and an [`Original`] to call the
    /// original code with.
    ///
    /// ```no_run
    /// # use hao_il2cpp_runtime::{Il2Cpp, Object, Original};
    /// # fn f(il2cpp: &'static Il2Cpp) -> hao_il2cpp_runtime::Result<()> {
    /// let heal = il2cpp.class("Assembly-CSharp", "Game", "PlayerHealth")?.method("Heal", 2)?;
    /// heal.hook(|this: Object, amount: f32, overheal: bool, original: Original<_, i32>| {
    ///     original.call((this, amount * 2.0, overheal))
    /// })?;
    /// # Ok(()) }
    /// ```
    pub fn hook<Args, R, F: HookCallback<Args, R>>(
        &self,
        callback: F,
    ) -> Result<crate::MethodHook> {
        let signature = F::signature();
        self.check(&signature.parameters, signature.ret)?;
        let mut hook = unsafe { hooking::Hook::create(self.code() as *mut u8, F::trampoline()) }
            .map_err(Error::Hook)?;
        register(
            hook.data.original_fn_call_stub_data.as_ptr() as usize,
            callback,
        );
        unsafe { hook.apply_hook() }.map_err(Error::Hook)?;
        tracing::debug!(method = %self.signature(), code = ?self.code(), "hooked");
        Ok(crate::MethodHook(hook))
    }

    /// Calls the method's compiled code after checking the argument and return
    /// types. `args` is a tuple, with the instance first for instance methods.
    ///
    /// Managed exceptions thrown by the method unwind through the caller.
    pub fn call<Args: MethodArgs, R: Il2CppValue>(&self, args: Args) -> Result<R> {
        self.check(&Args::parameters(), ValueCheck::of::<R>())?;
        Ok(unsafe { args.invoke(self.code(), self.ptr) })
    }

    /// [`Method::call`] without the type check, for calls made often after
    /// [`Method::check_types`] has passed once.
    ///
    /// # Safety
    /// `Args` and `R` must pass [`Method::check_types`] for this method.
    pub unsafe fn call_unchecked<Args: MethodArgs, R: Il2CppValue>(&self, args: Args) -> R {
        unsafe { args.invoke(self.code(), self.ptr) }
    }

    /// Hooks a **void** method with a conditional block: when
    /// [`VoidBlockHook::set_blocked(true)`] is active, calls to the method
    /// return immediately without running the original code.
    ///
    /// Works for any void method regardless of its parameter types — the
    /// trampoline preserves all argument registers and either tail-calls the
    /// original or returns.
    ///
    /// ```no_run
    /// # use hao_il2cpp_runtime::Il2Cpp;
    /// # fn f(il2cpp: &'static Il2Cpp) -> hao_il2cpp_runtime::Result<()> {
    /// let damage = il2cpp
    ///     .class("Assembly-CSharp", "Actors", "PlayerHealth")?
    ///     .method("DamagePlayer", 3)?;
    /// let hook = damage.hook_void_block()?;
    /// hook.set_blocked(true); // godmode — DamagePlayer is now a no-op
    /// # Ok(()) }
    /// ```
    #[cfg(target_arch = "x86_64")]
    pub fn hook_void_block(&self) -> Result<VoidBlockHook> {
        let ret = self.return_type_name();
        if ret != "System.Void" {
            return Err(Error::Signature(format!(
                "hook_void_block requires a void method, but `{}` returns {}",
                self.name(),
                ret,
            )));
        }

        let state = Box::leak(Box::new(std::sync::atomic::AtomicBool::new(false)));
        let state_ptr = state as *const std::sync::atomic::AtomicBool;

        let mut hook =
            unsafe { hooking::Hook::create(self.code() as *mut u8, void_block_trampoline as *mut u8) }
                .map_err(Error::Hook)?;
        let original = hook.data.original_fn_call_stub_data.as_ptr() as usize;
        register(original, state_ptr as usize);
        unsafe { hook.apply_hook() }.map_err(Error::Hook)?;

        tracing::debug!(method = %self.signature(), code = ?self.code(), "hooked (void block)");

        Ok(VoidBlockHook {
            state,
            _hook: crate::MethodHook(hook),
        })
    }
}

/// A hook that conditionally blocks a void method. Created by
/// [`Method::hook_void_block`].
pub struct VoidBlockHook {
    /// The flag that controls whether the method is blocked.
    pub state: &'static std::sync::atomic::AtomicBool,
    _hook: crate::MethodHook,
}

impl VoidBlockHook {
    /// When `true`, calls to the hooked method return immediately.
    pub fn set_blocked(&self, blocked: bool) {
        self.state.store(blocked, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn is_blocked(&self) -> bool {
        self.state.load(std::sync::atomic::Ordering::Relaxed)
    }
}

// ── Generic hooks ───────────────────────────────────────────────────

/// How a method parameter maps to the System V AMD64 ABI.
#[cfg(target_arch = "x86_64")]
#[derive(Debug, Clone, Copy)]
pub enum ParamSlot {
    /// Integer/pointer in register index 0..5 (rdi, rsi, rdx, rcx, r8, r9).
    IntReg(u8),
    /// Float in SSE register index 0..7 (xmm0..xmm7).
    FloatReg(u8),
}

/// One parameter of a hooked method, mapped to its ABI slot at install time.
#[cfg(target_arch = "x86_64")]
#[derive(Debug, Clone)]
pub struct GenericParam {
    pub slot: ParamSlot,
    pub type_info: TypeInfo,
    pub name: String,
}

/// Metadata about a generically-hooked method, built once at install time.
///
/// The `params` list includes the instance pointer (if any) as the first
/// entry, followed by the declared parameters. Each is mapped to its ABI
/// register slot.
#[cfg(target_arch = "x86_64")]
#[derive(Debug, Clone)]
pub struct GenericHookMeta {
    pub params: Vec<GenericParam>,
    pub return_type: TypeInfo,
    pub is_instance: bool,
    pub signature: String,
}

#[cfg(target_arch = "x86_64")]
struct GenericHookEntry {
    meta: GenericHookMeta,
    callback: Box<dyn Fn(usize, &GenericHookMeta, &mut hooking::SavedRegs) -> hooking::HookAction + Send + Sync>,
}

#[cfg(target_arch = "x86_64")]
impl Method {
    /// Hooks this method with a generic callback that receives the saved
    /// argument registers and metadata about each parameter's type and ABI
    /// slot. The callback inspects or modifies arguments through
    /// [`hooking::SavedRegs`] and returns a [`hooking::HookAction`] to pass
    /// through to the original or override the return value.
    ///
    /// Unlike [`Method::hook`], the callback doesn't need to declare the
    /// method's parameter types at compile time — they are mapped to ABI
    /// registers at install time using Il2Cpp metadata.
    ///
    /// ```no_run
    /// # use hao_il2cpp_runtime::Il2Cpp;
    /// # fn f(il2cpp: &'static Il2Cpp) -> hao_il2cpp_runtime::Result<()> {
    /// let heal = il2cpp
    ///     .class("Assembly-CSharp", "Game", "PlayerHealth")?
    ///     .method("Heal", 2)?;
    /// heal.hook_generic(|original, meta, regs| {
    ///     // Read the second declared parameter (index 1 if instance method,
    ///     // since index 0 is `this`).
    ///     let amount = regs.read_int(1);
    ///     // Double it and pass through to the original.
    ///     regs.write_int(1, amount * 2);
    ///     hooking::HookAction::Passthrough
    /// })?;
    /// # Ok(()) }
    /// ```
    pub fn hook_generic(
        &self,
        callback: impl Fn(usize, &GenericHookMeta, &mut hooking::SavedRegs) -> hooking::HookAction + Send + Sync + 'static,
    ) -> Result<crate::MethodHook> {
        let meta = self.build_generic_meta();
        let entry = GenericHookEntry {
            meta,
            callback: Box::new(callback),
        };

        let mut hook = unsafe {
            hooking::Hook::create_save_registers(
                self.code() as *mut u8,
                generic_hook_dispatch,
            )
        }
        .map_err(Error::Hook)?;

        let stub_addr = hook.data.original_fn_call_stub_data.as_ptr() as usize;
        register(stub_addr, entry);

        unsafe { hook.apply_hook() }.map_err(Error::Hook)?;
        tracing::debug!(method = %self.signature(), code = ?self.code(), "hooked (generic)");
        Ok(crate::MethodHook(hook))
    }

    fn build_generic_meta(&self) -> GenericHookMeta {
        let is_instance = self.is_instance();
        let param_count = self.parameter_count();

        let mut params = Vec::new();
        let mut int_idx: u8 = 0;
        let mut float_idx: u8 = 0;

        if is_instance {
            params.push(GenericParam {
                slot: ParamSlot::IntReg(int_idx),
                type_info: TypeInfo {
                    element: element::OBJECT,
                    value_type: false,
                    is_enum: false,
                    byref: false,
                    namespace: "",
                    name: "",
                },
                name: "this".to_owned(),
            });
            int_idx += 1;
        }

        for i in 0..param_count {
            let ty = unsafe { (self.il2cpp.api.il2cpp_method_get_param)(self.ptr, i) };
            let info = self.type_info(ty);
            let name = self.parameter_name(i);

            let is_float = matches!(info.element, element::R4 | element::R8) && !info.byref;
            let slot = if is_float {
                if float_idx < 8 {
                    let s = ParamSlot::FloatReg(float_idx);
                    float_idx += 1;
                    s
                } else {
                    continue;
                }
            } else if int_idx < 6 {
                let s = ParamSlot::IntReg(int_idx);
                int_idx += 1;
                s
            } else {
                continue;
            };

            params.push(GenericParam { slot, type_info: info, name });
        }

        let ret_ty = unsafe { (self.il2cpp.api.il2cpp_method_get_return_type)(self.ptr) };

        GenericHookMeta {
            params,
            return_type: self.type_info(ret_ty),
            is_instance,
            signature: self.signature(),
        }
    }
}

/// Called from hooking's `save_registers_trampoline` via the dispatch shim.
#[cfg(target_arch = "x86_64")]
extern "C" fn generic_hook_dispatch(
    original: usize,
    int_regs: *mut u64,
    sse_regs: *mut [u8; 16],
    result: *mut hooking::DispatchResult,
) {
    let entry = callback::<GenericHookEntry>(original);
    let mut regs = hooking::SavedRegs { int_regs, sse_regs };

    let action = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        (entry.callback)(original, &entry.meta, &mut regs)
    }));

    let action = match action {
        Ok(a) => a,
        Err(_) => {
            tracing::error!("generic hook callback panicked, calling original");
            hooking::HookAction::Passthrough
        }
    };

    unsafe { *result = hooking::DispatchResult::from(action) };
}

/// Looks up the block flag for the hook whose original function stub is at
/// `original`. Called from the naked trampoline.
extern "C" fn void_block_check(original: usize) -> bool {
    let state_ptr = callback::<usize>(original);
    let state = unsafe { &*(*state_ptr as *const std::sync::atomic::AtomicBool) };
    state.load(std::sync::atomic::Ordering::Relaxed)
}

/// Generic trampoline for [`Method::hook_void_block`]. Preserves all
/// argument registers (integer and SSE) so it works for any void method
/// regardless of parameter types.
#[unsafe(naked)]
#[cfg(target_arch = "x86_64")]
unsafe extern "C" fn void_block_trampoline() {
    // The hooking crate's redirect stub puts the original function pointer
    // in r10 and a checksum in r11 before jumping here.
    core::arch::naked_asm!(
        // Save callee-saved register we'll use to hold the original ptr.
        "push rbx",

        // Verify and capture the original function pointer from r10/r11.
        "mov rbx, r10",
        "movabs rax, 0xDEADBEEFCAFEBABE",
        "xor rax, r10",
        "cmp rax, r11",
        "jne 9f",

        // Save all integer argument registers.
        "push rdi",
        "push rsi",
        "push rdx",
        "push rcx",
        "push r8",
        "push r9",

        // Save all SSE argument registers (8 × 16 = 128 bytes).
        "sub rsp, 0x80",
        "movdqu [rsp+0x00], xmm0",
        "movdqu [rsp+0x10], xmm1",
        "movdqu [rsp+0x20], xmm2",
        "movdqu [rsp+0x30], xmm3",
        "movdqu [rsp+0x40], xmm4",
        "movdqu [rsp+0x50], xmm5",
        "movdqu [rsp+0x60], xmm6",
        "movdqu [rsp+0x70], xmm7",

        // Check if this hook is blocked.
        // void_block_check(original: usize) -> bool
        "mov rdi, rbx",
        "call {check}",
        "test al, al",
        "jnz 2f",

        // Not blocked — restore registers and tail-call original.
        "movdqu xmm0, [rsp+0x00]",
        "movdqu xmm1, [rsp+0x10]",
        "movdqu xmm2, [rsp+0x20]",
        "movdqu xmm3, [rsp+0x30]",
        "movdqu xmm4, [rsp+0x40]",
        "movdqu xmm5, [rsp+0x50]",
        "movdqu xmm6, [rsp+0x60]",
        "movdqu xmm7, [rsp+0x70]",
        "add rsp, 0x80",
        "pop r9",
        "pop r8",
        "pop rcx",
        "pop rdx",
        "pop rsi",
        "pop rdi",
        "mov rax, rbx",
        "pop rbx",
        "jmp rax",

        "2:",
        // Blocked — clean up and return (method is void).
        "add rsp, 0x80",
        "pop r9",
        "pop r8",
        "pop rcx",
        "pop rdx",
        "pop rsi",
        "pop rdi",
        "pop rbx",
        "ret",

        "9:",
        // Checksum mismatch — not in a hook, just return.
        "pop rbx",
        "ret",

        check = sym void_block_check,
    );
}
