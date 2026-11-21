//! Parses a file once with oxc and runs the enabled rules on it. Shared by
//! the Node add-on (what oxlint runs) and the parity-test harness.

use std::path::Path;

use oxc_allocator::Allocator;
use oxc_ast::ast::CommentKind;
use oxc_parser::{Parser, config::RuntimeParserConfig};
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use serde_json::Value;

use crate::diagnostic::{Diagnostic, Severity};
use crate::directives::{self, Directive};
use crate::rule::{Ctx, Rule};

/// A rule enabled for the file being linted.
pub struct ActiveRule<'r> {
    pub rule: &'r dyn Rule,
    pub severity: Severity,
    pub options: &'r [Value],
}

pub struct Linted {
    pub diagnostics: Vec<Diagnostic>,
    /// Disable directives in the file. oxlint applies these itself for the
    /// add-on; the harness applies them the way ESLint does.
    pub directives: Vec<Directive>,
}

/// Runs `active` on `source`. Files that don't parse produce nothing: oxlint
/// reports parse errors itself.
pub fn run(
    path: &Path,
    root: &Path,
    source: &str,
    active: &[ActiveRule<'_>],
    settings: &Value,
) -> Linted {
    let empty = Linted {
        diagnostics: Vec::new(),
        directives: Vec::new(),
    };
    if active.is_empty() {
        return empty;
    }
    let source_type = SourceType::from_path(path).unwrap_or_default();
    let allocator = Allocator::default();
    let needs_tokens = active.iter().any(|a| a.rule.needs_tokens());
    let parsed = Parser::new(&allocator, source, source_type)
        .with_config(RuntimeParserConfig::new(needs_tokens))
        .parse();
    if parsed.fatal_error || !parsed.diagnostics.is_empty() {
        return empty;
    }
    let tokens = parsed.tokens;
    let program = parsed.program;
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&program)
        .semantic;

    let mut diagnostics = Vec::new();
    for a in active {
        let mut ctx = Ctx {
            path,
            root,
            source,
            program: &program,
            tokens: &tokens,
            semantic: &semantic,
            options: a.options,
            settings,
            rule_name: a.rule.name(),
            severity: a.severity,
            diagnostics: Vec::new(),
        };
        a.rule.run(&mut ctx);
        diagnostics.append(&mut ctx.diagnostics);
    }

    let comments: Vec<(usize, usize, bool, &str)> = program
        .comments
        .iter()
        .map(|c| {
            let (s, e) = (c.span.start as usize, c.span.end as usize);
            let line = c.kind == CommentKind::Line;
            // Strip `//` or `/*` `*/`.
            let inner = if line {
                &source[s + 2..e]
            } else {
                &source[s + 2..e.saturating_sub(2).max(s + 2)]
            };
            (s, e, line, inner)
        })
        .collect();
    Linted {
        diagnostics,
        directives: directives::collect(&comments),
    }
}
