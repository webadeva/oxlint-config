//! `storybook/context-in-play-function`: pass a context when invoking the
//! play function of another story.

use oxc_ast::AstKind;
use oxc_ast::ast::{Argument, BindingPattern, CallExpression, Expression, ObjectPropertyKind};
use oxc_semantic::{AstNodes, NodeId};

use super::{ident_name, is_stories_file, key_ident_name, member_parts};
use crate::rule::{Ctx, Rule};

pub struct ContextInPlayFunction;

fn is_play_function_from_another_story(call: &CallExpression<'_>) -> bool {
    match call.callee.without_parentheses() {
        Expression::TSNonNullExpression(n) => {
            member_parts(&n.expression).is_some_and(|(_, p)| p == Some("play"))
        }
        callee => member_parts(callee).is_some_and(|(_, p)| p == Some("play")),
    }
}

/// Upstream `getParentParameterName`: the first parameter of the nearest
/// enclosing arrow function, as the name the context is bound to.
fn parent_parameter_name<'a>(nodes: &AstNodes<'a>, id: NodeId) -> Option<&'a str> {
    let arrow = nodes.ancestors(id).find_map(|n| match n.kind() {
        AstKind::ArrowFunctionExpression(a) => Some(a),
        _ => None,
    })?;
    let param = arrow.params.items.first()?;
    if param.initializer.is_some() {
        return None;
    }
    match &param.pattern {
        BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
        BindingPattern::ObjectPattern(obj) => {
            if obj
                .properties
                .iter()
                .any(|p| key_ident_name(&p.key) == Some("context"))
            {
                return Some("context");
            }
            match &obj.rest.as_ref()?.argument {
                BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
                _ => None,
            }
        }
        _ => None,
    }
}

fn is_not_passing_context_correctly<'a>(
    nodes: &AstNodes<'a>,
    id: NodeId,
    call: &'a CallExpression<'a>,
) -> bool {
    let Some(first) = call.arguments.first() else {
        return true;
    };
    let Some(context_name) = parent_parameter_name(nodes, id) else {
        return true;
    };
    let first = match first {
        Argument::SpreadElement(_) => return true,
        a => a.to_expression().without_parentheses(),
    };
    let passes_identifier = call.arguments.len() == 1 && ident_name(first) == Some(context_name);
    let spreads_context = matches!(first, Expression::ObjectExpression(o) if o.properties.iter().any(|p| matches!(
        p,
        ObjectPropertyKind::SpreadProperty(s) if ident_name(&s.argument) == Some(context_name)
    )));
    !(passes_identifier || spreads_context)
}

impl Rule for ContextInPlayFunction {
    fn name(&self) -> &'static str {
        "storybook/context-in-play-function"
    }

    fn applies_to(&self, rel_path: &str) -> bool {
        is_stories_file(rel_path)
    }

    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        let nodes = ctx.semantic.nodes();
        for node in nodes.iter() {
            if let AstKind::CallExpression(call) = node.kind()
                && is_play_function_from_another_story(call)
                && is_not_passing_context_correctly(nodes, node.id(), call)
            {
                ctx.report(
                    call.span,
                    "Pass a context when invoking play function of another story",
                );
            }
        }
    }
}
