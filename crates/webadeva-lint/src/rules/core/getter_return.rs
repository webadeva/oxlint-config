//! ESLint's `getter-return` (lib/rules/getter-return.js). Runs on every file:
//! oxlint's native rule skips TypeScript files and differs from ESLint on
//! JavaScript (infinite loops, `finally`, wrong descriptor arguments,
//! shadowed `Object`, nested functions).
//!
//! ESLint asks its code-path analysis whether the end of a getter is
//! reachable. oxc's control-flow graph is behind a feature we don't build, so
//! this ports the parts of ESLint's code-path rules that decide reachability
//! of a function's end: `return`/`throw`, `break`/`continue` with labels,
//! loops with literal-constant tests (`while (true)`, `for (;;)`), `switch`
//! fall-through and `default`, and `try`/`catch`/`finally` (a `catch` is only
//! reachable when the `try` block reaches something that can throw).

use oxc_ast::AstKind;
use oxc_ast::ast::{
    Argument, ArrowFunctionExpression, Expression, Function, FunctionBody, FunctionType,
    MethodDefinitionKind, PropertyKey, PropertyKind, Statement,
};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::{AstNodes, NodeId, Semantic};
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;
use serde_json::Value;

use super::util::estree_parent;
use crate::rule::{Ctx, Rule};

pub struct GetterReturn;

// ---------------------------------------------------------------------------
// Reachability

#[derive(Default)]
struct Completion<'a> {
    /// The statement can complete normally (the code after it is reachable).
    normal: bool,
    /// Pending `break`s (`None` = unlabeled).
    breaks: Vec<Option<&'a str>>,
    continues: Vec<Option<&'a str>>,
    /// Reaches something that can throw (an identifier reference, call,
    /// member access, `new`, `import()` or `throw`).
    throws: bool,
}

impl<'a> Completion<'a> {
    fn normal() -> Self {
        Self {
            normal: true,
            ..Self::default()
        }
    }
    fn absorb(&mut self, other: Completion<'a>) {
        self.breaks.extend(other.breaks);
        self.continues.extend(other.continues);
        self.throws |= other.throws;
    }
}

/// Whether evaluating this subtree touches a throwable node, ignoring nested
/// functions (which are code paths of their own).
struct Throwable {
    found: bool,
}

impl<'a> Visit<'a> for Throwable {
    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}
    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}
    fn visit_property_definition(&mut self, it: &oxc_ast::ast::PropertyDefinition<'a>) {
        // Field initializers are code paths of their own.
        if it.computed {
            self.visit_property_key(&it.key);
        }
    }
    fn visit_static_block(&mut self, _it: &oxc_ast::ast::StaticBlock<'a>) {}
    fn visit_identifier_reference(&mut self, _it: &oxc_ast::ast::IdentifierReference<'a>) {
        self.found = true;
    }
    fn visit_expression(&mut self, it: &Expression<'a>) {
        if matches!(
            it,
            Expression::CallExpression(_)
                | Expression::NewExpression(_)
                | Expression::ImportExpression(_)
                | Expression::StaticMemberExpression(_)
                | Expression::ComputedMemberExpression(_)
                | Expression::PrivateFieldExpression(_)
        ) {
            self.found = true;
            return;
        }
        walk::walk_expression(self, it);
    }
    fn visit_binding_pattern(&mut self, it: &oxc_ast::ast::BindingPattern<'a>) {
        // ESTree identifiers in object patterns count as references.
        if let oxc_ast::ast::BindingPattern::ObjectPattern(_) = it {
            self.found = true;
            return;
        }
        walk::walk_binding_pattern(self, it);
    }
}

fn throwable_stmt(s: &Statement<'_>) -> bool {
    let mut v = Throwable { found: false };
    v.visit_statement(s);
    v.found
}

fn throwable_expr(e: &Expression<'_>) -> bool {
    let mut v = Throwable { found: false };
    v.visit_expression(e);
    v.found
}

/// `getBooleanValueIfSimpleConstant`: only literals count.
fn constant_truthy(e: &Expression<'_>) -> Option<bool> {
    match e.without_parentheses() {
        Expression::BooleanLiteral(b) => Some(b.value),
        Expression::NullLiteral(_) => Some(false),
        Expression::NumericLiteral(n) => Some(n.value != 0.0 && !n.value.is_nan()),
        Expression::StringLiteral(s) => Some(!s.value.as_bytes().is_empty()),
        Expression::BigIntLiteral(b) => Some(b.value.as_str() != "0"),
        Expression::RegExpLiteral(_) => Some(true),
        _ => None,
    }
}

fn list<'a>(stmts: &'a [Statement<'a>]) -> Completion<'a> {
    let mut out = Completion::normal();
    for s in stmts {
        if !out.normal {
            break;
        }
        let c = stmt(s, &[]);
        out.normal = c.normal;
        out.absorb(c);
    }
    out
}

fn consumes(target: &Option<&str>, labels: &[&str]) -> bool {
    target.is_none_or(|l| labels.contains(&l))
}

/// A loop body's completion, with its breaks/continues to this loop resolved:
/// returns (body, broke_out, continued).
fn loop_body<'a>(body: &'a Statement<'a>, labels: &[&'a str]) -> (Completion<'a>, bool, bool) {
    let mut c = stmt(body, &[]);
    let broke = c.breaks.iter().any(|b| consumes(b, labels));
    let continued = c.continues.iter().any(|b| consumes(b, labels));
    c.breaks.retain(|b| !consumes(b, labels));
    c.continues.retain(|b| !consumes(b, labels));
    (c, broke, continued)
}

fn stmt<'a>(s: &'a Statement<'a>, labels: &[&'a str]) -> Completion<'a> {
    match s {
        Statement::ReturnStatement(r) => Completion {
            normal: false,
            throws: r.argument.as_ref().is_some_and(throwable_expr),
            ..Completion::default()
        },
        Statement::ThrowStatement(_) => Completion {
            normal: false,
            throws: true,
            ..Completion::default()
        },
        Statement::BreakStatement(b) => Completion {
            normal: false,
            breaks: vec![b.label.as_ref().map(|l| l.name.as_str())],
            ..Completion::default()
        },
        Statement::ContinueStatement(b) => Completion {
            normal: false,
            continues: vec![b.label.as_ref().map(|l| l.name.as_str())],
            ..Completion::default()
        },
        Statement::BlockStatement(b) => list(&b.body),
        Statement::IfStatement(i) => {
            let mut c = Completion {
                throws: throwable_expr(&i.test),
                ..Completion::default()
            };
            let cons = stmt(&i.consequent, &[]);
            let alt = i.alternate.as_ref().map(|a| stmt(a, &[]));
            c.normal = cons.normal || alt.as_ref().is_none_or(|a| a.normal);
            c.absorb(cons);
            if let Some(a) = alt {
                c.absorb(a);
            }
            c
        }
        Statement::LabeledStatement(l) => {
            let mut own = labels.to_vec();
            own.push(l.label.name.as_str());
            let mut c = stmt(&l.body, &own);
            let name = l.label.name.as_str();
            if c.breaks.contains(&Some(name)) {
                c.normal = true;
                c.breaks.retain(|b| *b != Some(name));
            }
            c
        }
        Statement::WhileStatement(w) => {
            let (body, broke, _) = loop_body(&w.body, labels);
            let infinite = constant_truthy(&w.test) == Some(true);
            let mut c = Completion {
                normal: !infinite || broke,
                throws: throwable_expr(&w.test),
                ..Completion::default()
            };
            c.absorb(body);
            c
        }
        Statement::DoWhileStatement(d) => {
            let (body, broke, continued) = loop_body(&d.body, labels);
            let test_reachable = body.normal || continued;
            let infinite = constant_truthy(&d.test) == Some(true);
            let mut c = Completion {
                normal: (test_reachable && !infinite) || broke,
                throws: test_reachable && throwable_expr(&d.test),
                ..Completion::default()
            };
            c.absorb(body);
            c
        }
        Statement::ForStatement(f) => {
            let (body, broke, _) = loop_body(&f.body, labels);
            let infinite = f
                .test
                .as_ref()
                .is_none_or(|t| constant_truthy(t) == Some(true));
            let mut v = Throwable { found: false };
            if let Some(init) = &f.init {
                v.visit_for_statement_init(init);
            }
            if let Some(t) = &f.test {
                v.visit_expression(t);
            }
            let mut c = Completion {
                normal: !infinite || broke,
                throws: v.found,
                ..Completion::default()
            };
            c.absorb(body);
            c
        }
        Statement::ForInStatement(f) => {
            let (body, _, _) = loop_body(&f.body, labels);
            let mut c = Completion {
                normal: true,
                throws: true,
                ..Completion::default()
            };
            c.absorb(body);
            c
        }
        Statement::ForOfStatement(f) => {
            let (body, _, _) = loop_body(&f.body, labels);
            let mut c = Completion {
                normal: true,
                throws: true,
                ..Completion::default()
            };
            c.absorb(body);
            c
        }
        Statement::SwitchStatement(sw) => {
            let mut c = Completion {
                throws: throwable_expr(&sw.discriminant),
                ..Completion::default()
            };
            let has_default = sw.cases.iter().any(|k| k.test.is_none());
            let mut last_normal = true;
            for case in &sw.cases {
                if let Some(t) = &case.test {
                    c.throws |= throwable_expr(t);
                }
                let k = list(&case.consequent);
                last_normal = k.normal;
                c.absorb(k);
            }
            let broke = c.breaks.iter().any(|b| consumes(b, labels));
            c.breaks.retain(|b| !consumes(b, labels));
            c.normal = !has_default || broke || last_normal;
            c
        }
        Statement::TryStatement(t) => {
            let block = list(&t.block.body);
            let catch = t.handler.as_ref().map(|h| {
                if block.throws {
                    list(&h.body.body)
                } else {
                    Completion::default()
                }
            });
            let catch_normal = catch.as_ref().is_some_and(|c| c.normal);
            let mut c = Completion {
                normal: block.normal || catch_normal,
                ..Completion::default()
            };
            let block_throws_out = block.throws && t.handler.is_none();
            let block_breaks = (block.breaks, block.continues);
            let catch_parts = catch
                .map(|k| (k.breaks, k.continues, k.throws))
                .unwrap_or_default();
            c.throws = block_throws_out || catch_parts.2;
            if let Some(f) = &t.finalizer {
                let fin = list(&f.body);
                if fin.normal {
                    c.breaks.extend(block_breaks.0);
                    c.continues.extend(block_breaks.1);
                    c.breaks.extend(catch_parts.0);
                    c.continues.extend(catch_parts.1);
                } else {
                    c.normal = false;
                }
                c.absorb(fin);
            } else {
                c.breaks.extend(block_breaks.0);
                c.continues.extend(block_breaks.1);
                c.breaks.extend(catch_parts.0);
                c.continues.extend(catch_parts.1);
            }
            c
        }
        Statement::WithStatement(w) => {
            let mut c = stmt(&w.body, &[]);
            c.throws = true;
            c
        }
        other => Completion {
            normal: true,
            throws: throwable_stmt(other),
            ..Completion::default()
        },
    }
}

/// Whether the end of the function body is reachable.
fn end_reachable(body: &FunctionBody<'_>) -> bool {
    list(&body.statements).normal
}

// ---------------------------------------------------------------------------
// The rule

/// `getStaticStringValue` / `getStaticPropertyName` for a property key.
fn static_name(key: &PropertyKey<'_>, computed: bool) -> Option<String> {
    match key {
        PropertyKey::StaticIdentifier(id) if !computed => Some(id.name.to_string()),
        PropertyKey::StaticIdentifier(_) | PropertyKey::PrivateIdentifier(_) => None,
        _ => match key.as_expression()? {
            Expression::StringLiteral(s) => Some(s.value.display().to_string()),
            Expression::NumericLiteral(n) => {
                Some(if n.value.fract() == 0.0 && n.value.abs() < 1e21 {
                    format!("{}", n.value as i64)
                } else {
                    format!("{}", n.value)
                })
            }
            Expression::NullLiteral(_) => Some("null".into()),
            Expression::BooleanLiteral(b) => Some(b.value.to_string()),
            Expression::BigIntLiteral(b) => Some(b.value.to_string()),
            Expression::RegExpLiteral(r) => Some(r.regex.to_string()),
            Expression::TemplateLiteral(t) if t.expressions.is_empty() && t.quasis.len() == 1 => t
                .quasis[0]
                .value
                .cooked
                .as_ref()
                .map(|c| c.display().to_string()),
            _ => None,
        },
    }
}

/// `isArgumentOfGlobalMethodCall`: `node` is `arguments[index]` of a call to
/// `<object>.<method>` where `<object>` is the global.
fn is_global_method_arg(
    semantic: &Semantic<'_>,
    node: NodeId,
    object: &str,
    methods: &[&str],
    index: usize,
) -> bool {
    let nodes = semantic.nodes();
    let span = nodes.kind(node).span();
    let AstKind::CallExpression(call) = nodes.kind(estree_parent(nodes, node)) else {
        return false;
    };
    let is_arg = call.arguments.get(index).is_some_and(|a| match a {
        Argument::SpreadElement(_) => false,
        a => a
            .as_expression()
            .is_some_and(|e| e.without_parentheses().span() == span),
    });
    if !is_arg {
        return false;
    }
    let (obj, name) = match call.callee.without_parentheses() {
        Expression::StaticMemberExpression(m) => (&m.object, Some(m.property.name.to_string())),
        Expression::ComputedMemberExpression(m) => (
            &m.object,
            match m.expression.without_parentheses() {
                Expression::StringLiteral(s) => Some(s.value.display().to_string()),
                Expression::TemplateLiteral(t) if t.expressions.is_empty() => t.quasis[0]
                    .value
                    .cooked
                    .as_ref()
                    .map(|c| c.display().to_string()),
                _ => None,
            },
        ),
        _ => return false,
    };
    if !name.is_some_and(|n| methods.contains(&n.as_str())) {
        return false;
    }
    let Expression::Identifier(id) = obj else {
        return false;
    };
    if id.name != object {
        return false;
    }
    // `sourceCode.isGlobalReference`: not bound to a local declaration.
    id.reference_id
        .get()
        .is_none_or(|r| semantic.scoping().get_reference(r).symbol_id().is_none())
}

/// `isPropertyDescriptor`
fn is_property_descriptor(semantic: &Semantic<'_>, object: NodeId) -> bool {
    if is_global_method_arg(semantic, object, "Object", &["defineProperty"], 2)
        || is_global_method_arg(semantic, object, "Reflect", &["defineProperty"], 2)
    {
        return true;
    }
    let nodes = semantic.nodes();
    let span = nodes.kind(object).span();
    let parent = estree_parent(nodes, object);
    if let AstKind::ObjectProperty(p) = nodes.kind(parent)
        && p.value.without_parentheses().span() == span
    {
        let grand = nodes.parent_id(parent);
        return matches!(nodes.kind(grand), AstKind::ObjectExpression(_))
            && is_global_method_arg(
                semantic,
                grand,
                "Object",
                &["create", "defineProperties"],
                1,
            );
    }
    false
}

struct Getter {
    /// `getFunctionNameWithKind`
    name: String,
    /// `getFunctionHeadLoc`
    head: Span,
}

fn opening_paren(source: &str, params_span: Span) -> u32 {
    if source.as_bytes().get(params_span.start as usize) == Some(&b'(') {
        return params_span.start;
    }
    // A single unparenthesized arrow parameter is its own head end.
    params_span.start
}

/// `isGetter`, plus the name and head location used in reports.
fn getter_info(ctx: &Ctx<'_>, nodes: &AstNodes<'_>, id: NodeId) -> Option<Getter> {
    let (is_async, is_generator, fn_id, params_span) = match nodes.kind(id) {
        AstKind::Function(f)
            if f.r#type == FunctionType::FunctionExpression && f.body.is_some() =>
        {
            (
                f.r#async,
                f.generator,
                f.id.as_ref().map(|i| i.name.to_string()),
                f.params.span,
            )
        }
        AstKind::ArrowFunctionExpression(a) if !a.is_expression() => {
            (a.r#async, false, None, a.params.span)
        }
        _ => return None,
    };
    let fn_span = nodes.kind(id).span();
    let parent_id = estree_parent(nodes, id);
    let mut tokens: Vec<String> = Vec::new();
    let (is_getter, kind_word, name, start) = match nodes.kind(parent_id) {
        AstKind::MethodDefinition(m) if m.value.span == fn_span => {
            if m.r#static {
                tokens.push("static".into());
            }
            let private = matches!(m.key, PropertyKey::PrivateIdentifier(_));
            if private && !m.computed {
                tokens.push("private".into());
            }
            let kind_word = match m.kind {
                MethodDefinitionKind::Get => "getter",
                MethodDefinitionKind::Set => "setter",
                MethodDefinitionKind::Constructor => "constructor",
                MethodDefinitionKind::Method => "method",
            };
            let name = match &m.key {
                PropertyKey::PrivateIdentifier(p) if !m.computed => Some(format!("#{}", p.name)),
                key => static_name(key, m.computed)
                    .map(|n| format!("'{n}'"))
                    .or(fn_id.map(|n| format!("'{n}'"))),
            };
            (
                m.kind == MethodDefinitionKind::Get,
                kind_word,
                name,
                m.span.start,
            )
        }
        AstKind::ObjectProperty(p) if p.value.without_parentheses().span() == fn_span => {
            let kind_word = match p.kind {
                PropertyKind::Get => "getter",
                PropertyKind::Set => "setter",
                PropertyKind::Init => "method",
            };
            let static_key = static_name(&p.key, p.computed);
            let is_getter = p.kind == PropertyKind::Get
                || (static_key.as_deref() == Some("get")
                    && matches!(nodes.parent_kind(parent_id), AstKind::ObjectExpression(_))
                    && is_property_descriptor(ctx.semantic, nodes.parent_id(parent_id)));
            let name = static_key
                .map(|n| format!("'{n}'"))
                .or(fn_id.map(|n| format!("'{n}'")));
            (is_getter, kind_word, name, p.span.start)
        }
        _ => return None,
    };
    if !is_getter {
        return None;
    }
    if is_async {
        tokens.push("async".into());
    }
    if is_generator {
        tokens.push("generator".into());
    }
    if kind_word == "constructor" {
        tokens = vec!["constructor".into()];
    } else {
        tokens.push(kind_word.into());
        if let Some(n) = name {
            tokens.push(n);
        }
    }
    let end = opening_paren(ctx.source, params_span);
    Some(Getter {
        name: tokens.join(" "),
        head: Span::new(start, end),
    })
}

/// Return statements of this function (not of nested ones).
struct Returns<'s> {
    root: Span,
    out: &'s mut Vec<(Span, bool)>,
}

impl<'a> Visit<'a> for Returns<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if it.span == self.root {
            walk::walk_function(self, it, flags);
        }
    }
    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        if it.span == self.root {
            walk::walk_arrow_function_expression(self, it);
        }
    }
    fn visit_return_statement(&mut self, it: &oxc_ast::ast::ReturnStatement<'a>) {
        self.out.push((it.span, it.argument.is_some()));
        walk::walk_return_statement(self, it);
    }
}

impl Rule for GetterReturn {
    fn name(&self) -> &'static str {
        "getter-return"
    }

    fn validate(&self, options: &[Value], _settings: &Value) -> Result<(), String> {
        let Some(opt) = options.first() else {
            return Ok(());
        };
        let Value::Object(map) = opt else {
            return Err("options must be an object".into());
        };
        for (k, v) in map {
            if k != "allowImplicit" {
                return Err(format!("unknown option {k:?}"));
            }
            if !v.is_boolean() {
                return Err("`allowImplicit` must be a boolean".into());
            }
        }
        Ok(())
    }

    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        let allow_implicit = ctx
            .option()
            .get("allowImplicit")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let nodes = ctx.semantic.nodes();
        let mut reports: Vec<(Span, String)> = Vec::new();
        for node in nodes.iter() {
            let Some(getter) = getter_info(ctx, nodes, node.id()) else {
                continue;
            };
            let (body, span) = match node.kind() {
                AstKind::Function(f) => (f.body.as_deref(), f.span),
                AstKind::ArrowFunctionExpression(a) => (a.get_function_body(), a.span),
                _ => continue,
            };
            let Some(body) = body else { continue };
            let mut returns = Vec::new();
            let mut v = Returns {
                root: span,
                out: &mut returns,
            };
            match node.kind() {
                AstKind::Function(f) => v.visit_function(f, ScopeFlags::Function),
                AstKind::ArrowFunctionExpression(a) => v.visit_arrow_function_expression(a),
                _ => {}
            }
            if !allow_implicit {
                for (s, has_arg) in &returns {
                    if !has_arg {
                        reports.push((
                            *s,
                            format!("Expected to return a value in {}.", getter.name),
                        ));
                    }
                }
            }
            if end_reachable(body) {
                let msg = if returns.is_empty() {
                    format!("Expected to return a value in {}.", getter.name)
                } else {
                    format!("Expected {} to always return a value.", getter.name)
                };
                reports.push((getter.head, msg));
            }
        }
        for (span, msg) in reports {
            ctx.report(span, msg);
        }
    }
}
