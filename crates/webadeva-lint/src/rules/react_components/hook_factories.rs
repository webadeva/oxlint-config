//! `@eslint-react/component-hook-factories` from @eslint-react 3.0.0
//! (eslint-plugin-react-x). @eslint-react 5 dropped the rule, so the
//! presets, which follow @webadeva/eslint-config, leave it off. Configs that
//! came from older @eslint-react versions can turn it on.
//!
//! Reports components (function and class) and hooks defined inside another
//! function: a function that makes components or hooks gives them a new
//! identity on every call, so React remounts them and loses their state.

use oxc_ast::AstKind;
use oxc_ast::ast::CallExpression;
use oxc_semantic::{AstNodes, NodeId};

use super::helpers::*;
use super::no_options;
use crate::rule::{Ctx, Rule};

pub struct ComponentHookFactories;

/// `isComponentWrapperCallLoose` from @eslint-react/core 3: `memo`,
/// `forwardRef`, or a `useCallback` hook call.
fn is_wrapper_call_loose(source: &str, call: &CallExpression<'_>) -> bool {
    is_fc_wrapper_call(source, call) || callee_name(call) == Some("useCallback")
}

/// `getFunctionComponentId` from @eslint-react/core 3, which unlike v5 also
/// names a function passed to `useCallback` after its variable.
fn component_name(nodes: &AstNodes<'_>, source: &str, func: NodeId) -> Option<String> {
    if let Some(id) = get_function_component_id(nodes, source, func) {
        return Some(id.fqn(source));
    }
    let declarator = |id: NodeId| match nodes.kind(id) {
        AstKind::VariableDeclarator(d) => {
            Some(IdRef::Pattern(&d.id, d.type_annotation.as_ref().map(|a| a.span)).fqn(source))
        }
        _ => None,
    };
    let p = skip_type_parents(nodes, func).1?;
    let call = as_call(nodes.kind(p))?;
    if !is_wrapper_call_loose(source, call) {
        return None;
    }
    let pp = parent(nodes, p)?;
    if let Some(name) = declarator(pp) {
        return Some(name);
    }
    let outer = as_call(nodes.kind(pp))?;
    if !is_wrapper_call_loose(source, outer) {
        return None;
    }
    declarator(parent(nodes, pp)?)
}

const COMPONENT: &str =
    "Components should be defined at the module level. Move it to the top level.";
const HOOK: &str = "Hooks should be defined at the module level. Move it to the top level.";

impl Rule for ComponentHookFactories {
    fn name(&self) -> &'static str {
        "@eslint-react/component-hook-factories"
    }

    fn validate(&self, options: &[serde_json::Value], _: &serde_json::Value) -> Result<(), String> {
        no_options(options)
    }

    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        // v3 has separate `map` and `flatMap` callback hints; this rule sets
        // only the `map` one.
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
        let collected = collect(
            ctx,
            Want {
                components: Some(h),
                hooks: true,
                classes: true,
            },
        );
        let nodes = ctx.semantic.nodes();
        let in_function =
            |id: NodeId| find_parent(nodes, id, |_, k| as_func(k).is_some(), no_stop).is_some();

        let mut reported: Vec<NodeId> = Vec::new();
        let mut reports = Vec::new();
        for c in &collected.components {
            let id = c.func.node_id();
            let Some(name) = component_name(nodes, ctx.source, id) else {
                continue;
            };
            if !in_function(id) || reported.contains(&id) {
                continue;
            }
            reports.push((
                c.func.span(),
                format!("Do not define component '{name}' inside a function. {COMPONENT}"),
            ));
            reported.push(id);
        }
        for c in &collected.classes {
            if !in_function(c.class.node_id()) {
                continue;
            }
            // Upstream defaults only an undefined name to "unknown"; an
            // anonymous class has a null one, printed as "null".
            let name = class_id_text(nodes, ctx.source, c.class).unwrap_or("null");
            reports.push((
                c.class.span,
                format!("Do not define component '{name}' inside a function. {COMPONENT}"),
            ));
        }
        for hk in &collected.hooks {
            let id = hk.func.node_id();
            if !in_function(id) || reported.contains(&id) {
                continue;
            }
            let name = hk.name.as_deref().unwrap_or_default();
            reports.push((
                hk.func.span(),
                format!("Do not define hook '{name}' inside a function. {HOOK}"),
            ));
            reported.push(id);
        }
        for (span, message) in reports {
            ctx.report(span, message);
        }
    }
}
