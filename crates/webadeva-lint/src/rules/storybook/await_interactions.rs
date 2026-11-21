//! `storybook/await-interactions`: interactions should be awaited.

use oxc_ast::AstKind;
use oxc_ast::ast::{
    BindingPattern, CallExpression, Expression, ImportDeclaration, ImportDeclarationSpecifier,
};
use oxc_semantic::{AstNodes, NodeId};

use super::{estree_parent, ident_name, is_stories_file, member_parts, module_export_ident};
use crate::fix::Edit;
use crate::rule::{Ctx, Rule};

pub struct AwaitInteractions;

const FUNCTIONS_TO_BE_AWAITED: &[&str] = &[
    "waitFor",
    "waitForElementToBeRemoved",
    "wait",
    "waitForElement",
    "waitForDomChange",
    "userEvent",
    "play",
];

fn should_await(name: &str) -> bool {
    FUNCTIONS_TO_BE_AWAITED.contains(&name) || name.starts_with("findBy")
}

fn imports_named(d: &ImportDeclaration<'_>, imported: &str, local: Option<&str>) -> bool {
    d.specifiers.as_ref().is_some_and(|specs| {
        specs.iter().any(|s| match s {
            ImportDeclarationSpecifier::ImportSpecifier(s) => {
                module_export_ident(&s.imported) == Some(imported)
                    && local.is_none_or(|l| s.local.name == l)
            }
            _ => false,
        })
    })
}

fn is_storybook_import(d: &ImportDeclaration<'_>) -> bool {
    let src = d.source.value;
    let user_event = (src == "@storybook/testing-library" || src == "@storybook/test")
        && imports_named(d, "userEvent", Some("userEvent"));
    let expect =
        (src == "@storybook/jest" || src == "@storybook/test") && imports_named(d, "expect", None);
    user_event || expect
}

/// Upstream `getMethodThatShouldBeAwaited`, returning the method name.
fn method_that_should_be_awaited<'a>(
    nodes: &AstNodes<'a>,
    id: NodeId,
    call: &'a CallExpression<'a>,
) -> Option<&'a str> {
    if let Some(p) = estree_parent(nodes, id)
        && matches!(
            nodes.kind(p),
            AstKind::ArrowFunctionExpression(_) | AstKind::ReturnStatement(_)
        )
    {
        return None;
    }
    let callee = call.callee.without_parentheses();
    if let Some((object, property)) = member_parts(callee) {
        if let Some(obj) = ident_name(object)
            && should_await(obj)
        {
            return Some(obj);
        }
        let property = property?;
        if should_await(property) {
            return Some(property);
        }
        if let Expression::CallExpression(inner) = object.without_parentheses()
            && ident_name(&inner.callee) == Some("expect")
        {
            return Some(property);
        }
        return None;
    }
    match callee {
        Expression::TSNonNullExpression(n) => {
            let (_, property) = member_parts(&n.expression)?;
            property.filter(|p| should_await(p))
        }
        Expression::Identifier(i) if should_await(&i.name) => Some(i.name.as_str()),
        _ => None,
    }
}

/// Upstream `getClosestFunctionAncestor`: start offset of the nearest
/// enclosing function, and whether it is already `async`. oxc's `Function`
/// span of a method starts at its parameters (or type parameters), like the
/// ESTree `FunctionExpression` the fixer inserts `async ` before.
fn closest_function(nodes: &AstNodes<'_>, id: NodeId) -> Option<(u32, bool)> {
    nodes.ancestor_kinds(id).find_map(|k| match k {
        AstKind::Function(f) => Some((f.span.start, f.r#async)),
        AstKind::ArrowFunctionExpression(f) => Some((f.span.start, f.r#async)),
        _ => None,
    })
}

impl Rule for AwaitInteractions {
    fn name(&self) -> &'static str {
        "storybook/await-interactions"
    }

    fn applies_to(&self, rel_path: &str) -> bool {
        is_stories_file(rel_path)
    }

    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        let nodes = ctx.semantic.nodes();
        let mut imported_from_storybook = true;
        let mut invocations = Vec::new();
        for node in nodes.iter() {
            match node.kind() {
                AstKind::ImportDeclaration(d) => imported_from_storybook = is_storybook_import(d),
                AstKind::VariableDeclarator(d) => {
                    imported_from_storybook = imported_from_storybook
                        && matches!(&d.id, BindingPattern::BindingIdentifier(id) if id.name != "userEvent");
                }
                AstKind::CallExpression(call) => {
                    let Some(method) = method_that_should_be_awaited(nodes, node.id(), call) else {
                        continue;
                    };
                    let parent = estree_parent(nodes, node.id());
                    let grandparent = parent.and_then(|p| estree_parent(nodes, p));
                    let is_await = |n: Option<NodeId>| {
                        n.is_some_and(|n| matches!(nodes.kind(n), AstKind::AwaitExpression(_)))
                    };
                    if !is_await(parent) && !is_await(grandparent) {
                        let function = closest_function(nodes, node.id());
                        invocations.push((call.span, method, function));
                    }
                }
                _ => {}
            }
        }
        if !imported_from_storybook {
            return;
        }
        for (span, method, function) in invocations {
            let mut edits = vec![Edit::insert_before(span.start, "await ")];
            if let Some((start, false)) = function {
                edits.push(Edit::insert_before(start, "async "));
            }
            ctx.report_with_fix(
                span,
                format!("Interaction should be awaited: {method}"),
                edits,
            );
        }
    }
}
