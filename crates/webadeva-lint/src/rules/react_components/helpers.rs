//! Ports of the @eslint-react (v5) core helpers these rules are built on:
//! `@eslint-react/ast` (unwrap, fully qualified names, callee names),
//! `@eslint-react/core` (API matchers, function ids, the function component,
//! hook and class component collectors, JSX-likeness) and
//! `@eslint-react/shared` (settings / React version).
//!
//! oxc's AST differs from ESTree in a few ways that matter here, all handled
//! below: parentheses are kept as `ParenthesizedExpression` nodes (ESTree has
//! none, so they are skipped when walking parents and unwrapped when looking
//! at children), member expressions are split into static / computed /
//! private variants, and default values of parameters live on
//! `FormalParameter` instead of an `AssignmentPattern`.

use std::collections::HashMap;
use std::sync::OnceLock;

use oxc_ast::AstKind;
use oxc_ast::ast::*;
use oxc_semantic::{AstNodes, NodeId, Semantic};
use oxc_span::{GetSpan, Span};
use serde_json::Value;

use crate::rule::Ctx;

// ---------------------------------------------------------------------------
// Parents (ESTree view)
// ---------------------------------------------------------------------------

/// ESTree parent of a node: skips `ParenthesizedExpression`. `None` at the
/// program.
pub fn parent(nodes: &AstNodes<'_>, id: NodeId) -> Option<NodeId> {
    let mut cur = id;
    loop {
        if matches!(nodes.kind(cur), AstKind::Program(_)) {
            return None;
        }
        let p = nodes.parent_id(cur);
        if p == cur {
            return None;
        }
        if matches!(nodes.kind(p), AstKind::ParenthesizedExpression(_)) {
            cur = p;
            continue;
        }
        return Some(p);
    }
}

/// ESTree ancestors (closest first), skipping parentheses, ending at the
/// program (included).
pub fn ancestors<'n, 'a>(nodes: &'n AstNodes<'a>, id: NodeId) -> impl Iterator<Item = NodeId> + 'n {
    nodes
        .ancestor_ids(id)
        .filter(move |p| !matches!(nodes.kind(*p), AstKind::ParenthesizedExpression(_)))
}

/// `Traverse.findParent(node, test, stop)`.
pub fn find_parent<'a>(
    nodes: &AstNodes<'a>,
    id: NodeId,
    mut test: impl FnMut(NodeId, AstKind<'a>) -> bool,
    mut stop: impl FnMut(NodeId, AstKind<'a>) -> bool,
) -> Option<NodeId> {
    for p in ancestors(nodes, id) {
        let k = nodes.kind(p);
        if stop(p, k) {
            return None;
        }
        if test(p, k) {
            return Some(p);
        }
        if matches!(k, AstKind::Program(_)) {
            return None;
        }
    }
    None
}

pub fn no_stop(_: NodeId, _: AstKind<'_>) -> bool {
    false
}

/// `Check.isTypeExpression` (as / <T> / ! / satisfies / instantiation).
pub fn is_type_expression(k: AstKind<'_>) -> bool {
    matches!(
        k,
        AstKind::TSAsExpression(_)
            | AstKind::TSTypeAssertion(_)
            | AstKind::TSNonNullExpression(_)
            | AstKind::TSSatisfiesExpression(_)
            | AstKind::TSInstantiationExpression(_)
    )
}

/// `Check.isTypeAssertionExpression` (no instantiation expressions).
pub fn is_type_assertion_expression(k: AstKind<'_>) -> bool {
    matches!(
        k,
        AstKind::TSAsExpression(_)
            | AstKind::TSTypeAssertion(_)
            | AstKind::TSNonNullExpression(_)
            | AstKind::TSSatisfiesExpression(_)
    )
}

/// Walks up through type expressions: returns `(child, parent)` where
/// `parent` is the first non-type-expression ancestor.
pub fn skip_type_parents(nodes: &AstNodes<'_>, id: NodeId) -> (NodeId, Option<NodeId>) {
    let mut child = id;
    let mut p = parent(nodes, id);
    while let Some(pid) = p {
        if is_type_expression(nodes.kind(pid)) {
            child = pid;
            p = parent(nodes, pid);
        } else {
            break;
        }
    }
    (child, p)
}

// ---------------------------------------------------------------------------
// Expressions
// ---------------------------------------------------------------------------

pub fn strip_parens<'b, 'a>(mut e: &'b Expression<'a>) -> &'b Expression<'a> {
    while let Expression::ParenthesizedExpression(p) = e {
        e = &p.expression;
    }
    e
}

/// Whether `e` (ignoring parentheses) is the node `id`.
pub fn is_node(e: &Expression<'_>, id: NodeId) -> bool {
    strip_parens(e).node_id() == id
}

/// Result of `Extract.unwrap`: the expression with TS wrappers, chain
/// expressions and parentheses removed.
#[derive(Clone, Copy)]
pub enum Un<'b, 'a> {
    Expr(&'b Expression<'a>),
    Member(&'b MemberExpression<'a>),
    Call(&'b CallExpression<'a>),
}

impl<'b, 'a> Un<'b, 'a> {
    pub fn span(self) -> Span {
        match self {
            Un::Expr(e) => e.span(),
            Un::Member(m) => m.span(),
            Un::Call(c) => c.span,
        }
    }
    pub fn node_id(self) -> NodeId {
        match self {
            Un::Expr(e) => e.node_id(),
            Un::Member(m) => m.node_id(),
            Un::Call(c) => c.node_id(),
        }
    }
    pub fn is_this(self) -> bool {
        matches!(self, Un::Expr(Expression::ThisExpression(_)))
    }
}

pub fn unwrap<'b, 'a>(mut e: &'b Expression<'a>) -> Un<'b, 'a> {
    loop {
        match e {
            Expression::ParenthesizedExpression(p) => e = &p.expression,
            Expression::TSAsExpression(x) => e = &x.expression,
            Expression::TSSatisfiesExpression(x) => e = &x.expression,
            Expression::TSTypeAssertion(x) => e = &x.expression,
            Expression::TSNonNullExpression(x) => e = &x.expression,
            Expression::TSInstantiationExpression(x) => e = &x.expression,
            Expression::ChainExpression(c) => match &c.expression {
                ChainElement::CallExpression(call) => return Un::Call(call),
                ChainElement::TSNonNullExpression(x) => e = &x.expression,
                other => return Un::Member(other.as_member_expression().expect("member")),
            },
            Expression::CallExpression(call) => return Un::Call(call),
            _ => {
                if let Some(m) = e.as_member_expression() {
                    return Un::Member(m);
                }
                return Un::Expr(e);
            }
        }
    }
}

/// `Extract.getFullyQualifiedName` of an unwrapped expression.
pub fn fqn(source: &str, u: Un<'_, '_>) -> String {
    let text = |s: Span| source[s.start as usize..s.end as usize].to_string();
    match u {
        Un::Expr(Expression::Identifier(id)) => id.name.to_string(),
        Un::Member(MemberExpression::StaticMemberExpression(m)) => {
            format!("{}.{}", fqn(source, unwrap(&m.object)), m.property.name)
        }
        Un::Member(MemberExpression::PrivateFieldExpression(m)) => {
            format!("{}.{}", fqn(source, unwrap(&m.object)), m.field.name)
        }
        other => text(other.span()),
    }
}

/// `isAPI(api)`: the fully qualified name is `api` or ends with `.api`.
pub fn is_api(source: &str, u: Un<'_, '_>, api: &str) -> bool {
    let last = api.rsplit('.').next().unwrap_or(api);
    match u {
        Un::Expr(Expression::Identifier(id)) => return id.name.as_str() == api,
        Un::Member(MemberExpression::StaticMemberExpression(m))
            if m.property.name.as_str() != last =>
        {
            return false;
        }
        Un::Member(MemberExpression::PrivateFieldExpression(m))
            if m.field.name.as_str() != last =>
        {
            return false;
        }
        _ => {}
    }
    let name = fqn(source, u);
    name == api
        || (name.len() > api.len()
            && name.ends_with(api)
            && name.as_bytes()[name.len() - api.len() - 1] == b'.')
}

/// `isAPICall(api)`.
pub fn is_api_call(source: &str, call: &CallExpression<'_>, api: &str) -> bool {
    is_api(source, unwrap(&call.callee), api)
}

/// `Extract.getCalleeName`.
pub fn callee_name<'b>(call: &'b CallExpression<'_>) -> Option<&'b str> {
    match unwrap(&call.callee) {
        Un::Expr(Expression::Identifier(id)) => Some(id.name.as_str()),
        Un::Member(MemberExpression::StaticMemberExpression(m)) => Some(m.property.name.as_str()),
        _ => None,
    }
}

/// The call expression of a node kind, if it is one.
pub fn as_call<'a>(k: AstKind<'a>) -> Option<&'a CallExpression<'a>> {
    match k {
        AstKind::CallExpression(c) => Some(c),
        _ => None,
    }
}

pub fn is_create_element_call(source: &str, call: &CallExpression<'_>) -> bool {
    is_api_call(source, call, "createElement")
}

/// ESTree `Check.isIdentifier(property, name?)` for a member's property:
/// a non-computed identifier or a computed bare identifier.
pub fn member_property_ident<'b>(m: &'b MemberExpression<'_>) -> Option<&'b str> {
    match m {
        MemberExpression::StaticMemberExpression(s) => Some(s.property.name.as_str()),
        MemberExpression::ComputedMemberExpression(c) => match strip_parens(&c.expression) {
            Expression::Identifier(id) => Some(id.name.as_str()),
            _ => None,
        },
        MemberExpression::PrivateFieldExpression(_) => None,
    }
}

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

/// An ESTree function (`FunctionDeclaration`, `FunctionExpression`,
/// `ArrowFunctionExpression`); bodiless TS declarations are excluded.
#[derive(Clone, Copy)]
pub enum Func<'a> {
    F(&'a Function<'a>),
    A(&'a ArrowFunctionExpression<'a>),
}

impl<'a> Func<'a> {
    pub fn node_id(self) -> NodeId {
        match self {
            Func::F(f) => f.node_id(),
            Func::A(a) => a.node_id(),
        }
    }
    pub fn span(self) -> Span {
        match self {
            Func::F(f) => f.span,
            Func::A(a) => a.span,
        }
    }
    pub fn is_async(self) -> bool {
        match self {
            Func::F(f) => f.r#async,
            Func::A(a) => a.r#async,
        }
    }
    pub fn is_arrow(self) -> bool {
        matches!(self, Func::A(_))
    }
    /// ESTree `FunctionExpression` (includes method values).
    pub fn is_function_expression(self) -> bool {
        matches!(self, Func::F(f) if f.r#type == FunctionType::FunctionExpression)
    }
    /// Block body (`None` for an arrow with an expression body).
    pub fn block_body(self) -> Option<&'a FunctionBody<'a>> {
        match self {
            Func::F(f) => f.body.as_deref(),
            Func::A(a) => match &a.body {
                ArrowFunctionBody::FunctionBody(b) => Some(b),
                _ => None,
            },
        }
    }
    /// Expression body of an arrow.
    pub fn expression_body(self) -> Option<&'a Expression<'a>> {
        match self {
            Func::A(a) => a.body.as_expression(),
            Func::F(_) => None,
        }
    }
    /// Own `id` (named function declarations / expressions).
    pub fn own_id(self) -> Option<&'a BindingIdentifier<'a>> {
        match self {
            Func::F(f) => f.id.as_ref(),
            Func::A(_) => None,
        }
    }
    /// `getFunctionDirectives` values.
    pub fn directives(self) -> impl Iterator<Item = &'a str> {
        self.block_body()
            .into_iter()
            .flat_map(|b| b.directives.iter().map(|d| d.directive.as_str()))
    }
    /// `isFunctionEmpty`: a block body with no statements (directives count).
    pub fn is_empty(self) -> bool {
        self.block_body()
            .is_some_and(|b| b.directives.is_empty() && b.statements.is_empty())
    }
}

pub fn as_func<'a>(k: AstKind<'a>) -> Option<Func<'a>> {
    match k {
        AstKind::Function(f) if f.body.is_some() => Some(Func::F(f)),
        AstKind::ArrowFunctionExpression(a) => Some(Func::A(a)),
        _ => None,
    }
}

/// `Check.isFunctionOrClass`.
pub fn is_function_or_class(k: AstKind<'_>) -> bool {
    as_func(k).is_some() || matches!(k, AstKind::Class(_))
}

/// The node an ESTree "id" refers to (see `getFunctionId`).
#[derive(Clone, Copy)]
pub enum IdRef<'a> {
    Binding(&'a BindingIdentifier<'a>),
    /// A declarator / default-value pattern, with the declarator's type
    /// annotation (ESTree includes it in the id's range).
    Pattern(&'a BindingPattern<'a>, Option<Span>),
    Target(&'a AssignmentTarget<'a>),
    TargetIdent(&'a IdentifierReference<'a>),
    Key(&'a PropertyKey<'a>),
}

/// ESTree shape of an id, enough for the name checks upstream does.
pub enum IdShape<'b> {
    /// `Identifier` with its name.
    Ident(&'b str),
    /// `MemberExpression`: whether it is computed, the property name when the
    /// property is an `Identifier` (computed or not), and the name when it is
    /// non-computed and named (identifier or private name).
    Member {
        computed: bool,
        ident_prop: Option<&'b str>,
        named_prop: Option<&'b str>,
    },
    Other,
}

impl<'a> IdRef<'a> {
    pub fn span(self) -> Span {
        match self {
            IdRef::Binding(b) => b.span,
            IdRef::Pattern(p, ann) => {
                let s = p.span();
                match ann {
                    Some(a) if a.end > s.end => Span::new(s.start, a.end),
                    _ => s,
                }
            }
            IdRef::Target(t) => t.span(),
            IdRef::TargetIdent(i) => i.span,
            IdRef::Key(k) => k.span(),
        }
    }

    pub fn shape(self) -> IdShape<'a> {
        match self {
            IdRef::Binding(b) => IdShape::Ident(b.name.as_str()),
            IdRef::Pattern(BindingPattern::BindingIdentifier(b), _) => {
                IdShape::Ident(b.name.as_str())
            }
            IdRef::Pattern(..) => IdShape::Other,
            IdRef::TargetIdent(i) => IdShape::Ident(i.name.as_str()),
            IdRef::Target(t) => match t {
                AssignmentTarget::AssignmentTargetIdentifier(i) => IdShape::Ident(i.name.as_str()),
                _ => match t.as_member_expression() {
                    Some(m) => member_shape(m),
                    None => IdShape::Other,
                },
            },
            IdRef::Key(k) => match k {
                PropertyKey::StaticIdentifier(i) => IdShape::Ident(i.name.as_str()),
                _ => IdShape::Other,
            },
        }
    }

    /// `Extract.getFullyQualifiedName(id)`.
    pub fn fqn(self, source: &str) -> String {
        let text = |s: Span| source[s.start as usize..s.end as usize].to_string();
        match self {
            IdRef::Binding(b) => b.name.to_string(),
            IdRef::Pattern(BindingPattern::BindingIdentifier(b), _) => b.name.to_string(),
            IdRef::TargetIdent(i) => i.name.to_string(),
            IdRef::Target(AssignmentTarget::AssignmentTargetIdentifier(i)) => i.name.to_string(),
            IdRef::Target(t) => match t.as_member_expression() {
                Some(m) => fqn(source, Un::Member(m)),
                None => text(self.span()),
            },
            IdRef::Key(PropertyKey::StaticIdentifier(i)) => i.name.to_string(),
            IdRef::Key(PropertyKey::PrivateIdentifier(p)) => p.name.to_string(),
            _ => text(self.span()),
        }
    }
}

fn member_shape<'b>(m: &'b MemberExpression<'_>) -> IdShape<'b> {
    match m {
        MemberExpression::StaticMemberExpression(s) => IdShape::Member {
            computed: false,
            ident_prop: Some(s.property.name.as_str()),
            named_prop: Some(s.property.name.as_str()),
        },
        MemberExpression::PrivateFieldExpression(p) => IdShape::Member {
            computed: false,
            ident_prop: None,
            named_prop: Some(p.field.name.as_str()),
        },
        MemberExpression::ComputedMemberExpression(c) => IdShape::Member {
            computed: true,
            ident_prop: match strip_parens(&c.expression) {
                Expression::Identifier(i) => Some(i.name.as_str()),
                _ => None,
            },
            named_prop: None,
        },
    }
}

/// `getFunctionId(node)` (also used on call expressions).
pub fn get_function_id<'a>(nodes: &AstNodes<'a>, id: NodeId) -> Option<IdRef<'a>> {
    if let Some(f) = as_func(nodes.kind(id))
        && let Some(own) = f.own_id()
    {
        return Some(IdRef::Binding(own));
    }
    let p = parent(nodes, id)?;
    match nodes.kind(p) {
        AstKind::VariableDeclarator(d) if d.init.as_ref().is_some_and(|i| is_node(i, id)) => Some(
            IdRef::Pattern(&d.id, d.type_annotation.as_ref().map(|a| a.span)),
        ),
        AstKind::AssignmentExpression(a)
            if a.operator == AssignmentOperator::Assign && is_node(&a.right, id) =>
        {
            Some(IdRef::Target(&a.left))
        }
        AstKind::ObjectProperty(prop) if is_node(&prop.value, id) && !prop.computed => {
            Some(IdRef::Key(&prop.key))
        }
        AstKind::MethodDefinition(m)
            if m.r#type == MethodDefinitionType::MethodDefinition
                && m.value.node_id() == id
                && !m.computed =>
        {
            Some(IdRef::Key(&m.key))
        }
        AstKind::PropertyDefinition(pd)
            if pd.r#type == PropertyDefinitionType::PropertyDefinition
                && pd.value.as_ref().is_some_and(|v| is_node(v, id))
                && !pd.computed =>
        {
            Some(IdRef::Key(&pd.key))
        }
        AstKind::AssignmentPattern(ap) if is_node(&ap.right, id) => {
            Some(IdRef::Pattern(&ap.left, None))
        }
        AstKind::FormalParameter(fp) if fp.initializer.as_ref().is_some_and(|i| is_node(i, id)) => {
            Some(IdRef::Pattern(
                &fp.pattern,
                fp.type_annotation.as_ref().map(|a| a.span),
            ))
        }
        AstKind::AssignmentTargetWithDefault(t) if is_node(&t.init, id) => {
            Some(IdRef::Target(&t.binding))
        }
        AstKind::AssignmentTargetPropertyIdentifier(t)
            if t.init.as_ref().is_some_and(|i| is_node(i, id)) =>
        {
            Some(IdRef::TargetIdent(&t.binding))
        }
        AstKind::ConditionalExpression(c)
            if is_node(&c.consequent, id) || is_node(&c.alternate, id) =>
        {
            get_function_id(nodes, p)
        }
        k if is_type_assertion_expression(k) => get_function_id(nodes, p),
        _ => None,
    }
}

/// `isHookName`.
pub fn is_hook_name(name: &str) -> bool {
    name == "use"
        || (name.len() > 3
            && name.starts_with("use")
            && name.as_bytes()[3].is_ascii_uppercase() | name.as_bytes()[3].is_ascii_digit())
}

/// `isHookId`.
pub fn is_hook_id(id: IdRef<'_>) -> bool {
    match id.shape() {
        IdShape::Ident(n) => is_hook_name(n),
        IdShape::Member {
            computed: false,
            named_prop: Some(n),
            ..
        } => is_hook_name(n),
        _ => false,
    }
}

/// `isHookCall`.
pub fn is_hook_call(call: &CallExpression<'_>) -> bool {
    callee_name(call).is_some_and(is_hook_name)
}

/// `isHookTag`.
pub fn is_hook_tag(tag: &Expression<'_>) -> bool {
    match unwrap(tag) {
        Un::Expr(Expression::Identifier(i)) => is_hook_name(i.name.as_str()),
        Un::Member(MemberExpression::StaticMemberExpression(s)) => {
            is_hook_name(s.property.name.as_str())
        }
        Un::Member(MemberExpression::PrivateFieldExpression(p)) => {
            is_hook_name(p.field.name.as_str())
        }
        _ => false,
    }
}

/// `RE_COMPONENT_NAME` (`/^[A-Z]/`).
pub fn is_component_name(name: &str) -> bool {
    name.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
}

/// `RE_COMPONENT_NAME_LOOSE` (`/^_?[A-Z]/`).
pub fn is_component_name_loose(name: &str) -> bool {
    is_component_name(name.strip_prefix('_').unwrap_or(name))
}

/// `isFunctionComponentWrapperCall` (memo / forwardRef).
pub fn is_fc_wrapper_call(source: &str, call: &CallExpression<'_>) -> bool {
    is_api_call(source, call, "memo") || is_api_call(source, call, "forwardRef")
}

/// `getFunctionComponentId`.
pub fn get_function_component_id<'a>(
    nodes: &AstNodes<'a>,
    source: &str,
    id: NodeId,
) -> Option<IdRef<'a>> {
    if let Some(fid) = get_function_id(nodes, id) {
        return Some(fid);
    }
    let (_, p) = skip_type_parents(nodes, id);
    let p = p?;
    let call = as_call(nodes.kind(p))?;
    if !is_fc_wrapper_call(source, call) {
        return None;
    }
    let pp = parent(nodes, p)?;
    match nodes.kind(pp) {
        AstKind::VariableDeclarator(d) => Some(IdRef::Pattern(
            &d.id,
            d.type_annotation.as_ref().map(|a| a.span),
        )),
        AstKind::CallExpression(outer) if is_fc_wrapper_call(source, outer) => {
            match nodes.kind(parent(nodes, pp)?) {
                AstKind::VariableDeclarator(d) => Some(IdRef::Pattern(
                    &d.id,
                    d.type_annotation.as_ref().map(|a| a.span),
                )),
                _ => None,
            }
        }
        _ => None,
    }
}

/// `isFunctionWithLooseComponentName`.
pub fn has_loose_component_name(
    nodes: &AstNodes<'_>,
    source: &str,
    id: NodeId,
    allow_none: bool,
) -> bool {
    match get_function_component_id(nodes, source, id) {
        None => allow_none,
        Some(fid) => match fid.shape() {
            IdShape::Ident(n) => is_component_name_loose(n),
            IdShape::Member {
                ident_prop: Some(n),
                ..
            } => is_component_name_loose(n),
            _ => false,
        },
    }
}

// ---------------------------------------------------------------------------
// Classes
// ---------------------------------------------------------------------------

/// `getStaticKeyName` for a member expression used as a superclass.
fn static_member_name<'b>(m: &'b MemberExpression<'_>) -> Option<&'b str> {
    match m {
        MemberExpression::StaticMemberExpression(s) => Some(s.property.name.as_str()),
        MemberExpression::ComputedMemberExpression(c) => match &c.expression {
            Expression::StringLiteral(s) => s.value.as_str(),
            _ => None,
        },
        MemberExpression::PrivateFieldExpression(_) => None,
    }
}

fn super_class_name<'b>(class: &'b Class<'_>) -> Option<&'b str> {
    let sup = strip_parens(&class.heritage.as_ref()?.expression);
    match sup {
        Expression::Identifier(i) => Some(i.name.as_str()),
        _ => static_member_name(sup.as_member_expression()?),
    }
}

/// `isClassComponent`: extends `Component` / `PureComponent` (bare or as a
/// member).
pub fn is_class_component(class: &Class<'_>) -> bool {
    matches!(super_class_name(class), Some("Component" | "PureComponent"))
}

pub fn is_pure_component(class: &Class<'_>) -> bool {
    super_class_name(class) == Some("PureComponent")
}

pub fn kind_is_class_component(k: AstKind<'_>) -> bool {
    matches!(k, AstKind::Class(c) if is_class_component(c))
}

/// `getClassId` as text (ESTree `getText(id)`), when there is one.
pub fn class_id_text<'a>(
    nodes: &AstNodes<'a>,
    source: &'a str,
    class: &'a Class<'a>,
) -> Option<&'a str> {
    let span = if let Some(id) = &class.id {
        id.span
    } else {
        match nodes.kind(parent(nodes, class.node_id())?) {
            AstKind::VariableDeclarator(d) => {
                IdRef::Pattern(&d.id, d.type_annotation.as_ref().map(|a| a.span)).span()
            }
            _ => return None,
        }
    };
    Some(&source[span.start as usize..span.end as usize])
}

/// Static key name of a class member key (`getStaticKeyName`).
pub fn static_key_name<'b>(key: &'b PropertyKey<'_>, computed: bool) -> Option<&'b str> {
    match key {
        PropertyKey::StaticIdentifier(i) if !computed => Some(i.name.as_str()),
        PropertyKey::StringLiteral(s) => s.value.as_str(),
        _ => None,
    }
}

/// A class member that ESTree calls `MethodDefinition` / `PropertyDefinition`.
#[derive(Clone, Copy)]
pub struct Member<'a> {
    pub span: Span,
    pub key: &'a PropertyKey<'a>,
    pub computed: bool,
    pub is_static: bool,
    pub is_method: bool,
    pub accessibility: Option<TSAccessibility>,
    /// Method function / property value.
    pub value: Option<&'a Expression<'a>>,
    pub method_fn: Option<&'a Function<'a>>,
}

pub fn as_member<'a>(k: AstKind<'a>) -> Option<Member<'a>> {
    match k {
        AstKind::MethodDefinition(m) if m.r#type == MethodDefinitionType::MethodDefinition => {
            Some(Member {
                span: m.span,
                key: &m.key,
                computed: m.computed,
                is_static: m.r#static,
                is_method: true,
                accessibility: m.accessibility,
                value: None,
                method_fn: Some(&m.value),
            })
        }
        AstKind::PropertyDefinition(p)
            if p.r#type == PropertyDefinitionType::PropertyDefinition =>
        {
            Some(Member {
                span: p.span,
                key: &p.key,
                computed: p.computed,
                is_static: p.r#static,
                is_method: false,
                accessibility: p.accessibility,
                value: p.value.as_ref(),
                method_fn: None,
            })
        }
        _ => None,
    }
}

/// `createLifecycleChecker(name, isStatic)`.
pub fn is_lifecycle(k: AstKind<'_>, name: &str, is_static: bool) -> bool {
    as_member(k).is_some_and(|m| {
        m.is_static == is_static && !m.computed && static_key_name(m.key, m.computed) == Some(name)
    })
}

/// `isRenderMethodLike`: a class member whose (identifier) key starts with
/// `render`.
pub fn is_render_method_like(nodes: &AstNodes<'_>, id: NodeId) -> bool {
    let Some(m) = as_member(nodes.kind(id)) else {
        return false;
    };
    let name = match m.key {
        PropertyKey::StaticIdentifier(i) => i.name.as_str(),
        PropertyKey::Identifier(i) => i.name.as_str(),
        _ => return false,
    };
    name.starts_with("render") && class_of_member(nodes, id).is_some()
}

/// The class that owns a member node (member -> ClassBody -> Class).
pub fn class_of_member<'a>(nodes: &AstNodes<'a>, member: NodeId) -> Option<&'a Class<'a>> {
    let body = parent(nodes, member)?;
    match nodes.kind(parent(nodes, body)?) {
        AstKind::Class(c) => Some(c),
        _ => None,
    }
}

/// `isRenderMethodCallback`.
pub fn is_render_method_callback(nodes: &AstNodes<'_>, id: NodeId) -> bool {
    let (_, Some(p)) = skip_type_parents(nodes, id) else {
        return false;
    };
    is_render_method_like(nodes, p) && class_of_member(nodes, p).is_some_and(is_class_component)
}

/// `isThisSetStateCall`.
pub fn is_this_set_state_call(call: &CallExpression<'_>) -> bool {
    match unwrap(&call.callee) {
        Un::Member(m) => unwrap(m.object()).is_this() && callee_name(call) == Some("setState"),
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Variables
// ---------------------------------------------------------------------------

pub enum Resolved<'a> {
    /// No variable (unresolved reference / global).
    NoVariable,
    /// A variable whose definition is not something `resolve` returns.
    Nothing,
    Expr(&'a Expression<'a>),
    Func(Func<'a>),
    Class,
}

/// `resolve(context, identifier)` from `@eslint-react/var`.
pub fn resolve<'a>(sem: &Semantic<'a>, id: &IdentifierReference<'_>) -> Resolved<'a> {
    let Some(rid) = id.reference_id.get() else {
        return Resolved::NoVariable;
    };
    let Some(sym) = sem.scoping().get_reference(rid).symbol_id() else {
        return Resolved::NoVariable;
    };
    resolve_symbol(sem, sym)
}

pub fn resolve_symbol<'a>(sem: &Semantic<'a>, sym: oxc_semantic::SymbolId) -> Resolved<'a> {
    let decl = sem.scoping().symbol_declaration(sym);
    match sem.nodes().kind(decl) {
        AstKind::VariableDeclarator(d) => match (&d.id, &d.init) {
            (BindingPattern::BindingIdentifier(b), Some(init))
                if b.symbol_id.get() == Some(sym) =>
            {
                Resolved::Expr(init)
            }
            _ => Resolved::Nothing,
        },
        AstKind::Function(f) if f.body.is_some() => Resolved::Func(Func::F(f)),
        AstKind::Class(_) => Resolved::Class,
        _ => Resolved::Nothing,
    }
}

// ---------------------------------------------------------------------------
// JSX detection
// ---------------------------------------------------------------------------

pub mod hint {
    pub const NULL: u32 = 1 << 0;
    pub const NUMBER: u32 = 1 << 1;
    pub const BIGINT: u32 = 1 << 2;
    pub const STRING: u32 = 1 << 3;
    pub const BOOLEAN: u32 = 1 << 4;
    pub const UNDEFINED: u32 = 1 << 5;
    pub const EMPTY_ARRAY: u32 = 1 << 6;
    pub const CREATE_ELEMENT: u32 = 1 << 7;
    pub const ALL_ARRAY: u32 = 1 << 8;
    pub const BOTH_LOGICAL: u32 = 1 << 9;
    pub const BOTH_CONDITIONAL: u32 = 1 << 10;
    pub const CLASS_METHOD: u32 = 1 << 11;
    pub const CLASS_PROPERTY: u32 = 1 << 12;
    pub const OBJECT_METHOD: u32 = 1 << 13;
    pub const ARRAY_EXPRESSION_ELEMENT: u32 = 1 << 14;
    pub const ARRAY_PATTERN_ELEMENT: u32 = 1 << 15;
    pub const FLAT_MAP_CALLBACK: u32 = 1 << 16;
    pub const MAP_CALLBACK: u32 = 1 << 17;
    pub const ARBITRARY_CALLBACK: u32 = 1 << 18;

    /// `DEFAULT_COMPONENT_DETECTION_HINT`.
    pub const DEFAULT_COMPONENT: u32 = BIGINT
        | BOOLEAN
        | NUMBER
        | STRING
        | UNDEFINED
        | ARBITRARY_CALLBACK
        | ARRAY_EXPRESSION_ELEMENT
        | FLAT_MAP_CALLBACK
        | MAP_CALLBACK
        | ARRAY_PATTERN_ELEMENT
        | ALL_ARRAY
        | BOTH_CONDITIONAL
        | BOTH_LOGICAL;
}

/// `isJsxLike(context, node, hint)`.
pub fn is_jsx_like(sem: &Semantic<'_>, source: &str, e: Option<&Expression<'_>>, h: u32) -> bool {
    let mut seen = Vec::new();
    jsx_visit(sem, source, e, h, &mut seen)
}

fn jsx_visit(
    sem: &Semantic<'_>,
    source: &str,
    e: Option<&Expression<'_>>,
    h: u32,
    seen: &mut Vec<NodeId>,
) -> bool {
    let Some(e) = e else { return false };
    match e {
        Expression::JSXElement(_) | Expression::JSXFragment(_) => true,
        Expression::ParenthesizedExpression(p) => {
            jsx_visit(sem, source, Some(&p.expression), h, seen)
        }
        Expression::TSAsExpression(x) => jsx_visit(sem, source, Some(&x.expression), h, seen),
        Expression::TSSatisfiesExpression(x) => {
            jsx_visit(sem, source, Some(&x.expression), h, seen)
        }
        Expression::TSTypeAssertion(x) => jsx_visit(sem, source, Some(&x.expression), h, seen),
        Expression::TSNonNullExpression(x) => jsx_visit(sem, source, Some(&x.expression), h, seen),
        Expression::AwaitExpression(x) => jsx_visit(sem, source, Some(&x.argument), h, seen),
        Expression::NullLiteral(_) => h & hint::NULL == 0,
        Expression::BooleanLiteral(_) => h & hint::BOOLEAN == 0,
        Expression::StringLiteral(_) => h & hint::STRING == 0,
        Expression::NumericLiteral(_) => h & hint::NUMBER == 0,
        Expression::BigIntLiteral(_) => h & hint::BIGINT == 0,
        Expression::RegExpLiteral(_) => false,
        Expression::TemplateLiteral(_) => h & hint::STRING == 0,
        Expression::ArrayExpression(a) => {
            if a.elements.is_empty() {
                return h & hint::EMPTY_ARRAY == 0;
            }
            let mut it = a.elements.iter().map(|el| el.as_expression());
            if h & hint::ALL_ARRAY != 0 {
                it.all(|x| jsx_visit(sem, source, x, h, seen))
            } else {
                it.any(|x| jsx_visit(sem, source, x, h, seen))
            }
        }
        Expression::LogicalExpression(l) => {
            if h & hint::BOTH_LOGICAL != 0 {
                jsx_visit(sem, source, Some(&l.left), h, seen)
                    && jsx_visit(sem, source, Some(&l.right), h, seen)
            } else {
                jsx_visit(sem, source, Some(&l.left), h, seen)
                    || jsx_visit(sem, source, Some(&l.right), h, seen)
            }
        }
        Expression::ConditionalExpression(c) => {
            if h & hint::BOTH_CONDITIONAL != 0 {
                jsx_visit(sem, source, Some(&c.consequent), h, seen)
                    && jsx_visit(sem, source, Some(&c.alternate), h, seen)
            } else {
                jsx_visit(sem, source, Some(&c.consequent), h, seen)
                    || jsx_visit(sem, source, Some(&c.alternate), h, seen)
            }
        }
        Expression::SequenceExpression(s) => jsx_visit(sem, source, s.expressions.last(), h, seen),
        Expression::CallExpression(c) => {
            if h & hint::CREATE_ELEMENT != 0 {
                return false;
            }
            is_create_element_call(source, c)
        }
        Expression::Identifier(id) => {
            let resolved = resolve(sem, id);
            if id.name.as_str() == "undefined" && matches!(resolved, Resolved::NoVariable) {
                return h & hint::UNDEFINED == 0;
            }
            let nid = id.node_id();
            if seen.contains(&nid) {
                return false;
            }
            seen.push(nid);
            let r = match resolved {
                Resolved::Expr(init) => jsx_visit(sem, source, Some(init), h, seen),
                _ => false,
            };
            seen.retain(|x| *x != nid);
            r
        }
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Collectors
// ---------------------------------------------------------------------------

/// `isFunctionComponentDefinition(context, node, hint)`.
pub fn is_function_component_definition(ctx: &Ctx<'_>, func: Func<'_>, h: u32) -> bool {
    let nodes = ctx.semantic.nodes();
    let src = ctx.source;
    let id = func.node_id();
    if !has_loose_component_name(nodes, src, id, true) {
        return false;
    }
    let (child, parent_id) = skip_type_parents(nodes, id);
    let Some(parent_id) = parent_id else {
        return true;
    };
    if is_render_method_callback(nodes, id) {
        return false;
    }
    let pk = nodes.kind(parent_id);
    // isCreateElementChildrenArgument
    if let AstKind::CallExpression(call) = pk
        && is_create_element_call(src, call)
        && call.arguments.iter().skip(2).any(|a| {
            a.as_expression()
                .is_some_and(|e| unwrap(e).node_id() == id || e.node_id() == child)
        })
    {
        return false;
    }
    let is_fn_expr = func.is_arrow() || func.is_function_expression();
    let (parent_callee_member, parent_callee_name) = match pk {
        AstKind::CallExpression(c) => (matches!(unwrap(&c.callee), Un::Member(_)), callee_name(c)),
        _ => (false, None),
    };
    let object_prop = is_fn_expr
        && matches!(pk, AstKind::ObjectProperty(_))
        && parent(nodes, parent_id)
            .is_some_and(|pp| matches!(nodes.kind(pp), AstKind::ObjectExpression(_)));
    if object_prop {
        if h & hint::OBJECT_METHOD != 0 {
            return false;
        }
    } else if is_fn_expr && matches!(pk, AstKind::MethodDefinition(_)) {
        if h & hint::CLASS_METHOD != 0 {
            return false;
        }
    } else if is_fn_expr && matches!(pk, AstKind::PropertyDefinition(_)) {
        if h & hint::CLASS_PROPERTY != 0 {
            return false;
        }
    } else if matches!(pk, AstKind::ArrayPattern(_)) {
        if h & hint::ARRAY_PATTERN_ELEMENT != 0 {
            return false;
        }
    } else if matches!(pk, AstKind::ArrayExpression(_)) {
        if h & hint::ARRAY_EXPRESSION_ELEMENT != 0 {
            return false;
        }
    } else if parent_callee_member && parent_callee_name == Some("map") {
        if h & hint::MAP_CALLBACK != 0 {
            return false;
        }
    } else if parent_callee_member && parent_callee_name == Some("flatMap") {
        if h & hint::FLAT_MAP_CALLBACK != 0 {
            return false;
        }
    } else if let AstKind::CallExpression(c) = pk
        && get_function_id(nodes, id).is_none()
        && !is_fc_wrapper_call(src, c)
        && !is_create_element_call(src, c)
        && h & hint::ARBITRARY_CALLBACK != 0
    {
        return false;
    }
    let significant = find_parent(
        nodes,
        id,
        |_, k| {
            matches!(
                k,
                AstKind::JSXExpressionContainer(_)
                    | AstKind::ArrowFunctionExpression(_)
                    | AstKind::ObjectProperty(_)
                    | AstKind::BindingProperty(_)
                    | AstKind::AssignmentTargetPropertyIdentifier(_)
                    | AstKind::AssignmentTargetPropertyProperty(_)
                    | AstKind::ClassBody(_)
            ) || matches!(k, AstKind::Function(f) if f.r#type == FunctionType::FunctionExpression && f.body.is_some())
        },
        no_stop,
    );
    match significant {
        None => true,
        Some(s) => !matches!(nodes.kind(s), AstKind::JSXExpressionContainer(_)),
    }
}

pub struct FnEntry<'a> {
    pub func: Func<'a>,
    pub name: Option<String>,
    pub is_fc_def: bool,
    pub hook_calls: usize,
    pub is_component: bool,
}

pub struct HookEntry<'a> {
    pub func: Func<'a>,
    /// `getFunctionId`.
    pub id: Option<IdRef<'a>>,
    pub name: Option<String>,
    pub hook_calls: usize,
}

pub struct ClassEntry<'a> {
    pub class: &'a Class<'a>,
}

#[derive(Default)]
pub struct Collected<'a> {
    pub components: Vec<FnEntry<'a>>,
    pub hooks: Vec<HookEntry<'a>>,
    pub classes: Vec<ClassEntry<'a>>,
}

impl<'a> Collected<'a> {
    pub fn component_ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.components.iter().map(|c| c.func.node_id())
    }
    pub fn hook_ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.hooks.iter().map(|c| c.func.node_id())
    }
}

/// What to collect.
#[derive(Clone, Copy)]
pub struct Want {
    pub components: Option<u32>,
    pub hooks: bool,
    pub classes: bool,
}

fn nearest_function(nodes: &AstNodes<'_>, id: NodeId) -> Option<NodeId> {
    nodes
        .ancestor_ids(id)
        .find(|p| as_func(nodes.kind(*p)).is_some())
}

/// Runs `getFunctionComponentCollector` (with `hint`), `getHookCollector`
/// and `getClassComponentCollector` over the program.
pub fn collect<'a>(ctx: &Ctx<'a>, want: Want) -> Collected<'a> {
    let sem = ctx.semantic;
    let nodes = sem.nodes();
    let src = ctx.source;
    let mut fn_entries: Vec<FnEntry<'a>> = Vec::new();
    let mut hook_entries: Vec<(HookEntry<'a>, bool)> = Vec::new();
    let mut fn_index: HashMap<NodeId, usize> = HashMap::new();
    let mut classes = Vec::new();
    let h = want.components.unwrap_or(0);

    for node in nodes.iter() {
        let k = node.kind();
        if let Some(func) = as_func(k) {
            let nid = func.node_id();
            fn_index.insert(nid, fn_entries.len().max(hook_entries.len()));
            if want.components.is_some() {
                let id = get_function_component_id(nodes, src, nid);
                let is_fc_def = is_function_component_definition(ctx, func, h);
                let mut is_component = false;
                if is_fc_def
                    && has_loose_component_name(nodes, src, nid, false)
                    && func
                        .directives()
                        .any(|d| d == "use memo" || d == "use no memo")
                {
                    is_component = true;
                }
                if is_fc_def
                    && !is_component
                    && let Some(body) = func.expression_body()
                {
                    is_component = is_jsx_like(sem, src, Some(body), h);
                }
                fn_entries.push(FnEntry {
                    func,
                    name: id.map(|i| i.fqn(src)),
                    is_fc_def,
                    hook_calls: 0,
                    is_component,
                });
            }
            if want.hooks {
                let id = get_function_id(nodes, nid);
                let is_hook = id.is_some_and(is_hook_id);
                hook_entries.push((
                    HookEntry {
                        func,
                        name: id.map(|i| i.fqn(src)),
                        id,
                        hook_calls: 0,
                    },
                    is_hook,
                ));
            }
            continue;
        }
        match k {
            AstKind::Class(c) if want.classes && is_class_component(c) => {
                classes.push(ClassEntry { class: c })
            }
            AstKind::CallExpression(call) if is_hook_call(call) => {
                record_hook_call(
                    nodes,
                    node.id(),
                    &fn_index,
                    &mut fn_entries,
                    &mut hook_entries,
                    want,
                );
            }
            AstKind::TaggedTemplateExpression(t) if is_hook_tag(&t.tag) => {
                record_hook_call(
                    nodes,
                    node.id(),
                    &fn_index,
                    &mut fn_entries,
                    &mut hook_entries,
                    want,
                );
            }
            AstKind::ReturnStatement(r) if want.components.is_some() => {
                if let Some(f) = nearest_function(nodes, node.id())
                    && let Some(&i) = fn_index.get(&f)
                {
                    let e = &mut fn_entries[i];
                    if e.is_fc_def
                        && !e.is_component
                        && is_jsx_like(sem, src, r.argument.as_ref(), h)
                    {
                        e.is_component = true;
                    }
                }
            }
            _ => {}
        }
    }

    Collected {
        components: fn_entries.into_iter().filter(|e| e.is_component).collect(),
        hooks: hook_entries
            .into_iter()
            .filter(|(_, is)| *is)
            .map(|(e, _)| e)
            .collect(),
        classes,
    }
}

fn record_hook_call(
    nodes: &AstNodes<'_>,
    id: NodeId,
    fn_index: &HashMap<NodeId, usize>,
    fn_entries: &mut [FnEntry<'_>],
    hook_entries: &mut [(HookEntry<'_>, bool)],
    want: Want,
) {
    if want.components.is_some()
        && let Some(f) = nearest_function(nodes, id)
        && let Some(&i) = fn_index.get(&f)
    {
        let e = &mut fn_entries[i];
        e.hook_calls += 1;
        if e.is_fc_def {
            e.is_component = true;
        }
    }
    if want.hooks {
        for p in nodes.ancestor_ids(id) {
            if as_func(nodes.kind(p)).is_some()
                && let Some(&i) = fn_index.get(&p)
                && hook_entries[i].1
            {
                hook_entries[i].0.hook_calls += 1;
                break;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

/// `settings["react-x"]` when it is an object (upstream falls back to the
/// defaults when the settings object fails its schema).
fn react_x_settings(settings: &Value) -> Option<&serde_json::Map<String, Value>> {
    let obj = settings.get("react-x")?.as_object()?;
    // Schema: every known key must have the right type.
    for key in [
        "importSource",
        "polymorphicPropName",
        "version",
        "additionalRefHooks",
        "additionalStateHooks",
        "additionalEffectHooks",
    ] {
        if obj.get(key).is_some_and(|v| !v.is_string()) {
            return None;
        }
    }
    if let Some(m) = obj.get("compilationMode")
        && !matches!(m.as_str(), Some("infer" | "annotation" | "syntax" | "all"))
    {
        return None;
    }
    Some(obj)
}

/// React version as `@eslint-react/shared` normalizes it: the
/// `settings["react-x"].version` string, or, for absent / `""` /
/// `"detect"`, the version of the `react` package resolved from the working
/// directory, falling back to `19.3.0`. (`settings.react.version`, which is
/// eslint-plugin-react's setting, is not read by @eslint-react.)
pub fn react_version(ctx: &Ctx<'_>) -> String {
    let configured = react_x_settings(ctx.settings)
        .and_then(|o| o.get("version"))
        .and_then(Value::as_str)
        .filter(|v| !v.is_empty() && *v != "detect");
    if let Some(v) = configured {
        return v.to_string();
    }
    static DETECTED: OnceLock<String> = OnceLock::new();
    DETECTED
        .get_or_init(|| detect_react_version(ctx.root).unwrap_or_else(|| "19.3.0".to_string()))
        .clone()
}

fn detect_react_version(root: &std::path::Path) -> Option<String> {
    let mut dir = Some(root);
    while let Some(d) = dir {
        let pkg = d.join("node_modules").join("react").join("package.json");
        if let Ok(text) = std::fs::read_to_string(&pkg) {
            let v: Value = serde_json::from_str(&text).ok()?;
            return v.get("version").and_then(Value::as_str).map(str::to_string);
        }
        dir = d.parent();
    }
    None
}

/// `compare(version, "19.0.0", "<")` from compare-versions.
pub fn version_lt(version: &str, other: &str) -> bool {
    cmp_versions(version, other) == std::cmp::Ordering::Less
}

fn cmp_versions(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    fn split(v: &str) -> (Vec<u64>, Option<String>) {
        let v = v.trim().trim_start_matches(['v', '=']);
        let v = v.split('+').next().unwrap_or("");
        let (core, pre) = match v.split_once('-') {
            Some((c, p)) => (c, Some(p.to_string())),
            None => (v, None),
        };
        let nums = core
            .split('.')
            .map(|p| p.parse::<u64>().unwrap_or(0))
            .collect();
        (nums, pre)
    }
    let (an, ap) = split(a);
    let (bn, bp) = split(b);
    for i in 0..an.len().max(bn.len()).max(3) {
        let x = an.get(i).copied().unwrap_or(0);
        let y = bn.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            Ordering::Equal => {}
            o => return o,
        }
    }
    match (ap, bp) {
        (None, None) => Ordering::Equal,
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (Some(x), Some(y)) => {
            let xs: Vec<&str> = x.split('.').collect();
            let ys: Vec<&str> = y.split('.').collect();
            for i in 0..xs.len().max(ys.len()) {
                match (xs.get(i), ys.get(i)) {
                    (None, Some(_)) => return Ordering::Less,
                    (Some(_), None) => return Ordering::Greater,
                    (Some(p), Some(q)) => {
                        let o = match (p.parse::<u64>(), q.parse::<u64>()) {
                            (Ok(m), Ok(n)) => m.cmp(&n),
                            _ => p.cmp(q),
                        };
                        if o != Ordering::Equal {
                            return o;
                        }
                    }
                    (None, None) => {}
                }
            }
            Ordering::Equal
        }
    }
}

/// `toRegExp(settings["react-x"][key])` for the `additional*Hooks`
/// settings. Plain strings match exactly; `/pattern/flags` strings are
/// regular expressions upstream, which this crate (no regex engine) does not
/// evaluate, so they match nothing.
pub fn additional_hooks_matcher(ctx: &Ctx<'_>, key: &str) -> Option<String> {
    let s = react_x_settings(ctx.settings)?.get(key)?.as_str()?;
    let is_regex = s.len() > 2 && s.starts_with('/') && s[1..].contains('/');
    if is_regex { None } else { Some(s.to_string()) }
}
