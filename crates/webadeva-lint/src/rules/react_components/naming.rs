//! eslint-plugin-react-naming-convention: `context-name`, `id-name`,
//! `ref-name` (registered as `@eslint-react/naming-convention-*`).

use oxc_ast::AstKind;
use oxc_ast::ast::*;
use oxc_semantic::{AstNodes, NodeId};
use oxc_span::{GetSpan, Span};
use serde_json::Value;

use super::helpers::*;
use super::no_options;
use crate::rule::{Ctx, Rule};

/// `getStaticName(key, computed)`.
fn static_name<'b>(key: &'b PropertyKey<'_>, computed: bool) -> Option<&'b str> {
    match key {
        PropertyKey::StaticIdentifier(i) if !computed => Some(i.name.as_str()),
        PropertyKey::StringLiteral(s) => s.value.as_str(),
        _ => None,
    }
}

type Target<'a> = (Span, &'a str);

/// `resolveNamingTarget(node)`.
fn resolve_naming_target<'a>(nodes: &AstNodes<'a>, id: NodeId) -> Option<Target<'a>> {
    let mut node = id;
    let mut property_target: Option<Target<'a>> = None;
    loop {
        let p = parent(nodes, node)?;
        match nodes.kind(p) {
            AstKind::ObjectProperty(prop) => {
                if !is_node(&prop.value, node) {
                    return None;
                }
                let name = static_name(&prop.key, prop.computed)?;
                property_target.get_or_insert((prop.key.span(), name));
            }
            AstKind::VariableDeclarator(d) => {
                let BindingPattern::BindingIdentifier(b) = &d.id else {
                    return None;
                };
                return Some(property_target.unwrap_or((b.span, b.name.as_str())));
            }
            AstKind::AssignmentExpression(a) => {
                let left = match &a.left {
                    AssignmentTarget::AssignmentTargetIdentifier(i) => (i.span, i.name.as_str()),
                    t => match t.as_member_expression()? {
                        MemberExpression::StaticMemberExpression(m) => {
                            (m.span, m.property.name.as_str())
                        }
                        MemberExpression::ComputedMemberExpression(m) => {
                            match strip_parens(&m.expression) {
                                Expression::StringLiteral(s) => (m.span, s.value.as_str()?),
                                _ => return None,
                            }
                        }
                        MemberExpression::PrivateFieldExpression(_) => return None,
                    },
                };
                return Some(property_target.unwrap_or(left));
            }
            AstKind::PropertyDefinition(pd) => {
                let name = static_name(&pd.key, pd.computed)?;
                return Some(property_target.unwrap_or((pd.key.span(), name)));
            }
            AstKind::ConditionalExpression(_)
            | AstKind::LogicalExpression(_)
            | AstKind::AwaitExpression(_)
            | AstKind::ChainExpression(_)
            | AstKind::TSAsExpression(_)
            | AstKind::TSTypeAssertion(_)
            | AstKind::TSNonNullExpression(_)
            | AstKind::TSSatisfiesExpression(_)
            | AstKind::TSInstantiationExpression(_)
            | AstKind::ObjectExpression(_) => {}
            AstKind::SequenceExpression(s) => {
                if !s.expressions.last().is_some_and(|e| is_node(e, node)) {
                    return None;
                }
            }
            AstKind::CallExpression(c) => {
                if !c
                    .arguments
                    .iter()
                    .any(|a| a.as_expression().is_some_and(|e| is_node(e, node)))
                {
                    return None;
                }
            }
            AstKind::NewExpression(c) => {
                if !c
                    .arguments
                    .iter()
                    .any(|a| a.as_expression().is_some_and(|e| is_node(e, node)))
                {
                    return None;
                }
            }
            _ => return None,
        }
        node = p;
    }
}

fn call_node_id(k: AstKind<'_>) -> Option<&CallExpression<'_>> {
    as_call(k)
}

/// `naming-convention-context-name`.
pub struct ContextName;

impl Rule for ContextName {
    fn name(&self) -> &'static str {
        "@eslint-react/naming-convention-context-name"
    }
    fn validate(&self, options: &[Value], _: &Value) -> Result<(), String> {
        no_options(options)
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        if !ctx.source.contains("createContext") {
            return;
        }
        let nodes = ctx.semantic.nodes();
        let mut spans = Vec::new();
        for node in nodes.iter() {
            let Some(call) = call_node_id(node.kind()) else {
                continue;
            };
            if !is_api_call(ctx.source, call, "createContext") {
                continue;
            }
            let Some((span, name)) = resolve_naming_target(nodes, node.id()) else {
                continue;
            };
            if is_component_name(name) && name.ends_with("Context") {
                continue;
            }
            spans.push(span);
        }
        for s in spans {
            ctx.report(
                s,
                "A context name must be a valid component name with the suffix 'Context'.",
            );
        }
    }
}

/// `naming-convention-id-name`.
pub struct IdName;

impl Rule for IdName {
    fn name(&self) -> &'static str {
        "@eslint-react/naming-convention-id-name"
    }
    fn validate(&self, options: &[Value], _: &Value) -> Result<(), String> {
        no_options(options)
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        if !ctx.source.contains("useId") {
            return;
        }
        let nodes = ctx.semantic.nodes();
        let mut spans = Vec::new();
        for node in nodes.iter() {
            let Some(call) = call_node_id(node.kind()) else {
                continue;
            };
            if !is_api_call(ctx.source, call, "useId") {
                continue;
            }
            let Some((span, name)) = resolve_naming_target(nodes, node.id()) else {
                continue;
            };
            if name.ends_with("Id") || name == "id" {
                continue;
            }
            spans.push(span);
        }
        for s in spans {
            ctx.report(
                s,
                "An identifier assigned from 'useId' must be named 'id' or end with 'Id'.",
            );
        }
    }
}

/// `naming-convention-ref-name`.
pub struct RefName;

impl Rule for RefName {
    fn name(&self) -> &'static str {
        "@eslint-react/naming-convention-ref-name"
    }
    fn validate(&self, options: &[Value], _: &Value) -> Result<(), String> {
        no_options(options)
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        let extra = additional_hooks_matcher(ctx, "additionalRefHooks");
        if !ctx.source.contains("useRef")
            && extra.as_deref().is_none_or(|e| !ctx.source.contains(e))
        {
            return;
        }
        let nodes = ctx.semantic.nodes();
        let mut spans = Vec::new();
        for node in nodes.iter() {
            let Some(call) = call_node_id(node.kind()) else {
                continue;
            };
            let Some(cname) = callee_name(call) else {
                continue;
            };
            if cname != "useRef" && extra.as_deref() != Some(cname) {
                continue;
            }
            let (_, p) = skip_type_parents(nodes, node.id());
            if p.is_some_and(|p| {
                matches!(
                    nodes.kind(p),
                    AstKind::StaticMemberExpression(_)
                        | AstKind::ComputedMemberExpression(_)
                        | AstKind::PrivateFieldExpression(_)
                )
            }) {
                continue;
            }
            let Some((span, name)) = resolve_naming_target(nodes, node.id()) else {
                continue;
            };
            if name.ends_with("Ref") || name == "ref" {
                continue;
            }
            spans.push(span);
        }
        for s in spans {
            ctx.report(
                s,
                "A ref identifier must be named 'ref' or ending in 'Ref'.",
            );
        }
    }
}
