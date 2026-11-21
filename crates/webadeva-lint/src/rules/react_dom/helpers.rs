//! Shared helpers for the `react_dom` group: an ESTree-flavoured view of the
//! oxc AST (parents without parentheses, `unwrap` through TS wrappers), the
//! variable resolution helpers of `@eslint-react/var`, a port of
//! `getStaticValue` (eslint-utils), and the JSX helpers of `@eslint-react/jsx`.
//!
//! Nodes are handled as [`NodeId`]s so identity comparisons (`a === b` in
//! the upstream code) are cheap and every node kind can be passed around.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use oxc_ast::AstKind;
use oxc_ast::ast::*;
use oxc_semantic::Semantic;
use oxc_span::{GetSpan, Span};
use oxc_syntax::node::NodeId;
use oxc_syntax::symbol::SymbolId;
use serde_json::Value;

use crate::rule::Ctx;

// ---------------------------------------------------------------------------
// Tree navigation
// ---------------------------------------------------------------------------

#[inline]
pub fn kind<'a>(sem: &Semantic<'a>, id: NodeId) -> AstKind<'a> {
    sem.nodes().kind(id)
}

pub fn span_of(sem: &Semantic<'_>, id: NodeId) -> Span {
    kind(sem, id).span()
}

pub fn text<'a>(sem: &Semantic<'a>, id: NodeId) -> &'a str {
    let s = span_of(sem, id);
    &sem.source_text()[s.start as usize..s.end as usize]
}

/// Parent as ESTree sees it: parentheses are not nodes there.
pub fn parent(sem: &Semantic<'_>, id: NodeId) -> Option<NodeId> {
    if matches!(kind(sem, id), AstKind::Program(_)) {
        return None;
    }
    let mut p = sem.nodes().parent_id(id);
    while matches!(kind(sem, p), AstKind::ParenthesizedExpression(_)) {
        p = sem.nodes().parent_id(p);
    }
    Some(p)
}

/// `Traverse.findParent(node, test, stop)`.
pub fn find_parent(
    sem: &Semantic<'_>,
    id: NodeId,
    mut test: impl FnMut(NodeId) -> bool,
    mut stop: impl FnMut(NodeId) -> bool,
) -> Option<NodeId> {
    let mut cur = parent(sem, id);
    while let Some(c) = cur {
        if stop(c) {
            return None;
        }
        if test(c) {
            return Some(c);
        }
        if matches!(kind(sem, c), AstKind::Program(_)) {
            return None;
        }
        cur = parent(sem, c);
    }
    None
}

pub fn find_parent_simple(
    sem: &Semantic<'_>,
    id: NodeId,
    test: impl FnMut(NodeId) -> bool,
) -> Option<NodeId> {
    find_parent(sem, id, test, |_| false)
}

pub fn has_ancestor(sem: &Semantic<'_>, id: NodeId, target: NodeId) -> bool {
    find_parent_simple(sem, id, |n| n == target).is_some()
}

fn inner_of_wrapper(k: AstKind<'_>, chain: bool) -> Option<NodeId> {
    Some(match k {
        AstKind::TSAsExpression(e) => e.expression.node_id(),
        AstKind::TSSatisfiesExpression(e) => e.expression.node_id(),
        AstKind::TSTypeAssertion(e) => e.expression.node_id(),
        AstKind::TSNonNullExpression(e) => e.expression.node_id(),
        AstKind::TSInstantiationExpression(e) => e.expression.node_id(),
        AstKind::ParenthesizedExpression(e) => e.expression.node_id(),
        AstKind::ChainExpression(e) if chain => e.expression.node_id(),
        _ => return None,
    })
}

/// `Extract.unwrap`: through TS wrappers, chain expressions (and parens).
pub fn unwrap(sem: &Semantic<'_>, mut id: NodeId) -> NodeId {
    while let Some(inner) = inner_of_wrapper(kind(sem, id), true) {
        id = inner;
    }
    id
}

/// Unwrap only `Check.isTypeExpression` wrappers (and parens).
pub fn unwrap_type(sem: &Semantic<'_>, mut id: NodeId) -> NodeId {
    while let Some(inner) = inner_of_wrapper(kind(sem, id), false) {
        id = inner;
    }
    id
}

pub fn skip_parens(sem: &Semantic<'_>, mut id: NodeId) -> NodeId {
    while let AstKind::ParenthesizedExpression(p) = kind(sem, id) {
        id = p.expression.node_id();
    }
    id
}

pub fn is_type_expression(k: AstKind<'_>) -> bool {
    matches!(
        k,
        AstKind::TSAsExpression(_)
            | AstKind::TSSatisfiesExpression(_)
            | AstKind::TSTypeAssertion(_)
            | AstKind::TSNonNullExpression(_)
            | AstKind::TSInstantiationExpression(_)
    )
}

pub fn is_type_assertion_expression(k: AstKind<'_>) -> bool {
    matches!(
        k,
        AstKind::TSAsExpression(_)
            | AstKind::TSSatisfiesExpression(_)
            | AstKind::TSTypeAssertion(_)
            | AstKind::TSNonNullExpression(_)
    )
}

pub fn is_type_or_chain(k: AstKind<'_>) -> bool {
    is_type_expression(k) || matches!(k, AstKind::ChainExpression(_))
}

pub fn is_function(k: AstKind<'_>) -> bool {
    matches!(
        k,
        AstKind::Function(_) | AstKind::ArrowFunctionExpression(_)
    )
}

pub fn is_member(k: AstKind<'_>) -> bool {
    matches!(
        k,
        AstKind::StaticMemberExpression(_)
            | AstKind::ComputedMemberExpression(_)
            | AstKind::PrivateFieldExpression(_)
    )
}

pub fn is_identifier(k: AstKind<'_>) -> bool {
    ident_name(k).is_some()
}

/// Name of anything ESTree calls an `Identifier`.
pub fn ident_name<'a>(k: AstKind<'a>) -> Option<&'a str> {
    match k {
        AstKind::IdentifierReference(r) => Some(r.name.as_str()),
        AstKind::BindingIdentifier(b) => Some(b.name.as_str()),
        AstKind::IdentifierName(n) => Some(n.name.as_str()),
        _ => None,
    }
}

/// Member object node.
pub fn member_object(k: AstKind<'_>) -> Option<NodeId> {
    match k {
        AstKind::StaticMemberExpression(m) => Some(m.object.node_id()),
        AstKind::ComputedMemberExpression(m) => Some(m.object.node_id()),
        AstKind::PrivateFieldExpression(m) => Some(m.object.node_id()),
        _ => None,
    }
}

/// `Check.isConditional`.
pub fn is_conditional(k: AstKind<'_>) -> bool {
    matches!(
        k,
        AstKind::DoWhileStatement(_)
            | AstKind::ForInStatement(_)
            | AstKind::ForOfStatement(_)
            | AstKind::ForStatement(_)
            | AstKind::WhileStatement(_)
            | AstKind::IfStatement(_)
            | AstKind::SwitchStatement(_)
            | AstKind::LogicalExpression(_)
            | AstKind::ConditionalExpression(_)
    )
}

/// `Extract.getCalleeName`.
pub fn callee_name<'a>(sem: &Semantic<'a>, call: &CallExpression<'a>) -> Option<&'a str> {
    let callee = unwrap(sem, call.callee.node_id());
    match kind(sem, callee) {
        AstKind::IdentifierReference(r) => Some(r.name.as_str()),
        AstKind::StaticMemberExpression(m) => Some(m.property.name.as_str()),
        _ => None,
    }
}

pub fn callee_name_of_id<'a>(sem: &Semantic<'a>, id: NodeId) -> Option<&'a str> {
    match kind(sem, id) {
        AstKind::CallExpression(c) => callee_name(sem, c),
        _ => None,
    }
}

/// `Extract.getFullyQualifiedName`.
pub fn fully_qualified_name(sem: &Semantic<'_>, id: NodeId) -> String {
    let id = unwrap(sem, id);
    match kind(sem, id) {
        AstKind::IdentifierReference(r) => r.name.to_string(),
        AstKind::BindingIdentifier(r) => r.name.to_string(),
        AstKind::IdentifierName(r) => r.name.to_string(),
        AstKind::PrivateIdentifier(r) => r.name.to_string(),
        AstKind::JSXIdentifier(r) => r.name.to_string(),
        AstKind::StaticMemberExpression(m) => {
            format!(
                "{}.{}",
                fully_qualified_name(sem, m.object.node_id()),
                m.property.name
            )
        }
        AstKind::PrivateFieldExpression(m) => {
            format!(
                "{}.{}",
                fully_qualified_name(sem, m.object.node_id()),
                m.field.name
            )
        }
        _ => text(sem, id).to_string(),
    }
}

/// `isAPI(api)` of `@eslint-react/core`: purely name based.
pub fn is_api(sem: &Semantic<'_>, id: NodeId, api: &str) -> bool {
    let name = fully_qualified_name(sem, id);
    name == api
        || (name.len() > api.len()
            && name.ends_with(api)
            && name.as_bytes()[name.len() - api.len() - 1] == b'.')
}

/// `isAPICall(api)`.
pub fn is_api_call(sem: &Semantic<'_>, call: &CallExpression<'_>, api: &str) -> bool {
    // Cheap pre-check: the last segment must be the API's last segment.
    let last = api.rsplit('.').next().unwrap_or(api);
    let callee = unwrap(sem, call.callee.node_id());
    let ok = match kind(sem, callee) {
        AstKind::IdentifierReference(r) => r.name == last,
        AstKind::StaticMemberExpression(m) => m.property.name == last,
        AstKind::PrivateFieldExpression(m) => m.field.name == last,
        AstKind::ComputedMemberExpression(_) => false,
        _ => text(sem, callee).ends_with(last),
    };
    ok && is_api(sem, callee, api)
}

/// `/^use\w*Effect$/`
pub fn is_use_effect_like_name(name: &str) -> bool {
    name.starts_with("use")
        && name.ends_with("Effect")
        && name.len() >= 9
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

pub fn is_use_effect_like_call(sem: &Semantic<'_>, id: NodeId) -> bool {
    callee_name_of_id(sem, id).is_some_and(is_use_effect_like_name)
}

pub fn is_use_ref_like_call(sem: &Semantic<'_>, id: NodeId) -> bool {
    callee_name_of_id(sem, id) == Some("useRef")
}

/// `isUseEffectSetupCallback`.
pub fn is_use_effect_setup_callback(sem: &Semantic<'_>, id: NodeId) -> bool {
    let expr = unwrap(sem, id);
    let Some(call) = find_parent_simple(sem, expr, |n| !is_type_or_chain(kind(sem, n))) else {
        return false;
    };
    let AstKind::CallExpression(c) = kind(sem, call) else {
        return false;
    };
    let Some(arg0) = c.arguments.first() else {
        return false;
    };
    unwrap(sem, arg0.node_id()) == expr && is_use_effect_like_call(sem, call)
}

/// `isUseEffectCleanupCallback`.
pub fn is_use_effect_cleanup_callback(sem: &Semantic<'_>, id: NodeId) -> bool {
    let expr = unwrap(sem, id);
    let Some(func) = find_parent_simple(sem, expr, |n| is_function(kind(sem, n))) else {
        return false;
    };
    if !is_use_effect_setup_callback(sem, func) {
        return false;
    }
    let stat = find_parent(
        sem,
        expr,
        |n| matches!(kind(sem, n), AstKind::ReturnStatement(_)),
        |n| is_function(kind(sem, n)),
    );
    if let Some(stat) = stat {
        let AstKind::ReturnStatement(r) = kind(sem, stat) else {
            return false;
        };
        return r
            .argument
            .as_ref()
            .is_some_and(|a| unwrap(sem, a.node_id()) == expr);
    }
    if !is_function(kind(sem, expr)) {
        return false;
    }
    match kind(sem, func) {
        AstKind::ArrowFunctionExpression(a) => a
            .get_expression()
            .is_some_and(|body| unwrap(sem, body.node_id()) == expr),
        _ => false,
    }
}

pub fn is_use_effect_callback(sem: &Semantic<'_>, id: NodeId) -> bool {
    is_use_effect_setup_callback(sem, id) || is_use_effect_cleanup_callback(sem, id)
}

/// `resolveEnclosingAssignmentTarget`, starting at `id` itself.
pub fn resolve_enclosing_assignment_target(sem: &Semantic<'_>, id: NodeId) -> Option<NodeId> {
    let mut cur = id;
    loop {
        match kind(sem, cur) {
            AstKind::VariableDeclarator(d) => return Some(d.id.node_id()),
            AstKind::AssignmentExpression(a) => return Some(a.left.node_id()),
            AstKind::PropertyDefinition(p) => return Some(p.key.node_id()),
            AstKind::ExportDefaultDeclaration(e) => return Some(e.declaration.node_id()),
            AstKind::BlockStatement(_) | AstKind::FunctionBody(_) | AstKind::Program(_) => {
                return None;
            }
            _ => cur = parent(sem, cur)?,
        }
    }
}

/// Identifier name of `getFunctionId(fn)` when it is an `Identifier`.
pub fn function_id_name<'a>(sem: &Semantic<'a>, id: NodeId) -> Option<&'a str> {
    if let AstKind::Function(f) = kind(sem, id)
        && let Some(fid) = &f.id
    {
        return Some(fid.name.as_str());
    }
    let p = parent(sem, id)?;
    let ident_of = |nid: NodeId| ident_name(kind(sem, nid));
    match kind(sem, p) {
        AstKind::VariableDeclarator(d)
            if d.init
                .as_ref()
                .is_some_and(|i| skip_parens(sem, i.node_id()) == id) =>
        {
            ident_of(d.id.node_id())
        }
        AstKind::AssignmentExpression(a)
            if a.operator == oxc_syntax::operator::AssignmentOperator::Assign
                && skip_parens(sem, a.right.node_id()) == id =>
        {
            ident_of(a.left.node_id())
        }
        AstKind::ObjectProperty(prop)
            if !prop.computed && skip_parens(sem, prop.value.node_id()) == id =>
        {
            ident_of(prop.key.node_id())
        }
        AstKind::PropertyDefinition(prop) if !prop.computed => ident_of(prop.key.node_id()),
        AstKind::MethodDefinition(prop) if !prop.computed => ident_of(prop.key.node_id()),
        AstKind::AssignmentPattern(ap) if skip_parens(sem, ap.right.node_id()) == id => {
            ident_of(ap.left.node_id())
        }
        AstKind::ConditionalExpression(_) => function_id_name(sem, p),
        k if is_type_assertion_expression(k) => function_id_name(sem, p),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Variables
// ---------------------------------------------------------------------------

/// The variable an identifier refers to / declares (`findVariable`).
pub fn symbol_of(sem: &Semantic<'_>, id: NodeId) -> Option<SymbolId> {
    match kind(sem, id) {
        AstKind::IdentifierReference(r) => {
            let rid = r.reference_id.get()?;
            sem.scoping().get_reference(rid).symbol_id()
        }
        AstKind::BindingIdentifier(b) => b.symbol_id.get(),
        AstKind::IdentifierName(n) => {
            let scope = sem.nodes().get_node(id).scope_id();
            sem.scoping().find_binding(scope, n.name.as_str().into())
        }
        _ => None,
    }
}

/// Find a variable by name from the scope of `at`.
pub fn symbol_by_name(sem: &Semantic<'_>, at: NodeId, name: &str) -> Option<SymbolId> {
    let scope = sem.nodes().get_node(at).scope_id();
    sem.scoping().find_binding(scope, name.into())
}

#[derive(Clone, Copy, Debug)]
pub enum Def {
    FunctionName(NodeId),
    ClassName(NodeId),
    /// The `VariableDeclarator`.
    Variable(NodeId),
    /// The function owning the parameter.
    Parameter(NodeId),
    /// Import specifier and its declaration.
    ImportBinding(NodeId, NodeId),
    CatchClause,
    EnumName(NodeId),
    EnumMember(NodeId),
    Other,
}

pub fn def_of(sem: &Semantic<'_>, sym: SymbolId) -> Def {
    let decl = sem.scoping().symbol_declaration(sym);
    match kind(sem, decl) {
        AstKind::VariableDeclarator(_) => Def::Variable(decl),
        AstKind::Function(_) => Def::FunctionName(decl),
        AstKind::Class(_) => Def::ClassName(decl),
        AstKind::FormalParameter(_)
        | AstKind::FormalParameterRest(_)
        | AstKind::FormalParameters(_) => {
            match find_parent_simple(sem, decl, |n| is_function(kind(sem, n))) {
                Some(f) => Def::Parameter(f),
                None => Def::Other,
            }
        }
        AstKind::ArrowFunctionExpression(_) => Def::Parameter(decl),
        AstKind::ImportSpecifier(_)
        | AstKind::ImportDefaultSpecifier(_)
        | AstKind::ImportNamespaceSpecifier(_) => {
            let d = find_parent_simple(sem, decl, |n| {
                matches!(kind(sem, n), AstKind::ImportDeclaration(_))
            })
            .unwrap_or(decl);
            Def::ImportBinding(decl, d)
        }
        AstKind::CatchParameter(_) | AstKind::CatchClause(_) => Def::CatchClause,
        AstKind::TSEnumDeclaration(_) => Def::EnumName(decl),
        AstKind::TSEnumMember(_) => Def::EnumMember(decl),
        _ => Def::Other,
    }
}

fn declarator_init(sem: &Semantic<'_>, decl: NodeId) -> Option<NodeId> {
    match kind(sem, decl) {
        AstKind::VariableDeclarator(d) => d.init.as_ref().map(|i| skip_parens(sem, i.node_id())),
        _ => None,
    }
}

fn enum_member_init(sem: &Semantic<'_>, m: NodeId) -> Option<NodeId> {
    match kind(sem, m) {
        AstKind::TSEnumMember(e) => e
            .initializer
            .as_ref()
            .map(|i| skip_parens(sem, i.node_id())),
        _ => None,
    }
}

/// `resolve` of `@eslint-react/var`: the value node a variable is bound to.
pub fn resolve(sem: &Semantic<'_>, ident: NodeId) -> Option<NodeId> {
    let sym = symbol_of(sem, ident)?;
    match def_of(sem, sym) {
        Def::FunctionName(n) | Def::ClassName(n) | Def::EnumName(n) => Some(n),
        Def::Variable(d) => {
            let AstKind::VariableDeclarator(decl) = kind(sem, d) else {
                return None;
            };
            // `id !== def.name`: destructured declarations resolve to nothing.
            if !matches!(decl.id, BindingPattern::BindingIdentifier(_)) {
                return None;
            }
            declarator_init(sem, d)
        }
        Def::EnumMember(m) => enum_member_init(sem, m),
        _ => None,
    }
}

/// `resolveOrigin` of `@eslint-react/var`.
pub fn resolve_origin(sem: &Semantic<'_>, ident: NodeId) -> Option<NodeId> {
    let sym = symbol_of(sem, ident)?;
    match def_of(sem, sym) {
        Def::FunctionName(n) | Def::ClassName(n) | Def::EnumName(n) | Def::Parameter(n) => Some(n),
        Def::Variable(d) => declarator_init(sem, d),
        Def::EnumMember(m) => enum_member_init(sem, m),
        Def::ImportBinding(spec, _) => Some(spec),
        _ => None,
    }
}

/// `resolveLocalFunction`: `defs.at(-1).node` when it is a function
/// declaration, or a declarator's (unwrapped) init when it is a function.
pub fn resolve_local_function(sem: &Semantic<'_>, ident: NodeId) -> Option<NodeId> {
    let sym = symbol_of(sem, ident)?;
    let is_decl = |f: NodeId| matches!(kind(sem, f), AstKind::Function(f) if f.r#type == FunctionType::FunctionDeclaration);
    match def_of(sem, sym) {
        Def::FunctionName(f) | Def::Parameter(f) => is_decl(f).then_some(f),
        Def::Variable(d) => {
            let init = unwrap(sem, declarator_init(sem, d)?);
            is_function(kind(sem, init)).then_some(init)
        }
        _ => None,
    }
}

/// `resolveImportSource(name, scope)`.
pub fn resolve_import_source(
    sem: &Semantic<'_>,
    at: NodeId,
    name: &str,
    seen: &mut Vec<String>,
) -> Option<String> {
    if seen.iter().any(|s| s == name) {
        return None;
    }
    seen.push(name.to_string());
    let sym = symbol_by_name(sem, at, name)?;
    match def_of(sem, sym) {
        Def::Variable(d) => {
            let init = unwrap(sem, declarator_init(sem, d)?);
            match kind(sem, init) {
                AstKind::StaticMemberExpression(_) | AstKind::ComputedMemberExpression(_) => {
                    let obj = skip_parens(sem, member_object(kind(sem, init))?);
                    if let AstKind::IdentifierReference(r) = kind(sem, obj) {
                        return resolve_import_source(sem, at, r.name.as_str(), seen);
                    }
                }
                AstKind::IdentifierReference(r) => {
                    return resolve_import_source(sem, at, r.name.as_str(), seen);
                }
                _ => {}
            }
            let args = require_arguments(sem, init)?;
            match args.first() {
                Some(Argument::StringLiteral(s)) => s.value.as_str().map(str::to_string),
                _ => None,
            }
        }
        Def::ImportBinding(_, decl) => match kind(sem, decl) {
            AstKind::ImportDeclaration(d) => d.source.value.as_str().map(str::to_string),
            _ => None,
        },
        _ => None,
    }
}

fn require_arguments<'a>(
    sem: &Semantic<'a>,
    id: NodeId,
) -> Option<&'a oxc_allocator::Vec<'a, Argument<'a>>> {
    let expr = unwrap(sem, id);
    match kind(sem, expr) {
        AstKind::CallExpression(c) => {
            let callee = unwrap(sem, c.callee.node_id());
            match kind(sem, callee) {
                AstKind::IdentifierReference(r) if r.name == "require" => Some(&c.arguments),
                _ => None,
            }
        }
        k @ (AstKind::StaticMemberExpression(_)
        | AstKind::ComputedMemberExpression(_)
        | AstKind::PrivateFieldExpression(_)) => require_arguments(sem, member_object(k)?),
        _ => None,
    }
}

/// `isInitializedFromReactNative`.
pub fn is_initialized_from_react_native(sem: &Semantic<'_>, at: NodeId, name: &str) -> bool {
    let lower = name.to_lowercase();
    if matches!(lower.as_str(), "react_native" | "reactnative" | "rn") {
        return true;
    }
    resolve_import_source(sem, at, name, &mut Vec::new())
        .is_some_and(|s| s.starts_with("react-native"))
}

// ---------------------------------------------------------------------------
// Static values (`getStaticValue` of eslint-utils, the parts that matter)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum SV {
    Undefined,
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    BigInt(String),
    Obj(Vec<(String, SV)>),
    Arr(Vec<SV>),
    /// A non-primitive value we do not model (e.g. a RegExp).
    Opaque,
}

impl SV {
    pub fn truthy(&self) -> bool {
        match self {
            SV::Undefined | SV::Null => false,
            SV::Bool(b) => *b,
            SV::Num(n) => *n != 0.0 && !n.is_nan(),
            SV::Str(s) => !s.is_empty(),
            SV::BigInt(b) => b != "0",
            SV::Obj(_) | SV::Arr(_) | SV::Opaque => true,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            SV::Str(s) => Some(s),
            _ => None,
        }
    }

    /// `===` (objects are never equal: they come from distinct literals).
    pub fn strict_eq(&self, other: &SV) -> bool {
        match (self, other) {
            (SV::Undefined, SV::Undefined) | (SV::Null, SV::Null) => true,
            (SV::Bool(a), SV::Bool(b)) => a == b,
            (SV::Num(a), SV::Num(b)) => a == b,
            (SV::Str(a), SV::Str(b)) => a == b,
            (SV::BigInt(a), SV::BigInt(b)) => a == b,
            _ => false,
        }
    }

    fn to_number(&self) -> f64 {
        match self {
            SV::Undefined => f64::NAN,
            SV::Null => 0.0,
            SV::Bool(b) => f64::from(u8::from(*b)),
            SV::Num(n) => *n,
            SV::Str(s) => {
                let t = s.trim();
                if t.is_empty() {
                    0.0
                } else if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
                    i64::from_str_radix(h, 16)
                        .map(|v| v as f64)
                        .unwrap_or(f64::NAN)
                } else {
                    t.parse::<f64>().unwrap_or(f64::NAN)
                }
            }
            SV::Arr(a) if a.is_empty() => 0.0,
            _ => f64::NAN,
        }
    }

    pub fn to_js_string(&self) -> Option<String> {
        Some(match self {
            SV::Undefined => "undefined".into(),
            SV::Null => "null".into(),
            SV::Bool(b) => b.to_string(),
            SV::Num(n) => num_to_string(*n),
            SV::Str(s) => s.clone(),
            SV::BigInt(b) => b.clone(),
            SV::Obj(_) => "[object Object]".into(),
            SV::Arr(items) => {
                let mut parts = Vec::new();
                for i in items {
                    parts.push(match i {
                        SV::Undefined | SV::Null => String::new(),
                        other => other.to_js_string()?,
                    });
                }
                parts.join(",")
            }
            SV::Opaque => return None,
        })
    }

    fn type_of(&self) -> &'static str {
        match self {
            SV::Undefined => "undefined",
            SV::Null | SV::Obj(_) | SV::Arr(_) | SV::Opaque => "object",
            SV::Bool(_) => "boolean",
            SV::Num(_) => "number",
            SV::Str(_) => "string",
            SV::BigInt(_) => "bigint",
        }
    }
}

pub fn num_to_string(n: f64) -> String {
    if n.is_nan() {
        "NaN".into()
    } else if n.is_infinite() {
        if n > 0.0 {
            "Infinity".into()
        } else {
            "-Infinity".into()
        }
    } else if n == 0.0 {
        "0".into()
    } else if n == n.trunc() && n.abs() < 1e21 {
        format!("{n:.0}")
    } else {
        format!("{n}")
    }
}

fn js_str(s: Option<&str>) -> String {
    s.unwrap_or_default().to_string()
}

/// `getStaticValue(node, scope)?.value`. `None` means "not static".
pub fn static_value(sem: &Semantic<'_>, id: NodeId) -> Option<SV> {
    static_value_r(sem, id, 0)
}

fn static_value_r(sem: &Semantic<'_>, id: NodeId, depth: u32) -> Option<SV> {
    if depth > 64 {
        return None;
    }
    let d = depth + 1;
    let k = kind(sem, id);
    if let Some(inner) = inner_of_wrapper(k, true) {
        return static_value_r(sem, inner, d);
    }
    match k {
        AstKind::StringLiteral(s) => Some(SV::Str(js_str(s.value.as_str()))),
        AstKind::NumericLiteral(n) => Some(SV::Num(n.value)),
        AstKind::BooleanLiteral(b) => Some(SV::Bool(b.value)),
        AstKind::NullLiteral(_) => Some(SV::Null),
        AstKind::BigIntLiteral(b) => Some(SV::BigInt(b.value.to_string())),
        AstKind::RegExpLiteral(_) => Some(SV::Opaque),
        AstKind::TemplateLiteral(t) => {
            let mut out = String::new();
            for (i, q) in t.quasis.iter().enumerate() {
                out.push_str(&js_str(q.value.cooked?.as_str()));
                if let Some(e) = t.expressions.get(i) {
                    out.push_str(&static_value_r(sem, e.node_id(), d)?.to_js_string()?);
                }
            }
            Some(SV::Str(out))
        }
        AstKind::IdentifierReference(r) => {
            let sym = symbol_of(sem, id);
            let Some(sym) = sym else {
                return match r.name.as_str() {
                    "undefined" => Some(SV::Undefined),
                    "NaN" => Some(SV::Num(f64::NAN)),
                    "Infinity" => Some(SV::Num(f64::INFINITY)),
                    _ => None,
                };
            };
            let Def::Variable(decl) = def_of(sem, sym) else {
                return None;
            };
            if !sem.scoping().symbol_redeclarations(sym).is_empty() {
                return None;
            }
            let AstKind::VariableDeclarator(vd) = kind(sem, decl) else {
                return None;
            };
            if !matches!(vd.id, BindingPattern::BindingIdentifier(_)) {
                return None;
            }
            let declaration_kind = parent(sem, decl).and_then(|p| match kind(sem, p) {
                AstKind::VariableDeclaration(v) => Some(v.kind),
                _ => None,
            });
            let is_const = declaration_kind == Some(VariableDeclarationKind::Const)
                || (vd.init.is_some()
                    && !sem
                        .scoping()
                        .get_resolved_references(sym)
                        .any(|reference| reference.is_write()));
            if !is_const {
                return None;
            }
            static_value_r(sem, vd.init.as_ref()?.node_id(), d)
        }
        AstKind::UnaryExpression(u) => {
            use oxc_syntax::operator::UnaryOperator as U;
            if u.operator == U::Void {
                return Some(SV::Undefined);
            }
            let v = static_value_r(sem, u.argument.node_id(), d)?;
            Some(match u.operator {
                U::UnaryNegation => match v {
                    SV::BigInt(b) => SV::BigInt(if let Some(s) = b.strip_prefix('-') {
                        s.into()
                    } else {
                        format!("-{b}")
                    }),
                    other => SV::Num(-other.to_number()),
                },
                U::UnaryPlus => SV::Num(v.to_number()),
                U::LogicalNot => SV::Bool(!v.truthy()),
                U::BitwiseNot => SV::Num(f64::from(!(to_int32(v.to_number())))),
                U::Typeof => SV::Str(v.type_of().into()),
                _ => return None,
            })
        }
        AstKind::BinaryExpression(b) => {
            let l = static_value_r(sem, b.left.node_id(), d)?;
            let r = static_value_r(sem, b.right.node_id(), d)?;
            binary(b.operator, &l, &r)
        }
        AstKind::LogicalExpression(l) => {
            use oxc_syntax::operator::LogicalOperator as L;
            let left = static_value_r(sem, l.left.node_id(), d)?;
            let take_left = match l.operator {
                L::And => !left.truthy(),
                L::Or => left.truthy(),
                L::Coalesce => !matches!(left, SV::Undefined | SV::Null),
            };
            if take_left {
                Some(left)
            } else {
                static_value_r(sem, l.right.node_id(), d)
            }
        }
        AstKind::ConditionalExpression(c) => {
            let t = static_value_r(sem, c.test.node_id(), d)?;
            if t.truthy() {
                static_value_r(sem, c.consequent.node_id(), d)
            } else {
                static_value_r(sem, c.alternate.node_id(), d)
            }
        }
        AstKind::SequenceExpression(s) => static_value_r(sem, s.expressions.last()?.node_id(), d),
        AstKind::ArrayExpression(a) => {
            let mut out = Vec::new();
            for el in &a.elements {
                match el {
                    ArrayExpressionElement::Elision(_) => out.push(SV::Undefined),
                    ArrayExpressionElement::SpreadElement(s) => {
                        match static_value_r(sem, s.argument.node_id(), d)? {
                            SV::Arr(items) => out.extend(items),
                            SV::Str(s) => out.extend(s.chars().map(|c| SV::Str(c.to_string()))),
                            _ => return None,
                        }
                    }
                    other => out.push(static_value_r(sem, other.node_id(), d)?),
                }
            }
            Some(SV::Arr(out))
        }
        AstKind::ObjectExpression(o) => {
            let mut out: Vec<(String, SV)> = Vec::new();
            for p in &o.properties {
                match p {
                    ObjectPropertyKind::ObjectProperty(p) => {
                        if p.kind != PropertyKind::Init || p.method {
                            return None;
                        }
                        let key = property_key_static(sem, &p.key, p.computed, d)?;
                        let v = static_value_r(sem, p.value.node_id(), d)?;
                        out.retain(|(k, _)| *k != key);
                        out.push((key, v));
                    }
                    ObjectPropertyKind::SpreadProperty(s) => {
                        match static_value_r(sem, s.argument.node_id(), d)? {
                            SV::Obj(items) => {
                                for (k, v) in items {
                                    out.retain(|(e, _)| *e != k);
                                    out.push((k, v));
                                }
                            }
                            SV::Null | SV::Undefined => {}
                            _ => return None,
                        }
                    }
                }
            }
            Some(SV::Obj(out))
        }
        AstKind::StaticMemberExpression(m) => {
            let obj = static_value_r(sem, m.object.node_id(), d)?;
            if m.optional && matches!(obj, SV::Null | SV::Undefined) {
                return Some(SV::Undefined);
            }
            member_of(&obj, m.property.name.as_str())
        }
        AstKind::ComputedMemberExpression(m) => {
            let obj = static_value_r(sem, m.object.node_id(), d)?;
            if m.optional && matches!(obj, SV::Null | SV::Undefined) {
                return Some(SV::Undefined);
            }
            let key = static_value_r(sem, m.expression.node_id(), d)?.to_js_string()?;
            member_of(&obj, &key)
        }
        _ => None,
    }
}

fn property_key_static(
    sem: &Semantic<'_>,
    key: &PropertyKey<'_>,
    computed: bool,
    d: u32,
) -> Option<String> {
    if computed {
        return static_value_r(sem, key.node_id(), d)?.to_js_string();
    }
    match key {
        PropertyKey::StaticIdentifier(i) => Some(i.name.to_string()),
        PropertyKey::StringLiteral(s) => Some(js_str(s.value.as_str())),
        PropertyKey::NumericLiteral(n) => Some(num_to_string(n.value)),
        PropertyKey::PrivateIdentifier(_) => None,
        other => static_value_r(sem, other.node_id(), d)?.to_js_string(),
    }
}

fn member_of(obj: &SV, key: &str) -> Option<SV> {
    match obj {
        SV::Obj(items) => Some(
            items
                .iter()
                .rev()
                .find(|(k, _)| k == key)
                .map_or(SV::Undefined, |(_, v)| v.clone()),
        ),
        SV::Arr(items) => {
            if key == "length" {
                return Some(SV::Num(items.len() as f64));
            }
            key.parse::<usize>()
                .ok()
                .map(|i| items.get(i).cloned().unwrap_or(SV::Undefined))
        }
        SV::Str(s) => {
            if key == "length" {
                return Some(SV::Num(s.encode_utf16().count() as f64));
            }
            let i = key.parse::<usize>().ok()?;
            Some(
                s.chars()
                    .nth(i)
                    .map_or(SV::Undefined, |c| SV::Str(c.to_string())),
            )
        }
        _ => None,
    }
}

fn to_int32(n: f64) -> i32 {
    if !n.is_finite() {
        return 0;
    }
    (n.trunc() as i64 as u64 as u32) as i32
}

fn loose_eq(l: &SV, r: &SV) -> bool {
    match (l, r) {
        (SV::Undefined | SV::Null, SV::Undefined | SV::Null) => true,
        (SV::Undefined | SV::Null, _) | (_, SV::Undefined | SV::Null) => false,
        (SV::Str(_), SV::Str(_)) | (SV::Bool(_), SV::Bool(_)) | (SV::Num(_), SV::Num(_)) => {
            l.strict_eq(r)
        }
        (SV::Obj(_) | SV::Arr(_) | SV::Opaque, _) | (_, SV::Obj(_) | SV::Arr(_) | SV::Opaque) => {
            false
        }
        _ => l.to_number() == r.to_number(),
    }
}

fn binary(op: oxc_syntax::operator::BinaryOperator, l: &SV, r: &SV) -> Option<SV> {
    use oxc_syntax::operator::BinaryOperator as B;
    Some(match op {
        B::Equality => SV::Bool(loose_eq(l, r)),
        B::Inequality => SV::Bool(!loose_eq(l, r)),
        B::StrictEquality => SV::Bool(l.strict_eq(r)),
        B::StrictInequality => SV::Bool(!l.strict_eq(r)),
        B::Addition => {
            let is_prim_str = |v: &SV| matches!(v, SV::Str(_) | SV::Obj(_) | SV::Arr(_));
            if is_prim_str(l) || is_prim_str(r) {
                SV::Str(format!("{}{}", l.to_js_string()?, r.to_js_string()?))
            } else if matches!(l, SV::BigInt(_)) || matches!(r, SV::BigInt(_)) {
                return None;
            } else {
                SV::Num(l.to_number() + r.to_number())
            }
        }
        B::Subtraction => SV::Num(l.to_number() - r.to_number()),
        B::Multiplication => SV::Num(l.to_number() * r.to_number()),
        B::Division => SV::Num(l.to_number() / r.to_number()),
        B::Remainder => SV::Num(l.to_number() % r.to_number()),
        B::Exponential => SV::Num(l.to_number().powf(r.to_number())),
        B::LessThan | B::LessEqualThan | B::GreaterThan | B::GreaterEqualThan => {
            let ord = match (l, r) {
                (SV::Str(a), SV::Str(b)) => Some(a.cmp(b)),
                _ => l.to_number().partial_cmp(&r.to_number()),
            };
            let Some(ord) = ord else {
                return Some(SV::Bool(false));
            };
            SV::Bool(match op {
                B::LessThan => ord.is_lt(),
                B::LessEqualThan => ord.is_le(),
                B::GreaterThan => ord.is_gt(),
                _ => ord.is_ge(),
            })
        }
        B::BitwiseOR => SV::Num(f64::from(to_int32(l.to_number()) | to_int32(r.to_number()))),
        B::BitwiseAnd => SV::Num(f64::from(to_int32(l.to_number()) & to_int32(r.to_number()))),
        B::BitwiseXOR => SV::Num(f64::from(to_int32(l.to_number()) ^ to_int32(r.to_number()))),
        B::ShiftLeft => SV::Num(f64::from(
            to_int32(l.to_number()).wrapping_shl(to_int32(r.to_number()) as u32 & 31),
        )),
        B::ShiftRight => SV::Num(f64::from(
            to_int32(l.to_number()).wrapping_shr(to_int32(r.to_number()) as u32 & 31),
        )),
        B::ShiftRightZeroFill => SV::Num(f64::from(
            (to_int32(l.to_number()) as u32).wrapping_shr(to_int32(r.to_number()) as u32 & 31),
        )),
        _ => return None,
    })
}

/// Value of a literal node, as `Literal.value` (for `===` comparisons).
fn literal_value(k: AstKind<'_>) -> Option<SV> {
    Some(match k {
        AstKind::StringLiteral(s) => SV::Str(js_str(s.value.as_str())),
        AstKind::NumericLiteral(n) => SV::Num(n.value),
        AstKind::BooleanLiteral(b) => SV::Bool(b.value),
        AstKind::NullLiteral(_) => SV::Null,
        AstKind::BigIntLiteral(b) => SV::BigInt(b.value.to_string()),
        AstKind::RegExpLiteral(_) => SV::Opaque,
        _ => return None,
    })
}

fn is_literal(k: AstKind<'_>) -> bool {
    literal_value(k).is_some()
}

// ---------------------------------------------------------------------------
// Node equality (`Compare.isEqual`, `isValueEqual`, `isAssignmentTargetEqual`)
// ---------------------------------------------------------------------------

/// ESTree node type, coarse enough for `Compare.isEqual`.
#[derive(PartialEq, Eq, Clone, Copy)]
enum EType {
    Literal,
    TemplateLiteral,
    TemplateElement,
    Identifier,
    PrivateIdentifier,
    Member,
    Call,
    This,
    Other(u8),
}

fn etype(k: AstKind<'_>) -> EType {
    match k {
        _ if is_literal(k) => EType::Literal,
        AstKind::TemplateLiteral(_) => EType::TemplateLiteral,
        AstKind::TemplateElement(_) => EType::TemplateElement,
        AstKind::IdentifierReference(_)
        | AstKind::BindingIdentifier(_)
        | AstKind::IdentifierName(_) => EType::Identifier,
        AstKind::PrivateIdentifier(_) => EType::PrivateIdentifier,
        AstKind::StaticMemberExpression(_)
        | AstKind::ComputedMemberExpression(_)
        | AstKind::PrivateFieldExpression(_) => EType::Member,
        AstKind::CallExpression(_) => EType::Call,
        AstKind::ThisExpression(_) => EType::This,
        other => EType::Other(other.ty() as u8),
    }
}

/// Property node of a member expression (ESTree `property`).
fn member_property(k: AstKind<'_>) -> Option<NodeId> {
    match k {
        AstKind::StaticMemberExpression(m) => Some(m.property.node_id()),
        AstKind::ComputedMemberExpression(m) => Some(m.expression.node_id()),
        AstKind::PrivateFieldExpression(m) => Some(m.field.node_id()),
        _ => None,
    }
}

fn member_computed(k: AstKind<'_>) -> bool {
    matches!(k, AstKind::ComputedMemberExpression(_))
}

/// `Compare.isEqual`.
pub fn compare_is_equal(sem: &Semantic<'_>, a: NodeId, b: NodeId) -> bool {
    let a = unwrap_type(sem, a);
    let b = unwrap_type(sem, b);
    if a == b {
        return true;
    }
    let (ka, kb) = (kind(sem, a), kind(sem, b));
    if etype(ka) != etype(kb) {
        return false;
    }
    match (ka, kb) {
        _ if is_literal(ka) => literal_value(ka)
            .zip(literal_value(kb))
            .is_some_and(|(x, y)| x.strict_eq(&y)),
        (AstKind::TemplateLiteral(x), AstKind::TemplateLiteral(y)) => {
            x.quasis.len() == y.quasis.len()
                && x.expressions.len() == y.expressions.len()
                && x.quasis
                    .iter()
                    .zip(y.quasis.iter())
                    .all(|(p, q)| p.value.raw == q.value.raw)
                && x.expressions
                    .iter()
                    .zip(y.expressions.iter())
                    .all(|(p, q)| compare_is_equal(sem, p.node_id(), q.node_id()))
        }
        (AstKind::TemplateElement(x), AstKind::TemplateElement(y)) => x.value.raw == y.value.raw,
        (AstKind::PrivateIdentifier(x), AstKind::PrivateIdentifier(y)) => x.name == y.name,
        _ if etype(ka) == EType::Identifier => ident_name(ka) == ident_name(kb),
        _ if etype(ka) == EType::Member => {
            let (pa, pb) = (member_property(ka).unwrap(), member_property(kb).unwrap());
            let (oa, ob) = (member_object(ka).unwrap(), member_object(kb).unwrap());
            compare_is_equal(sem, pa, pb) && compare_is_equal(sem, oa, ob)
        }
        (AstKind::CallExpression(x), AstKind::CallExpression(y)) => {
            x.optional == y.optional
                && x.arguments.len() == y.arguments.len()
                && compare_is_equal(sem, x.callee.node_id(), y.callee.node_id())
                && x.arguments
                    .iter()
                    .zip(y.arguments.iter())
                    .all(|(p, q)| compare_is_equal(sem, p.node_id(), q.node_id()))
        }
        (AstKind::ThisExpression(_), AstKind::ThisExpression(_)) => true,
        _ => false,
    }
}

fn this_block(sem: &Semantic<'_>, id: NodeId) -> Option<NodeId> {
    find_parent_simple(sem, id, |n| {
        matches!(
            kind(sem, n),
            AstKind::Function(_) | AstKind::ClassBody(_) | AstKind::Program(_)
        )
    })
}

/// Variable declaration's parent's parent (`def.parent.parent`), for
/// `for (const x of xs)` detection.
fn for_of_of_symbol(sem: &Semantic<'_>, sym: SymbolId) -> Option<NodeId> {
    let Def::Variable(decl) = def_of(sem, sym) else {
        return None;
    };
    let declaration = parent(sem, decl)?;
    let pp = parent(sem, declaration)?;
    matches!(kind(sem, pp), AstKind::ForOfStatement(_)).then_some(pp)
}

/// `isValueEqual`.
pub fn is_value_equal(sem: &Semantic<'_>, a: NodeId, b: NodeId) -> bool {
    let a = unwrap_type(sem, a);
    let b = unwrap_type(sem, b);
    if a == b {
        return true;
    }
    let (ka, kb) = (kind(sem, a), kind(sem, b));
    if is_literal(ka) && is_literal(kb) {
        return literal_value(ka)
            .zip(literal_value(kb))
            .is_some_and(|(x, y)| x.strict_eq(&y));
    }
    if let (AstKind::TemplateElement(x), AstKind::TemplateElement(y)) = (ka, kb) {
        return x.value.cooked == y.value.cooked;
    }
    if is_identifier(ka) && is_identifier(kb) {
        let a_var = symbol_of(sem, a);
        let b_var = symbol_of(sem, b);
        // Parameters of callbacks passed to equal callees, at equal positions.
        let a_origin = resolve_origin(sem, a);
        let b_origin = resolve_origin(sem, b);
        if let (Some(ao), Some(bo)) = (a_origin, b_origin) {
            let (apar, bpar) = (parent(sem, ao), parent(sem, bo));
            if let (Some(ap), Some(bp)) = (apar, bpar)
                && let (AstKind::CallExpression(ac), AstKind::CallExpression(bc)) =
                    (kind(sem, ap), kind(sem, bp))
                && is_function(kind(sem, ao))
                && is_function(kind(sem, bo))
            {
                if !compare_is_equal(sem, ac.callee.node_id(), bc.callee.node_id()) {
                    return false;
                }
                let pos = |f: NodeId, x: NodeId| -> Option<usize> {
                    let params = match kind(sem, f) {
                        AstKind::Function(f) => &f.params,
                        AstKind::ArrowFunctionExpression(f) => &f.params,
                        _ => return None,
                    };
                    params
                        .items
                        .iter()
                        .position(|p| compare_is_equal(sem, p.pattern.node_id(), x))
                };
                let (ap, bp) = (pos(ao, a), pos(bo, b));
                return ap.is_some() && bp.is_some() && ap == bp;
            }
        }
        let a_for = a_var.and_then(|s| for_of_of_symbol(sem, s));
        let b_for = b_var.and_then(|s| for_of_of_symbol(sem, s));
        if let (Some(af), Some(bf)) = (a_for, b_for) {
            let (AstKind::ForOfStatement(x), AstKind::ForOfStatement(y)) =
                (kind(sem, af), kind(sem, bf))
            else {
                return false;
            };
            let left_kind = |l: &ForStatementLeft<'_>| -> u8 {
                match l {
                    ForStatementLeft::VariableDeclaration(_) => 0,
                    _ => 1,
                }
            };
            if left_kind(&x.left) != left_kind(&y.left) {
                return false;
            }
            if !compare_is_equal(sem, x.right.node_id(), y.right.node_id()) {
                return false;
            }
            if af == bf {
                return a_var.is_some() && a_var == b_var;
            }
            return true;
        }
        return a_var.is_some() && a_var == b_var;
    }
    if is_member(ka) && is_member(kb) {
        let props_equal = if member_computed(ka) && member_computed(kb) {
            is_value_equal(
                sem,
                member_property(ka).unwrap(),
                member_property(kb).unwrap(),
            )
        } else {
            compare_is_equal(
                sem,
                member_property(ka).unwrap(),
                member_property(kb).unwrap(),
            )
        };
        return props_equal
            && is_value_equal(sem, member_object(ka).unwrap(), member_object(kb).unwrap());
    }
    if matches!(ka, AstKind::ThisExpression(_)) && matches!(kb, AstKind::ThisExpression(_)) {
        return this_block(sem, a) == this_block(sem, b);
    }
    match (static_value(sem, a), static_value(sem, b)) {
        (Some(x), Some(y)) => x.strict_eq(&y),
        _ => false,
    }
}

/// `isAssignmentTargetEqual`.
pub fn is_assignment_target_equal(sem: &Semantic<'_>, a: NodeId, b: NodeId) -> bool {
    let ua = unwrap_type(sem, a);
    let ub = unwrap_type(sem, b);
    if is_identifier(kind(sem, ua)) && is_identifier(kind(sem, ub)) {
        return is_value_equal(sem, ua, ub);
    }
    compare_is_equal(sem, ua, ub) || is_value_equal(sem, ua, ub)
}

// ---------------------------------------------------------------------------
// JSX (`@eslint-react/jsx`)
// ---------------------------------------------------------------------------

pub fn attribute_name(attr: &JSXAttribute<'_>) -> String {
    match &attr.name {
        JSXAttributeName::Identifier(i) => i.name.to_string(),
        JSXAttributeName::NamespacedName(n) => format!("{}:{}", n.namespace.name, n.name.name),
    }
}

/// `isAttribute(name)`: a plain (non-namespaced) attribute with that name.
pub fn is_attribute(item: &JSXAttributeItem<'_>, name: &str) -> bool {
    matches!(item, JSXAttributeItem::Attribute(a) if matches!(&a.name, JSXAttributeName::Identifier(i) if i.name == name))
}

/// `findSpreadProperty`: the `ObjectProperty` providing `name` in a spread.
pub fn find_spread_property(sem: &Semantic<'_>, argument: NodeId, name: &str) -> Option<NodeId> {
    find_spread_property_r(sem, argument, name, &mut Vec::new())
}

fn find_spread_property_r(
    sem: &Semantic<'_>,
    argument: NodeId,
    name: &str,
    seen: &mut Vec<NodeId>,
) -> Option<NodeId> {
    let argument = skip_parens(sem, argument);
    let object = match kind(sem, argument) {
        AstKind::IdentifierReference(_) => {
            let mut init = resolve(sem, argument);
            while let Some(i) = init
                && matches!(kind(sem, i), AstKind::IdentifierReference(_))
                && !seen.contains(&i)
            {
                seen.push(i);
                init = resolve(sem, i);
            }
            match init {
                Some(i) if matches!(kind(sem, i), AstKind::ObjectExpression(_)) => i,
                _ => return None,
            }
        }
        AstKind::ObjectExpression(_) => argument,
        _ => return None,
    };
    if seen.contains(&object) {
        return None;
    }
    seen.push(object);
    let AstKind::ObjectExpression(o) = kind(sem, object) else {
        return None;
    };
    for p in o.properties.iter().rev() {
        match p {
            ObjectPropertyKind::ObjectProperty(p) => {
                if p.computed {
                    if static_value(sem, p.key.node_id()).is_some_and(|v| v.as_str() == Some(name))
                    {
                        return Some(p.node_id());
                    }
                    continue;
                }
                match &p.key {
                    PropertyKey::StaticIdentifier(i) if i.name == name => return Some(p.node_id()),
                    PropertyKey::StringLiteral(s) if s.value.as_str() == Some(name) => {
                        return Some(p.node_id());
                    }
                    _ => {}
                }
            }
            ObjectPropertyKind::SpreadProperty(s) => {
                if let Some(found) = find_spread_property_r(sem, s.argument.node_id(), name, seen) {
                    return Some(found);
                }
            }
        }
    }
    None
}

/// `findAttribute`: last attribute (or spread providing it) named `name`.
pub fn find_attribute<'b, 'a>(
    sem: &Semantic<'a>,
    opening: &'b JSXOpeningElement<'a>,
    name: &str,
) -> Option<&'b JSXAttributeItem<'a>> {
    opening.attributes.iter().rev().find(|item| match item {
        JSXAttributeItem::Attribute(a) => attribute_name(a) == name,
        JSXAttributeItem::SpreadAttribute(s) => {
            find_spread_property(sem, s.argument.node_id(), name).is_some()
        }
    })
}

pub fn has_attribute(sem: &Semantic<'_>, opening: &JSXOpeningElement<'_>, name: &str) -> bool {
    find_attribute(sem, opening, name).is_some()
}

/// Result of `resolveAttributeValue`: the static value (`toStatic()`) and the
/// node ESLint uses as `value.node` (`None` for a boolean attribute).
pub struct AttrValue {
    pub value: Option<SV>,
    pub node: Option<Span>,
}

pub fn resolve_attribute_value(
    sem: &Semantic<'_>,
    item: &JSXAttributeItem<'_>,
    name: Option<&str>,
) -> AttrValue {
    match item {
        JSXAttributeItem::Attribute(a) => match &a.value {
            None => AttrValue {
                value: Some(SV::Bool(true)),
                node: None,
            },
            Some(JSXAttributeValue::StringLiteral(s)) => AttrValue {
                value: Some(SV::Str(js_str(s.value.as_str()))),
                node: Some(s.span),
            },
            Some(JSXAttributeValue::ExpressionContainer(c)) => match &c.expression {
                JSXExpression::EmptyExpression(e) => AttrValue {
                    value: None,
                    node: Some(e.span),
                },
                expr => {
                    let e = expr.as_expression().expect("non-empty JSX expression");
                    AttrValue {
                        value: static_value(sem, e.node_id()),
                        node: Some(e.span()),
                    }
                }
            },
            Some(JSXAttributeValue::Element(e)) => AttrValue {
                value: None,
                node: Some(e.span),
            },
            Some(JSXAttributeValue::Fragment(f)) => AttrValue {
                value: None,
                node: Some(f.span),
            },
        },
        JSXAttributeItem::SpreadAttribute(s) => {
            let value = name.and_then(|n| {
                let prop = find_spread_property(sem, s.argument.node_id(), n)?;
                let AstKind::ObjectProperty(p) = kind(sem, prop) else {
                    return None;
                };
                static_value(sem, p.value.node_id())
            });
            AttrValue {
                value,
                node: Some(s.argument.span()),
            }
        }
    }
}

pub fn jsx_text_raw<'a>(source: &'a str, t: &JSXText<'_>) -> &'a str {
    &source[t.span.start as usize..t.span.end as usize]
}

pub fn is_whitespace_text(source: &str, child: &JSXChild<'_>) -> bool {
    matches!(child, JSXChild::Text(t) if jsx_text_raw(source, t).trim().is_empty())
}

pub fn is_empty_expression_container(child: &JSXChild<'_>) -> bool {
    matches!(child, JSXChild::ExpressionContainer(c) if matches!(c.expression, JSXExpression::EmptyExpression(_)))
}

pub fn is_empty_string_expression(child: &JSXChild<'_>) -> bool {
    matches!(child, JSXChild::ExpressionContainer(c)
        if matches!(&c.expression, JSXExpression::StringLiteral(s) if s.value.as_str() == Some("")))
}

pub fn has_children(source: &str, children: &[JSXChild<'_>]) -> bool {
    !children.is_empty()
        && !children.iter().all(|c| {
            is_whitespace_text(source, c)
                || is_empty_expression_container(c)
                || is_empty_string_expression(c)
        })
}

/// `isPaddingWhitespace`: multi-line whitespace-only text.
pub fn is_padding_whitespace(child: &JSXChild<'_>) -> bool {
    match child {
        JSXChild::Text(t) => {
            let v = t.value.as_str();
            v.contains('\n') && v.chars().all(|c| matches!(c, ' ' | '\t' | '\n' | '\r'))
        }
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

fn react_x_settings<'c>(ctx: &'c Ctx<'_>) -> Option<&'c serde_json::Map<String, Value>> {
    ctx.settings.get("react-x").and_then(Value::as_object)
}

/// `settings["react-x"].importSource` (default `react`).
pub fn import_source(ctx: &Ctx<'_>) -> String {
    react_x_settings(ctx)
        .and_then(|s| s.get("importSource"))
        .and_then(Value::as_str)
        .unwrap_or("react")
        .to_string()
}

/// `settings["react-x"].polymorphicPropName` (default `as`).
pub fn polymorphic_prop_name(ctx: &Ctx<'_>) -> String {
    react_x_settings(ctx)
        .and_then(|s| s.get("polymorphicPropName"))
        .and_then(Value::as_str)
        .unwrap_or("as")
        .to_string()
}

/// `settings["react-x"].version`, resolving `detect` like upstream: the
/// version of `react` installed next to the root, else 19.3.0.
pub fn react_version(ctx: &Ctx<'_>) -> String {
    let configured = react_x_settings(ctx)
        .and_then(|s| s.get("version"))
        .and_then(Value::as_str);
    match configured {
        Some(v) if !v.is_empty() && v != "detect" => v.to_string(),
        _ => detect_react_version(ctx.root),
    }
}

fn detect_react_version(root: &Path) -> String {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, String>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(v) = cache.lock().unwrap().get(root) {
        return v.clone();
    }
    let v = std::fs::read_to_string(root.join("node_modules/react/package.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|j| j.get("version").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_else(|| "19.3.0".to_string());
    cache.lock().unwrap().insert(root.to_path_buf(), v.clone());
    v
}

/// `compare(v, min, "<")` of compare-versions, for plain numeric versions.
pub fn version_lt(v: &str, min: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.trim_start_matches(['v', '=', '^', '~'])
            .split(['-', '+'])
            .next()
            .unwrap_or("")
            .split('.')
            .map(|p| {
                p.chars()
                    .take_while(char::is_ascii_digit)
                    .collect::<String>()
                    .parse()
                    .unwrap_or(0)
            })
            .collect()
    };
    let (a, b) = (parse(v), parse(min));
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (
            a.get(i).copied().unwrap_or(0),
            b.get(i).copied().unwrap_or(0),
        );
        if x != y {
            return x < y;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// JSX runtime (`getJsxConfig().jsx`), for jsx-no-key-after-spread
// ---------------------------------------------------------------------------

/// Whether the file is compiled with the automatic runtime (`react-jsx` /
/// `react-jsxdev`): `@jsxRuntime` annotations first, then the nearest
/// tsconfig.json / jsconfig.json (`compilerOptions.jsx`, following
/// `extends`), defaulting to `react-jsx` like upstream.
pub fn is_automatic_jsx_runtime(ctx: &Ctx<'_>) -> bool {
    if ctx.source.contains("@jsx") {
        let mut runtime: Option<&str> = None;
        for c in ctx.program.comments.iter() {
            let s = c.span;
            let text = &ctx.source[s.start as usize..s.end as usize];
            let mut rest = text;
            while let Some(i) = rest.find("@jsxRuntime") {
                let after = &rest[i + "@jsxRuntime".len()..];
                let trimmed = after.trim_start();
                if trimmed.len() < after.len() {
                    let word: &str = trimmed.split(char::is_whitespace).next().unwrap_or("");
                    // Comment closers are part of `\S+` only when attached.
                    if !word.is_empty() {
                        runtime = Some(word);
                    }
                    break;
                }
                rest = after;
            }
        }
        match runtime {
            Some(r) if r == "classic" || r.starts_with("classic*/") => return false,
            Some(r) if r == "automatic" || r.starts_with("automatic*/") => return true,
            _ => {}
        }
    }
    let dir = ctx.path.parent().unwrap_or(ctx.root);
    match tsconfig_jsx(dir) {
        Some(jsx) => matches!(
            jsx.to_ascii_lowercase().as_str(),
            "react-jsx" | "react-jsxdev"
        ),
        None => true,
    }
}

fn tsconfig_jsx(dir: &Path) -> Option<String> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, Option<String>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(v) = cache.lock().unwrap().get(dir) {
        return v.clone();
    }
    let mut found = None;
    let mut cur = Some(dir);
    while let Some(d) = cur {
        if d.file_name().is_some_and(|n| n == "node_modules") {
            break;
        }
        let ts = d.join("tsconfig.json");
        let js = d.join("jsconfig.json");
        if ts.is_file() {
            found = Some(ts);
            break;
        }
        if js.is_file() {
            found = Some(js);
            break;
        }
        cur = d.parent();
    }
    let v = found.and_then(|f| read_tsconfig_jsx(&f, 0));
    cache.lock().unwrap().insert(dir.to_path_buf(), v.clone());
    v
}

fn read_tsconfig_jsx(file: &Path, depth: u32) -> Option<String> {
    if depth > 16 {
        return None;
    }
    let json = read_jsonc(file)?;
    if let Some(jsx) = json
        .get("compilerOptions")
        .and_then(|c| c.get("jsx"))
        .and_then(Value::as_str)
    {
        return Some(jsx.to_string());
    }
    let dir = file.parent()?;
    let extends: Vec<String> = match json.get("extends") {
        Some(Value::String(s)) => vec![s.clone()],
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    };
    // Later entries override earlier ones.
    for spec in extends.iter().rev() {
        if let Some(path) = resolve_extends(dir, spec)
            && let Some(jsx) = read_tsconfig_jsx(&path, depth + 1)
        {
            return Some(jsx);
        }
    }
    None
}

fn resolve_extends(dir: &Path, spec: &str) -> Option<PathBuf> {
    let candidates = |base: PathBuf| -> Option<PathBuf> {
        let mut with_json = base.clone().into_os_string();
        with_json.push(".json");
        [
            base.clone(),
            PathBuf::from(with_json),
            base.join("tsconfig.json"),
        ]
        .into_iter()
        .find(|p| p.is_file())
    };
    if spec.starts_with("./") || spec.starts_with("../") || Path::new(spec).is_absolute() {
        return candidates(dir.join(spec));
    }
    let mut cur = Some(dir);
    while let Some(d) = cur {
        let base = d.join("node_modules").join(spec);
        if let Some(found) = candidates(base) {
            return Some(found);
        }
        cur = d.parent();
    }
    None
}

fn read_jsonc(file: &Path) -> Option<Value> {
    let raw = std::fs::read_to_string(file).ok()?;
    let mut stripped = String::new();
    std::io::Read::read_to_string(
        &mut json_comments::StripComments::new(raw.as_bytes()),
        &mut stripped,
    )
    .ok()?;
    serde_json::from_str(&strip_trailing_commas(&stripped)).ok()
}

fn strip_trailing_commas(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut in_str = false;
    let mut escape = false;
    for (i, ch) in s.char_indices() {
        if in_str {
            out.push(ch);
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_str = false;
            }
            continue;
        }
        if ch == '"' {
            in_str = true;
        } else if ch == ',' {
            let next = bytes[i + 1..].iter().find(|b| !b.is_ascii_whitespace());
            if matches!(next, Some(b'}' | b']')) {
                continue;
            }
        }
        out.push(ch);
    }
    out
}

// ---------------------------------------------------------------------------
// Imports (`createImportLookup` of `@eslint-react/var`)
// ---------------------------------------------------------------------------

pub struct ImportLookup {
    /// local name -> (kind is namespace, imported name)
    by_local: HashMap<String, (bool, String)>,
}

impl ImportLookup {
    pub fn new(program: &Program<'_>, source: &str, builtin_namespaces: &[&str]) -> Self {
        let mut by_local = HashMap::new();
        for ns in builtin_namespaces {
            by_local.insert((*ns).to_string(), (true, String::new()));
        }
        for stmt in &program.body {
            let Statement::ImportDeclaration(decl) = stmt else {
                continue;
            };
            let spec = decl.source.value.as_str().unwrap_or_default();
            if spec.split('/').next().unwrap_or("") != source {
                continue;
            }
            for s in decl.specifiers.iter().flatten() {
                match s {
                    ImportDeclarationSpecifier::ImportSpecifier(s) => {
                        let name = match &s.imported {
                            ModuleExportName::IdentifierName(n) => n.name.to_string(),
                            ModuleExportName::IdentifierReference(n) => n.name.to_string(),
                            ModuleExportName::StringLiteral(_) => continue,
                        };
                        by_local.insert(s.local.name.to_string(), (false, name));
                    }
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                        by_local.insert(s.local.name.to_string(), (false, "default".into()));
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => {
                        by_local.insert(s.local.name.to_string(), (true, String::new()));
                    }
                }
            }
        }
        Self { by_local }
    }

    pub fn has(&self, local: &str, name: &str) -> bool {
        self.by_local.get(local).is_some_and(|(_, n)| n == name)
    }

    pub fn has_namespace(&self, local: &str) -> bool {
        self.by_local
            .get(local)
            .is_some_and(|(ns, n)| *ns || n == "default")
    }
}

/// Shared shape of the react-dom call rules: `name(...)` imported from
/// react-dom, or `ns.name(...)` on a react-dom namespace/default import.
pub fn is_react_dom_call(
    sem: &Semantic<'_>,
    imports: &ImportLookup,
    call: &CallExpression<'_>,
    name: &str,
) -> bool {
    let callee = unwrap(sem, call.callee.node_id());
    match kind(sem, callee) {
        AstKind::IdentifierReference(r) => imports.has(r.name.as_str(), name),
        AstKind::StaticMemberExpression(m) => {
            m.property.name == name
                && matches!(kind(sem, skip_parens(sem, m.object.node_id())), AstKind::IdentifierReference(o) if imports.has_namespace(o.name.as_str()))
        }
        AstKind::ComputedMemberExpression(_) | AstKind::PrivateFieldExpression(_) => false,
        _ => false,
    }
}
