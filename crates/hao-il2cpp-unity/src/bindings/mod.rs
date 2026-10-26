//! Unity method bindings, grouped by area into one module per concern.
//!
//! Each binding is looked up by name and checked against the game's signature
//! the first time it's used, then cached (see [`binding`](crate::binding)). So a
//! mod only pays for the methods it actually calls.

mod component;
mod engine;
mod game_object;
mod input;
mod object;
mod rendering;
mod scene;
mod transform;
mod ui;

pub(crate) use self::{
    component::*, engine::*, game_object::*, input::*, object::*, rendering::*, scene::*,
    transform::*, ui::*,
};

/// Every binding, for [`check_bindings`](crate::check_bindings).
pub(crate) fn all() -> Vec<&'static dyn crate::binding::Check> {
    [
        object_checks(),
        game_object_checks(),
        component_checks(),
        transform_checks(),
        ui_checks(),
        scene_checks(),
        input_checks(),
        rendering_checks(),
        engine_checks(),
    ]
    .into_iter()
    .flatten()
    .collect()
}
