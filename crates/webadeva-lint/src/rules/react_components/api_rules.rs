//! React API rules from eslint-plugin-react-x that depend on the React
//! version (`no-context-provider`, `no-forward-ref`, `no-use-context`) plus
//! `no-create-ref`.

use std::collections::HashSet;

use oxc_ast::AstKind;
use serde_json::Value;

use super::helpers::*;
use super::no_options;
use crate::rule::{Ctx, Rule};
use crate::utils::jsx_element_name;

/// `no-context-provider` (React >= 19).
pub struct NoContextProvider;

impl Rule for NoContextProvider {
    fn name(&self) -> &'static str {
        "@eslint-react/no-context-provider"
    }
    fn validate(&self, options: &[Value], _: &Value) -> Result<(), String> {
        no_options(options)
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        if !ctx.source.contains("Provider") || version_lt(&react_version(ctx), "19.0.0") {
            return;
        }
        let mut spans = Vec::new();
        for node in ctx.semantic.nodes().iter() {
            let AstKind::JSXElement(el) = node.kind() else {
                continue;
            };
            let full = jsx_element_name(&el.opening_element.name);
            let mut parts: Vec<&str> = full.split('.').collect();
            let self_name = parts.pop();
            let context_self = parts.pop();
            if self_name != Some("Provider") {
                continue;
            }
            if context_self.is_some_and(|c| c.ends_with("Context")) {
                spans.push(el.span);
            }
        }
        for s in spans {
            ctx.report(s, "In React 19, you can render '<Context>' as a provider instead of '<Context.Provider>'.");
        }
    }
}

/// `no-forward-ref` (React >= 19).
pub struct NoForwardRef;

impl Rule for NoForwardRef {
    fn name(&self) -> &'static str {
        "@eslint-react/no-forward-ref"
    }
    fn validate(&self, options: &[Value], _: &Value) -> Result<(), String> {
        no_options(options)
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        if !ctx.source.contains("forwardRef") || version_lt(&react_version(ctx), "19.0.0") {
            return;
        }
        let nodes = ctx.semantic.nodes();
        let mut spans = Vec::new();
        for node in nodes.iter() {
            let AstKind::CallExpression(call) = node.kind() else {
                continue;
            };
            if !is_api_call(ctx.source, call, "forwardRef") {
                continue;
            }
            let span = get_function_id(nodes, node.id()).map_or(call.span, |id| id.span());
            spans.push(span);
        }
        for s in spans {
            ctx.report(
                s,
                "In React 19, 'forwardRef' is no longer necessary. Pass 'ref' as a prop instead.",
            );
        }
    }
}

/// `no-use-context` (React >= 19).
pub struct NoUseContext;

impl Rule for NoUseContext {
    fn name(&self) -> &'static str {
        "@eslint-react/no-use-context"
    }
    fn validate(&self, options: &[Value], _: &Value) -> Result<(), String> {
        no_options(options)
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        if !ctx.source.contains("useContext") || version_lt(&react_version(ctx), "19.0.0") {
            return;
        }
        let mut spans = Vec::new();
        for node in ctx.semantic.nodes().iter() {
            let AstKind::CallExpression(call) = node.kind() else {
                continue;
            };
            if is_api_call(ctx.source, call, "useContext") {
                spans.push(oxc_span::GetSpan::span(strip_parens(&call.callee)));
            }
        }
        for s in spans {
            ctx.report(
                s,
                "In React 19, 'use' is preferred over 'useContext' because it is more flexible.",
            );
        }
    }
}

/// `no-create-ref`: `createRef` inside function components and hooks.
pub struct NoCreateRef;

impl Rule for NoCreateRef {
    fn name(&self) -> &'static str {
        "@eslint-react/no-create-ref"
    }
    fn validate(&self, options: &[Value], _: &Value) -> Result<(), String> {
        no_options(options)
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        if !ctx.source.contains("createRef") {
            return;
        }
        let nodes = ctx.semantic.nodes();
        let calls: Vec<_> = nodes
            .iter()
            .filter_map(|n| match n.kind() {
                AstKind::CallExpression(c) if is_api_call(ctx.source, c, "createRef") => {
                    Some((n.id(), c.span))
                }
                _ => None,
            })
            .collect();
        if calls.is_empty() {
            return;
        }
        let collected = collect(
            ctx,
            Want {
                components: Some(hint::DEFAULT_COMPONENT),
                hooks: true,
                classes: false,
            },
        );
        let funcs: HashSet<_> = collected
            .component_ids()
            .chain(collected.hook_ids())
            .collect();
        let mut spans = Vec::new();
        for (id, span) in calls {
            if nodes.ancestor_ids(id).any(|p| funcs.contains(&p)) {
                spans.push(span);
            }
        }
        for s in spans {
            ctx.report(
                s,
                "Use 'useRef' instead of 'createRef' in function components and Hooks.",
            );
        }
    }
}
