//! `prefer-arrow-functions/prefer-arrow-functions` from
//! eslint-plugin-prefer-arrow-functions@3.10 (dist/rule.js + dist/guard.js):
//! reports plain functions that could be arrow functions, and arrow bodies
//! in the configured `returnStyle`, with upstream's autofix (dist/writer.js).

use std::cell::OnceCell;
use std::collections::HashSet;

use oxc_ast::AstKind;
use oxc_ast::ast::{
    AccessorProperty, ArrowFunctionExpression, AssignmentTarget, BindingPattern, ClassElement,
    Expression, Function, FunctionType, JSXElementName, JSXMemberExpressionObject,
    MethodDefinitionKind, MethodDefinitionType, PropertyDefinition, PropertyDefinitionType,
    PropertyKey, PropertyKind, Statement, TSType,
};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::{AstNodes, NodeId, Semantic};
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;
use serde_json::Value;

use super::no_dupe_args::duplicate_params;
use super::util::{estree_parent, is_js_space, js_trim, next_token_start, prev_token_end};
use crate::fix::Edit;
use crate::rule::{Ctx, Rule};

pub struct PreferArrowFunctions;

const USE_ARROW_WHEN_FUNCTION: &str = "Prefer using arrow functions over plain functions";
const USE_ARROW_WHEN_SINGLE_RETURN: &str =
    "Prefer using arrow functions when the function contains only a return";
const USE_EXPLICIT: &str =
    "Prefer using explicit returns when the arrow function contain only a return";
const USE_IMPLICIT: &str =
    "Prefer using implicit returns when the arrow function contain only a return";

#[derive(Clone, Copy, PartialEq, Eq)]
enum AllowNamed {
    No,
    Yes,
    OnlyExpressions,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ReturnStyle {
    Explicit,
    Implicit,
    Unchanged,
}

struct Options {
    allowed_names: Vec<String>,
    allow_named_functions: AllowNamed,
    allow_object_properties: bool,
    class_properties_allowed: bool,
    disallow_prototype: bool,
    return_style: ReturnStyle,
    single_return_only: bool,
}

impl Options {
    fn parse(opt: &Value) -> Self {
        let b = |k: &str| opt.get(k).and_then(Value::as_bool).unwrap_or(false);
        Self {
            allowed_names: opt
                .get("allowedNames")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            allow_named_functions: match opt.get("allowNamedFunctions") {
                Some(Value::Bool(true)) => AllowNamed::Yes,
                Some(Value::String(s)) if s == "only-expressions" => AllowNamed::OnlyExpressions,
                _ => AllowNamed::No,
            },
            allow_object_properties: b("allowObjectProperties"),
            class_properties_allowed: b("classPropertiesAllowed"),
            disallow_prototype: b("disallowPrototype"),
            return_style: match opt.get("returnStyle").and_then(Value::as_str) {
                Some("explicit") => ReturnStyle::Explicit,
                Some("implicit") => ReturnStyle::Implicit,
                _ => ReturnStyle::Unchanged,
            },
            single_return_only: b("singleReturnOnly"),
        }
    }
}

/// A function the guard looks at.
#[derive(Clone, Copy)]
enum Fn<'a, 'b> {
    Func(&'b Function<'a>),
    Arrow(&'b ArrowFunctionExpression<'a>),
}

impl<'a, 'b> Fn<'a, 'b> {
    fn span(self) -> Span {
        match self {
            Fn::Func(f) => f.span,
            Fn::Arrow(a) => a.span,
        }
    }
    fn node_id(self) -> NodeId {
        match self {
            Fn::Func(f) => f.node_id.get(),
            Fn::Arrow(a) => a.node_id.get(),
        }
    }
    fn name(self) -> Option<&'a str> {
        match self {
            Fn::Func(f) => f.id.as_ref().map(|id| id.name.as_str()),
            Fn::Arrow(_) => None,
        }
    }
    fn is_expression(self) -> bool {
        matches!(self, Fn::Func(f) if f.r#type == FunctionType::FunctionExpression)
    }
}

/// JS `String(number)` for the integers and simple decimals keys use.
fn js_number(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e21 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

/// `getStaticKeyName` for a property key.
fn static_key_name(key: &PropertyKey<'_>, computed: bool) -> Option<String> {
    match key {
        PropertyKey::PrivateIdentifier(p) => Some(format!("#{}", p.name)),
        PropertyKey::StaticIdentifier(id) if !computed => Some(id.name.to_string()),
        PropertyKey::StaticIdentifier(_) => None,
        _ => key.as_expression().and_then(static_expr_name),
    }
}

/// `getStaticKeyName` for an expression key (`Literal`, plain template).
fn static_expr_name(e: &Expression<'_>) -> Option<String> {
    match e {
        Expression::StringLiteral(s) => Some(s.value.display().to_string()),
        Expression::NumericLiteral(n) => Some(js_number(n.value)),
        Expression::BigIntLiteral(b) => Some(b.value.to_string()),
        Expression::BooleanLiteral(b) => Some(b.value.to_string()),
        Expression::NullLiteral(_) => Some("null".into()),
        Expression::RegExpLiteral(r) => Some(r.regex.to_string()),
        Expression::TemplateLiteral(t) if t.expressions.is_empty() => t
            .quasis
            .first()
            .and_then(|q| q.value.cooked.as_ref())
            .map(|c| c.display().to_string()),
        _ => None,
    }
}

/// Member expression key name: `a.b`, `a['b']`, `a.#b`.
fn member_key_name<'b, 'a>(e: &'b Expression<'a>) -> Option<(String, &'b Expression<'a>)> {
    match e {
        Expression::StaticMemberExpression(m) => Some((m.property.name.to_string(), &m.object)),
        Expression::ComputedMemberExpression(m) => {
            static_expr_name(&m.expression).map(|n| (n, &m.object))
        }
        Expression::PrivateFieldExpression(m) => Some((format!("#{}", m.field.name), &m.object)),
        _ => None,
    }
}

/// ESTree range of a property key (computed keys lose their parentheses).
fn key_span(key: &PropertyKey<'_>) -> Span {
    match key {
        PropertyKey::StaticIdentifier(id) => id.span,
        PropertyKey::PrivateIdentifier(p) => p.span,
        _ => key
            .as_expression()
            .map_or_else(|| key.span(), |e| e.without_parentheses().span()),
    }
}

/// The rule's `propName` for a method or property being rewritten.
fn prop_name(source: &str, key: &PropertyKey<'_>, computed: bool) -> String {
    let span = key_span(key);
    let text = &source[span.start as usize..span.end as usize];
    match key {
        PropertyKey::PrivateIdentifier(p) => format!("#{}", p.name),
        _ if computed => format!("[{text}]"),
        PropertyKey::StaticIdentifier(id) => id.name.to_string(),
        _ => text.to_string(),
    }
}

struct Guard<'s, 'a> {
    ctx_source: &'a str,
    semantic: &'s Semantic<'a>,
    comments: &'s [oxc_ast::Comment],
    opts: &'s Options,
    constructor_usages: OnceCell<HashSet<String>>,
    super_member_names: OnceCell<HashSet<String>>,
    /// (outer range, inner ESTree child ranges) of `declare function`s.
    declare_functions: OnceCell<Vec<(Span, Vec<Span>)>>,
    /// The file is `.tsx` (the writer's generic trailing comma).
    is_tsx: bool,
}

impl<'s, 'a> Guard<'s, 'a> {
    fn nodes(&self) -> &'s AstNodes<'a> {
        self.semantic.nodes()
    }

    /// Source text without whitespace (`normalizeText`).
    fn normalize(&self, span: Span) -> String {
        self.ctx_source[span.start as usize..span.end as usize]
            .chars()
            .filter(|c| !is_js_space(*c))
            .collect()
    }

    fn eparent(&self, id: NodeId) -> NodeId {
        estree_parent(self.nodes(), id)
    }

    fn is_any_function(kind: &AstKind<'_>) -> bool {
        matches!(kind, AstKind::Function(f) if matches!(f.r#type, FunctionType::FunctionDeclaration | FunctionType::FunctionExpression))
            || matches!(kind, AstKind::ArrowFunctionExpression(_))
    }

    /// `getEnclosingFunction`
    fn enclosing_function(&self, id: NodeId) -> Option<NodeId> {
        self.nodes()
            .ancestor_ids(id)
            .find(|&a| Self::is_any_function(&self.nodes().kind(a)))
    }

    fn walk_all(&self, mut f: impl FnMut(&AstKind<'a>)) {
        for n in self.nodes().iter() {
            f(&n.kind());
        }
    }

    // ----- overloads -----

    fn declare_functions(&self) -> &Vec<(Span, Vec<Span>)> {
        self.declare_functions.get_or_init(|| {
            let mut out = Vec::new();
            for n in self.nodes().iter() {
                let AstKind::Function(f) = n.kind() else {
                    continue;
                };
                if f.r#type != FunctionType::TSDeclareFunction {
                    continue;
                }
                let mut outer = f.span;
                if let AstKind::ExportNamedDeclaration(_) | AstKind::ExportDeclaration(_) =
                    self.nodes().parent_kind(n.id())
                {
                    outer = self.nodes().parent_kind(n.id()).span();
                }
                let mut inner: Vec<Span> = Vec::new();
                if let Some(id) = &f.id {
                    inner.push(id.span);
                }
                if let Some(tp) = &f.type_parameters {
                    inner.push(tp.span);
                }
                if let Some(t) = &f.this_param {
                    inner.push(t.span);
                }
                inner.extend(f.params.items.iter().map(|p| p.span));
                if let Some(r) = &f.params.rest {
                    inner.push(r.span);
                }
                if let Some(r) = &f.return_type {
                    inner.push(r.span);
                }
                out.push((outer, inner));
            }
            out
        })
    }

    /// `isOverloadedFunction`
    fn is_overloaded(&self, f: Fn<'a, '_>) -> bool {
        if !self.declare_functions().is_empty() {
            let parent = self.nodes().parent_kind(f.node_id());
            let start = match parent {
                AstKind::ExportNamedDeclaration(_) | AstKind::ExportDeclaration(_) => {
                    parent.span().start
                }
                _ => f.span().start,
            };
            let end = prev_token_end(self.ctx_source, self.comments, start);
            if end > 0 {
                let p = end - 1;
                let hit = self.declare_functions().iter().any(|(outer, inner)| {
                    outer.start <= p
                        && p < outer.end
                        && !inner.iter().any(|s| s.start <= p && p < s.end)
                });
                if hit {
                    return true;
                }
            }
        }
        self.is_overloaded_class_method(f)
    }

    fn is_overloaded_class_method(&self, f: Fn<'a, '_>) -> bool {
        let Fn::Func(_) = f else { return false };
        let member_id = self.nodes().parent_id(f.node_id());
        let AstKind::MethodDefinition(method) = self.nodes().kind(member_id) else {
            return false;
        };
        let AstKind::ClassBody(body) = self.nodes().parent_kind(member_id) else {
            return false;
        };
        let Some(name) = static_key_name(&method.key, method.computed) else {
            return false;
        };
        body.body.iter().any(|m| match m {
            ClassElement::MethodDefinition(other) => {
                other.span != method.span
                    && other.r#type == MethodDefinitionType::MethodDefinition
                    && other.value.r#type == FunctionType::TSEmptyBodyFunctionExpression
                    && other.r#static == method.r#static
                    && static_key_name(&other.key, other.computed).as_deref() == Some(name.as_str())
            }
            _ => false,
        })
    }

    // ----- this / arguments / super / new.target -----

    fn own_bindings_would_change(&self, f: Fn<'a, '_>) -> bool {
        let Fn::Func(func) = f else { return false };
        if func.this_param.is_some() {
            return true;
        }
        let mut v = OwnScope {
            root: func.span,
            found: false,
        };
        v.visit_function(func, ScopeFlags::Function);
        v.found
    }

    // ----- constructors -----

    fn constructor_usages(&self) -> &HashSet<String> {
        self.constructor_usages.get_or_init(|| {
            let mut set = HashSet::new();
            self.walk_all(|k| match k {
                AstKind::NewExpression(n) => {
                    set.insert(self.normalize(n.callee.without_parentheses().span()));
                }
                AstKind::BinaryExpression(b)
                    if b.operator == oxc_syntax::operator::BinaryOperator::Instanceof =>
                {
                    set.insert(self.normalize(b.right.without_parentheses().span()));
                }
                AstKind::Class(c) => {
                    if let Some(h) = &c.heritage {
                        set.insert(self.normalize(h.expression.without_parentheses().span()));
                    }
                }
                AstKind::StaticMemberExpression(m) if m.property.name == "prototype" => {
                    set.insert(self.normalize(m.object.without_parentheses().span()));
                }
                _ => {}
            });
            set
        })
    }

    /// `getBindingText`
    fn binding_text(&self, id: NodeId) -> Option<String> {
        let kind = self.nodes().kind(id);
        if let AstKind::Function(f) = kind
            && f.r#type == FunctionType::FunctionDeclaration
            && let Some(name) = &f.id
        {
            return Some(name.name.to_string());
        }
        let span = kind.span();
        let parent = self.eparent(id);
        match self.nodes().kind(parent) {
            AstKind::VariableDeclarator(d)
                if d.init
                    .as_ref()
                    .is_some_and(|i| i.without_parentheses().span() == span) =>
            {
                match &d.id {
                    BindingPattern::BindingIdentifier(b) => Some(b.name.to_string()),
                    _ => None,
                }
            }
            AstKind::AssignmentExpression(a) if a.right.without_parentheses().span() == span => {
                match &a.left {
                    AssignmentTarget::AssignmentTargetIdentifier(_)
                    | AssignmentTarget::StaticMemberExpression(_)
                    | AssignmentTarget::ComputedMemberExpression(_)
                    | AssignmentTarget::PrivateFieldExpression(_) => {
                        Some(self.normalize(a.left.span()))
                    }
                    _ => None,
                }
            }
            AstKind::ObjectProperty(p) if p.value.without_parentheses().span() == span => {
                let key = static_key_name(&p.key, p.computed)?;
                let obj = self.nodes().parent_id(parent);
                let text = self.binding_text(obj)?;
                Some(format!("{text}.{key}"))
            }
            _ => None,
        }
    }

    fn is_used_as_constructor(&self, f: Fn<'a, '_>) -> bool {
        self.binding_text(f.node_id())
            .is_some_and(|t| self.constructor_usages().contains(&t))
    }

    fn is_constructed_value(&self, f: Fn<'a, '_>) -> bool {
        let span = f.span();
        match self.nodes().kind(self.eparent(f.node_id())) {
            AstKind::NewExpression(n) => n.callee.without_parentheses().span() == span,
            AstKind::Class(c) => c
                .heritage
                .as_ref()
                .is_some_and(|h| h.expression.without_parentheses().span() == span),
            _ => false,
        }
    }

    // ----- super members / class fields -----

    fn super_member_names(&self) -> &HashSet<String> {
        self.super_member_names.get_or_init(|| {
            let mut set = HashSet::new();
            self.walk_all(|k| {
                let name = match k {
                    AstKind::StaticMemberExpression(m)
                        if matches!(m.object, Expression::Super(_)) =>
                    {
                        Some(m.property.name.to_string())
                    }
                    AstKind::ComputedMemberExpression(m)
                        if matches!(m.object, Expression::Super(_)) =>
                    {
                        static_expr_name(&m.expression)
                    }
                    _ => None,
                };
                if let Some(n) = name {
                    set.insert(n);
                }
            });
            set
        })
    }

    /// `isUnsafeAsClassProperty` for a method definition node.
    fn is_unsafe_as_class_property(&self, member_id: NodeId) -> bool {
        let AstKind::MethodDefinition(method) = self.nodes().kind(member_id) else {
            return false;
        };
        let AstKind::ClassBody(body) = self.nodes().parent_kind(member_id) else {
            return false;
        };
        let Some(name) = static_key_name(&method.key, method.computed) else {
            return false;
        };
        if self.super_member_names().contains(&name) {
            return true;
        }
        for el in &body.body {
            if el.span() == method.span {
                break;
            }
            if let ClassElement::PropertyDefinition(p) = el
                && p.r#type == PropertyDefinitionType::PropertyDefinition
                && p.r#static == method.r#static
                && let Some(value) = &p.value
            {
                let mut v = ReadsOwnMember {
                    name: &name,
                    found: false,
                };
                v.visit_expression(value);
                if v.found {
                    return true;
                }
            }
        }
        false
    }

    // ----- names -----

    /// `getContextualName` (function expressions only).
    fn contextual_name(&self, f: Fn<'a, '_>) -> Option<String> {
        let span = f.span();
        match self.nodes().kind(self.eparent(f.node_id())) {
            AstKind::MethodDefinition(m) => static_key_name(&m.key, m.computed),
            AstKind::PropertyDefinition(m) => static_key_name(&m.key, m.computed),
            AstKind::AccessorProperty(m) => static_key_name(&m.key, m.computed),
            AstKind::ObjectProperty(p) if p.value.without_parentheses().span() == span => {
                static_key_name(&p.key, p.computed)
            }
            AstKind::VariableDeclarator(d) => match &d.id {
                BindingPattern::BindingIdentifier(b) => Some(b.name.to_string()),
                _ => None,
            },
            AstKind::AssignmentExpression(a) if a.right.without_parentheses().span() == span => {
                match &a.left {
                    AssignmentTarget::AssignmentTargetIdentifier(id) => Some(id.name.to_string()),
                    AssignmentTarget::StaticMemberExpression(m) => {
                        Some(m.property.name.to_string())
                    }
                    AssignmentTarget::ComputedMemberExpression(m) => {
                        static_expr_name(&m.expression)
                    }
                    AssignmentTarget::PrivateFieldExpression(m) => {
                        Some(format!("#{}", m.field.name))
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn is_ignored(&self, f: Fn<'a, '_>) -> bool {
        let names = [
            f.name().map(String::from),
            if f.is_expression() {
                self.contextual_name(f)
            } else {
                None
            },
        ];
        names
            .iter()
            .flatten()
            .any(|n| self.opts.allowed_names.contains(n))
    }

    fn is_self_referencing_expression(&self, f: Fn<'a, '_>) -> bool {
        let Fn::Func(func) = f else { return false };
        if func.r#type != FunctionType::FunctionExpression {
            return false;
        }
        let Some(sid) = func.id.as_ref().and_then(|id| id.symbol_id.get()) else {
            return false;
        };
        self.semantic
            .scoping()
            .get_resolved_reference_ids(sid)
            .iter()
            .next()
            .is_some()
    }

    // ----- declarations -----

    fn has_no_statement_list(&self, f: Fn<'a, '_>) -> bool {
        !matches!(
            self.nodes().parent_kind(f.node_id()),
            AstKind::Program(_)
                | AstKind::BlockStatement(_)
                | AstKind::FunctionBody(_)
                | AstKind::StaticBlock(_)
                | AstKind::SwitchCase(_)
                | AstKind::TSModuleBlock(_)
                | AstKind::ExportNamedDeclaration(_)
                | AstKind::ExportDeclaration(_)
                | AstKind::ExportDefaultDeclaration(_)
        )
    }

    fn is_called_eagerly_before(&self, text: &str, before: u32) -> bool {
        self.nodes().iter().any(|n| {
            matches!(n.kind(), AstKind::CallExpression(c)
                if c.span.start < before
                    && self.normalize(c.callee.without_parentheses().span()) == text
                    && self.enclosing_function(n.id()).is_none())
        })
    }

    fn is_eager_reference(&self, ref_node: NodeId, f: Fn<'a, '_>) -> bool {
        let Some(enclosing) = self.enclosing_function(ref_node) else {
            return true;
        };
        let parent = self.eparent(enclosing);
        if let AstKind::CallExpression(call) = self.nodes().kind(parent)
            && call.callee.without_parentheses().span() == self.nodes().kind(enclosing).span()
        {
            return call.span.start < f.span().start && self.enclosing_function(parent).is_none();
        }
        self.binding_text(enclosing)
            .is_some_and(|t| self.is_called_eagerly_before(&t, f.span().start))
    }

    /// `declarationBindingWouldBreak`
    fn declaration_binding_would_break(&self, f: Fn<'a, '_>) -> bool {
        let Fn::Func(func) = f else { return false };
        if func.r#type != FunctionType::FunctionDeclaration || func.id.is_none() {
            return false;
        }
        if self.has_no_statement_list(f) {
            return true;
        }
        let Some(sid) = func.id.as_ref().and_then(|id| id.symbol_id.get()) else {
            return false;
        };
        let scoping = self.semantic.scoping();
        let refs: Vec<_> = scoping.get_resolved_references(sid).collect();
        // isUsedBeforeDefined
        if refs.iter().any(|r| {
            self.nodes().kind(r.node_id()).span().start < func.span.start
                && self.is_eager_reference(r.node_id(), f)
        }) {
            return true;
        }
        // isIncompatibleWithConst
        if !scoping.symbol_redeclarations(sid).is_empty() {
            return true;
        }
        if refs.iter().any(|r| {
            if r.is_write() {
                return true;
            }
            let span = self.nodes().kind(r.node_id()).span();
            matches!(self.nodes().kind(self.eparent(r.node_id())), AstKind::NewExpression(n)
                if n.callee.without_parentheses().span() == span)
        }) {
            return true;
        }
        // isBlockLeakUsedOutside
        let scope = scoping.symbol_scope_id(sid);
        let flags = scoping.scope_flags(scope);
        if flags.is_top() || flags.is_function() {
            return false;
        }
        let name = scoping.symbol_name(sid);
        let uppers: Vec<_> = scoping.scope_ancestors(scope).skip(1).collect();
        // Every reference to the name, with the scope it resolves in.
        let mut all: Vec<(oxc_semantic::ScopeId, Option<oxc_semantic::ScopeId>)> = Vec::new();
        for other in scoping.symbol_ids() {
            if scoping.symbol_name(other) == name {
                let decl_scope = scoping.symbol_scope_id(other);
                for r in scoping.get_resolved_references(other) {
                    all.push((r.scope_id(), Some(decl_scope)));
                }
            }
        }
        if let Some(ids) = scoping.root_unresolved_references().get(name) {
            for &rid in ids {
                all.push((scoping.get_reference(rid).scope_id(), None));
            }
        }
        let within = |s: oxc_semantic::ScopeId, u: oxc_semantic::ScopeId| {
            scoping.scope_ancestors(s).any(|a| a == u)
        };
        uppers.iter().any(|&u| {
            all.iter().any(|&(ref_scope, decl)| {
                within(ref_scope, u) && decl.is_none_or(|d| !within(d, u))
            })
        })
    }

    // ----- prototype -----

    fn is_prototype_assignment(&self, f: Fn<'a, '_>) -> bool {
        self.nodes().ancestor_kinds(f.node_id()).any(|k| {
            let AstKind::AssignmentExpression(a) = k else { return false };
            let prop_is_prototype = |t: &AssignmentTarget<'_>| match t {
                AssignmentTarget::StaticMemberExpression(m) => m.property.name == "prototype",
                AssignmentTarget::ComputedMemberExpression(m) => {
                    matches!(&m.expression, Expression::Identifier(id) if id.name == "prototype")
                }
                AssignmentTarget::PrivateFieldExpression(m) => m.field.name == "prototype",
                _ => false,
            };
            let object_prop_is_prototype = |t: &AssignmentTarget<'_>| {
                let object = match t {
                    AssignmentTarget::StaticMemberExpression(m) => &m.object,
                    AssignmentTarget::ComputedMemberExpression(m) => &m.object,
                    AssignmentTarget::PrivateFieldExpression(m) => &m.object,
                    _ => return false,
                };
                match object.without_parentheses() {
                    Expression::StaticMemberExpression(m) => m.property.name == "prototype",
                    Expression::ComputedMemberExpression(m) => {
                        matches!(&m.expression, Expression::Identifier(id) if id.name == "prototype")
                    }
                    Expression::PrivateFieldExpression(m) => m.field.name == "prototype",
                    _ => false,
                }
            };
            prop_is_prototype(&a.left) || object_prop_is_prototype(&a.left)
        })
    }

    // ----- misc -----

    fn returns_immediately(f: Fn<'a, '_>) -> bool {
        match f {
            Fn::Arrow(a) => a.is_expression() || Self::is_single_return(a.get_function_body()),
            Fn::Func(func) => Self::is_single_return(func.body.as_deref()),
        }
    }

    /// `isBlockStatementWithSingleReturn`
    /// (ESTree keeps directives as statements of the body.)
    fn is_single_return(body: Option<&oxc_ast::ast::FunctionBody<'_>>) -> bool {
        body.is_some_and(|b| {
            b.directives.is_empty() && matches!(&b.statements[..], [Statement::ReturnStatement(r)] if r.argument.is_some())
        })
    }

    fn is_object_property(&self, f: Fn<'a, '_>) -> bool {
        matches!(self.nodes().kind(self.eparent(f.node_id())), AstKind::ObjectProperty(p)
            if p.value.without_parentheses().span() == f.span())
    }

    fn is_class_member(&self, f: Fn<'a, '_>) -> bool {
        let p = self.nodes().parent_id(f.node_id());
        matches!(
            self.nodes().kind(p),
            AstKind::MethodDefinition(_) | AstKind::PropertyDefinition(_)
        ) && matches!(self.nodes().parent_kind(p), AstKind::ClassBody(_))
    }

    fn has_name_and_is_exported_as_default(&self, f: Fn<'a, '_>) -> bool {
        f.name().is_some()
            && matches!(
                self.nodes().parent_kind(f.node_id()),
                AstKind::ExportDefaultDeclaration(_)
            )
    }

    /// `isSafeTransformation`
    fn is_safe(&self, f: Fn<'a, '_>) -> bool {
        if let Fn::Func(func) = f
            && func.generator
        {
            return false;
        }
        let return_type = match f {
            Fn::Func(func) => func.return_type.as_ref(),
            Fn::Arrow(a) => a.return_type.as_ref(),
        };
        if return_type
            .is_some_and(|r| matches!(&r.type_annotation, TSType::TSTypePredicate(p) if p.asserts))
        {
            return false;
        }
        if self.is_overloaded(f) || self.own_bindings_would_change(f) {
            return false;
        }
        if let Fn::Func(func) = f
            && !duplicate_params(&func.params).is_empty()
        {
            return false;
        }
        if self.is_constructed_value(f)
            || self.is_used_as_constructor(f)
            || self.is_self_referencing_expression(f)
            || self.declaration_binding_would_break(f)
            || self.is_ignored(f)
        {
            return false;
        }
        match self.opts.allow_named_functions {
            AllowNamed::Yes if f.name().is_some() => return false,
            AllowNamed::OnlyExpressions if f.name().is_some() && f.is_expression() => return false,
            _ => {}
        }
        if !self.opts.disallow_prototype && self.is_prototype_assignment(f) {
            return false;
        }
        if self.opts.single_return_only && !Self::returns_immediately(f) {
            return false;
        }
        if self.is_object_property(f) && self.opts.allow_object_properties {
            return false;
        }
        !self.has_name_and_is_exported_as_default(f)
    }

    fn message(&self, f: Fn<'a, '_>) -> &'static str {
        if self.opts.single_return_only && Self::returns_immediately(f) {
            USE_ARROW_WHEN_SINGLE_RETURN
        } else {
            USE_ARROW_WHEN_FUNCTION
        }
    }
}

/// The parts of a function the writer copies (ESTree ranges).
struct Parts<'b, 'a> {
    span: Span,
    is_arrow: bool,
    is_async: bool,
    /// `typeParameters` range and number of params.
    type_params: Option<(Span, usize)>,
    params: Vec<Span>,
    return_type: Option<Span>,
    body: Body<'b, 'a>,
}

enum Body<'b, 'a> {
    /// `{ ... }`, with the argument of its lone `return` when that is all it holds.
    Block {
        span: Span,
        single_return: Option<&'b Expression<'a>>,
    },
    /// Expression body: `outer` keeps its parentheses, `inner` is the ESTree node.
    Expr {
        outer: Span,
        inner: &'b Expression<'a>,
    },
}

/// Where `getSignatureBounds` puts the copied signature.
struct SignatureBounds {
    start: u32,
    close_paren_end: u32,
    end: u32,
}

fn single_return_argument<'b, 'a>(
    body: &'b oxc_ast::ast::FunctionBody<'a>,
) -> Option<&'b Expression<'a>> {
    if !body.directives.is_empty() {
        return None;
    }
    match &body.statements[..] {
        [Statement::ReturnStatement(r)] => r.argument.as_ref(),
        _ => None,
    }
}

fn param_spans(params: &oxc_ast::ast::FormalParameters<'_>) -> Vec<Span> {
    let mut v: Vec<Span> = params.items.iter().map(|p| p.span).collect();
    if let Some(r) = &params.rest {
        v.push(r.span);
    }
    v
}

impl<'b, 'a> Parts<'b, 'a> {
    fn of(f: Fn<'a, 'b>) -> Option<Self> {
        Some(match f {
            Fn::Func(func) => {
                let body = func.body.as_deref()?;
                Parts {
                    span: func.span,
                    is_arrow: false,
                    is_async: func.r#async,
                    type_params: func
                        .type_parameters
                        .as_ref()
                        .map(|t| (t.span, t.params.len())),
                    params: param_spans(&func.params),
                    return_type: func.return_type.as_ref().map(|r| r.span),
                    body: Body::Block {
                        span: body.span,
                        single_return: single_return_argument(body),
                    },
                }
            }
            Fn::Arrow(a) => Parts {
                span: a.span,
                is_arrow: true,
                is_async: a.r#async,
                type_params: a.type_parameters.as_ref().map(|t| (t.span, t.params.len())),
                params: param_spans(&a.params),
                return_type: a.return_type.as_ref().map(|r| r.span),
                body: match a.get_expression() {
                    Some(e) => Body::Expr {
                        outer: e.span(),
                        inner: e.without_parentheses(),
                    },
                    None => {
                        let body = a.get_function_body()?;
                        Body::Block {
                            span: body.span,
                            single_return: single_return_argument(body),
                        }
                    }
                },
            },
        })
    }
}

// ----- fixer (dist/writer.js) -----
impl<'s, 'a> Guard<'s, 'a> {
    fn text(&self, span: Span) -> &'a str {
        &self.ctx_source[span.start as usize..span.end as usize]
    }

    /// End of the token before `offset`, when that token is `punct`.
    fn punct_before(&self, offset: u32, punct: &str) -> Option<u32> {
        let end = prev_token_end(self.ctx_source, self.comments, offset);
        let start = (end as usize).checked_sub(punct.len())?;
        (self.ctx_source.get(start..end as usize) == Some(punct)).then_some(end)
    }

    /// `getSignatureBounds`
    fn signature_bounds(&self, p: &Parts<'_, 'a>) -> Option<SignatureBounds> {
        let (end, before) = match &p.body {
            Body::Expr { outer, .. } => {
                // `=>` always sits right before the (parenthesized) body.
                let e = self.punct_before(outer.start, "=>")? - 2;
                (e, e)
            }
            Body::Block { span, .. } if p.is_arrow => {
                let e = self.punct_before(span.start, "=>")? - 2;
                (e, e)
            }
            Body::Block { span, .. } => (span.start, span.start),
        };
        let close_paren_end = match p.return_type {
            Some(rt) => self.punct_before(rt.start, ")")?,
            None => self.punct_before(before, ")")?,
        };
        let open_end = match p.params.first() {
            Some(first) => self.punct_before(first.start, "(")?,
            None => self.punct_before(close_paren_end - 1, "(")?,
        };
        let open_start = open_end - 1;
        let tb = prev_token_end(self.ctx_source, self.comments, open_start);
        let token_before_end = if tb == 0 { open_start } else { tb };
        Some(SignatureBounds {
            start: p.span.start.max(token_before_end),
            close_paren_end,
            end,
        })
    }

    fn signature_source(&self, p: &Parts<'_, 'a>) -> Option<&'a str> {
        let b = self.signature_bounds(p)?;
        Some(js_trim(&self.ctx_source[b.start as usize..b.end as usize]))
    }

    /// `signatureBreaksArrowRestriction`
    fn signature_breaks_arrow_restriction(&self, p: &Parts<'_, 'a>) -> bool {
        let Some(b) = self.signature_bounds(p) else {
            return false;
        };
        self.ctx_source[b.close_paren_end as usize..b.end as usize]
            .trim_end_matches(is_js_space)
            .contains(['\r', '\n', '\u{2028}', '\u{2029}'])
    }

    /// `getEmittedBodyNode`: the returned expression when collapsing a
    /// single `return`, else `None` (the body itself).
    fn emitted_argument<'b>(&self, p: &Parts<'b, 'a>) -> Option<&'b Expression<'a>> {
        if self.opts.return_style == ReturnStyle::Explicit {
            return None;
        }
        let Body::Block {
            span,
            single_return: Some(arg),
        } = &p.body
        else {
            return None;
        };
        let a = arg.without_parentheses().span();
        let comments_outside = self
            .comments
            .iter()
            .filter(|c| c.span.start >= span.start && c.span.end <= span.end)
            .any(|c| c.span.start < a.start || c.span.end > a.end);
        (!comments_outside).then_some(*arg)
    }

    fn emitted_body_span(&self, p: &Parts<'_, 'a>) -> Span {
        if let Some(arg) = self.emitted_argument(p) {
            return arg.without_parentheses().span();
        }
        match &p.body {
            Body::Block { span, .. } => *span,
            Body::Expr { inner, .. } => inner.span(),
        }
    }

    /// `getBodySource`
    fn body_source(&self, p: &Parts<'_, 'a>) -> String {
        if let Some(arg) = self.emitted_argument(p) {
            let arg = arg.without_parentheses();
            let source = self.text(arg.span());
            let needs_wrapping = matches!(
                arg,
                Expression::ObjectExpression(_) | Expression::SequenceExpression(_)
            ) || source.starts_with('{');
            return if needs_wrapping {
                format!("({source})")
            } else {
                source.to_string()
            };
        }
        match &p.body {
            Body::Expr { inner, .. } if self.opts.return_style != ReturnStyle::Implicit => {
                format!("{{ return {} }}", self.text(inner.span()))
            }
            Body::Expr { inner, .. } => self.text(inner.span()).to_string(),
            Body::Block { span, .. } => self.text(*span).to_string(),
        }
    }

    /// `getGenericSource`
    fn generic_source(&self, p: &Parts<'_, 'a>) -> String {
        let Some((span, count)) = p.type_params else {
            return String::new();
        };
        let source = self.text(span);
        if !self.is_tsx || count > 1 {
            return source.to_string();
        }
        // A single type param in TSX needs a trailing comma.
        let inner = source
            .get(1..source.len().saturating_sub(1))
            .unwrap_or("")
            .trim_end_matches(is_js_space);
        let comma = if inner.ends_with(',') { "" } else { "," };
        format!("<{inner}{comma}>")
    }

    /// `isParenthesized`
    fn is_parenthesized(&self, span: Span) -> bool {
        let src = self.ctx_source;
        let before = prev_token_end(src, self.comments, span.start) as usize;
        let after = next_token_start(src, self.comments, span.end) as usize;
        before > 0 && src.as_bytes()[before - 1] == b'(' && src.as_bytes().get(after) == Some(&b')')
    }

    /// `needsParentheses`
    fn needs_parentheses(&self, f: Fn<'a, '_>) -> bool {
        let span = f.span();
        let is = |e: &Expression<'_>| e.without_parentheses().span() == span;
        match self.nodes().kind(self.eparent(f.node_id())) {
            AstKind::BinaryExpression(b) => is(&b.left) || is(&b.right),
            AstKind::LogicalExpression(l) => is(&l.left) || is(&l.right),
            AstKind::ConditionalExpression(c) => is(&c.test),
            AstKind::CallExpression(c) => is(&c.callee),
            AstKind::StaticMemberExpression(m) => is(&m.object),
            AstKind::ComputedMemberExpression(m) => is(&m.object),
            AstKind::PrivateFieldExpression(m) => is(&m.object),
            AstKind::TaggedTemplateExpression(t) => is(&t.tag),
            AstKind::UnaryExpression(_) | AstKind::AwaitExpression(_) => true,
            AstKind::TSAsExpression(e) => is(&e.expression),
            AstKind::TSSatisfiesExpression(e) => is(&e.expression),
            AstKind::TSNonNullExpression(e) => is(&e.expression),
            _ => false,
        }
    }

    /// `writeArrowFunction`
    fn write_arrow_function(&self, f: Fn<'a, '_>, p: &Parts<'_, 'a>) -> String {
        let async_ = if p.is_async { "async " } else { "" };
        let generic = self.generic_source(p);
        let body = self.body_source(p);
        let signature = match self.signature_source(p) {
            Some(s) => s.to_string(),
            None => {
                let params: Vec<&str> = p.params.iter().map(|s| self.text(*s)).collect();
                let return_type = p.return_type.map_or("", |r| self.text(r));
                format!("({}){return_type}", params.join(", "))
            }
        };
        let arrow = format!("{async_}{generic}{signature} => {body}");
        if self.needs_parentheses(f) && !self.is_parenthesized(f.span()) {
            format!("({arrow})")
        } else {
            arrow
        }
    }

    /// `willDropComments`
    fn will_drop_comments(&self, p: &Parts<'_, 'a>, container: Span, also: &[Span]) -> bool {
        let mut copied: Vec<Span> = also.to_vec();
        if let Some((tp, _)) = p.type_params {
            copied.push(tp);
        }
        if let Some(b) = self.signature_bounds(p) {
            copied.push(Span::new(b.start, b.end));
        } else {
            copied.extend(p.params.iter().copied());
            copied.extend(p.return_type);
        }
        copied.push(self.emitted_body_span(p));
        self.comments
            .iter()
            .filter(|c| c.span.start >= container.start && c.span.end <= container.end)
            .any(|c| {
                !copied
                    .iter()
                    .any(|r| c.span.start >= r.start && c.span.end <= r.end)
            })
    }

    /// `fixUnlessUnsafe`: replaces `container` unless the rewrite would drop
    /// comments, break the arrow syntax, or change nothing.
    fn fix_unless_unsafe(
        &self,
        f: Fn<'a, '_>,
        get_text: impl FnOnce(&Parts<'_, 'a>) -> String,
        container: Span,
        also: &[Span],
    ) -> Vec<Edit> {
        let Some(p) = Parts::of(f) else {
            return Vec::new();
        };
        if self.signature_breaks_arrow_restriction(&p)
            || self.will_drop_comments(&p, container, also)
        {
            return Vec::new();
        }
        let replacement = get_text(&p);
        if replacement == self.text(container) {
            return Vec::new();
        }
        vec![Edit::replace(container, replacement)]
    }
}

/// `containsInOwnScope` for this/super/arguments/new.target.
struct OwnScope {
    root: Span,
    found: bool,
}

impl<'a> Visit<'a> for OwnScope {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if it.span == self.root && !self.found {
            walk::walk_function(self, it, flags);
        }
    }
    fn visit_static_block(&mut self, _it: &oxc_ast::ast::StaticBlock<'a>) {}
    fn visit_property_definition(&mut self, it: &PropertyDefinition<'a>) {
        for d in &it.decorators {
            self.visit_decorator(d);
        }
        if it.computed {
            self.visit_property_key(&it.key);
        }
    }
    fn visit_accessor_property(&mut self, it: &AccessorProperty<'a>) {
        for d in &it.decorators {
            self.visit_decorator(d);
        }
        if it.computed {
            self.visit_property_key(&it.key);
        }
    }
    fn visit_this_expression(&mut self, _it: &oxc_ast::ast::ThisExpression) {
        self.found = true;
    }
    fn visit_super(&mut self, _it: &oxc_ast::ast::Super) {
        self.found = true;
    }
    fn visit_new_target(&mut self, _it: &oxc_ast::ast::NewTarget) {
        self.found = true;
    }
    fn visit_identifier_reference(&mut self, it: &oxc_ast::ast::IdentifierReference<'a>) {
        if it.name == "arguments" {
            self.found = true;
        }
    }
    fn visit_binding_identifier(&mut self, it: &oxc_ast::ast::BindingIdentifier<'a>) {
        if it.name == "arguments" {
            self.found = true;
        }
    }
    // ESTree spells JSX `this` as a JSXIdentifier, not a ThisExpression.
    fn visit_jsx_element_name(&mut self, it: &JSXElementName<'a>) {
        if !matches!(it, JSXElementName::ThisExpression(_)) {
            walk::walk_jsx_element_name(self, it);
        }
    }
    fn visit_jsx_member_expression_object(&mut self, it: &JSXMemberExpressionObject<'a>) {
        if !matches!(it, JSXMemberExpressionObject::ThisExpression(_)) {
            walk::walk_jsx_member_expression_object(self, it);
        }
    }
}

/// `readsOwnMemberEagerly`: `this.name` outside nested functions.
struct ReadsOwnMember<'n> {
    name: &'n str,
    found: bool,
}

impl<'a> Visit<'a> for ReadsOwnMember<'_> {
    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}
    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}
    fn visit_expression(&mut self, it: &Expression<'a>) {
        if let Some((name, object)) = member_key_name(it)
            && matches!(object, Expression::ThisExpression(_))
            && name == self.name
        {
            self.found = true;
            return;
        }
        walk::walk_expression(self, it);
    }
}

impl Rule for PreferArrowFunctions {
    fn name(&self) -> &'static str {
        "prefer-arrow-functions/prefer-arrow-functions"
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
                "allowedNames" => v.as_array().is_some_and(|a| a.iter().all(Value::is_string)),
                "allowNamedFunctions" => v.is_boolean() || v.as_str() == Some("only-expressions"),
                "allowObjectProperties"
                | "classPropertiesAllowed"
                | "disallowPrototype"
                | "singleReturnOnly" => v.is_boolean(),
                "returnStyle" => matches!(v.as_str(), Some("explicit" | "implicit" | "unchanged")),
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
        let guard = Guard {
            ctx_source: ctx.source,
            semantic: ctx.semantic,
            comments: &ctx.program.comments,
            opts: &opts,
            constructor_usages: OnceCell::new(),
            super_member_names: OnceCell::new(),
            declare_functions: OnceCell::new(),
            is_tsx: ctx.path.to_string_lossy().ends_with(".tsx"),
        };
        let nodes = ctx.semantic.nodes();
        let mut reports: Vec<(Span, &'static str, Vec<Edit>)> = Vec::new();
        let source = ctx.source;
        for node in nodes.iter() {
            match node.kind() {
                AstKind::Function(func) => {
                    let f = Fn::Func(func);
                    match func.r#type {
                        FunctionType::FunctionDeclaration => {
                            if guard.is_safe(f) {
                                let export_default = matches!(
                                    nodes.parent_kind(node.id()),
                                    AstKind::ExportDefaultDeclaration(_)
                                );
                                let fix = guard.fix_unless_unsafe(
                                    f,
                                    |p| {
                                        let arrow = guard.write_arrow_function(f, p);
                                        if export_default {
                                            format!("{arrow};")
                                        } else {
                                            let name = f.name().unwrap_or("");
                                            format!("const {name} = {arrow};")
                                        }
                                    },
                                    func.span,
                                    &[],
                                );
                                reports.push((func.span, guard.message(f), fix));
                            }
                        }
                        FunctionType::FunctionExpression => {
                            let parent_id = guard.eparent(node.id());
                            match nodes.kind(parent_id) {
                                AstKind::MethodDefinition(m) => {
                                    if m.value.span != func.span
                                        || m.r#type != MethodDefinitionType::MethodDefinition
                                        || matches!(
                                            m.kind,
                                            MethodDefinitionKind::Get
                                                | MethodDefinitionKind::Set
                                                | MethodDefinitionKind::Constructor
                                        )
                                        || !m.decorators.is_empty()
                                        || guard.is_unsafe_as_class_property(parent_id)
                                    {
                                        continue;
                                    }
                                    if guard.is_safe(f)
                                        && (!guard.is_class_member(f)
                                            || opts.class_properties_allowed)
                                        && !func
                                            .params
                                            .items
                                            .iter()
                                            .any(|p| !p.decorators.is_empty())
                                    {
                                        let name = prop_name(source, &m.key, m.computed);
                                        let accessibility = m
                                            .accessibility
                                            .map_or(String::new(), |a| format!("{} ", a.as_str()));
                                        let modifiers = format!(
                                            "{accessibility}{}{}",
                                            if m.r#static { "static " } else { "" },
                                            if m.r#override { "override " } else { "" },
                                        );
                                        let optional = if m.optional { "?" } else { "" };
                                        let fix = guard.fix_unless_unsafe(
                                            f,
                                            |p| {
                                                format!(
                                                    "{modifiers}{name}{optional} = {};",
                                                    guard.write_arrow_function(f, p)
                                                )
                                            },
                                            m.span,
                                            &[key_span(&m.key)],
                                        );
                                        reports.push((func.span, guard.message(f), fix));
                                    }
                                }
                                AstKind::ObjectProperty(p) => {
                                    if p.value.without_parentheses().span() != func.span
                                        || matches!(p.kind, PropertyKind::Get | PropertyKind::Set)
                                    {
                                        continue;
                                    }
                                    // `{ __proto__() {} }`
                                    if p.method
                                        && !p.computed
                                        && static_key_name(&p.key, p.computed).as_deref()
                                            == Some("__proto__")
                                    {
                                        continue;
                                    }
                                    if guard.is_safe(f)
                                        && !func
                                            .params
                                            .items
                                            .iter()
                                            .any(|p| !p.decorators.is_empty())
                                    {
                                        let name = prop_name(source, &p.key, p.computed);
                                        let fix = guard.fix_unless_unsafe(
                                            f,
                                            |parts| {
                                                format!(
                                                    "{name}: {}",
                                                    guard.write_arrow_function(f, parts)
                                                )
                                            },
                                            p.span,
                                            &[key_span(&p.key)],
                                        );
                                        reports.push((func.span, guard.message(f), fix));
                                    }
                                }
                                _ => {
                                    if guard.is_safe(f) {
                                        let fix = guard.fix_unless_unsafe(
                                            f,
                                            |p| guard.write_arrow_function(f, p),
                                            func.span,
                                            &[],
                                        );
                                        reports.push((func.span, guard.message(f), fix));
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                AstKind::ArrowFunctionExpression(a) => {
                    let f = Fn::Arrow(a);
                    let msg = if a.is_expression() {
                        (opts.return_style == ReturnStyle::Explicit && guard.is_safe(f))
                            .then_some(USE_EXPLICIT)
                    } else {
                        (Guard::is_single_return(a.get_function_body())
                            && opts.return_style == ReturnStyle::Implicit
                            && guard.is_safe(f))
                        .then_some(USE_IMPLICIT)
                    };
                    if let Some(msg) = msg {
                        let fix = guard.fix_unless_unsafe(
                            f,
                            |p| guard.write_arrow_function(f, p),
                            a.span,
                            &[],
                        );
                        reports.push((a.span, msg, fix));
                    }
                }
                _ => {}
            }
        }
        for (span, msg, fix) in reports {
            ctx.report_with_fix(span, msg, fix);
        }
    }
}
