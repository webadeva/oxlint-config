//! Rule registry. Each group lives in its own module behind a Cargo feature
//! (all on by default), so groups can be built and tested independently.

#[cfg(feature = "core")]
pub mod core;
#[cfg(feature = "react_components")]
pub mod react_components;
#[cfg(feature = "react_dom")]
pub mod react_dom;
#[cfg(feature = "storybook")]
pub mod storybook;

use crate::rule::Rule;

pub fn all() -> Vec<Box<dyn Rule>> {
    #[allow(unused_mut)]
    let mut rules: Vec<Box<dyn Rule>> = Vec::new();
    #[cfg(feature = "core")]
    rules.extend(core::rules());
    #[cfg(feature = "react_dom")]
    rules.extend(react_dom::rules());
    #[cfg(feature = "react_components")]
    rules.extend(react_components::rules());
    #[cfg(feature = "storybook")]
    rules.extend(storybook::rules());
    rules
}
