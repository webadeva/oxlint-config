//! ESLint's `no-dupe-args`. Duplicate parameters are only legal in sloppy
//! code; oxc parses them everywhere without a parse error (its semantic
//! checker reports them, which we don't run), so this fires where ESLint's
//! TypeScript parser lets them through.

use oxc_ast::AstKind;
use oxc_ast::ast::{BindingPattern, FormalParameters, FunctionType};
use oxc_span::{GetSpan, Span};

use super::util::{next_token_start, prev_token_end};
use crate::rule::{Ctx, Rule};

pub struct NoDupeArgs;

fn bound_names<'a>(pattern: &BindingPattern<'a>, out: &mut Vec<&'a str>) {
    match pattern {
        BindingPattern::BindingIdentifier(id) => out.push(id.name.as_str()),
        BindingPattern::ObjectPattern(o) => {
            for p in &o.properties {
                bound_names(&p.value, out);
            }
            if let Some(r) = &o.rest {
                bound_names(&r.argument, out);
            }
        }
        BindingPattern::ArrayPattern(a) => {
            for e in a.elements.iter().flatten() {
                bound_names(e, out);
            }
            if let Some(r) = &a.rest {
                bound_names(&r.argument, out);
            }
        }
        BindingPattern::AssignmentPattern(a) => bound_names(&a.left, out),
    }
}

/// Names bound more than once by a parameter list, in first-seen order.
pub fn duplicate_params<'a>(params: &FormalParameters<'a>) -> Vec<&'a str> {
    let mut names = Vec::new();
    for p in &params.items {
        bound_names(&p.pattern, &mut names);
    }
    if let Some(rest) = &params.rest {
        bound_names(&rest.rest.argument, &mut names);
    }
    let mut dupes: Vec<&str> = Vec::new();
    for (i, n) in names.iter().enumerate() {
        if names[..i].contains(n) && !dupes.contains(n) {
            dupes.push(n);
        }
    }
    // Report in declaration order of the first occurrence.
    dupes.sort_by_key(|d| names.iter().position(|n| n == d));
    dupes
}

impl Rule for NoDupeArgs {
    fn name(&self) -> &'static str {
        "no-dupe-args"
    }

    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        let mut reports = Vec::new();
        for node in ctx.semantic.nodes().iter() {
            let AstKind::Function(f) = node.kind() else {
                continue;
            };
            if !matches!(
                f.r#type,
                FunctionType::FunctionDeclaration | FunctionType::FunctionExpression
            ) {
                continue;
            }
            let Some(body) = &f.body else { continue };
            let dupes = duplicate_params(&f.params);
            if dupes.is_empty() {
                continue;
            }
            // From the opening paren of the parameters to the token before the body.
            let after = f
                .type_parameters
                .as_ref()
                .map(|t| t.span.end)
                .or(f.id.as_ref().map(|id| id.span.end))
                .unwrap_or(f.span.start);
            let mut start = f.params.span.start;
            if ctx.source.as_bytes().get(start as usize) != Some(&b'(') {
                let s = next_token_start(ctx.source, &ctx.program.comments, after);
                start = ctx.source[s as usize..]
                    .find('(')
                    .map_or(s, |i| s + i as u32);
            }
            let end = prev_token_end(ctx.source, &ctx.program.comments, body.span().start);
            for name in dupes {
                reports.push((Span::new(start, end), format!("Duplicate param '{name}'.")));
            }
        }
        for (span, msg) in reports {
            ctx.report(span, msg);
        }
    }
}
