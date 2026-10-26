//! The `il2cpp_*` functions exported by the game binary.

use std::ffi::{c_char, c_void};

use libloading::Library;

use crate::error::{Error, Result};

pub type Ptr = *mut c_void;

macro_rules! api {
    ($($name:ident: fn($($arg:ty),*) $(-> $ret:ty)?;)*) => {
        /// Function pointers into the game's Il2Cpp runtime.
        #[allow(non_snake_case)]
        pub struct Api {
            $(pub $name: unsafe extern "C" fn($($arg),*) $(-> $ret)?,)*
        }

        impl Api {
            pub(crate) fn load(library: &Library) -> Result<Self> {
                unsafe {
                    Ok(Self {
                        $($name: *library
                            .get(concat!(stringify!($name), "\0").as_bytes())
                            .map_err(|e| Error::MissingExport(stringify!($name), e))?,)*
                    })
                }
            }
        }
    };
}

api! {
    il2cpp_init: fn(*const c_char) -> i32;
    il2cpp_set_data_dir: fn(*const c_char);
    il2cpp_domain_get: fn() -> Ptr;
    il2cpp_thread_attach: fn(Ptr) -> Ptr;
    il2cpp_domain_assembly_open: fn(Ptr, *const c_char) -> Ptr;
    il2cpp_assembly_get_image: fn(Ptr) -> Ptr;
    il2cpp_class_from_name: fn(Ptr, *const c_char, *const c_char) -> Ptr;
    il2cpp_class_get_name: fn(Ptr) -> *const c_char;
    il2cpp_class_get_namespace: fn(Ptr) -> *const c_char;
    il2cpp_class_get_method_from_name: fn(Ptr, *const c_char, i32) -> Ptr;
    il2cpp_class_get_field_from_name: fn(Ptr, *const c_char) -> Ptr;
    il2cpp_field_get_offset: fn(Ptr) -> usize;
    il2cpp_method_get_name: fn(Ptr) -> *const c_char;
    il2cpp_object_get_class: fn(Ptr) -> Ptr;
    il2cpp_string_new: fn(*const c_char) -> Ptr;
    il2cpp_string_chars: fn(Ptr) -> *const u16;
    il2cpp_string_length: fn(Ptr) -> i32;
    il2cpp_method_get_param_count: fn(Ptr) -> u32;
    il2cpp_method_get_param: fn(Ptr, u32) -> Ptr;
    il2cpp_method_get_return_type: fn(Ptr) -> Ptr;
    il2cpp_method_is_instance: fn(Ptr) -> bool;
    il2cpp_type_get_type: fn(Ptr) -> i32;
    il2cpp_type_is_byref: fn(Ptr) -> bool;
    il2cpp_type_get_name: fn(Ptr) -> *mut c_char;
    il2cpp_class_get_methods: fn(Ptr, *mut Ptr) -> Ptr;
    il2cpp_runtime_invoke: fn(Ptr, Ptr, *mut Ptr, *mut Ptr) -> Ptr;
    il2cpp_type_get_class_or_element_class: fn(Ptr) -> Ptr;
    il2cpp_class_is_valuetype: fn(Ptr) -> bool;
}
