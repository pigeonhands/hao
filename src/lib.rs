//! `hao` is a library for reading and writing .net modules and assemblies in rust.
//!
//! Currently, `hao` is quite struct with the binaries it recieved, erroring with malfomed
//! binaries rather than attempting to ignore the issues. Though the plan is to impliment
//! a feature flag that can optionally continue parsing the file and expose the raw, non-sanitary
//! parts that it encounters.
//!
//! The current state of this library is read-only with .net framework/core files and largely untested with mono, however
//! mono support is definetly going to be supported in the future.
//!
//! The entrypoint for this library is the [`Module`] struct. You can get started with loading a module like so:
//!
//! ```no_run
//! use hao::{dotnet::md::streams::tables_stream::FieldFlags, Module};
//!
//! fn main() {
//!     let module = Module::from_path(r#"C:\re\dnspy\bin\dnlib.dll"#).unwrap();
//!     println!("loaded");
//!
//!     for ty in module.types().values() {
//!         println!("{} {{", ty);
//!         if ty.is_enum() {
//!             for field in ty
//!                 .fields()
//!                 .values()
//!                 .filter(|x| !x.flags().contains(FieldFlags::SpecialName))
//!             {
//!                 println!("\t{},", field.name());
//!             }
//!         } else {
//!             for field in ty.fields().values() {
//!                 println!("\t{};", field);
//!             }
//!         }
//!
//!         println!("}}");
//!     }
//! }
//! ```
//!
//! this will print out all the types close-to c# syntax.

pub mod dotnet;
pub mod error;
pub mod io;

pub use dotnet::{module::resolver, Module};
