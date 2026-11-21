//! `@eslint-react/web-api-*` (eslint-plugin-react-web-api): resources set up
//! in an effect (`use*Effect` setup callback) must be released in its
//! cleanup function.

use std::collections::{HashMap, HashSet};

use oxc_ast::AstKind;
use oxc_ast::ast::*;
use oxc_semantic::Semantic;
use oxc_span::Span;
use oxc_syntax::node::NodeId;

use super::helpers::*;
use crate::rule::{Ctx, Rule};

fn has_use_effect_text(source: &str) -> bool {
    // `/use\w*Effect/u`
    let mut from = 0;
    while let Some(i) = source[from..].find("use") {
        let start = from + i + 3;
        let word_end = source[start..]
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .map_or(source.len(), |e| start + e);
        if source[start..word_end].contains("Effect") {
            return true;
        }
        from = start;
    }
    false
}

/// `getEffectKind`: callee name of the call the effect callback is passed to.
fn effect_kind<'a>(sem: &Semantic<'a>, fn_id: NodeId) -> &'a str {
    let call = find_parent_simple(sem, unwrap(sem, fn_id), |n| !is_type_or_chain(kind(sem, n)));
    call.and_then(|c| callee_name_of_id(sem, c))
        .unwrap_or("useEffect")
}

fn effect_callback(sem: &Semantic<'_>, id: NodeId) -> Option<NodeId> {
    find_parent_simple(sem, id, |n| is_use_effect_callback(sem, n))
}

// ---------------------------------------------------------------------------
// no-leaked-timeout / no-leaked-interval
// ---------------------------------------------------------------------------

struct TimerRule {
    name: &'static str,
    set: &'static str,
    clear: &'static str,
}

impl Rule for TimerRule {
    fn name(&self) -> &'static str {
        self.name
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        if !ctx.source.contains(self.set) {
            return;
        }
        let sem = ctx.semantic;
        let mut reports: Vec<(Span, String)> = Vec::new();
        let mut sets: Vec<(Span, NodeId, &str)> = Vec::new();
        let mut clears: Vec<NodeId> = Vec::new();
        for node in sem.nodes().iter() {
            let AstKind::CallExpression(call) = node.kind() else {
                continue;
            };
            let Some(name) = callee_name(sem, call) else {
                continue;
            };
            if name != self.set && name != self.clear {
                continue;
            }
            let Some(fn_id) = effect_callback(sem, node.id()) else {
                continue;
            };
            if name == self.set {
                match resolve_enclosing_assignment_target(sem, node.id()) {
                    None => reports.push((
                        call.span,
                        format!(
                            "A '{}' must be assigned to a variable for proper cleanup.",
                            self.set
                        ),
                    )),
                    Some(target) => sets.push((call.span, target, effect_kind(sem, fn_id))),
                }
            } else if let Some(arg) = call.arguments.first() {
                clears.push(skip_parens(sem, arg.node_id()));
            }
        }
        for (span, target, kind_name) in sets {
            if clears
                .iter()
                .any(|&c| is_assignment_target_equal(sem, target, c))
            {
                continue;
            }
            reports.push((
                span,
                format!(
                    "A '{}' created in '{kind_name}' must be cleared with '{}' in the cleanup function.",
                    self.set, self.clear
                ),
            ));
        }
        for (s, m) in reports {
            ctx.report(s, m);
        }
    }
}

// ---------------------------------------------------------------------------
// no-leaked-event-listener
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct ListenerOptions {
    capture: bool,
    signal: Option<NodeId>,
}

const DEFAULT_OPTIONS: ListenerOptions = ListenerOptions {
    capture: false,
    signal: None,
};

/// `isSignalPropertyBinding`: `const { signal } = …`.
fn is_signal_property_binding(sem: &Semantic<'_>, ident: NodeId, name: &str) -> bool {
    let Some(sym) = symbol_of(sem, ident) else {
        return false;
    };
    let Def::Variable(decl) = def_of(sem, sym) else {
        return false;
    };
    let AstKind::VariableDeclarator(d) = kind(sem, decl) else {
        return false;
    };
    let BindingPattern::ObjectPattern(op) = &d.id else {
        return false;
    };
    for prop in &op.properties {
        if prop.computed {
            continue;
        }
        let PropertyKey::StaticIdentifier(key) = &prop.key else {
            continue;
        };
        if key.name != "signal" {
            continue;
        }
        return matches!(&prop.value, BindingPattern::BindingIdentifier(b) if b.name == name);
    }
    false
}

fn signal_value_expression(sem: &Semantic<'_>, id: Option<NodeId>, depth: u32) -> Option<NodeId> {
    let id = skip_parens(sem, id?);
    if depth > 32 {
        return None;
    }
    match kind(sem, id) {
        AstKind::IdentifierReference(r) => {
            let unwrapped = resolve_origin(sem, id).map(|n| unwrap(sem, n));
            if let Some(u) = unwrapped {
                if is_function(kind(sem, u)) {
                    return Some(id);
                }
                if let AstKind::NewExpression(n) = kind(sem, u)
                    && is_signal_property_binding(sem, id, r.name.as_str())
                    && matches!(kind(sem, unwrap(sem, n.callee.node_id())), AstKind::IdentifierReference(c) if c.name == "AbortController")
                {
                    return Some(id);
                }
            }
            signal_value_expression(sem, unwrapped, depth + 1)
        }
        k if is_member(k) => Some(id),
        _ => None,
    }
}

/// `Extract.findProperty(properties, name)`.
fn find_property<'b, 'a>(
    sem: &Semantic<'a>,
    props: &'b oxc_allocator::Vec<'a, ObjectPropertyKind<'a>>,
    name: &str,
) -> Option<&'b ObjectProperty<'a>> {
    for p in props {
        match p {
            ObjectPropertyKind::ObjectProperty(p) => {
                let key = unwrap(sem, p.key.node_id());
                if !p.computed && ident_name(kind(sem, key)) == Some(name) {
                    return Some(p);
                }
            }
            ObjectPropertyKind::SpreadProperty(s) => {
                if let AstKind::ObjectExpression(o) =
                    kind(sem, skip_parens(sem, s.argument.node_id()))
                    && let Some(found) = find_property(sem, &o.properties, name)
                {
                    return Some(found);
                }
            }
        }
    }
    None
}

fn listener_options(sem: &Semantic<'_>, id: NodeId, depth: u32) -> ListenerOptions {
    let id = skip_parens(sem, id);
    if depth > 32 {
        return DEFAULT_OPTIONS;
    }
    let k = kind(sem, id);
    match k {
        AstKind::IdentifierReference(_) => match resolve(sem, id) {
            Some(init) if matches!(kind(sem, init), AstKind::ObjectExpression(_)) => {
                listener_options(sem, init, depth + 1)
            }
            _ => DEFAULT_OPTIONS,
        },
        AstKind::ObjectExpression(o) => {
            let capture = find_property(sem, &o.properties, "capture").is_some_and(|p| {
                let v = skip_parens(sem, p.value.node_id());
                static_value(sem, v).is_some_and(|v| v.truthy())
            });
            let signal = find_property(sem, &o.properties, "signal")
                .and_then(|p| signal_value_expression(sem, Some(p.value.node_id()), 0));
            ListenerOptions { capture, signal }
        }
        _ => match static_literal_truthy(k) {
            Some(capture) => ListenerOptions {
                capture,
                signal: None,
            },
            None => DEFAULT_OPTIONS,
        },
    }
}

/// `Boolean(literal.value)` for ESTree `Literal` nodes.
fn static_literal_truthy(k: AstKind<'_>) -> Option<bool> {
    Some(match k {
        AstKind::StringLiteral(s) => !s.value.as_str().unwrap_or("").is_empty(),
        AstKind::NumericLiteral(n) => n.value != 0.0 && !n.value.is_nan(),
        AstKind::BooleanLiteral(b) => b.value,
        AstKind::NullLiteral(_) => false,
        AstKind::BigIntLiteral(b) => b.value.as_str() != "0",
        AstKind::RegExpLiteral(_) => true,
        _ => return None,
    })
}

enum CleanupCallee<'a> {
    Function(NodeId),
    Name(&'a str),
}

fn resolve_cleanup_callee<'a>(
    sem: &Semantic<'a>,
    call: &CallExpression<'a>,
) -> Option<CleanupCallee<'a>> {
    let callee = unwrap(sem, call.callee.node_id());
    match kind(sem, callee) {
        AstKind::IdentifierReference(r) => Some(match resolve_local_function(sem, callee) {
            Some(f) => CleanupCallee::Function(f),
            None => CleanupCallee::Name(r.name.as_str()),
        }),
        AstKind::StaticMemberExpression(m) => {
            let object = unwrap(sem, m.object.node_id());
            if !matches!(kind(sem, object), AstKind::IdentifierReference(_)) {
                return None;
            }
            let sym = symbol_of(sem, object)?;
            let Def::Variable(decl) = def_of(sem, sym) else {
                return None;
            };
            let AstKind::VariableDeclarator(d) = kind(sem, decl) else {
                return None;
            };
            let init = unwrap(sem, d.init.as_ref()?.node_id());
            let AstKind::ObjectExpression(o) = kind(sem, init) else {
                return None;
            };
            for p in &o.properties {
                let ObjectPropertyKind::ObjectProperty(p) = p else {
                    continue;
                };
                if p.computed {
                    continue;
                }
                let key = match &p.key {
                    PropertyKey::StaticIdentifier(i) => Some(i.name.as_str()),
                    PropertyKey::StringLiteral(s) => s.value.as_str(),
                    _ => None,
                };
                if key != Some(m.property.name.as_str()) {
                    continue;
                }
                let value = unwrap(sem, p.value.node_id());
                if is_function(kind(sem, value)) {
                    return Some(CleanupCallee::Function(value));
                }
            }
            None
        }
        _ => None,
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Setup,
    Cleanup,
}

struct ListenerEntry {
    ty: NodeId,
    callee: NodeId,
    capture: bool,
    signal: Option<NodeId>,
    effect: Option<NodeId>,
    listener: NodeId,
    node: NodeId,
    span: Span,
    phase: Phase,
}

pub struct NoLeakedEventListener;
impl Rule for NoLeakedEventListener {
    fn name(&self) -> &'static str {
        "@eslint-react/web-api-no-leaked-event-listener"
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        if !ctx.source.contains("addEventListener") || !has_use_effect_text(ctx.source) {
            return;
        }
        let sem = ctx.semantic;
        let mut reports: Vec<(Span, String)> = Vec::new();
        let mut adds: Vec<ListenerEntry> = Vec::new();
        let mut removes: Vec<ListenerEntry> = Vec::new();
        let mut cleanup_functions: HashMap<NodeId, HashSet<NodeId>> = HashMap::new();
        let mut cleanup_names: HashMap<NodeId, HashSet<&str>> = HashMap::new();

        for node in sem.nodes().iter() {
            let AstKind::CallExpression(call) = node.kind() else {
                continue;
            };
            let Some(fn_id) = effect_callback(sem, node.id()) else {
                continue;
            };
            let phase = if is_use_effect_setup_callback(sem, fn_id) {
                Phase::Setup
            } else {
                Phase::Cleanup
            };
            let setup_fn = match phase {
                Phase::Setup => Some(fn_id),
                Phase::Cleanup => find_parent_simple(sem, fn_id, |n| is_function(kind(sem, n))),
            };
            let effect = setup_fn.and_then(|f| parent(sem, unwrap(sem, f)));
            let callee = unwrap(sem, call.callee.node_id());
            let method = callee_name(sem, call);
            match method {
                Some(m @ ("addEventListener" | "removeEventListener")) => {
                    if m == "addEventListener"
                        && let AstKind::StaticMemberExpression(mem) = kind(sem, callee)
                        && let AstKind::IdentifierReference(obj) =
                            kind(sem, skip_parens(sem, mem.object.node_id()))
                        && is_initialized_from_react_native(sem, node.id(), obj.name.as_str())
                    {
                        continue;
                    }
                    let (Some(ty), Some(listener)) =
                        (call.arguments.first(), call.arguments.get(1))
                    else {
                        continue;
                    };
                    let opts = call
                        .arguments
                        .get(2)
                        .map_or(DEFAULT_OPTIONS, |o| listener_options(sem, o.node_id(), 0));
                    let listener = skip_parens(sem, listener.node_id());
                    if is_function(kind(sem, listener)) && opts.signal.is_none() {
                        reports.push((
                            span_of(sem, listener),
                            format!("A/an '{m}' should not have an inline listener function."),
                        ));
                    }
                    let entry = ListenerEntry {
                        ty: skip_parens(sem, ty.node_id()),
                        callee,
                        capture: opts.capture,
                        signal: opts.signal,
                        effect,
                        listener,
                        node: node.id(),
                        span: call.span,
                        phase,
                    };
                    if m == "addEventListener" {
                        adds.push(entry)
                    } else {
                        removes.push(entry)
                    }
                }
                _ => {
                    if phase != Phase::Cleanup {
                        continue;
                    }
                    let Some(effect) = effect else { continue };
                    match resolve_cleanup_callee(sem, call) {
                        Some(CleanupCallee::Function(f)) => {
                            cleanup_functions.entry(effect).or_default().insert(f);
                            cleanup_names.entry(effect).or_default();
                        }
                        Some(CleanupCallee::Name(n)) => {
                            cleanup_functions.entry(effect).or_default();
                            cleanup_names.entry(effect).or_default().insert(n);
                        }
                        None => {}
                    }
                }
            }
        }

        let is_global_object = |id: NodeId| matches!(kind(sem, skip_parens(sem, id)), AstKind::IdentifierReference(r) if matches!(r.name.as_str(), "window" | "globalThis" | "self"));
        let is_same_object = |a: NodeId, b: NodeId| {
            let (ka, kb) = (kind(sem, a), kind(sem, b));
            match (member_object(ka), member_object(kb)) {
                (Some(oa), Some(ob)) => compare_is_equal(sem, oa, ob),
                (Some(oa), None) => is_global_object(oa),
                (None, Some(ob)) => is_global_object(ob),
                (None, None) => true,
            }
        };
        let is_inverse = |a: &ListenerEntry, r: &ListenerEntry| {
            if a.phase != Phase::Setup {
                return false;
            }
            if !is_same_object(a.callee, r.callee)
                || !compare_is_equal(sem, a.listener, r.listener)
                || !is_value_equal(sem, a.ty, r.ty)
                || a.capture != r.capture
            {
                return false;
            }
            if r.phase == Phase::Cleanup {
                return true;
            }
            let Some(effect) = a.effect else { return false };
            if r.effect != Some(effect) {
                return false;
            }
            let Some(enclosing) = find_parent_simple(sem, r.node, |n| is_function(kind(sem, n)))
            else {
                return false;
            };
            let Some(functions) = cleanup_functions.get(&effect) else {
                return false;
            };
            if functions.contains(&enclosing) {
                return true;
            }
            function_id_name(sem, enclosing)
                .is_some_and(|n| cleanup_names.get(&effect).is_some_and(|s| s.contains(n)))
        };
        for a in &adds {
            if a.signal.is_some() {
                continue;
            }
            if removes.iter().any(|r| is_inverse(a, r)) {
                continue;
            }
            let effect_method = a
                .effect
                .filter(|e| matches!(kind(sem, *e), AstKind::CallExpression(_)))
                .and_then(|e| callee_name_of_id(sem, e))
                .unwrap_or("useEffect");
            reports.push((
                a.span,
                format!(
                    "An 'addEventListener' in '{effect_method}' should have a corresponding 'removeEventListener' in its cleanup function."
                ),
            ));
        }
        for (s, m) in reports {
            ctx.report(s, m);
        }
    }
}

// ---------------------------------------------------------------------------
// no-leaked-fetch
// ---------------------------------------------------------------------------

fn resolve_to_object_expression(sem: &Semantic<'_>, id: NodeId) -> Option<NodeId> {
    let id = unwrap(sem, id);
    match kind(sem, id) {
        AstKind::ObjectExpression(_) => Some(id),
        AstKind::IdentifierReference(_) => {
            let r = unwrap(sem, resolve(sem, id)?);
            matches!(kind(sem, r), AstKind::ObjectExpression(_)).then_some(r)
        }
        _ => None,
    }
}

/// `(controller, isParamSignal)`.
fn controller_from_signal(
    sem: &Semantic<'_>,
    id: NodeId,
    seen: &mut Vec<NodeId>,
) -> (Option<NodeId>, bool) {
    let id = unwrap(sem, id);
    let k = kind(sem, id);
    if let Some(obj) = member_object(k) {
        return (Some(obj), false);
    }
    if !matches!(k, AstKind::IdentifierReference(_)) || seen.contains(&id) {
        return (None, false);
    }
    seen.push(id);
    let resolved = resolve_origin(sem, id);
    let unwrapped = resolved.map(|r| unwrap(sem, r));
    if let Some(u) = unwrapped
        && let Some(obj) = member_object(kind(sem, u))
    {
        return (Some(obj), false);
    }
    if resolved.is_some_and(|r| is_function(kind(sem, r))) {
        return (Some(id), true);
    }
    match unwrapped {
        Some(u) => controller_from_signal(sem, u, seen),
        None => (None, false),
    }
}

pub struct NoLeakedFetch;
impl Rule for NoLeakedFetch {
    fn name(&self) -> &'static str {
        "@eslint-react/web-api-no-leaked-fetch"
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        if !ctx.source.contains("fetch") || !has_use_effect_text(ctx.source) {
            return;
        }
        let sem = ctx.semantic;
        let mut fetches: Vec<(Span, Option<NodeId>, bool)> = Vec::new();
        let mut aborts: Vec<NodeId> = Vec::new();
        for node in sem.nodes().iter() {
            let AstKind::CallExpression(call) = node.kind() else {
                continue;
            };
            match callee_name(sem, call) {
                Some("fetch") => {
                    let Some(f) = find_parent_simple(sem, node.id(), |n| is_function(kind(sem, n)))
                    else {
                        continue;
                    };
                    if !is_use_effect_setup_callback(sem, f) {
                        continue;
                    }
                    let (controller, is_param) = call
                        .arguments
                        .get(1)
                        .and_then(|o| resolve_to_object_expression(sem, o.node_id()))
                        .and_then(|o| match kind(sem, o) {
                            AstKind::ObjectExpression(o) => {
                                find_property(sem, &o.properties, "signal")
                            }
                            _ => None,
                        })
                        .map_or((None, false), |p| {
                            controller_from_signal(sem, p.value.node_id(), &mut Vec::new())
                        });
                    fetches.push((call.span, controller, is_param));
                }
                Some("abort") => {
                    let Some(f) = effect_callback(sem, node.id()) else {
                        continue;
                    };
                    if !is_use_effect_cleanup_callback(sem, f) {
                        continue;
                    }
                    let callee = unwrap(sem, call.callee.node_id());
                    if let Some(obj) = member_object(kind(sem, callee)) {
                        aborts.push(obj);
                    }
                }
                _ => {}
            }
        }
        let mut reports = Vec::new();
        for (span, controller, is_param) in fetches {
            let Some(controller) = controller else {
                reports.push((
                    span,
                    "A 'fetch' must be provided with an 'AbortController' for proper cleanup.",
                ));
                continue;
            };
            if is_param {
                continue;
            }
            if !aborts
                .iter()
                .any(|&a| is_assignment_target_equal(sem, a, controller))
            {
                reports.push((
                    span,
                    "A 'fetch' started in effect must be aborted with 'AbortController.abort' in the cleanup function.",
                ));
            }
        }
        for (s, m) in reports {
            ctx.report(s, m);
        }
    }
}

// ---------------------------------------------------------------------------
// no-leaked-intersection-observer / no-leaked-resize-observer
// ---------------------------------------------------------------------------

fn is_new_observer(sem: &Semantic<'_>, id: NodeId, name: &str) -> bool {
    match kind(sem, skip_parens(sem, id)) {
        AstKind::NewExpression(n) => {
            matches!(kind(sem, unwrap(sem, n.callee.node_id())), AstKind::IdentifierReference(r) if r.name == name)
        }
        _ => false,
    }
}

fn is_from_observer(sem: &Semantic<'_>, id: NodeId, name: &str, depth: u32) -> bool {
    if depth > 32 {
        return false;
    }
    let id = skip_parens(sem, id);
    let k = kind(sem, id);
    if let AstKind::IdentifierReference(_) = k {
        let Some(u) = resolve(sem, id).map(|n| unwrap(sem, n)) else {
            return false;
        };
        if is_new_observer(sem, u, name) {
            return true;
        }
        if let AstKind::CallExpression(c) = kind(sem, u)
            && is_use_ref_like_call(sem, u)
        {
            return c
                .arguments
                .first()
                .is_some_and(|a| is_new_observer(sem, unwrap(sem, a.node_id()), name));
        }
        if is_member(kind(sem, u)) {
            return is_from_observer(sem, u, name, depth + 1);
        }
        return false;
    }
    match member_object(k) {
        Some(obj) => is_from_observer(sem, obj, name, depth + 1),
        None => false,
    }
}

fn is_from_ref_current(sem: &Semantic<'_>, id: NodeId, ref_id: NodeId) -> bool {
    let mut expr = unwrap_type(sem, id);
    if matches!(kind(sem, expr), AstKind::IdentifierReference(_))
        && let Some(init) = resolve(sem, expr)
    {
        expr = unwrap(sem, init);
    }
    match kind(sem, expr) {
        AstKind::StaticMemberExpression(m) => {
            m.property.name == "current"
                && is_assignment_target_equal(sem, m.object.node_id(), ref_id)
        }
        _ => false,
    }
}

fn is_use_ref_initial_value(sem: &Semantic<'_>, id: NodeId) -> bool {
    let mut child = id;
    let mut p = parent(sem, id);
    while let Some(pp) = p
        && is_type_expression(kind(sem, pp))
    {
        child = pp;
        p = parent(sem, pp);
    }
    let Some(p) = p else { return false };
    match kind(sem, p) {
        AstKind::CallExpression(c) => {
            c.arguments
                .first()
                .is_some_and(|a| skip_parens(sem, a.node_id()) == child)
                && is_use_ref_like_call(sem, p)
        }
        _ => false,
    }
}

struct ObserverRule {
    name: &'static str,
    class: &'static str,
}

struct ObserverEntry {
    observer: NodeId,
    element: NodeId,
    node: NodeId,
    span: Span,
}

impl Rule for ObserverRule {
    fn name(&self) -> &'static str {
        self.name
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        if !ctx.source.contains(self.class) {
            return;
        }
        let sem = ctx.semantic;
        let class = self.class;
        let mut reports: Vec<(Span, String)> = Vec::new();
        // (id, node, phaseNode)
        let mut observers: Vec<(NodeId, NodeId, Option<NodeId>)> = Vec::new();
        let mut o_entries: Vec<ObserverEntry> = Vec::new();
        let mut u_entries: Vec<ObserverEntry> = Vec::new();
        let mut d_entries: Vec<(NodeId, NodeId)> = Vec::new();
        let mut cleanup_fns: Vec<NodeId> = Vec::new();

        for node in sem.nodes().iter() {
            match node.kind() {
                AstKind::CallExpression(call) => {
                    let Some(effect_cb) = effect_callback(sem, node.id()) else {
                        continue;
                    };
                    let callee = unwrap(sem, call.callee.node_id());
                    let ck = kind(sem, callee);
                    if matches!(ck, AstKind::IdentifierReference(_)) {
                        if is_use_effect_cleanup_callback(sem, effect_cb)
                            && let Some(f) = resolve_local_function(sem, callee)
                        {
                            cleanup_fns.push(f);
                        }
                        continue;
                    }
                    let Some(object) = member_object(ck) else {
                        continue;
                    };
                    let object = skip_parens(sem, object);
                    let Some(method) = callee_name(sem, call) else {
                        continue;
                    };
                    if !matches!(method, "observe" | "unobserve" | "disconnect")
                        || !is_from_observer(sem, callee, class, 0)
                    {
                        continue;
                    }
                    if method == "disconnect" {
                        d_entries.push((node.id(), object));
                        continue;
                    }
                    let Some(element) = call.arguments.first() else {
                        continue;
                    };
                    let entry = ObserverEntry {
                        observer: object,
                        element: skip_parens(sem, element.node_id()),
                        node: node.id(),
                        span: call.span,
                    };
                    if method == "observe" {
                        o_entries.push(entry)
                    } else {
                        u_entries.push(entry)
                    }
                }
                AstKind::NewExpression(n) => {
                    if !is_new_observer(sem, node.id(), class) {
                        continue;
                    }
                    let f = find_parent_simple(sem, node.id(), |p| {
                        is_function(kind(sem, p)) && is_use_effect_callback(sem, p)
                    });
                    if f.is_none() && !is_use_ref_initial_value(sem, node.id()) {
                        continue;
                    }
                    match resolve_enclosing_assignment_target(sem, node.id()) {
                        None => reports.push((
                            n.span,
                            format!(
                                "A{} '{class}' instance created in component or custom hook must be assigned to a variable for proper cleanup.",
                                if class.starts_with('I') { "n" } else { "" }
                            ),
                        )),
                        Some(id) => observers.push((id, node.id(), f)),
                    }
                }
                AstKind::ReturnStatement(r) => {
                    let Some(arg) = &r.argument else { continue };
                    let arg = unwrap(sem, arg.node_id());
                    if !matches!(kind(sem, arg), AstKind::IdentifierReference(_)) {
                        continue;
                    }
                    let Some(setup) =
                        find_parent_simple(sem, node.id(), |p| is_function(kind(sem, p)))
                    else {
                        continue;
                    };
                    if !is_use_effect_setup_callback(sem, setup) {
                        continue;
                    }
                    if let Some(f) = resolve_local_function(sem, arg) {
                        cleanup_fns.push(f);
                    }
                }
                _ => {}
            }
        }

        let is_in_cleanup_phase = |n: NodeId| {
            find_parent_simple(sem, n, |p| is_use_effect_cleanup_callback(sem, p)).is_some()
                || cleanup_fns.iter().any(|&f| has_ancestor(sem, n, f))
        };
        for &(id, obs_node, phase_node) in &observers {
            let is_same_observer = |o: NodeId| match phase_node {
                None => is_from_ref_current(sem, o, id),
                Some(_) => is_assignment_target_equal(sem, o, id),
            };
            let inside_observer_callback = |n: NodeId| has_ancestor(sem, n, obs_node);
            if d_entries.iter().any(|&(n, o)| {
                !inside_observer_callback(n) && is_in_cleanup_phase(n) && is_same_observer(o)
            }) {
                continue;
            }
            let matched_o: Vec<&ObserverEntry> = o_entries
                .iter()
                .filter(|e| is_same_observer(e.observer))
                .collect();
            let matched_u: Vec<&ObserverEntry> = u_entries
                .iter()
                .filter(|e| is_same_observer(e.observer))
                .collect();
            let dynamic = matched_o.iter().any(|e| {
                let entry_phase = phase_node.or_else(|| effect_callback(sem, e.node));
                let found = find_parent_simple(sem, e.node, |p| {
                    let k = kind(sem, p);
                    matches!(k, AstKind::CallExpression(_))
                        || is_conditional(k)
                        || Some(p) == entry_phase
                });
                found != entry_phase
            });
            if dynamic {
                reports.push((
                    span_of(sem, obs_node),
                    format!(
                        "Dynamically added '{class}.observe' should be cleared all at once using '{class}.disconnect' in the cleanup function."
                    ),
                ));
                continue;
            }
            for o in &matched_o {
                if matched_u.iter().any(|u| {
                    !inside_observer_callback(u.node)
                        && is_in_cleanup_phase(u.node)
                        && is_assignment_target_equal(sem, u.element, o.element)
                }) {
                    continue;
                }
                reports.push((
                    o.span,
                    format!(
                        "A{} '{class}' instance created in 'useEffect' must be disconnected in the cleanup function.",
                        if class.starts_with('I') { "n" } else { "" }
                    ),
                ));
            }
        }
        for (s, m) in reports {
            ctx.report(s, m);
        }
    }
}

pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(NoLeakedEventListener),
        Box::new(NoLeakedFetch),
        Box::new(ObserverRule {
            name: "@eslint-react/web-api-no-leaked-intersection-observer",
            class: "IntersectionObserver",
        }),
        Box::new(TimerRule {
            name: "@eslint-react/web-api-no-leaked-interval",
            set: "setInterval",
            clear: "clearInterval",
        }),
        Box::new(ObserverRule {
            name: "@eslint-react/web-api-no-leaked-resize-observer",
            class: "ResizeObserver",
        }),
        Box::new(TimerRule {
            name: "@eslint-react/web-api-no-leaked-timeout",
            set: "setTimeout",
            clear: "clearTimeout",
        }),
    ]
}
