//! eslint-plugin-storybook@10.6 `flat/recommended` rules.
//!
//! Upstream reads an ESTree AST; oxc differs in a few places that matter here
//! (parentheses are kept as nodes, `export` declarations are split into
//! `ExportDeclaration` / `ExportNamedDeclaration` / `ExportFromDeclaration`,
//! directives live outside `Program.body`), so the helpers below translate.

use std::borrow::Cow;

use oxc_ast::AstKind;
use oxc_ast::ast::{
    BindingIdentifier, BindingPattern, Declaration, ExportDefaultDeclaration, Expression,
    ImportDeclarationSpecifier, ModuleExportName, ObjectExpression, ObjectProperty,
    ObjectPropertyKind, Program, PropertyKey, Statement, StringLiteral,
};
use oxc_semantic::{AstNodes, NodeId, Semantic};
use oxc_span::{GetSpan, Span};

use crate::rule::Rule;

mod await_interactions;
mod context_in_play_function;
mod csf;
mod default_exports;
mod hierarchy_separator;
mod no_redundant_story_name;
mod no_renderer_packages;
mod no_uninstalled_addons;
mod prefer_pascal_case;
mod story_exports;
mod use_storybook_expect;
mod use_storybook_testing_library;

pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(await_interactions::AwaitInteractions),
        Box::new(context_in_play_function::ContextInPlayFunction),
        Box::new(default_exports::DefaultExports),
        Box::new(hierarchy_separator::HierarchySeparator),
        Box::new(no_redundant_story_name::NoRedundantStoryName),
        Box::new(no_renderer_packages::NoRendererPackages),
        Box::new(prefer_pascal_case::PreferPascalCase),
        Box::new(story_exports::StoryExports),
        Box::new(use_storybook_expect::UseStorybookExpect),
        Box::new(use_storybook_testing_library::UseStorybookTestingLibrary),
        Box::new(no_uninstalled_addons::NoUninstalledAddons),
    ]
}

const STORY_EXTS: &[&str] = &["ts", "tsx", "js", "jsx", "mjs", "cjs"];

/// `**/*.stories.@(ts|tsx|js|jsx|mjs|cjs)` and `**/*.story.@(...)`.
pub(crate) fn is_stories_file(rel_path: &str) -> bool {
    let base = rel_path.rsplit('/').next().unwrap_or(rel_path);
    let Some((stem, ext)) = base.rsplit_once('.') else {
        return false;
    };
    if !STORY_EXTS.contains(&ext) {
        return false;
    }
    let Some((name, kind)) = stem.rsplit_once('.') else {
        return false;
    };
    !name.is_empty() && (kind == "stories" || kind == "story")
}

/// `.storybook/main.@(js|cjs|mjs|ts)`, relative to the config root (the
/// upstream pattern has no `**/`, so only the root `.storybook` matches).
pub(crate) fn is_storybook_main(rel_path: &str) -> bool {
    matches!(
        rel_path,
        ".storybook/main.js" | ".storybook/main.cjs" | ".storybook/main.mjs" | ".storybook/main.ts"
    )
}

/// The cooked value of a string literal.
pub(crate) fn lit_str<'a>(s: &'a StringLiteral<'a>) -> Cow<'a, str> {
    match s.value.as_str() {
        Some(v) => Cow::Borrowed(v),
        None => Cow::Owned(s.value.display().to_string()),
    }
}

/// The ESTree parent: the nearest ancestor that is not a
/// `ParenthesizedExpression` (ESTree has no parenthesis nodes).
pub(crate) fn estree_parent(nodes: &AstNodes<'_>, id: NodeId) -> Option<NodeId> {
    let mut cur = id;
    loop {
        let parent = nodes.parent_id(cur);
        if parent == cur {
            return None;
        }
        if !matches!(nodes.kind(parent), AstKind::ParenthesizedExpression(_)) {
            return Some(parent);
        }
        cur = parent;
    }
}

/// Name of an expression that is an ESTree `Identifier`.
pub(crate) fn ident_name<'a>(e: &'a Expression<'a>) -> Option<&'a str> {
    match e.without_parentheses() {
        Expression::Identifier(id) => Some(id.name.as_str()),
        _ => None,
    }
}

/// `(object, property name)` of an ESTree `MemberExpression`; the name is
/// set only when the property is an `Identifier` (`a.b`, or computed `a[b]`).
pub(crate) fn member_parts<'a>(
    e: &'a Expression<'a>,
) -> Option<(&'a Expression<'a>, Option<&'a str>)> {
    match e.without_parentheses() {
        Expression::StaticMemberExpression(m) => Some((&m.object, Some(m.property.name.as_str()))),
        Expression::ComputedMemberExpression(m) => Some((&m.object, ident_name(&m.expression))),
        Expression::PrivateFieldExpression(m) => Some((&m.object, None)),
        _ => None,
    }
}

/// `key.name` of an ESTree property key that is an `Identifier`
/// (`{ a: 1 }`, or computed `{ [a]: 1 }`).
pub(crate) fn key_ident_name<'a>(key: &'a PropertyKey<'a>) -> Option<&'a str> {
    match key {
        PropertyKey::StaticIdentifier(id) => Some(id.name.as_str()),
        PropertyKey::Identifier(id) => Some(id.name.as_str()),
        _ => None,
    }
}

/// ESTree `Literal` (string, number, boolean, null, bigint, regexp).
pub(crate) fn is_literal(e: &Expression<'_>) -> bool {
    matches!(
        e.without_parentheses(),
        Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::RegExpLiteral(_)
    )
}

pub(crate) fn string_literal<'a>(e: &'a Expression<'a>) -> Option<Cow<'a, str>> {
    match e.without_parentheses() {
        Expression::StringLiteral(s) => Some(lit_str(s)),
        _ => None,
    }
}

/// Upstream `getMetaObjectExpression`: the object literal of
/// `export default {...}`, or of `export default meta` where `meta` is a
/// variable initialised with one; `as`/`satisfies` are unwrapped once.
pub(crate) fn meta_object<'a>(
    semantic: &'a Semantic<'a>,
    node: &'a ExportDefaultDeclaration<'a>,
) -> Option<&'a ObjectExpression<'a>> {
    let mut meta: &'a Expression<'a> = node.declaration.as_expression()?.without_parentheses();
    if let Expression::Identifier(id) = meta {
        let scoping = semantic.scoping();
        let symbol = id
            .reference_id
            .get()
            .and_then(|r| scoping.get_reference(r).symbol_id());
        if let Some(symbol) = symbol {
            let nodes = semantic.nodes();
            let decl = std::iter::once(scoping.symbol_declaration(symbol))
                .chain(scoping.symbol_declarations(symbol))
                .find_map(|n| {
                    // The declaration node may be the binding itself; walk up
                    // to its declarator like ESLint's `def.node`.
                    std::iter::once(nodes.kind(n))
                        .chain(nodes.ancestors(n).map(|a| a.kind()))
                        .take(8)
                        .find_map(|k| match k {
                            AstKind::VariableDeclarator(d) => Some(Some(d)),
                            AstKind::Function(_)
                            | AstKind::ArrowFunctionExpression(_)
                            | AstKind::Class(_)
                            | AstKind::ImportDeclaration(_)
                            | AstKind::CatchClause(_)
                            | AstKind::Program(_) => Some(None),
                            _ => None,
                        })
                        .flatten()
                });
            if let Some(decl) = decl {
                meta = decl.init.as_ref()?.without_parentheses();
            }
        }
    }
    let meta = match meta {
        Expression::TSAsExpression(e) => e.expression.without_parentheses(),
        Expression::TSSatisfiesExpression(e) => e.expression.without_parentheses(),
        other => other,
    };
    match meta {
        Expression::ObjectExpression(o) => Some(o),
        _ => None,
    }
}

/// Upstream reports file-level problems on the first `Program.body` node
/// that is not an import (ESTree puts directives in `body`), else the first
/// node, else the program. typescript-eslint (the parser of the ESLint
/// config) starts an empty `Program` at its first token, i.e. at the end of
/// a file holding only comments.
pub(crate) fn first_non_import_span(program: &Program<'_>) -> Span {
    if let Some(d) = program.directives.first() {
        return d.span;
    }
    program
        .body
        .iter()
        .find(|s| !matches!(s, Statement::ImportDeclaration(_)))
        .or_else(|| program.body.first())
        .map_or(Span::new(program.span.end, program.span.end), GetSpan::span)
}

/// Whether some `import { storiesOf } from '...'` exists.
pub(crate) fn has_stories_of_import(program: &Program<'_>) -> bool {
    program.body.iter().any(|s| match s {
        Statement::ImportDeclaration(d) => d.specifiers.as_ref().is_some_and(|specs| {
            specs.iter().any(|sp| match sp {
                ImportDeclarationSpecifier::ImportSpecifier(sp) => {
                    module_export_ident(&sp.imported) == Some("storiesOf")
                }
                _ => false,
            })
        }),
        _ => false,
    })
}

/// `name` of a module export name that is an ESTree `Identifier`.
pub(crate) fn module_export_ident<'a>(n: &'a ModuleExportName<'a>) -> Option<&'a str> {
    match n {
        ModuleExportName::IdentifierName(id) => Some(id.name.as_str()),
        ModuleExportName::IdentifierReference(id) => Some(id.name.as_str()),
        ModuleExportName::StringLiteral(_) => None,
    }
}

/// First declarator of `export const x = ...` when its id is an identifier.
pub(crate) fn exported_first_var<'a>(
    decl: &'a Declaration<'a>,
) -> Option<(&'a BindingIdentifier<'a>, Option<&'a Expression<'a>>)> {
    let Declaration::VariableDeclaration(v) = decl else {
        return None;
    };
    let d = v.declarations.first()?;
    match &d.id {
        BindingPattern::BindingIdentifier(id) => Some((id, d.init.as_ref())),
        _ => None,
    }
}

/// First `Property` of an object literal whose key is the identifier `name`.
pub(crate) fn find_ident_prop<'a>(
    obj: &'a ObjectExpression<'a>,
    name: &str,
) -> Option<&'a ObjectProperty<'a>> {
    obj.properties.iter().find_map(|p| match p {
        ObjectPropertyKind::ObjectProperty(p) if key_ident_name(&p.key) == Some(name) => Some(&**p),
        _ => None,
    })
}
