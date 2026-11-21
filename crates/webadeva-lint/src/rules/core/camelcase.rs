//! ESLint's `camelcase` (lib/rules/camelcase.js), on oxc's scoping.
//!
//! ESLint visits declared variables per declaring node, unresolved
//! references at `Program`, and a handful of key/label selectors. Here the
//! same sets come from oxc's symbols, unresolved references and one pass
//! over the nodes.

use std::collections::HashSet;

use oxc_ast::AstKind;
use oxc_ast::ast::{
    AssignmentTarget, FunctionType, MethodDefinitionType, ModuleExportName, PropertyDefinitionType,
    PropertyKey,
};
use oxc_semantic::NodeId;
use oxc_span::{GetSpan, Span};
use serde_json::Value;

use super::util::estree_parent;
use crate::rule::{Ctx, Rule};

pub struct Camelcase;

struct Options {
    allow: Vec<String>,
    allow_regex: Vec<Option<regex::Regex>>,
    ignore_destructuring: bool,
    ignore_imports: bool,
    properties_never: bool,
}

impl Options {
    fn parse(opt: &Value) -> Self {
        let b = |k: &str| opt.get(k).and_then(Value::as_bool).unwrap_or(false);
        let allow: Vec<String> = opt
            .get("allow")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let allow_regex = allow.iter().map(|a| regex::Regex::new(a).ok()).collect();
        Self {
            allow,
            allow_regex,
            ignore_destructuring: b("ignoreDestructuring"),
            ignore_imports: b("ignoreImports"),
            properties_never: opt.get("properties").and_then(Value::as_str) == Some("never"),
        }
    }

    /// `isUnderscored`: an underscore inside, and not all upper-case.
    fn is_underscored(name: &str) -> bool {
        let body = name.trim_matches('_');
        body.contains('_') && body != body.to_uppercase()
    }

    fn is_allowed(&self, name: &str) -> bool {
        self.allow
            .iter()
            .zip(&self.allow_regex)
            .any(|(a, re)| name == a || re.as_ref().is_some_and(|re| re.is_match(name)))
    }

    fn is_good_name(&self, name: &str) -> bool {
        !Self::is_underscored(name) || self.is_allowed(name)
    }
}

struct Reporter {
    seen: HashSet<u32>,
    out: Vec<(Span, String)>,
}

impl Reporter {
    fn report(&mut self, span: Span, name: &str, private: bool) {
        if !self.seen.insert(span.start) {
            return;
        }
        let msg = if private {
            format!("#{name} is not in camel case.")
        } else {
            format!("Identifier '{name}' is not in camel case.")
        };
        self.out.push((span, msg));
    }
}

fn key_name<'a>(key: &PropertyKey<'a>) -> Option<&'a str> {
    match key {
        PropertyKey::StaticIdentifier(id) => Some(id.name.as_str()),
        _ => None,
    }
}

/// Simple assignment target as a member expression span, if it is `a.b`.
fn static_member_span(t: &AssignmentTarget<'_>) -> Option<Span> {
    match t {
        AssignmentTarget::StaticMemberExpression(m) => Some(m.span),
        _ => None,
    }
}

fn simple_ident_span(t: &AssignmentTarget<'_>) -> Option<Span> {
    match t {
        AssignmentTarget::AssignmentTargetIdentifier(id) => Some(id.span),
        _ => None,
    }
}

impl Rule for Camelcase {
    fn name(&self) -> &'static str {
        "camelcase"
    }

    fn validate(&self, options: &[Value], _settings: &Value) -> Result<(), String> {
        let Some(opt) = options.first() else {
            return Ok(());
        };
        let Value::Object(map) = opt else {
            return Err("options must be an object".into());
        };
        for (k, v) in map {
            let ok = match k.as_str() {
                "ignoreDestructuring" | "ignoreImports" | "ignoreGlobals" => v.is_boolean(),
                "properties" => matches!(v.as_str(), Some("always" | "never")),
                "allow" => v.as_array().is_some_and(|a| a.iter().all(Value::is_string)),
                _ => return Err(format!("unknown option {k:?}")),
            };
            if !ok {
                return Err(format!("invalid value for {k:?}"));
            }
        }
        Ok(())
    }

    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        let opts = Options::parse(ctx.option());
        let nodes = ctx.semantic.nodes();
        let scoping = ctx.semantic.scoping();
        let mut r = Reporter {
            seen: HashSet::new(),
            out: Vec::new(),
        };

        // `equalsToOriginalName` for a binding or reference identifier node.
        let equals_original = |id: NodeId, name: &str| -> bool {
            let mut value_span = nodes.kind(id).span();
            let mut parent = nodes.parent_id(id);
            match nodes.kind(parent) {
                AstKind::AssignmentPattern(_) => {
                    value_span = nodes.kind(parent).span();
                    parent = nodes.parent_id(parent);
                }
                AstKind::AssignmentTargetWithDefault(d)
                    if simple_ident_span(&d.binding) == Some(value_span) =>
                {
                    value_span = d.span;
                    parent = nodes.parent_id(parent);
                }
                _ => {}
            }
            match nodes.kind(parent) {
                AstKind::BindingProperty(p) => {
                    !p.computed && p.value.span() == value_span && key_name(&p.key) == Some(name)
                }
                AstKind::ObjectProperty(p) => {
                    !p.computed && p.value.span() == value_span && key_name(&p.key) == Some(name)
                }
                AstKind::AssignmentTargetPropertyIdentifier(p) => p.binding.span == value_span,
                AstKind::AssignmentTargetPropertyProperty(p) => {
                    !p.computed && p.binding.span() == value_span && key_name(&p.name) == Some(name)
                }
                AstKind::ImportSpecifier(s) => {
                    s.local.span == value_span && s.imported.name().as_str() == name
                }
                _ => false,
            }
        };

        // `reportReferenceId`
        let report_reference = |r: &mut Reporter, id: NodeId, name: &str| {
            let span = nodes.kind(id).span();
            let parent_id = estree_parent(nodes, id);
            match nodes.kind(parent_id) {
                // For backward compatibility, ignored in calls.
                AstKind::CallExpression(_) | AstKind::NewExpression(_) => return,
                // ... and as a default value.
                AstKind::AssignmentPattern(p) if p.right.span().contains_inclusive(span) => return,
                AstKind::FormalParameter(p)
                    if p.initializer
                        .as_ref()
                        .is_some_and(|i| i.span().contains_inclusive(span)) =>
                {
                    return;
                }
                AstKind::AssignmentTargetWithDefault(d)
                    if d.init.span().contains_inclusive(span) =>
                {
                    return;
                }
                AstKind::AssignmentTargetPropertyIdentifier(p)
                    if p.init
                        .as_ref()
                        .is_some_and(|i| i.span().contains_inclusive(span)) =>
                {
                    return;
                }
                _ => {}
            }
            if opts.ignore_destructuring && equals_original(id, name) {
                return;
            }
            r.report(span, name, false);
        };

        // Program: references to undeclared (or configured) globals.
        for (name, refs) in scoping.root_unresolved_references() {
            if opts.is_good_name(name) {
                continue;
            }
            // `ignoreGlobals` only skips *configured* globals, which we cannot
            // see; none of the config's browser/node globals is non-camelcase,
            // so it changes nothing for the preset.
            for &rid in refs {
                let node = scoping.get_reference(rid).node_id();
                report_reference(&mut r, node, name);
            }
        }

        // Declared variables of the node types ESLint visits.
        for sid in scoping.symbol_ids() {
            let name = scoping.symbol_name(sid);
            if opts.is_good_name(name) {
                continue;
            }
            let decl = scoping.symbol_declaration(sid);
            let mut is_import = false;
            let selected = match nodes.kind(decl) {
                AstKind::VariableDeclarator(_) | AstKind::Class(_) | AstKind::CatchParameter(_) => true,
                AstKind::Function(f) => match f.r#type {
                    FunctionType::FunctionDeclaration | FunctionType::FunctionExpression => true,
                    // Overload signatures: selected through the implementation.
                    FunctionType::TSDeclareFunction => scoping.symbol_redeclarations(sid).iter().any(|rd| {
                        matches!(nodes.kind(rd.declaration), AstKind::Function(f) if f.r#type == FunctionType::FunctionDeclaration)
                    }),
                    FunctionType::TSEmptyBodyFunctionExpression => false,
                },
                AstKind::FormalParameter(_) | AstKind::FormalParameterRest(_) | AstKind::FormalParameters(_) => {
                    params_owner_selected(nodes, decl)
                }
                AstKind::ImportSpecifier(_) | AstKind::ImportDefaultSpecifier(_) | AstKind::ImportNamespaceSpecifier(_) => {
                    is_import = true;
                    true
                }
                _ => false,
            };
            if !selected {
                continue;
            }
            let id_span = scoping.symbol_span(sid);
            let id_node = find_binding_node(nodes, decl, id_span);
            let skip = id_node.is_some_and(|n| {
                (if is_import {
                    opts.ignore_imports
                } else {
                    opts.ignore_destructuring
                }) && equals_original(n, name)
            });
            if !skip {
                let span = id_node.map_or(id_span, |n| estree_identifier_span(nodes, n));
                r.report(span, name, false);
            }
            for reference in scoping.get_resolved_references(sid) {
                report_reference(&mut r, reference.node_id(), name);
            }
        }

        // Keys, member assignments, exports and labels.
        for node in nodes.iter() {
            match node.kind() {
                AstKind::ObjectProperty(p) if !opts.properties_never && !p.computed => {
                    if let PropertyKey::StaticIdentifier(id) = &p.key
                        && !opts.is_good_name(&id.name)
                    {
                        r.report(id.span, &id.name, false);
                    }
                }
                AstKind::MethodDefinition(m)
                    if !opts.properties_never
                        && m.r#type == MethodDefinitionType::MethodDefinition =>
                {
                    check_member_key(&mut r, &opts, &m.key, m.computed);
                }
                AstKind::PropertyDefinition(m)
                    if !opts.properties_never
                        && m.r#type == PropertyDefinitionType::PropertyDefinition =>
                {
                    check_member_key(&mut r, &opts, &m.key, m.computed);
                }
                AstKind::StaticMemberExpression(m) if !opts.properties_never => {
                    if !opts.is_good_name(&m.property.name)
                        && is_assignment_target(nodes, node.id(), m.span)
                    {
                        r.report(m.property.span, &m.property.name, false);
                    }
                }
                AstKind::ExportAllDeclaration(e) => {
                    if let Some(ModuleExportName::IdentifierName(id)) = &e.exported
                        && !opts.is_good_name(&id.name)
                    {
                        r.report(id.span, &id.name, false);
                    }
                }
                AstKind::ExportSpecifier(e) => {
                    if let ModuleExportName::IdentifierName(id) = &e.exported
                        && !opts.is_good_name(&id.name)
                    {
                        r.report(id.span, &id.name, false);
                    }
                }
                AstKind::LabelIdentifier(l) if !opts.is_good_name(&l.name) => {
                    r.report(l.span, &l.name, false);
                }
                _ => {}
            }
        }

        for (span, msg) in r.out {
            ctx.report(span, msg);
        }
    }
}

fn check_member_key(r: &mut Reporter, opts: &Options, key: &PropertyKey<'_>, computed: bool) {
    match key {
        PropertyKey::StaticIdentifier(id) if !computed && !opts.is_good_name(&id.name) => {
            r.report(id.span, &id.name, false);
        }
        PropertyKey::PrivateIdentifier(id) if !opts.is_good_name(&id.name) => {
            r.report(id.span, &id.name, true);
        }
        _ => {}
    }
}

/// Whether the function owning these parameters is one ESLint visits
/// (`FunctionDeclaration`, `FunctionExpression`, `ArrowFunctionExpression`),
/// not a TypeScript signature or a body-less overload.
fn params_owner_selected(nodes: &oxc_semantic::AstNodes<'_>, decl: NodeId) -> bool {
    for kind in std::iter::once(nodes.kind(decl)).chain(nodes.ancestor_kinds(decl)) {
        match kind {
            AstKind::Function(f) => {
                return matches!(
                    f.r#type,
                    FunctionType::FunctionDeclaration | FunctionType::FunctionExpression
                );
            }
            AstKind::ArrowFunctionExpression(_) => return true,
            AstKind::FormalParameter(_)
            | AstKind::FormalParameterRest(_)
            | AstKind::FormalParameters(_) => {}
            _ => return false,
        }
    }
    false
}

/// The `BindingIdentifier` node at `span` under `decl`.
fn find_binding_node(
    nodes: &oxc_semantic::AstNodes<'_>,
    decl: NodeId,
    span: Span,
) -> Option<NodeId> {
    // Binding identifiers are created right after their declaring node, so a
    // short forward scan finds them.
    let start = decl.index();
    nodes
        .iter_enumerated()
        .skip(start)
        .take(4096)
        .find(|(_, n)| matches!(n.kind(), AstKind::BindingIdentifier(b) if b.span == span))
        .map(|(id, _)| id)
}

/// ESTree's `Identifier` range includes a type annotation (and `?`) that oxc
/// keeps on the declarator / parameter.
fn estree_identifier_span(nodes: &oxc_semantic::AstNodes<'_>, id: NodeId) -> Span {
    let span = nodes.kind(id).span();
    let annotated_end = match nodes.parent_kind(id) {
        AstKind::VariableDeclarator(d) if d.id.span() == span => {
            d.type_annotation.as_ref().map(|t| t.span.end)
        }
        AstKind::FormalParameter(p) if p.pattern.span() == span => p
            .type_annotation
            .as_ref()
            .map(|t| t.span.end)
            .or(p.optional.then_some(span.end + 1)),
        AstKind::CatchParameter(c) if c.pattern.span() == span => {
            c.type_annotation.as_ref().map(|t| t.span.end)
        }
        _ => None,
    };
    annotated_end.map_or(span, |end| Span::new(span.start, end))
}

/// `isAssignmentTarget(memberExpression)`.
fn is_assignment_target(nodes: &oxc_semantic::AstNodes<'_>, id: NodeId, span: Span) -> bool {
    let parent = nodes.parent_kind(id);
    match parent {
        AstKind::AssignmentExpression(a) => static_member_span(&a.left) == Some(span),
        AstKind::AssignmentTargetWithDefault(d) => static_member_span(&d.binding) == Some(span),
        AstKind::AssignmentTargetPropertyProperty(p) => p.binding.span() == span,
        AstKind::ArrayAssignmentTarget(_) | AstKind::AssignmentTargetRest(_) => true,
        _ => false,
    }
}
