//! Component / hook rules from eslint-plugin-react-x:
//! `no-nested-component-definitions`, `no-nested-lazy-component-declarations`,
//! `no-unnecessary-use-prefix` and `use-state`.

use std::collections::HashSet;

use oxc_ast::AstKind;
use oxc_ast::ast::*;
use oxc_semantic::{AstNodes, NodeId};
use oxc_span::{GetSpan, Span};
use serde_json::Value;

use super::helpers::*;
use super::no_options;
use crate::rule::{Ctx, Rule};

// ---------------------------------------------------------------------------
// no-nested-component-definitions
// ---------------------------------------------------------------------------

pub struct NoNestedComponentDefinitions;

const WRAPPER_APIS: &[&str] = &[
    "connect",
    "createFragmentContainer",
    "createPaginationContainer",
    "createRefetchContainer",
    "forwardRef",
    "graphql",
    "memo",
    "observer",
    "useCallback",
    "withFormik",
    "withHandlers",
    "withLifecycle",
    "withProps",
    "withRouter",
    "withState",
];

/// `isComponentWrapperCall(context, call, arg)`.
fn is_component_wrapper_call(source: &str, call: &CallExpression<'_>, arg: NodeId) -> bool {
    if unwrap(&call.callee).node_id() == arg {
        return false;
    }
    let mut inner = call;
    while let Un::Call(c) = unwrap(&inner.callee) {
        inner = c;
    }
    WRAPPER_APIS
        .iter()
        .any(|api| is_api_call(source, inner, api))
}

/// `getWrapperCallBoundName`.
fn wrapper_call_bound_name<'a>(nodes: &AstNodes<'a>, source: &str, id: NodeId) -> Option<&'a str> {
    let (mut current, mut p) = skip_type_parents(nodes, id);
    while let Some(pid) = p {
        let AstKind::CallExpression(call) = nodes.kind(pid) else {
            break;
        };
        if !is_component_wrapper_call(source, call, current) {
            break;
        }
        (current, p) = skip_type_parents(nodes, pid);
    }
    match nodes.kind(p?) {
        AstKind::VariableDeclarator(d) => match &d.id {
            BindingPattern::BindingIdentifier(b) => Some(b.name.as_str()),
            _ => None,
        },
        _ => None,
    }
}

impl Rule for NoNestedComponentDefinitions {
    fn name(&self) -> &'static str {
        "@eslint-react/no-nested-component-definitions"
    }
    fn validate(&self, options: &[Value], _: &Value) -> Result<(), String> {
        no_options(options)
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        let h = hint::NUMBER
            | hint::BOOLEAN
            | hint::NULL
            | hint::STRING
            | hint::UNDEFINED
            | hint::BOTH_LOGICAL
            | hint::BOTH_CONDITIONAL
            | hint::ARRAY_PATTERN_ELEMENT
            | hint::ARRAY_EXPRESSION_ELEMENT
            | hint::MAP_CALLBACK
            | hint::FLAT_MAP_CALLBACK;
        let collected = collect(
            ctx,
            Want {
                components: Some(h),
                hooks: false,
                classes: true,
            },
        );
        if collected.components.is_empty() && collected.classes.is_empty() {
            return;
        }
        let nodes = ctx.semantic.nodes();
        let src = ctx.source;
        let f_ids: HashSet<NodeId> = collected.component_ids().collect();
        let c_ids: HashSet<NodeId> = collected
            .classes
            .iter()
            .map(|c| c.class.node_id())
            .collect();
        let enclosing = |id: NodeId| {
            find_parent(
                nodes,
                id,
                |n, k| {
                    if as_func(k).is_some() {
                        f_ids.contains(&n)
                    } else if matches!(k, AstKind::Class(_)) {
                        c_ids.contains(&n)
                    } else {
                        false
                    }
                },
                no_stop,
            )
        };
        let parent_is_property = |id: NodeId| {
            parent(nodes, id).is_some_and(|p| matches!(nodes.kind(p), AstKind::ObjectProperty(_)))
        };
        const PROP: &str = "Move it to the top level or pass it as a prop.";
        const TOP: &str = "Move it to the top level.";

        let mut reports: Vec<(Span, &str)> = Vec::new();
        for comp in &collected.components {
            let id = comp.func.node_id();
            let name = comp
                .name
                .clone()
                .or_else(|| wrapper_call_bound_name(nodes, src, id).map(str::to_string));
            let Some(name) = name else { continue };
            if !is_component_name_loose(&name) {
                continue;
            }
            let span = comp.func.span();
            // isInsideJSXAttributeValue
            let in_attr = parent(nodes, id)
                .is_some_and(|p| matches!(nodes.kind(p), AstKind::JSXAttribute(_)))
                || find_parent(
                    nodes,
                    id,
                    |_, k| {
                        matches!(k, AstKind::JSXAttribute(a)
                        if matches!(a.value, Some(JSXAttributeValue::ExpressionContainer(_))))
                    },
                    no_stop,
                )
                .is_some();
            if in_attr || is_inside_create_element_props(nodes, src, id) {
                reports.push((span, PROP));
                continue;
            }
            if enclosing(id).is_some() {
                reports.push((span, if parent_is_property(id) { PROP } else { TOP }));
                continue;
            }
            let in_render = find_parent(
                nodes,
                id,
                |n, _| {
                    is_render_method_like(nodes, n)
                        && class_of_member(nodes, n).is_some_and(is_class_component)
                },
                no_stop,
            )
            .is_some();
            if in_render {
                reports.push((span, TOP));
            }
        }
        for class in &collected.classes {
            let id = class.class.node_id();
            if enclosing(id).is_none() {
                continue;
            }
            reports.push((
                class.class.span,
                if parent_is_property(id) { PROP } else { TOP },
            ));
        }
        for (s, suggestion) in reports {
            ctx.report(
                s,
                format!("Do not nest component definitions inside other components or props. {suggestion}"),
            );
        }
    }
}

/// `isInsideCreateElementProps`.
fn is_inside_create_element_props(nodes: &AstNodes<'_>, src: &str, id: NodeId) -> bool {
    let Some(call) = find_parent(
        nodes,
        id,
        |_, k| matches!(k, AstKind::CallExpression(c) if is_create_element_call(src, c)),
        no_stop,
    ) else {
        return false;
    };
    let Some(obj) = find_parent(
        nodes,
        id,
        |_, k| matches!(k, AstKind::ObjectExpression(_)),
        no_stop,
    ) else {
        return false;
    };
    let AstKind::CallExpression(call) = nodes.kind(call) else {
        return false;
    };
    call.arguments
        .get(1)
        .and_then(Argument::as_expression)
        .is_some_and(|a| unwrap(a).node_id() == obj)
}

// ---------------------------------------------------------------------------
// no-nested-lazy-component-declarations
// ---------------------------------------------------------------------------

pub struct NoNestedLazyComponentDeclarations;

impl Rule for NoNestedLazyComponentDeclarations {
    fn name(&self) -> &'static str {
        "@eslint-react/no-nested-lazy-component-declarations"
    }
    fn validate(&self, options: &[Value], _: &Value) -> Result<(), String> {
        no_options(options)
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        let nodes = ctx.semantic.nodes();
        let src = ctx.source;
        let mut lazy_calls: Vec<NodeId> = Vec::new();
        for node in nodes.iter() {
            if !matches!(node.kind(), AstKind::ImportExpression(_)) {
                continue;
            }
            if let Some(c) = find_parent(
                nodes,
                node.id(),
                |_, k| matches!(k, AstKind::CallExpression(c) if is_api_call(src, c, "lazy")),
                no_stop,
            ) && !lazy_calls.contains(&c)
            {
                lazy_calls.push(c);
            }
        }
        if lazy_calls.is_empty() {
            return;
        }
        let collected = collect(
            ctx,
            Want {
                components: Some(hint::DEFAULT_COMPONENT),
                hooks: true,
                classes: true,
            },
        );
        let significant: HashSet<NodeId> = collected
            .component_ids()
            .chain(collected.hook_ids())
            .chain(collected.classes.iter().map(|c| c.class.node_id()))
            .collect();
        let mut spans = Vec::new();
        for lazy in lazy_calls {
            if nodes.ancestor_ids(lazy).any(|p| significant.contains(&p)) {
                spans.push(nodes.kind(lazy).span());
            }
        }
        for s in spans {
            ctx.report(
                s,
                "Do not declare lazy components that wrap dynamic imports inside other components or hooks. Instead, always declare them at the top level of your module.",
            );
        }
    }
}

// ---------------------------------------------------------------------------
// no-unnecessary-use-prefix
// ---------------------------------------------------------------------------

pub struct NoUnnecessaryUsePrefix;

/// `/use\([\s\S]*?\)/u.test(v) || /use[A-Z0-9]\w*\([\s\S]*?\)/u.test(v)`.
fn has_use_call_text(v: &str) -> bool {
    let b = v.as_bytes();
    let mut i = 0;
    while let Some(off) = v[i..].find("use") {
        let mut j = i + off + 3;
        let ok_open = if b.get(j) == Some(&b'(') {
            true
        } else if b
            .get(j)
            .is_some_and(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        {
            j += 1;
            while b
                .get(j)
                .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
            {
                j += 1;
            }
            b.get(j) == Some(&b'(')
        } else {
            false
        };
        if ok_open && v[j + 1..].contains(')') {
            return true;
        }
        i += off + 1;
    }
    false
}

fn is_test_mock_call(call: &CallExpression<'_>) -> bool {
    let Some(m) = strip_parens(&call.callee).as_member_expression() else {
        return false;
    };
    let obj_ident = match strip_parens(m.object()) {
        Expression::Identifier(i) => Some(i.name.as_str()),
        _ => None,
    };
    let prop = member_property_ident(m);
    (obj_ident.is_some() && prop == Some("mock"))
        || (obj_ident == Some("mock") && prop == Some("module"))
}

impl Rule for NoUnnecessaryUsePrefix {
    fn name(&self) -> &'static str {
        "@eslint-react/no-unnecessary-use-prefix"
    }
    fn validate(&self, options: &[Value], _: &Value) -> Result<(), String> {
        no_options(options)
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        if !ctx.source.contains("use") {
            return;
        }
        let collected = collect(
            ctx,
            Want {
                components: None,
                hooks: true,
                classes: false,
            },
        );
        let nodes = ctx.semantic.nodes();
        let mut spans = Vec::new();
        for hook in &collected.hooks {
            let Some(name) = &hook.name else { continue };
            if hook.hook_calls > 0 || hook.func.is_empty() {
                continue;
            }
            if name == "use" || name == "useMDXComponents" {
                continue;
            }
            let fspan = hook.func.span();
            let has_use_comment = ctx.program.comments.iter().any(|c| {
                c.span.start >= fspan.start && c.span.end <= fspan.end && {
                    let text = &ctx.source[c.span.start as usize..c.span.end as usize];
                    let inner = if let Some(line) = text.strip_prefix("//") {
                        line
                    } else {
                        &text[2..text.len().saturating_sub(2).max(2)]
                    };
                    has_use_call_text(inner)
                }
            });
            if has_use_comment {
                continue;
            }
            let in_mock = nodes
                .ancestor_ids(hook.func.node_id())
                .any(|p| match nodes.kind(p) {
                    AstKind::CallExpression(c) if is_test_mock_call(c) => c
                        .arguments
                        .iter()
                        .skip(1)
                        .any(|a| a.span().start <= fspan.start && fspan.end <= a.span().end),
                    _ => false,
                });
            if in_mock {
                continue;
            }
            spans.push(hook.id.map_or(fspan, |i| i.span()));
        }
        for s in spans {
            ctx.report(
                s,
                "If your function doesn't call any Hooks, avoid the 'use' prefix. Instead, write it as a regular function without the 'use' prefix.",
            );
        }
    }
}

// ---------------------------------------------------------------------------
// use-state
// ---------------------------------------------------------------------------

pub struct UseState;

struct UseStateOptions {
    enforce_assignment: bool,
    enforce_lazy_initialization: bool,
    enforce_setter_name: bool,
}

fn use_state_options(options: &[Value]) -> UseStateOptions {
    let o = options.first();
    let get = |k: &str| {
        o.and_then(|o| o.get(k))
            .and_then(Value::as_bool)
            .unwrap_or(true)
    };
    UseStateOptions {
        enforce_assignment: get("enforceAssignment"),
        enforce_lazy_initialization: get("enforceLazyInitialization"),
        enforce_setter_name: get("enforceSetterName"),
    }
}

const LAZY_INIT_ALLOW_LIST: &[&str] = &["Boolean", "String", "Number"];

/// `getNestedNewExpressions` / `getNestedCallExpressions`: walks the
/// expression-only visitor keys of upstream's `EXPRESSION_VISITOR_KEYS` (no
/// function bodies) and collects `new` and call expressions with their
/// callee name (when the callee is an identifier).
fn nested_calls<'b, 'a>(
    e: &'b Expression<'a>,
    news: &mut Vec<(NodeId, Span, Option<&'b str>)>,
    calls: &mut Vec<(NodeId, Span, Option<&'b str>)>,
) {
    let name = |c: &'b Expression<'a>| match strip_parens(c) {
        Expression::Identifier(i) => Some(i.name.as_str()),
        _ => None,
    };
    match e {
        Expression::ArrayExpression(a) => {
            for el in &a.elements {
                match el {
                    ArrayExpressionElement::SpreadElement(s) => {
                        nested_calls(&s.argument, news, calls)
                    }
                    ArrayExpressionElement::Elision(_) => {}
                    _ => nested_calls(el.to_expression(), news, calls),
                }
            }
        }
        Expression::AssignmentExpression(a) => {
            if let Some(m) = a.left.as_member_expression() {
                member_calls(m, news, calls);
            }
            nested_calls(&a.right, news, calls);
        }
        Expression::AwaitExpression(a) => nested_calls(&a.argument, news, calls),
        Expression::BinaryExpression(b) => {
            nested_calls(&b.left, news, calls);
            nested_calls(&b.right, news, calls);
        }
        Expression::PrivateInExpression(p) => nested_calls(&p.right, news, calls),
        Expression::CallExpression(c) => {
            calls.push((c.node_id(), c.span, name(&c.callee)));
            nested_calls(&c.callee, news, calls);
            args_calls(&c.arguments, news, calls);
        }
        Expression::NewExpression(n) => {
            news.push((n.node_id(), n.span, name(&n.callee)));
            nested_calls(&n.callee, news, calls);
            args_calls(&n.arguments, news, calls);
        }
        Expression::ChainExpression(c) => match &c.expression {
            ChainElement::CallExpression(call) => {
                calls.push((call.node_id(), call.span, name(&call.callee)));
                nested_calls(&call.callee, news, calls);
                args_calls(&call.arguments, news, calls);
            }
            ChainElement::TSNonNullExpression(n) => nested_calls(&n.expression, news, calls),
            other => member_calls(other.as_member_expression().expect("member"), news, calls),
        },
        Expression::ConditionalExpression(c) => {
            nested_calls(&c.test, news, calls);
            nested_calls(&c.consequent, news, calls);
            nested_calls(&c.alternate, news, calls);
        }
        Expression::ImportExpression(i) => nested_calls(&i.source, news, calls),
        Expression::LogicalExpression(l) => {
            nested_calls(&l.left, news, calls);
            nested_calls(&l.right, news, calls);
        }
        Expression::ObjectExpression(o) => {
            for p in &o.properties {
                match p {
                    ObjectPropertyKind::ObjectProperty(p) => nested_calls(&p.value, news, calls),
                    ObjectPropertyKind::SpreadProperty(s) => nested_calls(&s.argument, news, calls),
                }
            }
        }
        Expression::SequenceExpression(s) => {
            for x in &s.expressions {
                nested_calls(x, news, calls);
            }
        }
        Expression::TaggedTemplateExpression(t) => {
            nested_calls(&t.tag, news, calls);
            for x in &t.quasi.expressions {
                nested_calls(x, news, calls);
            }
        }
        Expression::TemplateLiteral(t) => {
            for x in &t.expressions {
                nested_calls(x, news, calls);
            }
        }
        Expression::TSAsExpression(x) => nested_calls(&x.expression, news, calls),
        Expression::TSSatisfiesExpression(x) => nested_calls(&x.expression, news, calls),
        Expression::TSTypeAssertion(x) => nested_calls(&x.expression, news, calls),
        Expression::TSNonNullExpression(x) => nested_calls(&x.expression, news, calls),
        Expression::TSInstantiationExpression(x) => nested_calls(&x.expression, news, calls),
        Expression::ParenthesizedExpression(p) => nested_calls(&p.expression, news, calls),
        Expression::UnaryExpression(u) => nested_calls(&u.argument, news, calls),
        Expression::UpdateExpression(u) => {
            if let Some(m) = u.argument.as_member_expression() {
                member_calls(m, news, calls);
            }
        }
        Expression::YieldExpression(y) => {
            if let Some(a) = &y.argument {
                nested_calls(a, news, calls);
            }
        }
        Expression::JSXElement(_) | Expression::JSXFragment(_) => {}
        _ => {
            if let Some(m) = e.as_member_expression() {
                member_calls(m, news, calls);
            }
        }
    }
}

fn member_calls<'b, 'a>(
    m: &'b MemberExpression<'a>,
    news: &mut Vec<(NodeId, Span, Option<&'b str>)>,
    calls: &mut Vec<(NodeId, Span, Option<&'b str>)>,
) {
    nested_calls(m.object(), news, calls);
    if let MemberExpression::ComputedMemberExpression(c) = m {
        nested_calls(&c.expression, news, calls);
    }
}

fn args_calls<'b, 'a>(
    args: &'b [Argument<'a>],
    news: &mut Vec<(NodeId, Span, Option<&'b str>)>,
    calls: &mut Vec<(NodeId, Span, Option<&'b str>)>,
) {
    for a in args {
        match a {
            Argument::SpreadElement(s) => nested_calls(&s.argument, news, calls),
            _ => nested_calls(a.to_expression(), news, calls),
        }
    }
}

/// `string-ts` `words()` + `snakeCase()`.
pub fn snake_case(s: &str) -> String {
    let s: String = s.chars().filter(|c| *c != '\'').collect();
    let s: String = s
        .chars()
        .map(|c| if "[]{}()|/-\\ _.".contains(c) { ' ' } else { c })
        .collect();
    let alpha = |c: char| c.is_ascii_alphabetic();
    let digit = |c: char| c.is_ascii_digit();
    let wordish = |c: char| c.is_ascii_alphanumeric() || "_-./".contains(c);
    let s = space_pairs(&s, |a, b| alpha(a) && digit(b));
    let s = space_pairs(&s, |a, b| digit(a) && alpha(b));
    let s = space_pairs(&s, |a, b| wordish(a) && !(wordish(b) || b == '\''));
    let s = space_pairs(&s, |a, b| !(wordish(a) || a == '\'') && wordish(b));
    let s = space_pairs(&s, |a, b| a.is_ascii_lowercase() && b.is_ascii_uppercase());
    // /([A-Z])([A-Z][a-z])/g
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if i + 2 < chars.len()
            && chars[i].is_ascii_uppercase()
            && chars[i + 1].is_ascii_uppercase()
            && chars[i + 2].is_ascii_lowercase()
        {
            out.push(chars[i]);
            out.push(' ');
            out.push(chars[i + 1]);
            out.push(chars[i + 2]);
            i += 3;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out.split_whitespace()
        .collect::<Vec<_>>()
        .join("_")
        .to_lowercase()
}

/// Global `/(A)(B)/g` → `"$1 $2"` over single characters.
fn space_pairs(s: &str, f: impl Fn(char, char) -> bool) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len() + 4);
    let mut i = 0;
    while i < chars.len() {
        if i + 1 < chars.len() && f(chars[i], chars[i + 1]) {
            out.push(chars[i]);
            out.push(' ');
            out.push(chars[i + 1]);
            i += 2;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// `resolveEnclosingAssignmentTarget` for the call: the closest declarator id
/// / assignment left / class property key / export default declaration.
enum Target<'a> {
    Pattern(&'a BindingPattern<'a>),
    Other(Span),
}

fn enclosing_assignment_target<'a>(nodes: &AstNodes<'a>, id: NodeId) -> Option<Target<'a>> {
    let mut cur = Some(id);
    while let Some(c) = cur {
        match nodes.kind(c) {
            AstKind::VariableDeclarator(d) => return Some(Target::Pattern(&d.id)),
            AstKind::AssignmentExpression(a) => return Some(Target::Other(a.left.span())),
            AstKind::PropertyDefinition(p) => return Some(Target::Other(p.key.span())),
            AstKind::ExportDefaultDeclaration(e) => {
                return Some(Target::Other(e.declaration.span()));
            }
            AstKind::BlockStatement(_)
            | AstKind::FunctionBody(_)
            | AstKind::StaticBlock(_)
            | AstKind::Program(_) => {
                return None;
            }
            _ => cur = parent(nodes, c),
        }
    }
    None
}

impl Rule for UseState {
    fn name(&self) -> &'static str {
        "@eslint-react/use-state"
    }
    fn validate(&self, options: &[Value], _: &Value) -> Result<(), String> {
        if options.len() > 1 {
            return Err("expected at most one options object".into());
        }
        if let Some(o) = options.first() {
            let obj = o.as_object().ok_or("options must be an object")?;
            for (k, v) in obj {
                if !matches!(
                    k.as_str(),
                    "enforceAssignment" | "enforceLazyInitialization" | "enforceSetterName"
                ) {
                    return Err(format!("unknown option {k:?}"));
                }
                if !v.is_boolean() {
                    return Err(format!("option {k:?} must be a boolean"));
                }
            }
        }
        Ok(())
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        let extra = additional_hooks_matcher(ctx, "additionalStateHooks");
        if !ctx.source.contains("useState")
            && extra.as_deref().is_none_or(|e| !ctx.source.contains(e))
        {
            return;
        }
        let opts = use_state_options(ctx.options);
        let nodes = ctx.semantic.nodes();
        let src = ctx.source;
        const INIT: &str = "To prevent re-computation, consider using lazy initial state for useState calls that involve function calls. Ex: 'useState(() => getValue())'.";
        const ASSIGN: &str = "useState should be destructured into a value and setter pair, e.g., const [state, setState] = useState(...).";
        const SETTER: &str = "The setter should be named 'set' followed by the capitalized state variable name, e.g., 'setState' for 'state'.";
        let mut reports: Vec<(Span, &str)> = Vec::new();
        let has_use_call_ancestor = |id: NodeId| {
            nodes
                .ancestor_ids(id)
                .any(|p| matches!(nodes.kind(p), AstKind::CallExpression(c) if is_api_call(src, c, "use")))
        };
        for node in nodes.iter() {
            let AstKind::CallExpression(call) = node.kind() else {
                continue;
            };
            let Some(cname) = callee_name(call) else {
                continue;
            };
            if cname != "useState" && extra.as_deref() != Some(cname) {
                continue;
            }
            if opts.enforce_lazy_initialization
                && let Some(input) = call.arguments.first()
            {
                let (mut news, mut calls) = (Vec::new(), Vec::new());
                match input {
                    Argument::SpreadElement(s) => nested_calls(&s.argument, &mut news, &mut calls),
                    _ => nested_calls(input.to_expression(), &mut news, &mut calls),
                }
                for (nid, span, name) in news {
                    let Some(name) = name else { continue };
                    if LAZY_INIT_ALLOW_LIST.contains(&name) || has_use_call_ancestor(nid) {
                        continue;
                    }
                    reports.push((span, INIT));
                }
                for (nid, span, name) in calls {
                    let Some(name) = name else { continue };
                    if is_hook_name(name)
                        || LAZY_INIT_ALLOW_LIST.contains(&name)
                        || has_use_call_ancestor(nid)
                    {
                        continue;
                    }
                    reports.push((span, INIT));
                }
            }
            let id = node.id();
            let (_, p) = skip_type_parents(nodes, id);
            let pk = p.map(|p| nodes.kind(p));
            if matches!(
                pk,
                Some(AstKind::ReturnStatement(_) | AstKind::ArrowFunctionExpression(_))
            ) {
                continue;
            }
            if !matches!(pk, Some(AstKind::VariableDeclarator(_))) {
                if opts.enforce_assignment {
                    reports.push((call.span, ASSIGN));
                }
                continue;
            }
            let target = enclosing_assignment_target(nodes, id);
            let arr = match &target {
                Some(Target::Pattern(BindingPattern::ArrayPattern(a))) => a,
                _ => {
                    if opts.enforce_assignment {
                        let span = match target {
                            Some(Target::Pattern(p)) => p.span(),
                            Some(Target::Other(s)) => s,
                            None => call.span,
                        };
                        reports.push((span, ASSIGN));
                    }
                    continue;
                }
            };
            // ESTree `elements` includes the rest element at the end.
            let value = arr.elements.first();
            let setter_rest = arr.rest.as_ref().filter(|_| arr.elements.len() == 1);
            let value = match value {
                Some(Some(v)) => v,
                Some(None) => {
                    if opts.enforce_assignment {
                        reports.push((arr.span, ASSIGN));
                    }
                    continue;
                }
                None => {
                    // `[]` or `[...rest]`
                    if arr.rest.is_none() {
                        if opts.enforce_assignment {
                            reports.push((arr.span, ASSIGN));
                        }
                    } else if opts.enforce_setter_name {
                        // value is the RestElement (not an Identifier); a
                        // setter is only present with two elements.
                    }
                    continue;
                }
            };
            let setter: Option<Result<&BindingPattern<'_>, Span>> = match arr.elements.get(1) {
                Some(Some(s)) => Some(Ok(s)),
                Some(None) => None,
                None => setter_rest.map(|r| Err(r.span)),
            };
            let Some(setter) = setter else { continue };
            if !opts.enforce_setter_name {
                continue;
            }
            let BindingPattern::BindingIdentifier(value_id) = value else {
                reports.push((value.span(), ASSIGN));
                continue;
            };
            let setter_id = match setter {
                Ok(BindingPattern::BindingIdentifier(s)) if s.name.starts_with("set") => s,
                Ok(other) => {
                    reports.push((other.span(), SETTER));
                    continue;
                }
                Err(span) => {
                    reports.push((span, SETTER));
                    continue;
                }
            };
            if snake_case(setter_id.name.as_str())
                != format!("set_{}", snake_case(value_id.name.as_str()))
            {
                reports.push((setter_id.span, SETTER));
            }
        }
        for (s, m) in reports {
            ctx.report(s, m);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::snake_case;

    #[test]
    fn snake() {
        assert_eq!(snake_case("setFooBar"), "set_foo_bar");
        assert_eq!(snake_case("fooBar"), "foo_bar");
        assert_eq!(snake_case("setHTMLParser"), "set_html_parser");
        assert_eq!(snake_case("HTMLParser"), "html_parser");
        assert_eq!(snake_case("set2D"), "set_2_d");
        assert_eq!(snake_case("value_1"), "value_1");
    }
}
