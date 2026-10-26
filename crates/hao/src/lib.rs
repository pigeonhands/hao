//! `hao` is a library for reading and writing .net modules and assemblies in rust.
//!
//! Currently, `hao` is quite struct with the binaries it recieved, erroring with malfomed
//! binaries rather than attempting to ignore the issues. Though the plan is to impliment
//! a feature flag that can optionally continue parsing the file and expose the raw, non-sanitary
//! parts that it encounters.
//!
//! The entrypoint for this library is the [`Module`] struct:
//!
//! ```no_run
//! use hao::Module;
//!
//! let module = Module::from_path("Example.dll").unwrap();
//!
//! for ty in module.all_types() {
//!     let ty = ty.get();
//!     println!("{}", ty.full_name());
//!     for method in &ty.methods {
//!         println!("\t{}", method.get().name);
//!     }
//! }
//! ```
//!
//! Entries are shared handles ([`dotnet::entries::Entry`]) backed by `Rc<RefCell<_>>`,
//! or `Arc<RwLock<_>>` with the `sync` feature.

pub mod dotnet;
pub mod error;
pub mod io;
pub mod sync;

pub use dotnet::{module::resolver, Module};
