//! `@eslint-react/dom-*` (eslint-plugin-react-dom).

use oxc_ast::AstKind;
use oxc_ast::ast::*;
use oxc_semantic::Semantic;
use oxc_span::{GetSpan, Span};

use super::helpers::*;
use crate::fix::Edit;
use crate::rule::{Ctx, Rule};

// ---------------------------------------------------------------------------
// dangerouslySetInnerHTML
// ---------------------------------------------------------------------------

pub struct NoDangerouslySetInnerhtml;
impl Rule for NoDangerouslySetInnerhtml {
    fn name(&self) -> &'static str {
        "@eslint-react/dom-no-dangerously-set-innerhtml"
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        if !ctx.source.contains("dangerouslySetInnerHTML") {
            return;
        }
        let sem = ctx.semantic;
        let mut out = Vec::new();
        for node in sem.nodes().iter() {
            let AstKind::JSXElement(e) = node.kind() else {
                continue;
            };
            if let Some(attr) = find_attribute(sem, &e.opening_element, "dangerouslySetInnerHTML") {
                out.push(attr.span());
            }
        }
        for s in out {
            ctx.report(
                s,
                "Using 'dangerouslySetInnerHTML' may have security implications.",
            );
        }
    }
}

pub struct NoDangerouslySetInnerhtmlWithChildren;
impl Rule for NoDangerouslySetInnerhtmlWithChildren {
    fn name(&self) -> &'static str {
        "@eslint-react/dom-no-dangerously-set-innerhtml-with-children"
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        if !ctx.source.contains("dangerouslySetInnerHTML") {
            return;
        }
        let sem = ctx.semantic;
        let mut out = Vec::new();
        for node in sem.nodes().iter() {
            let AstKind::JSXElement(e) = node.kind() else {
                continue;
            };
            if !has_attribute(sem, &e.opening_element, "dangerouslySetInnerHTML") {
                continue;
            }
            let fact = find_attribute(sem, &e.opening_element, "children")
                .map(GetSpan::span)
                .or_else(|| {
                    e.children
                        .iter()
                        .find(|c| !is_padding_whitespace(c))
                        .map(GetSpan::span)
                });
            if let Some(s) = fact {
                out.push(s);
            }
        }
        for s in out {
            ctx.report(
                s,
                "A DOM component cannot use both children and 'dangerouslySetInnerHTML'.",
            );
        }
    }
}

// ---------------------------------------------------------------------------
// react-dom API calls
// ---------------------------------------------------------------------------

struct ApiCallRule {
    name: &'static str,
    api: &'static str,
    /// Text the file must contain for the rule to run.
    needle: &'static str,
    builtin_namespaces: &'static [&'static str],
    /// The rule is off below this React version.
    min_version: Option<&'static str>,
    message: &'static str,
    /// `findDOMNode(...)` is reported even when not imported.
    match_bare_name: bool,
    /// Only report when the call's return value is used.
    require_used_return: bool,
    /// The upstream autofix, if any.
    fix: ApiFix,
}

/// Autofixes of the deprecated react-dom API rules.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ApiFix {
    None,
    /// `hydrate(a, b)` -> `hydrateRoot(b, a)` + import.
    Hydrate,
    /// `render(a, b)` -> `createRoot(b).render(a)` + import.
    Render,
    /// `useFormState` -> `useActionState` + import from `importSource`.
    UseFormState,
}

/// Start of the ESTree `Program` node (typescript-estree): its first token,
/// i.e. after leading comments. `fixer.insertTextBefore(ast, ...)` inserts
/// here.
fn program_start(program: &Program<'_>) -> u32 {
    let directive = program.directives.first().map(|d| d.span.start);
    let stmt = program.body.first().map(|s| s.span().start);
    match (directive, stmt) {
        (Some(a), Some(b)) => a.min(b),
        (a, b) => a.or(b).unwrap_or(program.span.start),
    }
}

/// Span of an expression as ESTree sees it (no parentheses node).
fn estree_span(mut e: &Expression<'_>) -> Span {
    while let Expression::ParenthesizedExpression(p) = e {
        e = &p.expression;
    }
    e.span()
}

fn argument_span(a: &Argument<'_>) -> Span {
    match a {
        Argument::SpreadElement(s) => s.span,
        _ => estree_span(a.to_expression()),
    }
}

impl ApiFix {
    fn edits(self, ctx: &Ctx<'_>, call: &CallExpression<'_>) -> Vec<Edit> {
        let text = |s: Span| ctx.source_text(s);
        let args = || -> Option<(&str, &str)> {
            let a0 = call.arguments.first()?;
            let a1 = call.arguments.get(1)?;
            Some((text(argument_span(a0)), text(argument_span(a1))))
        };
        let start = program_start(ctx.program);
        match self {
            Self::None => Vec::new(),
            Self::Hydrate => args().map_or_else(Vec::new, |(a0, a1)| {
                vec![
                    Edit::insert_before(
                        start,
                        "import { hydrateRoot } from \"react-dom/client\";\n",
                    ),
                    Edit::replace(call.span, format!("hydrateRoot({a1}, {a0})")),
                ]
            }),
            Self::Render => args().map_or_else(Vec::new, |(a0, a1)| {
                vec![
                    Edit::insert_before(
                        start,
                        "import { createRoot } from \"react-dom/client\";\n",
                    ),
                    Edit::replace(call.span, format!("createRoot({a1}).render({a0})")),
                ]
            }),
            Self::UseFormState => vec![
                Edit::insert_before(
                    start,
                    format!(
                        "import {{ useActionState }} from \"{}\";\n",
                        import_source(ctx)
                    ),
                ),
                Edit::replace(estree_span(&call.callee), "useActionState"),
            ],
        }
    }
}

/// `isReturnValueUsed` of no-render-return-value.
fn is_return_value_used(sem: &Semantic<'_>, call: oxc_syntax::node::NodeId) -> bool {
    let mut p = parent(sem, call);
    while let Some(pp) = p
        && is_type_expression(kind(sem, pp))
    {
        p = parent(sem, pp);
    }
    p.is_some_and(|p| {
        matches!(
            kind(sem, p),
            AstKind::VariableDeclarator(_)
                | AstKind::ObjectProperty(_)
                | AstKind::ReturnStatement(_)
                | AstKind::ArrowFunctionExpression(_)
                | AstKind::AssignmentExpression(_)
        )
    })
}

impl Rule for ApiCallRule {
    fn name(&self) -> &'static str {
        self.name
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        if !ctx.source.contains(self.needle) {
            return;
        }
        if let Some(min) = self.min_version
            && version_lt(&react_version(ctx), min)
        {
            return;
        }
        let sem = ctx.semantic;
        let imports = ImportLookup::new(ctx.program, "react-dom", self.builtin_namespaces);
        let mut out = Vec::new();
        for node in sem.nodes().iter() {
            let AstKind::CallExpression(call) = node.kind() else {
                continue;
            };
            let bare = self.match_bare_name
                && matches!(kind(sem, unwrap(sem, call.callee.node_id())), AstKind::IdentifierReference(r) if r.name == self.api);
            if !(bare || is_react_dom_call(sem, &imports, call, self.api)) {
                continue;
            }
            if self.require_used_return && !is_return_value_used(sem, node.id()) {
                continue;
            }
            out.push((call.span, self.fix.edits(ctx, call)));
        }
        for (s, edits) in out {
            ctx.report_with_fix(s, self.message, edits);
        }
    }
}

pub fn api_call_rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(ApiCallRule {
            name: "@eslint-react/dom-no-flush-sync",
            api: "flushSync",
            needle: "flushSync",
            builtin_namespaces: &[],
            min_version: None,
            message: "Using 'flushSync' is uncommon and can hurt the performance of your app.",
            match_bare_name: false,
            require_used_return: false,
            fix: ApiFix::None,
        }),
        Box::new(ApiCallRule {
            name: "@eslint-react/dom-no-hydrate",
            api: "hydrate",
            needle: "hydrate",
            builtin_namespaces: &[],
            min_version: Some("18.0.0"),
            message: "[Deprecated] Use 'hydrateRoot()' instead.",
            match_bare_name: false,
            require_used_return: false,
            fix: ApiFix::Hydrate,
        }),
        Box::new(ApiCallRule {
            name: "@eslint-react/dom-no-render",
            api: "render",
            needle: "render",
            builtin_namespaces: &["ReactDOM"],
            min_version: Some("18.0.0"),
            message: "[Deprecated] Use 'createRoot(node).render()' instead.",
            match_bare_name: false,
            require_used_return: false,
            fix: ApiFix::Render,
        }),
        Box::new(ApiCallRule {
            name: "@eslint-react/dom-no-use-form-state",
            api: "useFormState",
            needle: "useFormState",
            builtin_namespaces: &[],
            min_version: Some("19.0.0"),
            message: "[Deprecated] Use 'useActionState' from 'react' package instead.",
            match_bare_name: false,
            require_used_return: false,
            fix: ApiFix::UseFormState,
        }),
        Box::new(ApiCallRule {
            name: "@eslint-react/dom-no-find-dom-node",
            api: "findDOMNode",
            needle: "findDOMNode",
            builtin_namespaces: &["React", "ReactDOM"],
            min_version: None,
            message: "[Deprecated] Use alternatives instead.",
            match_bare_name: true,
            require_used_return: false,
            fix: ApiFix::None,
        }),
        Box::new(ApiCallRule {
            name: "@eslint-react/dom-no-render-return-value",
            api: "render",
            needle: "render",
            builtin_namespaces: &["ReactDOM"],
            min_version: None,
            message: "Do not depend on the return value from 'ReactDOM.render'.",
            match_bare_name: false,
            require_used_return: true,
            fix: ApiFix::None,
        }),
    ]
}

// ---------------------------------------------------------------------------
// dom-no-script-url
// ---------------------------------------------------------------------------

/// `RE_JAVASCRIPT_PROTOCOL`:
/// `/^[\u0000-\u001F ]*j[\t\n\r]*a[\t\n\r]*v[\t\n\r]*a[\t\n\r]*s[\t\n\r]*c[\t\n\r]*r[\t\n\r]*i[\t\n\r]*p[\t\n\r]*t[\t\n\r]*:/iu`
pub fn is_javascript_protocol(s: &str) -> bool {
    let mut chars = s.chars().peekable();
    while chars.peek().is_some_and(|&c| c <= '\u{1f}' || c == ' ') {
        chars.next();
    }
    for (i, expected) in "javascript:".chars().enumerate() {
        if i > 0 {
            while chars
                .peek()
                .is_some_and(|&c| matches!(c, '\t' | '\n' | '\r'))
            {
                chars.next();
            }
        }
        match chars.next() {
            // With the `iu` flags, `k` also matches U+212A KELVIN SIGN etc.;
            // none of the letters here have such folds except `s` (U+017F).
            Some(c)
                if c.to_ascii_lowercase() == expected || (expected == 's' && c == '\u{17f}') => {}
            _ => return false,
        }
    }
    true
}

pub struct NoScriptUrl;
impl Rule for NoScriptUrl {
    fn name(&self) -> &'static str {
        "@eslint-react/dom-no-script-url"
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        let sem = ctx.semantic;
        let mut out = Vec::new();
        for node in sem.nodes().iter() {
            let AstKind::JSXAttribute(a) = node.kind() else {
                continue;
            };
            if !matches!(a.name, JSXAttributeName::Identifier(_)) {
                continue;
            }
            let Some(value) = &a.value else { continue };
            let item_value = attribute_static_value(sem, a);
            if item_value
                .as_ref()
                .and_then(SV::as_str)
                .is_some_and(is_javascript_protocol)
            {
                out.push(value.span());
            }
        }
        for s in out {
            ctx.report(
                s,
                "Using a `javascript:` URL is a security risk and should be avoided.",
            );
        }
    }
}

fn attribute_static_value(sem: &Semantic<'_>, a: &JSXAttribute<'_>) -> Option<SV> {
    match &a.value {
        None => Some(SV::Bool(true)),
        Some(JSXAttributeValue::StringLiteral(s)) => {
            s.value.as_str().map(|v| SV::Str(v.to_string()))
        }
        Some(JSXAttributeValue::ExpressionContainer(c)) => c
            .expression
            .as_expression()
            .and_then(|e| static_value(sem, e.node_id())),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Element type resolution (`createJsxElementResolver`)
// ---------------------------------------------------------------------------

fn dom_element_type(sem: &Semantic<'_>, e: &JSXElement<'_>, polymorphic: &str) -> String {
    let name = crate::utils::jsx_element_name(&e.opening_element.name);
    let ident = match &e.opening_element.name {
        JSXElementName::Identifier(i) => Some(i.name.as_str()),
        JSXElementName::IdentifierReference(i) => Some(i.name.as_str()),
        JSXElementName::ThisExpression(_) => Some("this"),
        _ => None,
    };
    if let Some(first) = ident.and_then(|i| i.chars().next())
        && first.to_lowercase().eq(std::iter::once(first))
    {
        return name;
    }
    if let Some(item) = find_attribute(sem, &e.opening_element, polymorphic)
        && let Some(SV::Str(v)) = resolve_attribute_value(sem, item, Some(polymorphic)).value
    {
        return v.to_lowercase();
    }
    name
}

// ---------------------------------------------------------------------------
// dom-no-unsafe-iframe-sandbox
// ---------------------------------------------------------------------------

pub struct NoUnsafeIframeSandbox;
impl Rule for NoUnsafeIframeSandbox {
    fn name(&self) -> &'static str {
        "@eslint-react/dom-no-unsafe-iframe-sandbox"
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        if !ctx.source.contains("sandbox") {
            return;
        }
        let sem = ctx.semantic;
        let polymorphic = polymorphic_prop_name(ctx);
        let mut out = Vec::new();
        for node in sem.nodes().iter() {
            let AstKind::JSXElement(e) = node.kind() else {
                continue;
            };
            if dom_element_type(sem, e, &polymorphic) != "iframe" {
                continue;
            }
            let Some(prop) = find_attribute(sem, &e.opening_element, "sandbox") else {
                continue;
            };
            let v = resolve_attribute_value(sem, prop, Some("sandbox"));
            let Some(SV::Str(value)) = &v.value else {
                continue;
            };
            if !["allow-scripts", "allow-same-origin"]
                .iter()
                .all(|a| value.contains(a))
            {
                continue;
            }
            out.push(v.node.unwrap_or_else(|| prop.span()));
        }
        for s in out {
            ctx.report(s, "Unsafe 'sandbox' attribute value on 'iframe' component.");
        }
    }
}

// ---------------------------------------------------------------------------
// dom-no-void-elements-with-children
// ---------------------------------------------------------------------------

const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "keygen", "link", "menuitem",
    "meta", "param", "source", "track", "wbr",
];

pub struct NoVoidElementsWithChildren;
impl Rule for NoVoidElementsWithChildren {
    fn name(&self) -> &'static str {
        "@eslint-react/dom-no-void-elements-with-children"
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        let sem = ctx.semantic;
        let polymorphic = polymorphic_prop_name(ctx);
        let mut out: Vec<(Span, String)> = Vec::new();
        for node in sem.nodes().iter() {
            let AstKind::JSXElement(e) = node.kind() else {
                continue;
            };
            // Cheap pre-check: host void tags, or anything with the
            // polymorphic prop (which may resolve to a void tag).
            let ty = dom_element_type(sem, e, &polymorphic);
            if !VOID_ELEMENTS.contains(&ty.as_str()) {
                continue;
            }
            if !e.children.is_empty()
                || has_attribute(sem, &e.opening_element, "children")
                || has_attribute(sem, &e.opening_element, "dangerouslySetInnerHTML")
            {
                out.push((e.span, ty));
            }
        }
        for (s, ty) in out {
            ctx.report(
                s,
                format!("'{ty}' is a void element tag and must not have children."),
            );
        }
    }
}
