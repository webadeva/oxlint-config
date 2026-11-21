//! The rule interface every rule implements, and the per-file context it reads.

use std::path::Path;

use oxc_ast::ast::Program;
use oxc_semantic::Semantic;
use oxc_span::Span;
use serde_json::Value;

use crate::diagnostic::{Diagnostic, Severity};
use crate::fix::{Edit, Fix};

/// A lint rule. Rules are stateless: everything per-file lives in [`Ctx`].
pub trait Rule: Send + Sync {
    /// Full rule id as users configure it, e.g. `webadeva/max-lines`.
    fn name(&self) -> &'static str;

    /// Glob-free file filter, checked before parsing so a rule that only
    /// applies to e.g. `*.stories.tsx` costs nothing elsewhere. Paths are
    /// relative to the config root, with `/` separators.
    fn applies_to(&self, _rel_path: &str) -> bool {
        true
    }

    /// Validates options up front, so a bad config fails once with a clear
    /// message instead of once per file. `options` is the rule's options
    /// array (possibly empty); `settings` is `settings.webadeva` merged.
    fn validate(&self, _options: &[Value], _settings: &Value) -> Result<(), String> {
        Ok(())
    }

    /// Whether the rule reads [`Ctx::tokens`]; tokens are only collected
    /// when some active rule needs them.
    fn needs_tokens(&self) -> bool {
        false
    }

    fn run<'a>(&self, ctx: &mut Ctx<'a>);
}

pub struct Ctx<'a> {
    pub path: &'a Path,
    /// Root directory (where the root config lives / the cwd).
    pub root: &'a Path,
    pub source: &'a str,
    pub program: &'a Program<'a>,
    /// Lexed tokens in source order (empty unless [`Rule::needs_tokens`]).
    /// Comments are not tokens.
    pub tokens: &'a [oxc_parser::Token],
    pub semantic: &'a Semantic<'a>,
    /// The rule's configured options (everything after the severity).
    pub options: &'a [Value],
    /// Merged `settings` object of the config (the whole object, not only
    /// `webadeva`), e.g. `settings.react.version`.
    pub settings: &'a Value,
    pub(crate) rule_name: &'static str,
    pub(crate) severity: Severity,
    pub(crate) diagnostics: Vec<Diagnostic>,
}

impl<'a> Ctx<'a> {
    pub fn report(&mut self, span: Span, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic {
            rule: self.rule_name,
            severity: self.severity,
            message: message.into(),
            start: span.start,
            end: span.end,
            fix: None,
        });
    }

    /// Reports with an autofix (the upstream rule's `fix`, not a suggestion).
    /// `--fix` applies it; the edits refer to offsets in [`Ctx::source`].
    pub fn report_with_fix(&mut self, span: Span, message: impl Into<String>, edits: Vec<Edit>) {
        self.diagnostics.push(Diagnostic {
            rule: self.rule_name,
            severity: self.severity,
            message: message.into(),
            start: span.start,
            end: span.end,
            fix: (!edits.is_empty()).then(|| Fix::new(edits)),
        });
    }

    /// First option object, or `Value::Null` when absent.
    pub fn option(&self) -> &Value {
        self.options.first().unwrap_or(&Value::Null)
    }

    pub fn source_text(&self, span: Span) -> &'a str {
        &self.source[span.start as usize..span.end as usize]
    }
}
