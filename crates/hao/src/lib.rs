//! `hao` reads, edits, creates and writes .NET assemblies.
//!
//! The entry point is [`Module`]:
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
//! - [`dotnet::entries`]: types, methods, fields and the other metadata entries, as shared
//!   [`Entry`](dotnet::entries::Entry) handles, plus custom attribute values.
//! - [`dotnet::signature`]: type and method signatures.
//! - [`dotnet::il`]: method bodies, opcodes, and the [`Emitter`](dotnet::il::Emitter) for
//!   building new ones.
//! - [`dotnet::module`]: creating modules ([`Module::new`]), importing references and resolving
//!   assemblies.
//! - [`dotnet::csharp`]: C#-like output of declarations and IL.
//! - [`dotnet::md`]: the raw metadata tables and heaps.
//!
//! Modules are written with [`Module::write`], which rebuilds the metadata from the entries.
//!
//! Entries are backed by `Rc<RefCell<_>>`, or `Arc<RwLock<_>>` with the `sync` feature.
//!
//! Loading is strict: malformed metadata is an error rather than being skipped.

pub mod dotnet;
pub mod error;
pub mod io;
#[cfg(feature = "reflect")]
pub mod reflect;
pub mod sync;

pub use dotnet::{module::resolver, Module};
