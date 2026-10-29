//! Reflection roots for the `reflect` feature. See [`hao::reflect`](https://docs.rs/hao/latest/hao/reflect/index.html) for the
//! pattern. The overlay and binding handles hold engine pointers and are not
//! reflected; the plain-data types are.

use facet::{Facet, Shape};

/// The reflected data types: the active [`Scene`](crate::Scene) and the GL draw
/// [`Mode`](crate::gl::Mode).
pub fn roots() -> &'static [&'static Shape] {
    &[
        <crate::Scene as Facet>::SHAPE,
        <crate::gl::Mode as Facet>::SHAPE,
    ]
}
