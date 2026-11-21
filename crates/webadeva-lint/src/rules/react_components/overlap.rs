//! eslint-plugin-react-x rules that oxlint also has natively (as ports of
//! eslint-plugin-react / the React Compiler lints), but where @eslint-react
//! reports more. They run next to the native rules to reproduce the ESLint
//! config, which enabled both.
//!
//! - `error-boundaries`: also in hooks, and `try` blocks the compiler skips.
//! - `no-array-index-key`: also `createElement` / `cloneElement` key props
//!   and `Children.map` callbacks.
//! - `no-clone-element`: any `*.cloneElement(...)`.
//! - `no-direct-mutation-state`: anywhere but the constructor's top scope.
//! - `set-state-in-render`: `state[1](...)`, `state.at(1)(...)`, loops, try.
//! - `static-components`: syntactic, independent of compiler bailouts.
//! - `unsupported-syntax`: `eval` in any component / hook.
//! - `use-memo`: every `useMemo` call, not only compiled functions.

use std::collections::HashSet;

use oxc_ast::AstKind;
use oxc_ast::ast::*;
use oxc_semantic::{AstNodes, NodeId, Semantic, SymbolId};
use oxc_span::{GetSpan, Span};
use serde_json::Value;

use super::helpers::*;
use super::no_options;
use crate::rule::{Ctx, Rule};

fn nearest_fn(nodes: &AstNodes<'_>, id: NodeId) -> Option<NodeId> {
    find_parent(nodes, id, |_, k| as_func(k).is_some(), no_stop)
}

fn symbol_of(sem: &Semantic<'_>, id: &IdentifierReference<'_>) -> Option<SymbolId> {
    sem.scoping()
        .get_reference(id.reference_id.get()?)
        .symbol_id()
}

macro_rules! simple_rule {
    ($ty:ident, $id:literal) => {
        pub struct $ty;
        impl Rule for $ty {
            fn name(&self) -> &'static str {
                $id
            }
            fn validate(&self, options: &[Value], _: &Value) -> Result<(), String> {
                no_options(options)
            }
            fn run<'a>(&self, ctx: &mut Ctx<'a>) {
                let reports = self.check(ctx);
                for (s, m) in reports {
                    ctx.report(s, m);
                }
            }
        }
    };
}

// ---------------------------------------------------------------------------
// no-clone-element
// ---------------------------------------------------------------------------

simple_rule!(NoCloneElement, "@eslint-react/no-clone-element");

impl NoCloneElement {
    fn check(&self, ctx: &Ctx<'_>) -> Vec<(Span, String)> {
        if !ctx.source.contains("cloneElement") {
            return Vec::new();
        }
        ctx.semantic
            .nodes()
            .iter()
            .filter_map(|n| match n.kind() {
                AstKind::CallExpression(c) if is_api_call(ctx.source, c, "cloneElement") => Some((
                    c.span,
                    "Using 'cloneElement' is uncommon and can lead to fragile code. Use alternatives instead.".into(),
                )),
                _ => None,
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// no-direct-mutation-state
// ---------------------------------------------------------------------------

simple_rule!(
    NoDirectMutationState,
    "@eslint-react/no-direct-mutation-state"
);

/// `isAssignmentToThisState`: some member in the left chain is `this.state`.
fn is_assignment_to_this_state(left: &AssignmentTarget<'_>) -> bool {
    let mut cur = match left {
        AssignmentTarget::TSAsExpression(x) => unwrap(&x.expression),
        AssignmentTarget::TSSatisfiesExpression(x) => unwrap(&x.expression),
        AssignmentTarget::TSNonNullExpression(x) => unwrap(&x.expression),
        AssignmentTarget::TSTypeAssertion(x) => unwrap(&x.expression),
        t => match t.as_member_expression() {
            Some(m) => Un::Member(m),
            None => return false,
        },
    };
    while let Un::Member(m) = cur {
        let object = unwrap(m.object());
        if object.is_this() && member_property_ident(m) == Some("state") {
            return true;
        }
        cur = object;
    }
    false
}

impl NoDirectMutationState {
    fn check(&self, ctx: &Ctx<'_>) -> Vec<(Span, String)> {
        let sem = ctx.semantic;
        let nodes = sem.nodes();
        let mut out = Vec::new();
        for node in nodes.iter() {
            let AstKind::AssignmentExpression(a) = node.kind() else {
                continue;
            };
            if !is_assignment_to_this_state(&a.left) {
                continue;
            }
            let Some(class) = find_parent(
                nodes,
                node.id(),
                |_, k| matches!(k, AstKind::Class(_)),
                no_stop,
            ) else {
                continue;
            };
            if !kind_is_class_component(nodes.kind(class)) {
                continue;
            }
            let ctor = find_parent(
                nodes,
                node.id(),
                |p, k| {
                    matches!(k, AstKind::Function(f) if f.r#type == FunctionType::FunctionExpression)
                        && parent(nodes, p).is_some_and(|mp| {
                            matches!(nodes.kind(mp), AstKind::MethodDefinition(m)
                                if m.kind == MethodDefinitionKind::Constructor)
                        })
                },
                no_stop,
            );
            let scope_block = sem.scoping().get_node_id(node.scope_id());
            if Some(scope_block) != ctor {
                out.push((
                    a.span,
                    "Do not mutate state directly. Use 'setState()' instead.".into(),
                ));
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------
// unsupported-syntax
// ---------------------------------------------------------------------------

simple_rule!(UnsupportedSyntax, "@eslint-react/unsupported-syntax");

fn is_eval_call(call: &CallExpression<'_>) -> bool {
    match unwrap(&call.callee) {
        Un::Expr(Expression::Identifier(i)) => i.name == "eval",
        Un::Member(MemberExpression::StaticMemberExpression(m)) => {
            matches!(unwrap(&m.object), Un::Expr(Expression::Identifier(o)) if o.name == "globalThis")
                && m.property.name == "eval"
        }
        _ => false,
    }
}

fn component_and_hook_ids(ctx: &Ctx<'_>) -> (HashSet<NodeId>, HashSet<NodeId>) {
    let c = collect(
        ctx,
        Want {
            components: Some(hint::DEFAULT_COMPONENT),
            hooks: true,
            classes: false,
        },
    );
    (c.component_ids().collect(), c.hook_ids().collect())
}

impl UnsupportedSyntax {
    fn check(&self, ctx: &Ctx<'_>) -> Vec<(Span, String)> {
        let nodes = ctx.semantic.nodes();
        let mut found: Vec<(NodeId, Span, &str)> = Vec::new();
        for node in nodes.iter() {
            let (span, msg) = match node.kind() {
                AstKind::CallExpression(c) if is_eval_call(c) => (
                    c.span,
                    "Do not use 'eval' inside components or hooks. 'eval' cannot be statically analyzed and is not supported by React Compiler.",
                ),
                AstKind::WithStatement(w) => (
                    w.span,
                    "Do not use 'with' statements inside components or hooks. 'with' changes scope dynamically and is not supported by React Compiler.",
                ),
                _ => continue,
            };
            if let Some(f) = nearest_fn(nodes, node.id()) {
                found.push((f, span, msg));
            }
        }
        if found.is_empty() {
            return Vec::new();
        }
        let (comps, hooks) = component_and_hook_ids(ctx);
        found
            .into_iter()
            .filter(|(f, ..)| comps.contains(f) || hooks.contains(f))
            .map(|(_, s, m)| (s, m.to_string()))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// error-boundaries
// ---------------------------------------------------------------------------

simple_rule!(ErrorBoundaries, "@eslint-react/error-boundaries");

/// `Traverse.findEnclosingTryBlock`: the closest `try` whose block contains
/// the node.
fn enclosing_try_block(nodes: &AstNodes<'_>, id: NodeId) -> Option<NodeId> {
    let mut child = id;
    loop {
        let p = parent(nodes, child)?;
        match nodes.kind(p) {
            AstKind::Program(_) => return None,
            AstKind::TryStatement(t) if t.block.node_id() == child => return Some(p),
            _ => child = p,
        }
    }
}

impl ErrorBoundaries {
    fn check(&self, ctx: &Ctx<'_>) -> Vec<(Span, String)> {
        let src = ctx.source;
        if !src.contains("try") {
            return Vec::new();
        }
        let sem = ctx.semantic;
        let nodes = sem.nodes();
        let (comps, hooks) = component_and_hook_ids(ctx);
        let funcs: HashSet<NodeId> = comps.union(&hooks).copied().collect();
        if funcs.is_empty() {
            return Vec::new();
        }
        let mut reported: Vec<NodeId> = Vec::new();
        let mut out = Vec::new();
        for node in nodes.iter() {
            let AstKind::CallExpression(c) = node.kind() else {
                continue;
            };
            if !is_api_call(src, c, "use") {
                continue;
            }
            let Some(stmt) = enclosing_try_block(nodes, node.id()) else {
                continue;
            };
            if nodes.ancestor_ids(stmt).any(|a| funcs.contains(&a)) && !reported.contains(&stmt) {
                reported.push(stmt);
                out.push((nodes.kind(stmt).span(), "Use an Error Boundary instead of try/catch around the 'use' hook. The 'use' hook suspends the component, and its errors can only be caught by Error Boundaries.".into()));
            }
        }
        let h = hint::NULL
            | hint::NUMBER
            | hint::BIGINT
            | hint::STRING
            | hint::BOOLEAN
            | hint::UNDEFINED
            | hint::EMPTY_ARRAY;
        // `rets` of the components and hooks: their own return arguments and
        // arrow expression bodies.
        let mut rets: Vec<(NodeId, &Expression<'_>)> = Vec::new();
        for node in nodes.iter() {
            match node.kind() {
                AstKind::ReturnStatement(r) => {
                    if let Some(arg) = &r.argument
                        && nearest_fn(nodes, node.id()).is_some_and(|f| funcs.contains(&f))
                    {
                        rets.push((arg.node_id(), arg));
                    }
                }
                AstKind::ArrowFunctionExpression(a) if funcs.contains(&a.node_id()) => {
                    if let Some(body) = a.body.as_expression() {
                        rets.push((body.node_id(), body));
                    }
                }
                _ => {}
            }
        }
        for (id, ret) in rets {
            if !is_jsx_like(sem, src, Some(ret), h) {
                continue;
            }
            let id = if matches!(nodes.kind(id), AstKind::ParenthesizedExpression(_)) {
                strip_parens(ret).node_id()
            } else {
                id
            };
            if let Some(stmt) = enclosing_try_block(nodes, id)
                && !reported.contains(&stmt)
            {
                reported.push(stmt);
                out.push((
                    nodes.kind(stmt).span(),
                    "Use an Error Boundary to catch errors in child components. Try/catch can't catch errors during React's rendering process.".into(),
                ));
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------
// no-array-index-key
// ---------------------------------------------------------------------------

simple_rule!(NoArrayIndexKey, "@eslint-react/no-array-index-key");

fn index_position(method: &str) -> Option<usize> {
    Some(match method {
        "every" | "filter" | "find" | "findIndex" | "findLast" | "findLastIndex" | "flatMap"
        | "forEach" | "map" | "some" => 1,
        "reduce" | "reduceRight" => 2,
        _ => return None,
    })
}

/// `isArrayIndexReference`.
fn is_array_index(ctx: &Ctx<'_>, e: Option<&Expression<'_>>) -> Option<Span> {
    let Expression::Identifier(id) = strip_parens(e?) else {
        return None;
    };
    let sem = ctx.semantic;
    let nodes = sem.nodes();
    let sym = symbol_of(sem, id)?;
    let decl = sem.scoping().symbol_declaration(sym);
    let param_id = match nodes.kind(decl) {
        AstKind::FormalParameter(_) => decl,
        _ => {
            let p = nodes.parent_id(decl);
            if !matches!(nodes.kind(p), AstKind::FormalParameter(_)) {
                return None;
            }
            p
        }
    };
    let AstKind::FormalParameter(param) = nodes.kind(param_id) else {
        return None;
    };
    let BindingPattern::BindingIdentifier(b) = &param.pattern else {
        return None;
    };
    if b.symbol_id.get() != Some(sym) {
        return None;
    }
    let params_id = nodes.parent_id(param_id);
    let AstKind::FormalParameters(params) = nodes.kind(params_id) else {
        return None;
    };
    let fn_id = nodes.parent_id(params_id);
    let callback = match nodes.kind(fn_id) {
        AstKind::ArrowFunctionExpression(_) => fn_id,
        AstKind::Function(f) if f.r#type == FunctionType::FunctionExpression => fn_id,
        _ => return None,
    };
    let (argument, call) = skip_type_parents(nodes, callback);
    let AstKind::CallExpression(call) = nodes.kind(call?) else {
        return None;
    };
    let Un::Member(callee) = unwrap(&call.callee) else {
        return None;
    };
    let method = member_property_ident(callee)?;
    let pos = index_position(method)?;
    let cb_pos = if is_api(ctx.source, Un::Member(callee), "Children.map")
        || is_api(ctx.source, Un::Member(callee), "Children.forEach")
    {
        1
    } else {
        0
    };
    let arg = call.arguments.get(cb_pos)?.as_expression()?;
    if strip_parens(arg).node_id() != argument {
        return None;
    }
    let p = params.items.get(pos)?;
    (p.node_id() == param_id).then_some(id.span)
}

fn binary_identifiers<'b, 'a>(e: &'b Expression<'a>, out: &mut Vec<&'b Expression<'a>>) {
    match strip_parens(e) {
        x @ Expression::Identifier(_) => out.push(x),
        Expression::BinaryExpression(b) => {
            binary_identifiers(&b.left, out);
            binary_identifiers(&b.right, out);
        }
        _ => {}
    }
}

fn visit_key(ctx: &Ctx<'_>, e: &Expression<'_>, out: &mut Vec<Span>) {
    match strip_parens(e) {
        x @ Expression::Identifier(_) => out.extend(is_array_index(ctx, Some(x))),
        Expression::TemplateLiteral(t) => {
            for x in &t.expressions {
                out.extend(is_array_index(ctx, Some(x)));
            }
        }
        x @ Expression::BinaryExpression(_) => {
            let mut ids = Vec::new();
            binary_identifiers(x, &mut ids);
            for i in ids {
                out.extend(is_array_index(ctx, Some(i)));
            }
        }
        Expression::ConditionalExpression(c) => {
            visit_key(ctx, &c.consequent, out);
            visit_key(ctx, &c.alternate, out);
        }
        Expression::LogicalExpression(l) => {
            visit_key(ctx, &l.left, out);
            visit_key(ctx, &l.right, out);
        }
        Expression::CallExpression(c) => {
            if let Un::Member(m) = unwrap(&c.callee)
                && callee_name(c) == Some("toString")
                && let Some(s) = is_array_index(ctx, Some(m.object()))
            {
                out.push(s);
                return;
            }
            if let Un::Expr(Expression::Identifier(f)) = unwrap(&c.callee)
                && matches!(f.name.as_str(), "Number" | "String")
                && let Some(s) =
                    is_array_index(ctx, c.arguments.first().and_then(Argument::as_expression))
            {
                out.push(s);
            }
        }
        _ => {}
    }
}

impl NoArrayIndexKey {
    fn check(&self, ctx: &Ctx<'_>) -> Vec<(Span, String)> {
        let src = ctx.source;
        if !src.contains("key") {
            return Vec::new();
        }
        let mut spans = Vec::new();
        for node in ctx.semantic.nodes().iter() {
            match node.kind() {
                AstKind::CallExpression(c) => {
                    let props = if is_create_element_call(src, c) {
                        c.arguments
                            .get(1)
                            .and_then(Argument::as_expression)
                            .and_then(|a| match unwrap(a) {
                                Un::Expr(Expression::ObjectExpression(o)) => Some(o),
                                _ => None,
                            })
                    } else if is_api_call(src, c, "cloneElement") {
                        c.arguments
                            .get(1)
                            .and_then(Argument::as_expression)
                            .and_then(|a| match strip_parens(a) {
                                Expression::ObjectExpression(o) => Some(o),
                                _ => None,
                            })
                    } else {
                        None
                    };
                    let Some(props) = props else { continue };
                    for p in &props.properties {
                        let ObjectPropertyKind::ObjectProperty(p) = p else {
                            continue;
                        };
                        if p.computed || static_key_name(&p.key, false) != Some("key") {
                            continue;
                        }
                        visit_key(ctx, &p.value, &mut spans);
                    }
                }
                AstKind::JSXAttribute(a) => {
                    let JSXAttributeName::Identifier(n) = &a.name else {
                        continue;
                    };
                    if n.name != "key" {
                        continue;
                    }
                    if let Some(JSXAttributeValue::ExpressionContainer(c)) = &a.value
                        && let Some(e) = c.expression.as_expression()
                    {
                        visit_key(ctx, e, &mut spans);
                    }
                }
                _ => {}
            }
        }
        spans
            .into_iter()
            .map(|s| {
                (
                    s,
                    "Do not use item index in the array as its key.".to_string(),
                )
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// use-memo
// ---------------------------------------------------------------------------

simple_rule!(UseMemo, "@eslint-react/use-memo");

fn is_result_used(nodes: &AstNodes<'_>, id: NodeId) -> bool {
    let (child, Some(p)) = skip_type_parents(nodes, id) else {
        return false;
    };
    match nodes.kind(p) {
        AstKind::FormalParameter(fp) => fp.initializer.as_ref().is_some_and(|i| is_node(i, child)),
        AstKind::VariableDeclarator(_)
        | AstKind::AssignmentExpression(_)
        | AstKind::AssignmentPattern(_)
        | AstKind::AssignmentTargetWithDefault(_)
        | AstKind::AssignmentTargetPropertyIdentifier(_)
        | AstKind::ObjectProperty(_)
        | AstKind::ReturnStatement(_)
        | AstKind::JSXExpressionContainer(_)
        | AstKind::CallExpression(_)
        | AstKind::NewExpression(_)
        | AstKind::ArrayExpression(_)
        | AstKind::ConditionalExpression(_)
        | AstKind::LogicalExpression(_)
        | AstKind::SequenceExpression(_)
        | AstKind::SpreadElement(_)
        | AstKind::TemplateLiteral(_)
        | AstKind::BinaryExpression(_)
        | AstKind::UnaryExpression(_)
        | AstKind::StaticMemberExpression(_)
        | AstKind::ComputedMemberExpression(_)
        | AstKind::PrivateFieldExpression(_)
        | AstKind::TaggedTemplateExpression(_)
        | AstKind::ChainExpression(_)
        | AstKind::ArrowFunctionExpression(_)
        | AstKind::ForOfStatement(_)
        | AstKind::ForInStatement(_) => true,
        _ => false,
    }
}

/// `Extract.getAssignmentTargets`, identifiers only.
fn assignment_target_idents<'b, 'a>(
    t: &'b AssignmentTarget<'a>,
    out: &mut Vec<&'b IdentifierReference<'a>>,
) {
    match t {
        AssignmentTarget::AssignmentTargetIdentifier(i) => out.push(i),
        AssignmentTarget::TSAsExpression(x) => expr_target_idents(&x.expression, out),
        AssignmentTarget::TSSatisfiesExpression(x) => expr_target_idents(&x.expression, out),
        AssignmentTarget::TSNonNullExpression(x) => expr_target_idents(&x.expression, out),
        AssignmentTarget::TSTypeAssertion(x) => expr_target_idents(&x.expression, out),
        AssignmentTarget::ArrayAssignmentTarget(a) => {
            for el in a.elements.iter().flatten() {
                maybe_default_idents(el, out);
            }
            if let Some(r) = &a.rest {
                assignment_target_idents(&r.target, out);
            }
        }
        AssignmentTarget::ObjectAssignmentTarget(o) => {
            for p in &o.properties {
                match p {
                    AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(i) => {
                        out.push(&i.binding)
                    }
                    AssignmentTargetProperty::AssignmentTargetPropertyProperty(pp) => {
                        maybe_default_idents(&pp.binding, out);
                    }
                }
            }
            if let Some(r) = &o.rest {
                assignment_target_idents(&r.target, out);
            }
        }
        _ => {}
    }
}

fn maybe_default_idents<'b, 'a>(
    t: &'b AssignmentTargetMaybeDefault<'a>,
    out: &mut Vec<&'b IdentifierReference<'a>>,
) {
    match t {
        AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(d) => {
            assignment_target_idents(&d.binding, out)
        }
        other => assignment_target_idents(other.to_assignment_target(), out),
    }
}

fn expr_target_idents<'b, 'a>(e: &'b Expression<'a>, out: &mut Vec<&'b IdentifierReference<'a>>) {
    if let Un::Expr(Expression::Identifier(i)) = unwrap(e) {
        out.push(i);
    }
}

impl UseMemo {
    fn check(&self, ctx: &Ctx<'_>) -> Vec<(Span, String)> {
        let src = ctx.source;
        if !src.contains("useMemo") {
            return Vec::new();
        }
        const UNUSED: &str = "useMemo() result is unused.\n\nThis useMemo() value is unused. useMemo() is for computing and caching values, not for arbitrary side effects.";
        const PARAMS: &str = "useMemo() callbacks may not accept parameters.\n\nuseMemo() callbacks are called by React to cache calculations across re-renders. They should not take parameters. Instead, directly reference the props, state, or local variables needed for the computation.";
        const ASYNC: &str = "useMemo() callbacks may not be async or generator functions.\n\nuseMemo() callbacks are called once and must synchronously return a value.";
        const REASSIGN: &str = "useMemo() callbacks may not reassign variables declared outside of the callback.\n\nuseMemo() callbacks must be pure functions and cannot reassign variables defined outside of the callback function.";
        const RETURN: &str = "useMemo() callbacks must return a value.\n\nThis useMemo() callback doesn't return a value. useMemo() is for computing and caching values, not for arbitrary side effects.";
        let sem = ctx.semantic;
        let nodes = sem.nodes();
        let mut out: Vec<(Span, String)> = Vec::new();
        for node in nodes.iter() {
            let AstKind::CallExpression(call) = node.kind() else {
                continue;
            };
            if !is_api_call(src, call, "useMemo") {
                continue;
            }
            if !is_result_used(nodes, node.id()) {
                out.push((call.span, UNUSED.into()));
                continue;
            }
            let Some(arg) = call.arguments.first() else {
                continue;
            };
            let Some(arg_expr) = arg.as_expression() else {
                continue;
            };
            let (cb_id, params, is_async, generator, cb_span, block) = match unwrap(arg_expr) {
                Un::Expr(Expression::ArrowFunctionExpression(a)) => (
                    a.node_id(),
                    &*a.params,
                    a.r#async,
                    false,
                    a.span,
                    a.body.as_function_body(),
                ),
                Un::Expr(Expression::FunctionExpression(f)) if f.body.is_some() => (
                    f.node_id(),
                    &*f.params,
                    f.r#async,
                    f.generator,
                    f.span,
                    f.body.as_deref(),
                ),
                _ => continue,
            };
            if let Some(first) = params.items.first() {
                let span = match (&first.pattern, &first.initializer) {
                    (BindingPattern::BindingIdentifier(b), None) => b.span,
                    _ => cb_span,
                };
                out.push((span, PARAMS.into()));
            } else if params.rest.is_some() {
                out.push((cb_span, PARAMS.into()));
            }
            if is_async || generator {
                out.push((cb_span, ASYNC.into()));
            }
            // Writes to outer variables, not inside nested functions.
            let inner_fn = |id: NodeId| {
                find_parent(nodes, id, |_, k| as_func(k).is_some(), |n, _| n == cb_id).is_some()
            };
            let mut has_value_return = false;
            for d in nodes.iter() {
                if !d.span_within(cb_span) {
                    continue;
                }
                let mut targets = Vec::new();
                match d.kind() {
                    AstKind::AssignmentExpression(a) if !inner_fn(d.id()) => {
                        assignment_target_idents(&a.left, &mut targets);
                    }
                    AstKind::ForInStatement(f) if !inner_fn(d.id()) => {
                        if let Some(t) = f.left.as_assignment_target() {
                            assignment_target_idents(t, &mut targets);
                        }
                    }
                    AstKind::ForOfStatement(f) if !inner_fn(d.id()) => {
                        if let Some(t) = f.left.as_assignment_target() {
                            assignment_target_idents(t, &mut targets);
                        }
                    }
                    AstKind::ReturnStatement(r) if r.argument.is_some() && !inner_fn(d.id()) => {
                        has_value_return = true;
                    }
                    _ => {}
                }
                for t in targets {
                    let outer = match symbol_of(sem, t) {
                        None => true,
                        Some(sym) => {
                            let scope = sem.scoping().symbol_scope_id(sym);
                            !sem.scoping()
                                .scope_ancestors(scope)
                                .any(|s| sem.scoping().get_node_id(s) == cb_id)
                        }
                    };
                    if outer {
                        out.push((t.span, REASSIGN.into()));
                    }
                }
            }
            if block.is_some() && !has_value_return {
                out.push((strip_parens(arg_expr).span(), RETURN.into()));
            }
        }
        out
    }
}

trait SpanWithin {
    fn span_within(&self, outer: Span) -> bool;
}

impl SpanWithin for oxc_semantic::AstNode<'_> {
    fn span_within(&self, outer: Span) -> bool {
        let s = self.kind().span();
        s.start >= outer.start && s.end <= outer.end
    }
}

// ---------------------------------------------------------------------------
// set-state-in-render
// ---------------------------------------------------------------------------

simple_rule!(SetStateInRender, "@eslint-react/set-state-in-render");

/// `isComponentOrHookLikeFunction`.
fn is_component_or_hook_like(ctx: &Ctx<'_>, id: NodeId) -> bool {
    let nodes = ctx.semantic.nodes();
    let ok = |n: &str| is_component_name(n) || is_hook_name(n);
    match get_function_id(nodes, id) {
        None => {
            // isFunctionComponentWrapperCallback
            let (arg, Some(p)) = skip_type_parents(nodes, id) else {
                return false;
            };
            match nodes.kind(p) {
                AstKind::CallExpression(c) => {
                    c.arguments
                        .first()
                        .and_then(Argument::as_expression)
                        .is_some_and(|a| is_node(a, arg))
                        && is_fc_wrapper_call(ctx.source, c)
                }
                _ => false,
            }
        }
        Some(fid) => match fid.shape() {
            IdShape::Ident(n) => ok(n),
            IdShape::Member {
                ident_prop: Some(n),
                ..
            } => ok(n),
            _ => false,
        },
    }
}

/// Numeric value 1 (`getStaticValue(...)?.value === 1`) for literals.
fn is_static_one(e: &Expression<'_>) -> bool {
    matches!(strip_parens(e), Expression::NumericLiteral(n) if n.value == 1.0)
}

fn is_use_state_call(call: &CallExpression<'_>) -> bool {
    callee_name(call) == Some("useState")
}

/// `isSetStateId(id, at)`.
fn is_set_state_id(sem: &Semantic<'_>, id: &IdentifierReference<'_>, at: Option<usize>) -> bool {
    let Some(sym) = symbol_of(sem, id) else {
        return false;
    };
    let AstKind::VariableDeclarator(d) = sem.nodes().kind(sem.scoping().symbol_declaration(sym))
    else {
        return false;
    };
    let Some(Expression::CallExpression(init)) = d.init.as_ref().map(strip_parens) else {
        return false;
    };
    if !is_use_state_call(init) {
        return false;
    }
    let name = id.name.as_str();
    match &d.id {
        BindingPattern::ObjectPattern(o) => o.properties.iter().any(|p| {
            matches!(&p.key, PropertyKey::NumericLiteral(n) if n.value == at.unwrap_or(1) as f64)
                && matches!(&p.value, BindingPattern::BindingIdentifier(b) if b.name == name)
        }),
        BindingPattern::ArrayPattern(a) => {
            let idx = a.elements.iter().position(
                |e| matches!(e, Some(BindingPattern::BindingIdentifier(b)) if b.name == name),
            );
            idx.is_some() && idx == at
        }
        _ => true,
    }
}

fn is_set_state_call(sem: &Semantic<'_>, call: &CallExpression<'_>) -> bool {
    match unwrap(&call.callee) {
        Un::Call(inner) => {
            let Un::Member(m) = unwrap(&inner.callee) else {
                return false;
            };
            let Expression::Identifier(obj) = strip_parens(m.object()) else {
                return false;
            };
            callee_name(inner) == Some("at")
                && inner
                    .arguments
                    .first()
                    .and_then(Argument::as_expression)
                    .is_some_and(is_static_one)
                && is_set_state_id(sem, obj, None)
        }
        Un::Expr(Expression::Identifier(i)) => is_set_state_id(sem, i, Some(1)),
        Un::Member(MemberExpression::ComputedMemberExpression(m)) => {
            let Expression::Identifier(obj) = strip_parens(&m.object) else {
                return false;
            };
            is_static_one(&m.expression) && is_set_state_id(sem, obj, Some(1))
        }
        _ => false,
    }
}

impl SetStateInRender {
    fn check(&self, ctx: &Ctx<'_>) -> Vec<(Span, String)> {
        if !ctx.source.contains("useState") {
            return Vec::new();
        }
        let sem = ctx.semantic;
        let nodes = sem.nodes();
        // Early returns per component function: start offsets.
        let mut early: Vec<(NodeId, u32)> = Vec::new();
        for node in nodes.iter() {
            let AstKind::ReturnStatement(r) = node.kind() else {
                continue;
            };
            let Some(f) = nearest_fn(nodes, node.id()) else {
                continue;
            };
            if !is_component_or_hook_like(ctx, f) {
                continue;
            }
            let Some(body) = as_func(nodes.kind(f)).and_then(Func::block_body) else {
                continue;
            };
            let Some(idx) = body
                .statements
                .iter()
                .position(|s| s.span().start <= r.span.start && r.span.end <= s.span().end)
            else {
                continue;
            };
            if idx + 1 < body.statements.len() {
                early.push((f, r.span.start));
            }
        }
        let mut out = Vec::new();
        for node in nodes.iter() {
            let AstKind::CallExpression(call) = node.kind() else {
                continue;
            };
            if is_use_state_call(call) {
                continue;
            }
            let Some(f) = nearest_fn(nodes, node.id()) else {
                continue;
            };
            if !is_component_or_hook_like(ctx, f) || !is_set_state_call(sem, call) {
                continue;
            }
            let conditional = ancestors(nodes, node.id())
                .take_while(|a| *a != f)
                .any(|a| {
                    matches!(
                        nodes.kind(a),
                        AstKind::IfStatement(_)
                            | AstKind::ConditionalExpression(_)
                            | AstKind::LogicalExpression(_)
                            | AstKind::SwitchStatement(_)
                            | AstKind::SwitchCase(_)
                    )
                });
            if conditional
                || early
                    .iter()
                    .any(|(ef, pos)| *ef == f && *pos < call.span.start)
            {
                continue;
            }
            let name = ctx.source_text(unwrap(&call.callee).span());
            out.push((
                call.span,
                format!("Do not call the 'set' function '{name}' unconditionally during render. This will trigger an infinite render loop."),
            ));
        }
        out
    }
}

// ---------------------------------------------------------------------------
// static-components
// ---------------------------------------------------------------------------

simple_rule!(StaticComponents, "@eslint-react/static-components");

struct Render<'s> {
    comps: &'s HashSet<NodeId>,
}

impl Render<'_> {
    fn inside(&self, nodes: &AstNodes<'_>, id: NodeId) -> bool {
        nodes.ancestor_ids(id).any(|a| self.comps.contains(&a))
    }
}

fn creation_site(
    sem: &Semantic<'_>,
    e: &Expression<'_>,
    r: &Render<'_>,
    seen: &mut HashSet<SymbolId>,
) -> Option<Span> {
    match unwrap(e) {
        Un::Call(c) => Some(c.span),
        Un::Expr(x) => match x {
            Expression::ArrowFunctionExpression(_)
            | Expression::ClassExpression(_)
            | Expression::FunctionExpression(_)
            | Expression::NewExpression(_) => Some(x.span()),
            Expression::ConditionalExpression(c) => creation_site(sem, &c.consequent, r, seen)
                .or_else(|| creation_site(sem, &c.alternate, r, seen)),
            Expression::LogicalExpression(l) => creation_site(sem, &l.left, r, seen)
                .or_else(|| creation_site(sem, &l.right, r, seen)),
            Expression::SequenceExpression(s) => s
                .expressions
                .last()
                .and_then(|l| creation_site(sem, l, r, seen)),
            Expression::Identifier(i) => dynamic_origin(sem, symbol_of(sem, i)?, r, seen).flatten(),
            _ => None,
        },
        Un::Member(_) => None,
    }
}

/// `resolveDynamicComponentOrigin`: `None` = not dynamic, `Some(site)` =
/// dynamic (with the creation site when known).
fn dynamic_origin(
    sem: &Semantic<'_>,
    sym: SymbolId,
    r: &Render<'_>,
    seen: &mut HashSet<SymbolId>,
) -> Option<Option<Span>> {
    if !seen.insert(sym) {
        return None;
    }
    let nodes = sem.nodes();
    let decl = sem.scoping().symbol_declaration(sym);
    if !r.inside(nodes, decl) {
        return None;
    }
    match nodes.kind(decl) {
        AstKind::Function(f) if f.r#type == FunctionType::FunctionDeclaration => Some(Some(f.span)),
        AstKind::Class(c) if c.r#type == ClassType::ClassDeclaration => Some(Some(c.span)),
        AstKind::VariableDeclarator(d) => {
            if let Some(init) = &d.init
                && let Some(site) = creation_site(sem, init, r, seen)
            {
                return Some(Some(site));
            }
            for reference in sem.scoping().get_resolved_references(sym) {
                if !reference.is_write() {
                    continue;
                }
                let rid = reference.node_id();
                let Some(p) = parent(nodes, rid) else {
                    continue;
                };
                if let AstKind::AssignmentExpression(a) = nodes.kind(p)
                    && matches!(&a.left, AssignmentTarget::AssignmentTargetIdentifier(i) if i.node_id() == rid)
                    && let Some(site) = creation_site(sem, &a.right, r, seen)
                {
                    return Some(Some(site));
                }
            }
            None
        }
        _ => None,
    }
}

impl StaticComponents {
    fn check(&self, ctx: &Ctx<'_>) -> Vec<(Span, String)> {
        let sem = ctx.semantic;
        let nodes = sem.nodes();
        let usages: Vec<&IdentifierReference<'_>> = nodes
            .iter()
            .filter_map(|n| match n.kind() {
                AstKind::JSXOpeningElement(o) => match &o.name {
                    JSXElementName::IdentifierReference(i)
                        if is_component_name(i.name.as_str()) =>
                    {
                        Some(&**i)
                    }
                    _ => None,
                },
                _ => None,
            })
            .collect();
        if usages.is_empty() {
            return Vec::new();
        }
        let h = hint::NUMBER
            | hint::BOOLEAN
            | hint::NULL
            | hint::STRING
            | hint::UNDEFINED
            | hint::BOTH_LOGICAL
            | hint::BOTH_CONDITIONAL
            | hint::ARRAY_PATTERN_ELEMENT
            | hint::ARRAY_EXPRESSION_ELEMENT
            | hint::MAP_CALLBACK;
        let c = collect(
            ctx,
            Want {
                components: Some(h),
                hooks: false,
                classes: true,
            },
        );
        let comps: HashSet<NodeId> = c
            .component_ids()
            .chain(c.classes.iter().map(|k| k.class.node_id()))
            .collect();
        let render = Render { comps: &comps };
        let mut out = Vec::new();
        let mut created: Vec<Span> = Vec::new();
        for usage in usages {
            let Some(sym) = symbol_of(sem, usage) else {
                continue;
            };
            if !render.inside(nodes, sem.scoping().symbol_declaration(sym)) {
                continue;
            }
            let Some(site) = dynamic_origin(sem, sym, &render, &mut HashSet::new()) else {
                continue;
            };
            out.push((usage.span, "Cannot create components during render. Components created during render will reset their state each time they are created. Declare components outside of render.".to_string()));
            if let Some(site) = site
                && !created.contains(&site)
            {
                created.push(site);
                out.push((
                    site,
                    "The component is created during render here.".to_string(),
                ));
            }
        }
        out
    }
}
