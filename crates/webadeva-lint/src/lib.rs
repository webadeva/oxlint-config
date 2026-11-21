//! Rules of @webadeva/eslint-config that oxlint lacks (or implements
//! differently), built on oxc's parser.
//!
//! Shipped as a Node add-on (`napi` feature) that the package's oxlint JS
//! plugin calls once per file, so the rules run inside `oxlint` and are
//! configured in `.oxlintrc.json` like any other rule. The `cli` feature
//! builds a harness binary that runs them the way ESLint would, for the
//! parity tests.

pub mod diagnostic;
pub mod directives;
pub mod engine;
pub mod fix;
pub mod rule;
pub mod rules;
pub mod utils;

#[cfg(feature = "cli")]
pub mod config;
#[cfg(feature = "cli")]
pub mod linter;
#[cfg(feature = "napi")]
pub mod napi;
