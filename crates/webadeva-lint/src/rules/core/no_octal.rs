//! ESLint's `no-octal`: legacy octal literals (`/^0\d/`). In strict code
//! (modules, classes, `"use strict"`) they are syntax errors, which oxlint
//! already reports (and which ESLint's TypeScript parser reports as a parsing
//! error), so this only fires in sloppy code (e.g. `.cjs` scripts).

use oxc_ast::AstKind;

use crate::rule::{Ctx, Rule};

pub struct NoOctal;

impl Rule for NoOctal {
    fn name(&self) -> &'static str {
        "no-octal"
    }

    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        let mut reports = Vec::new();
        for node in ctx.semantic.nodes().iter() {
            if let AstKind::NumericLiteral(n) = node.kind() {
                if ctx
                    .semantic
                    .scoping()
                    .scope_flags(node.scope_id())
                    .is_strict_mode()
                {
                    continue;
                }
                let raw = ctx.source_text(n.span).as_bytes();
                if raw.len() >= 2 && raw[0] == b'0' && raw[1].is_ascii_digit() {
                    reports.push(n.span);
                }
            }
        }
        for span in reports {
            ctx.report(span, "Octal literals should not be used.");
        }
    }
}
