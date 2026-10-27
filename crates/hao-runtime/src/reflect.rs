//! Reflection roots for the `reflect` feature. See [`hao::reflect`] for the
//! pattern. The runtime handles (`Runtime`, `ClrHost`, the backends) hold raw
//! pointers and library handles and are not reflected; the configuration and
//! descriptor types are.

use facet::{Facet, Shape};

/// The reflected entry-point shapes. Which are present depends on the enabled
/// backend features. When the `hao` feature is on, [`hao::reflect::roots`] gives
/// the metadata graph as well.
pub fn roots() -> Vec<&'static Shape> {
    #[allow(unused_mut)]
    let mut roots: Vec<&'static Shape> = vec![
        <crate::Backend as Facet>::SHAPE,
        <crate::BoundAssembly as Facet>::SHAPE,
    ];
    #[cfg(feature = "coreclr")]
    roots.extend([
        <crate::CoreClrOptions as Facet>::SHAPE,
        <crate::profiler::Guid as Facet>::SHAPE,
    ]);
    #[cfg(feature = "mono")]
    roots.push(<crate::MonoOptions as Facet>::SHAPE);
    roots
}
