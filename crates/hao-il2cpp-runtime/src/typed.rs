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

/// The managed type of a parameter or return value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    /// The managed signature, e.g. `System.Int32 Heal(System.Single, System.Boolean)`.
    pub fn signature(&self) -> String {
        let api = &self.il2cpp.api;
        let ret = self.type_name(unsafe { (api.il2cpp_method_get_return_type)(self.ptr) });
        let params: Vec<String> = (0..self.parameter_count())
            .map(|i| self.type_name(unsafe { (api.il2cpp_method_get_param)(self.ptr, i) }))
            .collect();
        format!(
            "{}{} {}({})",
            if self.is_instance() { "" } else { "static " },
            ret,
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
}
