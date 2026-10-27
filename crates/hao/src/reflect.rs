//! Compile-time reflection, via [`facet`], for registering hao's types with an
//! external consumer such as a scripting runtime. Enabled by the `reflect`
//! feature (which also turns on `sync`).
//!
//! Every reflected type derives [`facet::Facet`], so a consumer never registers
//! them one by one: it starts from the shape in [`roots`] and walks that shape's
//! fields and variants (which reference other shapes) to reach every reflected
//! type transitively. The graph is cyclic — a method references its declaring
//! type, which lists the method again — but a shape describes a *type*, not a
//! value, so a walker that dedups by shape pointer terminates.
//!
//! The reflected surface is the modeled metadata graph reachable from
//! [`Module`](crate::Module): its entries (types, methods, fields, ...),
//! signatures and IL. The raw metadata tables and the backing image bytes are
//! load/write details and are left out.
//!
//! Some types are deliberately [opaque](facet::Type::User) — they appear in the
//! graph as sized black boxes rather than exposing their contents:
//!
//! - **bitflags** sets ([`TypeAttributes`](crate::dotnet::entries::TypeAttributes),
//!   etc.): their flag names are `const`s, which no derive can surface. Read them
//!   through [`bitflags::Flags::FLAGS`], which gives each flag's name and value.
//! - The row-id cell on each [`Entry`](crate::dotnet::entries::Entry), an
//!   implementation detail of how entries track their original table row.

use facet::{Facet, Shape};

use crate::Module;

/// The shape of hao's entry-point type. Walk its fields and variants to reach
/// every reflected type in the crate.
pub fn roots() -> &'static [&'static Shape] {
    &[<Module as Facet>::SHAPE]
}
