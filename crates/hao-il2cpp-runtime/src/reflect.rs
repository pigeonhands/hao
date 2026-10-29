//! Reflection roots for the `reflect` feature. See [`hao::reflect`](https://docs.rs/hao/latest/hao/reflect/index.html) for the
//! pattern. Only the plain-data types reflect; the runtime handles (raw
//! pointers, GC handles, hooks) do not.

use facet::{Facet, Shape};

/// The reflected data types describing a method's shape.
pub fn roots() -> &'static [&'static Shape] {
    &[
        <crate::typed::TypeInfo as Facet>::SHAPE,
        <crate::typed::Parameter as Facet>::SHAPE,
    ]
}
