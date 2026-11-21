//! eslint-plugin-react-rsc `function-definition` (registered as
//! `@eslint-react/rsc-function-definition`).

use oxc_ast::AstKind;
use oxc_ast::Comment;
use oxc_ast::ast::*;
use oxc_semantic::Semantic;
use oxc_span::{GetSpan, Span};
use serde_json::Value;

use super::helpers::*;
use super::no_options;
use crate::fix::Edit;
use crate::rule::{Ctx, Rule};

pub struct FunctionDefinition;

enum DirectiveKind {
    Misplaced,
    Backtick,
}

/// `matchDirective` for a statement outside the directive prologue (oxc
/// keeps well-formed directives separately).
fn match_statement(stmt: &Statement<'_>) -> Option<(DirectiveKind, &'static str, Span)> {
    let Statement::ExpressionStatement(es) = stmt else {
        return None;
    };
    let name = |v: &str| match v {
        "use client" => Some("use client"),
        "use server" => Some("use server"),
        _ => None,
    };
    match strip_parens(&es.expression) {
        Expression::StringLiteral(s) => {
            Some((DirectiveKind::Misplaced, name(s.value.as_str()?)?, es.span))
        }
        Expression::TemplateLiteral(t) if t.expressions.is_empty() && t.quasis.len() == 1 => {
            let cooked = t.quasis[0].value.cooked.as_ref().and_then(|c| c.as_str())?;
            Some((DirectiveKind::Backtick, name(cooked)?, es.span))
        }
        _ => None,
    }
}

const FILE: &str = "Functions exported from files with `use server` directive are React Server Functions and therefore must be async.";
const LOCAL: &str =
    "Functions with `use server` directive are React Server Functions and therefore must be async.";

/// `reportNonAsyncFunction` target: the unwrapped expression if it is a
/// non-async function.
fn non_async_expr<'a>(e: &'a Expression<'a>) -> Option<Func<'a>> {
    match unwrap(e) {
        Un::Expr(Expression::ArrowFunctionExpression(a)) if !a.r#async => Some(Func::A(a)),
        Un::Expr(Expression::FunctionExpression(f)) if !f.r#async && f.body.is_some() => {
            Some(Func::F(f))
        }
        _ => None,
    }
}

fn non_async_resolved<'a>(sem: &Semantic<'a>, id: &IdentifierReference<'_>) -> Option<Func<'a>> {
    match resolve(sem, id) {
        Resolved::Expr(e) => non_async_expr(e),
        Resolved::Func(f) if !f.is_async() => Some(f),
        _ => None,
    }
}

/// Start of the token before `offset` (skipping whitespace and comments),
/// with its first byte.
fn prev_token(src: &str, comments: &[Comment], mut offset: u32) -> Option<(u32, u8)> {
    let bytes = src.as_bytes();
    loop {
        while offset > 0 && bytes[offset as usize - 1].is_ascii_whitespace() {
            offset -= 1;
        }
        match comments.iter().find(|c| c.span.end == offset) {
            Some(c) => offset = c.span.start,
            None => break,
        }
    }
    (offset > 0).then(|| (offset - 1, bytes[offset as usize - 1]))
}

/// `buildFixForAsync`: where upstream inserts `async `, if it offers a fix.
fn async_fix_offset(ctx: &Ctx<'_>, func: Func<'_>) -> Option<u32> {
    let f = match func {
        Func::A(a) => return Some(a.span.start),
        Func::F(f) => f,
    };
    let nodes = ctx.semantic.nodes();
    if f.r#type == FunctionType::FunctionExpression {
        match nodes.parent_kind(f.node_id()) {
            AstKind::ObjectProperty(p) => {
                if p.kind != PropertyKind::Init {
                    return None;
                }
                if p.method {
                    return Some(p.span.start);
                }
            }
            AstKind::MethodDefinition(m) => {
                if m.kind != MethodDefinitionKind::Method {
                    return None;
                }
                let comments = &ctx.program.comments;
                let mut target = m.key.span().start;
                if m.computed {
                    let (pos, b) = prev_token(ctx.source, comments, target)?;
                    if b != b'[' {
                        return None;
                    }
                    target = pos;
                }
                if f.generator {
                    let (pos, b) = prev_token(ctx.source, comments, target)?;
                    if b != b'*' {
                        return None;
                    }
                    target = pos;
                }
                return Some(target);
            }
            _ => {}
        }
    }
    // `getFirstToken(node)`: the function's own start.
    Some(f.span.start)
}

impl Rule for FunctionDefinition {
    fn name(&self) -> &'static str {
        "@eslint-react/rsc-function-definition"
    }
    fn validate(&self, options: &[Value], _: &Value) -> Result<(), String> {
        no_options(options)
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        let src = ctx.source;
        if !src.contains("use server") && !src.contains("use client") {
            return;
        }
        let program = ctx.program;
        let sem = ctx.semantic;
        let file_use_server = program
            .directives
            .iter()
            .any(|d| d.directive.as_str() == "use server");
        let mut reports: Vec<(Span, String, Option<Func<'a>>)> = Vec::new();

        for stmt in &program.body {
            if let Some((kind, name, span)) = match_statement(stmt) {
                let msg = match kind {
                    DirectiveKind::Backtick => {
                        format!(
                            "The '{name}' directive must be written with single or double quotes, not backticks."
                        )
                    }
                    DirectiveKind::Misplaced => format!(
                        "The '{name}' directive must be at the very beginning of the file, before any imports or other code."
                    ),
                };
                reports.push((span, msg, None));
            }
        }

        for node in sem.nodes().iter() {
            let k = node.kind();
            if let Some(func) = as_func(k) {
                let Some(body) = func.block_body() else {
                    continue;
                };
                // Directives and statements in source order.
                for d in &body.directives {
                    if d.directive.as_str() == "use client" {
                        reports.push((
                            d.span,
                            "The 'use client' directive can only be used at the top of a file, not inside a function body.".into(),
                            None,
                        ));
                    }
                }
                for stmt in &body.statements {
                    let Some((kind, name, span)) = match_statement(stmt) else {
                        continue;
                    };
                    let msg = match kind {
                        DirectiveKind::Backtick => {
                            format!("The '{name}' directive must be written with single or double quotes, not backticks.")
                        }
                        DirectiveKind::Misplaced if name == "use client" => {
                            "The 'use client' directive can only be used at the top of a file, not inside a function body.".into()
                        }
                        DirectiveKind::Misplaced => {
                            format!("The '{name}' directive must be at the very beginning of the function body.")
                        }
                    };
                    reports.push((span, msg, None));
                }
                if func.directives().any(|d| d == "use server") && !func.is_async() {
                    reports.push((func.span(), LOCAL.into(), Some(func)));
                }
                continue;
            }
            if !file_use_server {
                continue;
            }
            match k {
                AstKind::ExportDefaultDeclaration(e) => {
                    let target = match &e.declaration {
                        ExportDefaultDeclarationKind::FunctionDeclaration(f) => {
                            (!f.r#async && f.body.is_some()).then_some(Func::F(f))
                        }
                        ExportDefaultDeclarationKind::ClassDeclaration(_)
                        | ExportDefaultDeclarationKind::TSInterfaceDeclaration(_) => None,
                        other => {
                            let expr = other.to_expression();
                            match unwrap(expr) {
                                Un::Expr(Expression::Identifier(id)) => non_async_resolved(sem, id),
                                _ => non_async_expr(expr),
                            }
                        }
                    };
                    if let Some(f) = target {
                        reports.push((f.span(), FILE.into(), Some(f)));
                    }
                }
                AstKind::ExportDeclaration(e) => match &e.declaration {
                    Declaration::VariableDeclaration(v) => {
                        for d in &v.declarations {
                            if let Some(f) = d.init.as_ref().and_then(non_async_expr) {
                                reports.push((f.span(), FILE.into(), Some(f)));
                            }
                        }
                    }
                    Declaration::FunctionDeclaration(f) if !f.r#async && f.body.is_some() => {
                        reports.push((f.span, FILE.into(), Some(Func::F(f))));
                    }
                    _ => {}
                },
                AstKind::ExportNamedDeclaration(e) => {
                    for spec in &e.specifiers {
                        if let ModuleExportName::IdentifierReference(id) = &spec.local
                            && let Some(f) = non_async_resolved(sem, id)
                        {
                            reports.push((f.span(), FILE.into(), Some(f)));
                        }
                    }
                }
                _ => {}
            }
        }
        for (s, m, func) in reports {
            match func.and_then(|f| async_fix_offset(ctx, f)) {
                Some(at) => ctx.report_with_fix(s, m, vec![Edit::insert_before(at, "async ")]),
                None => ctx.report(s, m),
            }
        }
    }
}
