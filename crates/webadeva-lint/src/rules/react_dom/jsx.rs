//! `@eslint-react/jsx-*` (eslint-plugin-react-jsx), `no-missing-key` and the
//! `no-children-*` rules (eslint-plugin-react-x).

use oxc_ast::AstKind;
use oxc_ast::ast::*;
use oxc_semantic::Semantic;
use oxc_span::{GetSpan, Span};
use oxc_syntax::node::NodeId;

use super::helpers::*;
use crate::rule::{Ctx, Rule};

// ---------------------------------------------------------------------------
// createElement helpers (`@eslint-react/core`)
// ---------------------------------------------------------------------------

fn is_create_element_call(sem: &Semantic<'_>, call: &CallExpression<'_>) -> bool {
    is_api_call(sem, call, "createElement")
}

/// `Extract.getPropertyName(prop, "max")`.
fn property_name_max(sem: &Semantic<'_>, p: &ObjectProperty<'_>) -> Option<String> {
    let key = unwrap(sem, p.key.node_id());
    match kind(sem, key) {
        AstKind::IdentifierName(i) if !p.computed => Some(i.name.to_string()),
        AstKind::IdentifierReference(i) if !p.computed => Some(i.name.to_string()),
        AstKind::StringLiteral(s) => s.value.as_str().map(str::to_string),
        AstKind::TemplateLiteral(t) if t.expressions.is_empty() => {
            let q = t.quasis.first()?;
            Some(
                q.value
                    .cooked
                    .and_then(|c| c.as_str())
                    .unwrap_or(q.value.raw.as_str())
                    .to_string(),
            )
        }
        _ => None,
    }
}

/// `getCreateElementProp(context, node, name)`: span of the property.
fn create_element_prop(sem: &Semantic<'_>, call: &CallExpression<'_>, name: &str) -> Option<Span> {
    if !is_create_element_call(sem, call) {
        return None;
    }
    let props = unwrap(sem, call.arguments.get(1)?.node_id());
    let AstKind::ObjectExpression(o) = kind(sem, props) else {
        return None;
    };
    o.properties.iter().find_map(|p| match p {
        ObjectPropertyKind::ObjectProperty(p)
            if property_name_max(sem, p).as_deref() == Some(name) =>
        {
            Some(p.span)
        }
        _ => None,
    })
}

fn create_element_children_count(call: &CallExpression<'_>) -> usize {
    call.arguments.len().saturating_sub(2)
}

// ---------------------------------------------------------------------------
// jsx-no-children-prop / jsx-no-children-prop-with-children
// ---------------------------------------------------------------------------

fn children_prop(ctx: &mut Ctx<'_>, with_children: bool, message: &str) {
    let sem = ctx.semantic;
    let src = ctx.source;
    let mut out = Vec::new();
    for node in sem.nodes().iter() {
        match node.kind() {
            AstKind::CallExpression(call) => {
                let Some(prop) = create_element_prop(sem, call, "children") else {
                    continue;
                };
                if (create_element_children_count(call) > 0) == with_children {
                    out.push(prop);
                }
            }
            AstKind::JSXElement(el) => {
                let Some(attr) = find_attribute(sem, &el.opening_element, "children") else {
                    continue;
                };
                if has_children(src, &el.children) == with_children {
                    out.push(attr.span());
                }
            }
            _ => {}
        }
    }
    for s in out {
        ctx.report(s, message);
    }
}

pub struct JsxNoChildrenProp;
impl Rule for JsxNoChildrenProp {
    fn name(&self) -> &'static str {
        "@eslint-react/jsx-no-children-prop"
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        if !ctx.source.contains("children") {
            return;
        }
        children_prop(ctx, false, "Do not pass 'children' as props.");
    }
}

pub struct JsxNoChildrenPropWithChildren;
impl Rule for JsxNoChildrenPropWithChildren {
    fn name(&self) -> &'static str {
        "@eslint-react/jsx-no-children-prop-with-children"
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        if !ctx.source.contains("children") {
            return;
        }
        children_prop(
            ctx,
            true,
            "Do not pass 'children' as a prop when the element already has children content.",
        );
    }
}

// ---------------------------------------------------------------------------
// jsx-no-key-after-spread
// ---------------------------------------------------------------------------

fn has_key_assignment(source: &str) -> bool {
    // `/key\s*=/`
    let bytes = source.as_bytes();
    let mut from = 0;
    while let Some(i) = source[from..].find("key") {
        let mut j = from + i + 3;
        while j < bytes.len() && (bytes[j] as char).is_ascii_whitespace() {
            j += 1;
        }
        if bytes.get(j) == Some(&b'=') {
            return true;
        }
        from += i + 3;
    }
    false
}

pub struct JsxNoKeyAfterSpread;
impl Rule for JsxNoKeyAfterSpread {
    fn name(&self) -> &'static str {
        "@eslint-react/jsx-no-key-after-spread"
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        if !has_key_assignment(ctx.source) || !is_automatic_jsx_runtime(ctx) {
            return;
        }
        let sem = ctx.semantic;
        let mut out = Vec::new();
        for node in sem.nodes().iter() {
            let AstKind::JSXOpeningElement(o) = node.kind() else {
                continue;
            };
            let after = o
                .attributes
                .iter()
                .skip_while(|a| !matches!(a, JSXAttributeItem::SpreadAttribute(_)));
            out.extend(after.filter(|a| is_attribute(a, "key")).map(GetSpan::span));
        }
        for s in out {
            ctx.report(
                s,
                "Placing 'key' after spread props causes deoptimization when using the automatic JSX runtime. Put 'key' before any spread props.",
            );
        }
    }
}

// ---------------------------------------------------------------------------
// jsx-no-leaked-dollar
// ---------------------------------------------------------------------------

fn is_non_substantive_child(src: &str, c: &JSXChild<'_>) -> bool {
    is_whitespace_text(src, c) || is_empty_string_expression(c) || is_empty_expression_container(c)
}

pub struct JsxNoLeakedDollar;
impl Rule for JsxNoLeakedDollar {
    fn name(&self) -> &'static str {
        "@eslint-react/jsx-no-leaked-dollar"
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        if !ctx.source.contains('$') {
            return;
        }
        let sem = ctx.semantic;
        let src = ctx.source;
        let mut out = Vec::new();
        for node in sem.nodes().iter() {
            let children = match node.kind() {
                AstKind::JSXElement(e) => &e.children,
                AstKind::JSXFragment(f) => &f.children,
                _ => continue,
            };
            for (index, child) in children.iter().enumerate() {
                let JSXChild::Text(t) = child else { continue };
                if !t.value.as_str().ends_with('$') {
                    continue;
                }
                if !matches!(
                    children.get(index + 1),
                    Some(JSXChild::ExpressionContainer(_))
                ) {
                    continue;
                }
                if t.value.as_str().trim() == "$"
                    && children.iter().enumerate().all(|(i, s)| {
                        i == index || i == index + 1 || is_non_substantive_child(src, s)
                    })
                {
                    continue;
                }
                if !jsx_text_raw(src, t).ends_with('$') {
                    continue;
                }
                out.push(Span::new(t.span.end - 1, t.span.end));
            }
        }
        for s in out {
            ctx.report(
                s,
                "Leaked '$' in JSX. This '$' will be rendered as text nodes.",
            );
        }
    }
}

// ---------------------------------------------------------------------------
// jsx-no-leaked-semicolon
// ---------------------------------------------------------------------------

pub struct JsxNoLeakedSemicolon;
impl Rule for JsxNoLeakedSemicolon {
    fn name(&self) -> &'static str {
        "@eslint-react/jsx-no-leaked-semicolon"
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        if !ctx.source.contains(';') {
            return;
        }
        let sem = ctx.semantic;
        let src = ctx.source;
        let mut out = Vec::new();
        for node in sem.nodes().iter() {
            let AstKind::JSXText(t) = node.kind() else {
                continue;
            };
            // `/^;+[ \t]*(?:\r\n|\r|\n)/`
            let raw = jsx_text_raw(src, t);
            let rest = raw.trim_start_matches(';');
            if rest.len() == raw.len() {
                continue;
            }
            let rest = rest.trim_start_matches([' ', '\t']);
            if rest.starts_with('\n') || rest.starts_with('\r') {
                out.push(t.span);
            }
        }
        for s in out {
            ctx.report(
                s,
                "Leaked ';' in JSX. This ';' will be rendered as text nodes.",
            );
        }
    }
}

// ---------------------------------------------------------------------------
// jsx-no-namespace
// ---------------------------------------------------------------------------

pub struct JsxNoNamespace;
impl Rule for JsxNoNamespace {
    fn name(&self) -> &'static str {
        "@eslint-react/jsx-no-namespace"
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        if !ctx.source.contains(':') {
            return;
        }
        let sem = ctx.semantic;
        let mut out = Vec::new();
        for node in sem.nodes().iter() {
            let AstKind::JSXElement(e) = node.kind() else {
                continue;
            };
            let name = crate::utils::jsx_element_name(&e.opening_element.name);
            if name.contains(':') {
                out.push((e.opening_element.name.span(), name));
            }
        }
        for (s, name) in out {
            ctx.report(
                s,
                format!("A React component '{name}' must not be in a namespace, as React does not support them."),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// no-missing-key
// ---------------------------------------------------------------------------

const MISSING_KEY: &str = "Missing 'key' for element when rendering list.";
const FRAGMENT_SYNTAX: &str =
    "Use fragment component instead of '<>' because it does not support `key`.";

fn visit_item_expression(sem: &Semantic<'_>, id: NodeId, out: &mut Vec<(Span, &'static str)>) {
    match kind(sem, id) {
        AstKind::ChainExpression(c) => visit_item_expression(sem, c.expression.node_id(), out),
        AstKind::ParenthesizedExpression(p) => {
            visit_item_expression(sem, p.expression.node_id(), out)
        }
        AstKind::TSAsExpression(e) => visit_item_expression(sem, e.expression.node_id(), out),
        AstKind::TSSatisfiesExpression(e) => {
            visit_item_expression(sem, e.expression.node_id(), out)
        }
        AstKind::TSTypeAssertion(e) => visit_item_expression(sem, e.expression.node_id(), out),
        AstKind::TSNonNullExpression(e) => visit_item_expression(sem, e.expression.node_id(), out),
        AstKind::TSInstantiationExpression(e) => {
            visit_item_expression(sem, e.expression.node_id(), out)
        }
        AstKind::JSXElement(e) => {
            if !has_attribute(sem, &e.opening_element, "key") {
                out.push((e.span, MISSING_KEY));
            }
        }
        AstKind::JSXFragment(f) => out.push((f.span, FRAGMENT_SYNTAX)),
        AstKind::ConditionalExpression(c) => {
            visit_item_expression(sem, c.consequent.node_id(), out);
            visit_item_expression(sem, c.alternate.node_id(), out);
        }
        AstKind::LogicalExpression(l) => {
            visit_item_expression(sem, l.left.node_id(), out);
            visit_item_expression(sem, l.right.node_id(), out);
        }
        _ => {}
    }
}

/// Return arguments of the return statements directly inside a function
/// body (not inside nested functions).
struct Returns {
    args: Vec<NodeId>,
}

impl<'a> oxc_ast_visit::Visit<'a> for Returns {
    fn visit_return_statement(&mut self, r: &ReturnStatement<'a>) {
        if let Some(arg) = &r.argument {
            self.args.push(arg.node_id());
        }
        oxc_ast_visit::walk::walk_return_statement(self, r);
    }
    fn visit_function(&mut self, _f: &Function<'a>, _flags: oxc_syntax::scope::ScopeFlags) {}
    fn visit_arrow_function_expression(&mut self, _f: &ArrowFunctionExpression<'a>) {}
}

fn iterator_callback(sem: &Semantic<'_>, call: &CallExpression<'_>) -> Option<NodeId> {
    let callee = unwrap(sem, call.callee.node_id());
    let prop_name = match kind(sem, callee) {
        AstKind::StaticMemberExpression(m) => m.property.name.as_str(),
        AstKind::ComputedMemberExpression(m) => {
            match kind(sem, skip_parens(sem, m.expression.node_id())) {
                AstKind::IdentifierReference(r) => r.name.as_str(),
                _ => return None,
            }
        }
        _ => return None,
    };
    let position = match prop_name {
        "flatMap" | "map" => 0,
        "from" => 1,
        _ => return None,
    };
    let cb = skip_parens(sem, call.arguments.get(position)?.node_id());
    is_function(kind(sem, cb)).then_some(cb)
}

pub struct NoMissingKey;
impl Rule for NoMissingKey {
    fn name(&self) -> &'static str {
        "@eslint-react/no-missing-key"
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        if !ctx.source.contains('<') && !ctx.source.contains("createElement") {
            // Only JSX elements/fragments are ever reported.
            return;
        }
        let sem = ctx.semantic;
        // Children.toArray calls: everything inside them is skipped.
        let to_array: Vec<NodeId> = if ctx.source.contains("toArray") {
            sem.nodes()
                .iter()
                .filter(|n| matches!(n.kind(), AstKind::CallExpression(c) if is_api_call(sem, c, "Children.toArray")))
                .map(|n| n.id())
                .collect()
        } else {
            Vec::new()
        };
        let inside_to_array = |id: NodeId| {
            !to_array.is_empty()
                && (to_array.contains(&id)
                    || sem.nodes().ancestor_ids(id).any(|a| to_array.contains(&a)))
        };
        let mut out = Vec::new();
        for node in sem.nodes().iter() {
            match node.kind() {
                AstKind::ArrayExpression(a) => {
                    if inside_to_array(node.id()) {
                        continue;
                    }
                    for el in &a.elements {
                        match el {
                            ArrayExpressionElement::Elision(_)
                            | ArrayExpressionElement::SpreadElement(_) => {}
                            other => visit_item_expression(sem, other.node_id(), &mut out),
                        }
                    }
                }
                AstKind::CallExpression(call) => {
                    let Some(cb) = iterator_callback(sem, call) else {
                        continue;
                    };
                    if inside_to_array(node.id()) {
                        continue;
                    }
                    match kind(sem, cb) {
                        AstKind::ArrowFunctionExpression(f) if f.get_expression().is_some() => {
                            visit_item_expression(
                                sem,
                                f.get_expression().unwrap().node_id(),
                                &mut out,
                            );
                        }
                        AstKind::ArrowFunctionExpression(f) => {
                            let mut r = Returns { args: Vec::new() };
                            oxc_ast_visit::Visit::visit_function_body(
                                &mut r,
                                f.get_function_body().unwrap(),
                            );
                            for a in r.args {
                                visit_item_expression(sem, a, &mut out);
                            }
                        }
                        AstKind::Function(f) => {
                            if let Some(body) = &f.body {
                                let mut r = Returns { args: Vec::new() };
                                oxc_ast_visit::Visit::visit_function_body(&mut r, body);
                                for a in r.args {
                                    visit_item_expression(sem, a, &mut out);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        for (s, m) in out {
            ctx.report(s, m);
        }
    }
}

// ---------------------------------------------------------------------------
// no-children-*
// ---------------------------------------------------------------------------

pub struct NoChildrenApi {
    name: &'static str,
    method: &'static str,
    api: &'static str,
}

impl Rule for NoChildrenApi {
    fn name(&self) -> &'static str {
        self.name
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        if !ctx.source.contains("Children") || !ctx.source.contains(self.method) {
            return;
        }
        let sem = ctx.semantic;
        let mut out = Vec::new();
        for node in sem.nodes().iter() {
            let AstKind::StaticMemberExpression(m) = node.kind() else {
                continue;
            };
            if m.property.name != self.method {
                continue;
            }
            if is_api(sem, node.id(), self.api) {
                out.push(m.property.span);
            }
        }
        let message = format!(
            "Using '{}' is uncommon and can lead to fragile code. Use alternatives instead.",
            self.api
        );
        for s in out {
            ctx.report(s, message.clone());
        }
    }
}

pub fn children_rules() -> Vec<Box<dyn Rule>> {
    [
        ("@eslint-react/no-children-count", "count", "Children.count"),
        (
            "@eslint-react/no-children-for-each",
            "forEach",
            "Children.forEach",
        ),
        ("@eslint-react/no-children-map", "map", "Children.map"),
        ("@eslint-react/no-children-only", "only", "Children.only"),
        (
            "@eslint-react/no-children-to-array",
            "toArray",
            "Children.toArray",
        ),
    ]
    .into_iter()
    .map(|(name, method, api)| Box::new(NoChildrenApi { name, method, api }) as Box<dyn Rule>)
    .collect()
}
