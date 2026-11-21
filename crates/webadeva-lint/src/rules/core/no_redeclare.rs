//! ESLint's `no-redeclare` as the ESLint config runs it: the core rule (not
//! `@typescript-eslint/no-redeclare`) on typescript-eslint's scope manager,
//! with `builtinGlobals`.
//!
//! Oxlint's own `no-redeclare` reports the *first* declaration of a name and
//! skips TypeScript declaration merges. ESLint reports every declaration
//! *after* the first, merges included (`const X = {…}; type X = …` reports the
//! `type`), so `eslint-disable-next-line no-redeclare` comments written for
//! ESLint sit above the line this rule reports.
//!
//! What counts, as in typescript-eslint's scope manager:
//! - Declarations of one name in one scope: `var` (hoisted to the function),
//!   `let`/`const`, functions (overload signatures included), classes,
//!   parameters, imports, `import x = …`, type aliases, interfaces, enums and
//!   namespaces. A function's parameters come before its type parameters.
//! - Only the scopes ESLint's rule visits: the module, functions, blocks,
//!   `for` with `let`/`const`, `switch` and static blocks. Class, catch,
//!   namespace, `declare module` and `declare global` scopes are not checked.
//! - `/* global */` comments, against each other and against the configured
//!   globals ([`super::builtin_globals`]).
//! - The reported identifier includes its `?` and type annotation, as in
//!   typescript-estree.

use oxc_ast::ast::{
    ArrowFunctionExpression, BindingPattern, BlockStatement, Class, ForInStatement, ForOfStatement,
    ForStatement, ForStatementInit, ForStatementLeft, FormalParameters, Function, FunctionType,
    ImportDeclaration, ImportDeclarationSpecifier, StaticBlock, SwitchStatement, TSEnumDeclaration,
    TSExternalModuleDeclaration, TSGlobalDeclaration, TSImportEqualsDeclaration,
    TSInterfaceDeclaration, TSNamespaceDeclaration, TSNamespaceDeclarationBody,
    TSTypeAliasDeclaration, TSTypeParameterDeclaration, VariableDeclaration,
    VariableDeclarationKind,
};
use oxc_ast_visit::{Visit, walk};
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;

use super::builtin_globals::is_builtin_global;
use crate::rule::{Ctx, Rule};

pub struct NoRedeclare;

struct Frame<'a> {
    /// Whether ESLint's rule looks at this scope.
    checked: bool,
    /// Whether `var` declarations stop here.
    var_scope: bool,
    decls: Vec<(&'a str, Span)>,
}

#[derive(Default)]
struct Collector<'a> {
    frames: Vec<Frame<'a>>,
    reports: Vec<(Span, String)>,
}

fn pattern_names<'a>(pattern: &BindingPattern<'a>, out: &mut Vec<(&'a str, Span)>) {
    match pattern {
        BindingPattern::BindingIdentifier(id) => out.push((id.name.as_str(), id.span)),
        BindingPattern::ObjectPattern(o) => {
            for p in &o.properties {
                pattern_names(&p.value, out);
            }
            if let Some(r) = &o.rest {
                pattern_names(&r.argument, out);
            }
        }
        BindingPattern::ArrayPattern(a) => {
            for e in a.elements.iter().flatten() {
                pattern_names(e, out);
            }
            if let Some(r) = &a.rest {
                pattern_names(&r.argument, out);
            }
        }
        BindingPattern::AssignmentPattern(a) => pattern_names(&a.left, out),
    }
}

/// Names a pattern binds. A bare identifier's span is widened to `end` (its
/// `?` or type annotation), as typescript-estree's `Identifier` range is.
fn names_with_end<'a>(pattern: &BindingPattern<'a>, end: Option<u32>) -> Vec<(&'a str, Span)> {
    let mut out = Vec::new();
    pattern_names(pattern, &mut out);
    if let (BindingPattern::BindingIdentifier(_), Some(end), [only]) = (pattern, end, &mut out[..])
    {
        only.1 = Span::new(only.1.start, end);
    }
    out
}

impl<'a> Collector<'a> {
    fn push(&mut self, checked: bool, var_scope: bool) {
        self.frames.push(Frame {
            checked,
            var_scope,
            decls: Vec::new(),
        });
    }

    fn pop(&mut self) {
        let frame = self.frames.pop().expect("balanced scopes");
        if !frame.checked {
            return;
        }
        let mut seen: Vec<&str> = Vec::new();
        for (name, span) in frame.decls {
            if seen.contains(&name) {
                self.reports
                    .push((span, format!("'{name}' is already defined.")));
            } else {
                seen.push(name);
            }
        }
    }

    fn declare(&mut self, name: &'a str, span: Span, var: bool) {
        let frame = if var {
            self.frames.iter_mut().rev().find(|f| f.var_scope)
        } else {
            self.frames.last_mut()
        };
        if let Some(frame) = frame {
            frame.decls.push((name, span));
        }
    }

    fn declare_params(&mut self, params: &FormalParameters<'a>) {
        for p in &params.items {
            let end = p
                .type_annotation
                .as_ref()
                .map(|t| t.span.end)
                .or(p.optional.then(|| p.pattern.span().end + 1));
            for (name, span) in names_with_end(&p.pattern, end) {
                self.declare(name, span, false);
            }
        }
        if let Some(rest) = &params.rest {
            for (name, span) in names_with_end(&rest.rest.argument, None) {
                self.declare(name, span, false);
            }
        }
    }

    fn declare_type_params(&mut self, tps: Option<&TSTypeParameterDeclaration<'a>>) {
        for tp in tps.iter().flat_map(|t| t.params.iter()) {
            self.declare(tp.name.name.as_str(), tp.name.span, false);
        }
    }

    /// A namespace body; `namespace A.B {}` nests B's body inside A's.
    fn visit_namespace_body(&mut self, body: &TSNamespaceDeclarationBody<'a>) {
        match body {
            TSNamespaceDeclarationBody::TSModuleBlock(b) => self.visit_ts_module_block(b),
            TSNamespaceDeclarationBody::TSNamespaceDeclaration(inner) => {
                self.visit_namespace_body(&inner.body);
            }
        }
    }
}

impl<'a> Visit<'a> for Collector<'a> {
    fn visit_program(&mut self, it: &oxc_ast::ast::Program<'a>) {
        self.push(true, true);
        walk::walk_program(self, it);
        self.pop();
    }

    fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'a>) {
        let var = it.kind == VariableDeclarationKind::Var;
        for d in &it.declarations {
            let end = d.type_annotation.as_ref().map(|t| t.span.end);
            for (name, span) in names_with_end(&d.id, end) {
                self.declare(name, span, var);
            }
        }
        walk::walk_variable_declaration(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, _flags: ScopeFlags) {
        match it.r#type {
            FunctionType::FunctionDeclaration | FunctionType::TSDeclareFunction => {
                if let Some(id) = &it.id {
                    self.declare(id.name.as_str(), id.span, false);
                }
            }
            FunctionType::FunctionExpression => {}
            FunctionType::TSEmptyBodyFunctionExpression => return,
        }
        let Some(body) = &it.body else { return };
        self.push(true, true);
        self.declare_params(&it.params);
        self.declare_type_params(it.type_parameters.as_deref());
        self.visit_formal_parameters(&it.params);
        // The body shares the function's scope: no block frame for it.
        for s in &body.statements {
            self.visit_statement(s);
        }
        self.pop();
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.push(true, true);
        self.declare_params(&it.params);
        self.declare_type_params(it.type_parameters.as_deref());
        self.visit_formal_parameters(&it.params);
        // A block body is a `FunctionBody`, not a block: no frame of its own.
        self.visit_arrow_function_body(&it.body);
        self.pop();
    }

    fn visit_block_statement(&mut self, it: &BlockStatement<'a>) {
        self.push(true, false);
        walk::walk_block_statement(self, it);
        self.pop();
    }

    fn visit_static_block(&mut self, it: &StaticBlock<'a>) {
        self.push(true, true);
        walk::walk_static_block(self, it);
        self.pop();
    }

    fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
        let scoped = matches!(&it.init, Some(ForStatementInit::VariableDeclaration(d)) if d.kind != VariableDeclarationKind::Var);
        self.push(scoped, false);
        walk::walk_for_statement(self, it);
        self.pop();
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        let scoped = matches!(&it.left, ForStatementLeft::VariableDeclaration(d) if d.kind != VariableDeclarationKind::Var);
        self.push(scoped, false);
        walk::walk_for_in_statement(self, it);
        self.pop();
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        let scoped = matches!(&it.left, ForStatementLeft::VariableDeclaration(d) if d.kind != VariableDeclarationKind::Var);
        self.push(scoped, false);
        walk::walk_for_of_statement(self, it);
        self.pop();
    }

    fn visit_switch_statement(&mut self, it: &SwitchStatement<'a>) {
        self.visit_expression(&it.discriminant);
        self.push(true, false);
        for case in &it.cases {
            self.visit_switch_case(case);
        }
        self.pop();
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        if it.is_declaration()
            && let Some(id) = &it.id
        {
            self.declare(id.name.as_str(), id.span, false);
        }
        self.push(false, false);
        walk::walk_class(self, it);
        self.pop();
    }

    fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {
        for s in it.specifiers.iter().flatten() {
            let local = match s {
                ImportDeclarationSpecifier::ImportSpecifier(s) => &s.local,
                ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => &s.local,
                ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => &s.local,
            };
            self.declare(local.name.as_str(), local.span, false);
        }
    }

    fn visit_ts_import_equals_declaration(&mut self, it: &TSImportEqualsDeclaration<'a>) {
        self.declare(it.id.name.as_str(), it.id.span, false);
    }

    fn visit_ts_type_alias_declaration(&mut self, it: &TSTypeAliasDeclaration<'a>) {
        self.declare(it.id.name.as_str(), it.id.span, false);
    }

    fn visit_ts_interface_declaration(&mut self, it: &TSInterfaceDeclaration<'a>) {
        self.declare(it.id.name.as_str(), it.id.span, false);
    }

    fn visit_ts_enum_declaration(&mut self, it: &TSEnumDeclaration<'a>) {
        self.declare(it.id.name.as_str(), it.id.span, false);
        self.push(false, true);
        walk::walk_ts_enum_declaration(self, it);
        self.pop();
    }

    fn visit_ts_namespace_declaration(&mut self, it: &TSNamespaceDeclaration<'a>) {
        self.declare(it.id.name.as_str(), it.id.span, false);
        self.push(false, true);
        self.visit_namespace_body(&it.body);
        self.pop();
    }

    fn visit_ts_external_module_declaration(&mut self, it: &TSExternalModuleDeclaration<'a>) {
        self.push(false, true);
        walk::walk_ts_external_module_declaration(self, it);
        self.pop();
    }

    fn visit_ts_global_declaration(&mut self, it: &TSGlobalDeclaration<'a>) {
        self.push(false, true);
        walk::walk_ts_global_declaration(self, it);
        self.pop();
    }
}

/// `/* global a, b:writable */` comments: per name, the comment spans that
/// declare it (the name's position in the comment) and its final value.
struct InlineGlobal {
    name: String,
    spans: Vec<Span>,
    off: bool,
}

/// ESLint's directive parsing (`@eslint/plugin-kit`'s `parseDirective` and
/// `parseStringConfig`) for one block comment's text, when it is a
/// `global`/`globals` directive: `(name, value)` pairs.
fn parse_global_directive(value: &str) -> Option<Vec<(String, Option<String>)>> {
    let directive = split_justification(value).trim();
    let label_end = directive
        .find(|c: char| c.is_whitespace())
        .unwrap_or(directive.len());
    let label = &directive[..label_end];
    if label != "global" && label != "globals" {
        return None;
    }
    let collapsed = collapse_separators(directive[label_end..].trim());
    let mut items: Vec<(String, Option<String>)> = Vec::new();
    for item in collapsed.split(|c: char| c.is_whitespace() || c == ',') {
        if item.is_empty() {
            continue;
        }
        let mut parts = item.split(':');
        let key = parts.next().unwrap_or_default().to_string();
        let val = parts.next().map(str::to_string);
        if let Some(existing) = items.iter_mut().find(|(k, _)| *k == key) {
            existing.1 = val;
        } else {
            items.push((key, val));
        }
    }
    Some(items)
}

/// The text before a ` -- justification` (`/\s-{2,}\s/`).
fn split_justification(value: &str) -> &str {
    let bytes = value.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = value[i..].chars().next().unwrap_or(' ');
        if c.is_whitespace() {
            let after = i + c.len_utf8();
            let dashes = value[after..].bytes().take_while(|b| *b == b'-').count();
            if dashes >= 2
                && value[after + dashes..]
                    .chars()
                    .next()
                    .is_some_and(char::is_whitespace)
            {
                return &value[..i];
            }
        }
        i += c.len_utf8();
    }
    value
}

/// `string.replace(/(?<!\s)\s*([:,])\s*/gu, "$1")`.
fn collapse_separators(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        let prev_ws = i > 0 && chars[i - 1].is_whitespace();
        if !prev_ws {
            let mut j = i;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() && (chars[j] == ':' || chars[j] == ',') {
                out.push(chars[j]);
                j += 1;
                while j < chars.len() && chars[j].is_whitespace() {
                    j += 1;
                }
                i = j;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// `normalizeConfigGlobal`: `Some(off)` for a valid value, `None` if invalid.
fn global_is_off(value: Option<&str>) -> Option<bool> {
    match value {
        Some("off") => Some(true),
        None | Some("true" | "writeable" | "writable" | "false" | "readable" | "readonly") => {
            Some(false)
        }
        Some(_) => None,
    }
}

/// `getNameLocationInGlobalDirectiveComment`: the name's span in the comment,
/// searched after the first `global`.
fn name_span(comment_value: &str, value_start: u32, name: &str) -> Span {
    let from = comment_value.find("global").map_or(0, |i| i + 6);
    let mut search = from;
    while let Some(rel) = comment_value.get(search..).and_then(|s| s.find(name)) {
        let at = search + rel;
        let before = comment_value[..at].chars().next_back();
        let after = comment_value[at + name.len()..].chars().next();
        // The `[\s,]` before the name must lie at or after `from`.
        let before_ok = at > from && before.is_some_and(|c| c.is_whitespace() || c == ',');
        let after_ok = after.is_none_or(|c| c.is_whitespace() || c == ',' || c == ':');
        if before_ok && after_ok {
            let start = value_start + at as u32;
            return Span::new(start, start + name.len() as u32);
        }
        search = at + name.len().max(1);
    }
    Span::new(value_start, value_start + 1)
}

fn inline_globals(ctx: &Ctx<'_>) -> Vec<InlineGlobal> {
    let mut globals: Vec<InlineGlobal> = Vec::new();
    for comment in &ctx.program.comments {
        if !comment.is_block() {
            continue;
        }
        let content = comment.content_span();
        let value = &ctx.source[content.start as usize..content.end as usize];
        let Some(items) = parse_global_directive(value) else {
            continue;
        };
        for (name, val) in items {
            let Some(off) = global_is_off(val.as_deref()) else {
                continue;
            };
            let span = name_span(value, content.start, &name);
            if let Some(g) = globals.iter_mut().find(|g| g.name == name) {
                g.spans.push(span);
                g.off = off;
            } else {
                globals.push(InlineGlobal {
                    name,
                    spans: vec![span],
                    off,
                });
            }
        }
    }
    globals
}

impl Rule for NoRedeclare {
    fn name(&self) -> &'static str {
        "no-redeclare"
    }

    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        let builtin_globals = ctx
            .option()
            .get("builtinGlobals")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);
        let mut collector = Collector::default();
        collector.visit_program(ctx.program);
        let mut reports = collector.reports;

        // The global scope: configured globals and `/* global */` comments.
        // Modules declare nothing else there.
        for g in inline_globals(ctx) {
            if g.off {
                continue;
            }
            let builtin = builtin_globals && is_builtin_global(&g.name);
            let extra = if builtin { &g.spans[..] } else { &g.spans[1..] };
            let message = if builtin {
                format!(
                    "'{}' is already defined as a built-in global variable.",
                    g.name
                )
            } else {
                format!("'{}' is already defined.", g.name)
            };
            for span in extra {
                reports.push((*span, message.clone()));
            }
        }

        reports.sort_by_key(|(span, _)| span.start);
        for (span, message) in reports {
            ctx.report(span, message);
        }
    }
}
