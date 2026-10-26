//! Unity methods looked up by name and checked against their signature once.

use std::{marker::PhantomData, sync::OnceLock};

use hao_il2cpp_runtime::{Error, Il2Cpp, Il2CppValue, Method, MethodArgs, Result};

pub(crate) struct Binding<Args, R> {
    assembly: &'static str,
    namespace: &'static str,
    class: &'static str,
    name: &'static str,
    method: OnceLock<Method>,
    _marker: PhantomData<fn(Args) -> R>,
}

impl<Args: MethodArgs, R: Il2CppValue> Binding<Args, R> {
    pub const fn new(
        assembly: &'static str,
        namespace: &'static str,
        class: &'static str,
        name: &'static str,
    ) -> Self {
        Self {
            assembly,
            namespace,
            class,
            name,
            method: OnceLock::new(),
            _marker: PhantomData,
        }
    }

    /// The overload whose signature matches `Args` and `R`.
    pub fn method(&self) -> Result<Method> {
        if let Some(method) = self.method.get() {
            return Ok(*method);
        }
        let il2cpp = Il2Cpp::get().ok_or_else(|| Error::NotFound("the Il2Cpp runtime".into()))?;
        let method = il2cpp
            .class(self.assembly, self.namespace, self.class)?
            .methods()
            .find(|m| m.name() == self.name && m.check_types::<Args, R>().is_ok())
            .ok_or_else(|| Error::NotFound(format!("{} with matching types", self.describe())))?;
        tracing::trace!(method = %method.signature(), "resolved Unity method");
        Ok(*self.method.get_or_init(|| method))
    }

    pub fn call(&self, args: Args) -> Result<R> {
        let method = self.method()?;
        Ok(unsafe { method.call_unchecked(args) })
    }
}

pub(crate) trait Check: Sync {
    fn check(&self) -> Result<()>;
    fn describe(&self) -> String;
}

impl<Args: MethodArgs, R: Il2CppValue> Check for Binding<Args, R> {
    fn check(&self) -> Result<()> {
        self.method().map(|_| ())
    }

    fn describe(&self) -> String {
        format!("{}.{}::{}", self.namespace, self.class, self.name)
    }
}

/// Declares a group of bindings plus a `$checks` function listing them (used to
/// aggregate [`check_bindings`](crate::check_bindings) across the concern files).
macro_rules! bindings {
    ($checks:ident; $($name:ident: $assembly:literal $namespace:literal $class:literal $method:literal ($($arg:ty),*) -> $ret:ty;)*) => {
        $(pub(crate) static $name: $crate::binding::Binding<($($arg,)*), $ret> =
            $crate::binding::Binding::new($assembly, $namespace, $class, $method);)*

        pub(crate) fn $checks() -> ::std::vec::Vec<&'static dyn $crate::binding::Check> {
            vec![$(&$name),*]
        }
    };
}

pub(crate) use bindings;
