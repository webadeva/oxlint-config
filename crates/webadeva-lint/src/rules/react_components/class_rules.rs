//! Class component rules from eslint-plugin-react-x.

use std::collections::{HashMap, HashSet};

use oxc_ast::AstKind;
use oxc_ast::ast::*;
use oxc_semantic::NodeId;
use oxc_span::{GetSpan, Span};
use serde_json::Value;

use super::helpers::*;
use super::no_options;
use crate::fix::Edit;
use crate::rule::{Ctx, Rule};

fn element_kind<'a>(e: &'a ClassElement<'a>) -> Option<AstKind<'a>> {
    match e {
        ClassElement::MethodDefinition(m) => Some(AstKind::MethodDefinition(m)),
        ClassElement::PropertyDefinition(p) => Some(AstKind::PropertyDefinition(p)),
        _ => None,
    }
}

/// `no-component-will-*` / `no-unsafe-component-will-*`: report a lifecycle
/// member of a class component.
pub struct LifecycleMember {
    pub id: &'static str,
    pub method: &'static str,
    pub message: &'static str,
}

impl Rule for LifecycleMember {
    fn name(&self) -> &'static str {
        self.id
    }
    fn validate(&self, options: &[Value], _: &Value) -> Result<(), String> {
        no_options(options)
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        if !ctx.source.contains(self.method) {
            return;
        }
        let mut spans = Vec::new();
        for node in ctx.semantic.nodes().iter() {
            let AstKind::Class(class) = node.kind() else {
                continue;
            };
            if !is_class_component(class) {
                continue;
            }
            for el in &class.body.body {
                if let Some(k) = element_kind(el)
                    && is_lifecycle(k, self.method, false)
                {
                    let m = as_member(k).expect("member");
                    spans.push((m.span, m.key.span()));
                }
            }
        }
        // `no-component-will-*` fix by renaming the key to `UNSAFE_*`; the
        // `no-unsafe-*` rules have no fixer.
        let fixed =
            (!self.method.starts_with("UNSAFE_")).then(|| format!("UNSAFE_{}", self.method));
        for (s, key) in spans {
            match &fixed {
                Some(name) => {
                    ctx.report_with_fix(s, self.message, vec![Edit::replace(key, name.as_str())])
                }
                None => ctx.report(s, self.message),
            }
        }
    }
}

/// `no-set-state-in-component-{did-mount,did-update,will-update}`.
pub struct SetStateIn {
    pub id: &'static str,
    pub method: &'static str,
}

impl Rule for SetStateIn {
    fn name(&self) -> &'static str {
        self.id
    }
    fn validate(&self, options: &[Value], _: &Value) -> Result<(), String> {
        no_options(options)
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        if !ctx.source.contains(self.method) {
            return;
        }
        let nodes = ctx.semantic.nodes();
        let mut spans = Vec::new();
        for node in nodes.iter() {
            let AstKind::CallExpression(call) = node.kind() else {
                continue;
            };
            if !is_this_set_state_call(call) {
                continue;
            }
            let id = node.id();
            let Some(component) =
                find_parent(nodes, id, |_, k| kind_is_class_component(k), no_stop)
            else {
                continue;
            };
            let Some(method) = find_parent(
                nodes,
                id,
                |_, k| is_lifecycle(k, self.method, false),
                |n, _| n == component,
            ) else {
                continue;
            };
            let m = as_member(nodes.kind(method)).expect("member");
            let lifecycle_fn = match (m.method_fn, m.value) {
                (Some(f), _) => Some(f.node_id()),
                (None, Some(v)) => Some(unwrap(v).node_id()),
                _ => None,
            };
            let owner_is_component =
                class_of_member(nodes, method).is_some_and(|c| c.node_id() == component);
            let enclosing = find_parent(nodes, id, |_, k| is_function_or_class(k), no_stop);
            if owner_is_component && lifecycle_fn.is_some() && lifecycle_fn == enclosing {
                spans.push(call.span);
            }
        }
        let msg = format!(
            "Do not call `this.setState` in `{}` outside functions such as callbacks.",
            self.method
        );
        for s in spans {
            ctx.report(s, msg.clone());
        }
    }
}

/// `no-access-state-in-setstate`.
pub struct NoAccessStateInSetstate;

/// `isAPI("this.state")` on a member expression node.
fn is_this_state(source: &str, k: AstKind<'_>) -> Option<Span> {
    let (object, span) = match k {
        AstKind::StaticMemberExpression(m) if m.property.name == "state" => (&m.object, m.span),
        AstKind::PrivateFieldExpression(m) if m.field.name == "state" => (&m.object, m.span),
        _ => return None,
    };
    let name = format!("{}.state", fqn(source, unwrap(object)));
    (name == "this.state" || name.ends_with(".this.state")).then_some(span)
}

impl Rule for NoAccessStateInSetstate {
    fn name(&self) -> &'static str {
        "@eslint-react/no-access-state-in-setstate"
    }
    fn validate(&self, options: &[Value], _: &Value) -> Result<(), String> {
        no_options(options)
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        if !ctx.source.contains("setState") {
            return;
        }
        let nodes = ctx.semantic.nodes();
        let mut spans = Vec::new();
        for node in nodes.iter() {
            let Some(span) = is_this_state(ctx.source, node.kind()) else {
                continue;
            };
            let Some(call) = find_parent(
                nodes,
                node.id(),
                |_, k| matches!(k, AstKind::CallExpression(c) if is_this_set_state_call(c)),
                no_stop,
            ) else {
                continue;
            };
            let class = find_parent(nodes, call, |_, k| matches!(k, AstKind::Class(_)), no_stop);
            if class.is_some_and(|c| kind_is_class_component(nodes.kind(c))) {
                spans.push(span);
            }
        }
        for s in spans {
            ctx.report(
                s,
                "Do not access 'this.state' within 'setState'. Use the update function instead.",
            );
        }
    }
}

/// `no-unused-class-component-members`.
pub struct NoUnusedClassComponentMembers;

const LIFECYCLE_METHODS: &[&str] = &[
    "componentDidCatch",
    "componentDidMount",
    "componentDidUpdate",
    "componentWillMount",
    "componentWillReceiveProps",
    "componentWillUnmount",
    "componentWillUpdate",
    "constructor",
    "getSnapshotBeforeUpdate",
    "render",
    "shouldComponentUpdate",
    "state",
    "UNSAFE_componentWillMount",
    "UNSAFE_componentWillReceiveProps",
    "UNSAFE_componentWillUpdate",
];

const KNOWN_HOST_CONVENTION_METHODS: &[&str] = &[
    "blur",
    "focus",
    "measure",
    "measureInWindow",
    "measureLayout",
    "setNativeProps",
];

struct Def<'a> {
    name: &'a str,
    span: Span,
    /// The member when the def is a method / property key (for
    /// `isPublicMethod`); `None` for `this.x = ...`.
    member: Option<Member<'a>>,
}

fn is_exported_class(nodes: &oxc_semantic::AstNodes<'_>, class: NodeId) -> bool {
    let Some(p) = parent(nodes, class) else {
        return false;
    };
    match nodes.kind(p) {
        AstKind::ExportDefaultDeclaration(_) | AstKind::ExportDeclaration(_) => true,
        AstKind::VariableDeclarator(_) => parent(nodes, p)
            .and_then(|pp| parent(nodes, pp))
            .is_some_and(|ppp| matches!(nodes.kind(ppp), AstKind::ExportDeclaration(_))),
        _ => false,
    }
}

fn is_public_method(m: &Member<'_>) -> bool {
    let public = matches!(m.accessibility, None | Some(TSAccessibility::Public));
    if m.is_method {
        public
    } else {
        public
            && matches!(
                m.value.map(strip_parens),
                Some(Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_))
            )
    }
}

impl Rule for NoUnusedClassComponentMembers {
    fn name(&self) -> &'static str {
        "@eslint-react/no-unused-class-component-members"
    }
    fn validate(&self, options: &[Value], _: &Value) -> Result<(), String> {
        no_options(options)
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        let nodes = ctx.semantic.nodes();
        // Only class components have defs to report.
        if !nodes.iter().any(|n| kind_is_class_component(n.kind())) {
            return;
        }
        let mut defs: HashMap<NodeId, Vec<Def<'a>>> = HashMap::new();
        let mut usages: HashMap<NodeId, HashSet<&'a str>> = HashMap::new();
        let mut class_order: Vec<NodeId> = Vec::new();
        let nearest_class =
            |id: NodeId| find_parent(nodes, id, |_, k| matches!(k, AstKind::Class(_)), no_stop);
        let nearest_member =
            |id: NodeId| find_parent(nodes, id, |_, k| as_member(k).is_some(), no_stop);

        for node in nodes.iter() {
            let k = node.kind();
            let id = node.id();
            match k {
                AstKind::Class(c) if is_class_component(c) => class_order.push(id),
                AstKind::MethodDefinition(_) | AstKind::PropertyDefinition(_) => {
                    let Some(m) = as_member(k) else { continue };
                    let Some(class) = nearest_class(id) else {
                        continue;
                    };
                    if !kind_is_class_component(nodes.kind(class)) || m.is_static || m.computed {
                        continue;
                    }
                    if let PropertyKey::StaticIdentifier(key) = m.key {
                        defs.entry(class).or_default().push(Def {
                            name: key.name.as_str(),
                            span: key.span,
                            member: Some(m),
                        });
                    }
                }
                AstKind::StaticMemberExpression(me) => {
                    let (Some(class), Some(method)) = (nearest_class(id), nearest_member(id))
                    else {
                        continue;
                    };
                    if !kind_is_class_component(nodes.kind(class))
                        || as_member(nodes.kind(method)).is_some_and(|m| m.is_static)
                    {
                        continue;
                    }
                    if !matches!(strip_parens(&me.object), Expression::ThisExpression(_)) {
                        continue;
                    }
                    let assigned = parent(nodes, id).is_some_and(|p| {
                        matches!(nodes.kind(p), AstKind::AssignmentExpression(a)
                            if a.left.as_member_expression().is_some_and(|l| l.node_id() == id))
                    });
                    if assigned {
                        defs.entry(class).or_default().push(Def {
                            name: me.property.name.as_str(),
                            span: me.property.span,
                            member: None,
                        });
                    } else {
                        usages
                            .entry(class)
                            .or_default()
                            .insert(me.property.name.as_str());
                    }
                }
                AstKind::VariableDeclarator(d) => {
                    let (Some(class), Some(method)) = (nearest_class(id), nearest_member(id))
                    else {
                        continue;
                    };
                    if !kind_is_class_component(nodes.kind(class))
                        || as_member(nodes.kind(method)).is_some_and(|m| m.is_static)
                    {
                        continue;
                    }
                    let is_this = d
                        .init
                        .as_ref()
                        .is_some_and(|i| matches!(strip_parens(i), Expression::ThisExpression(_)));
                    if let (true, BindingPattern::ObjectPattern(pat)) = (is_this, &d.id) {
                        for prop in &pat.properties {
                            if !prop.computed
                                && let PropertyKey::StaticIdentifier(key) = &prop.key
                            {
                                usages.entry(class).or_default().insert(key.name.as_str());
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        let mut reports = Vec::new();
        for class_id in class_order {
            let AstKind::Class(class) = nodes.kind(class_id) else {
                continue;
            };
            let Some(class_defs) = defs.get(&class_id) else {
                continue;
            };
            let used = usages.get(&class_id);
            let exported = is_exported_class(nodes, class_id);
            let class_name = class_id_text(nodes, ctx.source, class).unwrap_or("Component");
            for def in class_defs {
                if used.is_some_and(|u| u.contains(def.name)) {
                    continue;
                }
                if LIFECYCLE_METHODS.contains(&def.name)
                    && !(def.name == "shouldComponentUpdate" && is_pure_component(class))
                {
                    continue;
                }
                if KNOWN_HOST_CONVENTION_METHODS.contains(&def.name) {
                    continue;
                }
                if exported && def.member.as_ref().is_some_and(is_public_method) {
                    continue;
                }
                reports.push((
                    def.span,
                    format!(
                        "Unused method or property '{}' of class '{}'.",
                        def.name, class_name
                    ),
                ));
            }
        }
        for (s, m) in reports {
            ctx.report(s, m);
        }
    }
}
