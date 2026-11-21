//! `react-hooks/config` and `react-hooks/gating` (eslint-plugin-react-hooks
//! 7, `recommended-latest`).
//!
//! Upstream these are React Compiler diagnostics: each lint runs the compiler
//! (with the rule's first option object as compiler options, merged over
//! the plugin's fixed lint defaults) and reports compile errors of its
//! category that have a source location. With the preset's configuration
//! (no options, i.e. no compiler configured) neither can ever report:
//!
//! - `gating` errors come only from dynamic gating directives
//!   (`'use memo if(...)'`), which the compiler only parses when the
//!   `dynamicGating` option is set; it defaults to `null`.
//! - `config` errors come from invalid compiler options (thrown before any
//!   file is compiled, or logged without a location, so never reported), from
//!   a missing filename with `sources` set, or from a module type provider
//!   whose types disagree with hook naming. The default provider only knows
//!   `useForm` / `useReactTable` / `useVirtualizer`, all hooks with hook
//!   names, so imports from those modules can't produce that mismatch.
//!
//! So both rules are registered and report nothing. Options are accepted
//! like upstream's `additionalProperties: true` schema, except the compiler
//! options that could make upstream report (`dynamicGating`, `environment`,
//! `moduleTypeProvider`), which are rejected at config time rather than
//! silently ignored.

use serde_json::Value;

use crate::rule::{Ctx, Rule};

pub struct CompilerCategory {
    pub id: &'static str,
}

impl Rule for CompilerCategory {
    fn name(&self) -> &'static str {
        self.id
    }
    fn applies_to(&self, _rel_path: &str) -> bool {
        // Nothing to report in any file; skip parsing entirely.
        false
    }
    fn validate(&self, options: &[Value], _: &Value) -> Result<(), String> {
        match options {
            [] => Ok(()),
            // Compiler options that make upstream report would need the
            // React Compiler itself; refuse them instead of silently
            // reporting nothing.
            [o] if o.get("dynamicGating").is_some_and(|v| !v.is_null())
                || o.get("moduleTypeProvider").is_some_and(|v| !v.is_null())
                || o.get("environment").is_some_and(|v| !v.is_null()) =>
            {
                Err("compiler options (dynamicGating / environment / moduleTypeProvider) are not supported: \
                     webadeva-lint does not run the React Compiler"
                    .into())
            }
            [o] if o.is_object() => Ok(()),
            _ => Err("expected at most one options object".into()),
        }
    }
    fn run<'a>(&self, _ctx: &mut Ctx<'a>) {}
}
