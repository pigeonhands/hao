//! Reflection roots for the `reflect` feature. See [`hao::reflect`] for the
//! pattern: a consumer walks these shapes to reach every reflected type.

use facet::{Facet, Shape};

/// The entry-point shapes of hao-il2cpp. [`Game`](crate::Game) and
/// [`Binary`](crate::Binary) reach the live views and image structures,
/// [`Metadata`](crate::Metadata) the parsed definitions, and
/// [`GeneratedAssembly`](crate::GeneratedAssembly) the generated output.
pub fn roots() -> &'static [&'static Shape] {
    &[
        <crate::Game as Facet>::SHAPE,
        <crate::Binary as Facet>::SHAPE,
        <crate::Metadata as Facet>::SHAPE,
        <crate::GeneratedAssembly as Facet>::SHAPE,
    ]
}
