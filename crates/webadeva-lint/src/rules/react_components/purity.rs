//! `@eslint-react/purity` (eslint-plugin-react-x): known-impure calls and
//! constructors (`Math.random()`, `Date.now()`, `new Date()`, ...) directly
//! inside function components and custom hooks. Broader than the compiler's
//! `react-hooks/purity` (oxlint `react/purity`): it also covers hooks and
//! resolves aliases (`const { random } = Math`, `const w = window`).

use std::collections::HashSet;

use oxc_ast::AstKind;
use oxc_ast::ast::*;
use oxc_semantic::{NodeId, Semantic};
use oxc_span::Span;
use serde_json::Value;

use super::helpers::*;
use super::no_options;
use crate::rule::{Ctx, Rule};

const WINDOW_FUNCS: &[&str] = &[
    "addEventListener",
    "alert",
    "blur",
    "cancelAnimationFrame",
    "cancelIdleCallback",
    "clearInterval",
    "clearTimeout",
    "close",
    "confirm",
    "dispatchEvent",
    "fetch",
    "focus",
    "getComputedStyle",
    "getSelection",
    "matchMedia",
    "moveBy",
    "moveTo",
    "open",
    "postMessage",
    "print",
    "prompt",
    "queueMicrotask",
    "removeEventListener",
    "reportError",
    "requestAnimationFrame",
    "requestIdleCallback",
    "resizeBy",
    "resizeTo",
    "scroll",
    "scrollBy",
    "scrollTo",
    "setInterval",
    "setTimeout",
    "stop",
    "structuredClone",
];
const STORAGE_FUNCS: &[&str] = &["clear", "getItem", "key", "removeItem", "setItem"];

fn impure_funcs(object: &str) -> Option<&'static [&'static str]> {
    Some(match object {
        "Atomics" => &[
            "add",
            "and",
            "compareExchange",
            "exchange",
            "load",
            "notify",
            "or",
            "store",
            "sub",
            "wait",
            "waitAsync",
            "xor",
        ],
        "caches" => &["delete", "has", "keys", "match", "open"],
        "clipboard" => &["read", "readText", "write", "writeText"],
        "cookieStore" => &["delete", "get", "getAll", "set"],
        "crypto" => &["getRandomValues", "randomUUID"],
        "Date" => &["now"],
        "document" => &[
            "adoptNode",
            "append",
            "close",
            "createAttribute",
            "createAttributeNS",
            "createCDATASection",
            "createComment",
            "createDocumentFragment",
            "createDocumentType",
            "createElement",
            "createElementNS",
            "createEvent",
            "createNodeIterator",
            "createProcessingInstruction",
            "createRange",
            "createTextNode",
            "createTreeWalker",
            "elementFromPoint",
            "elementsFromPoint",
            "execCommand",
            "exitFullscreen",
            "exitPictureInPicture",
            "exitPointerLock",
            "getAnimations",
            "getElementById",
            "getElementsByClassName",
            "getElementsByName",
            "getElementsByTagName",
            "getElementsByTagNameNS",
            "getSelection",
            "hasFocus",
            "importNode",
            "open",
            "prepend",
            "querySelector",
            "querySelectorAll",
            "replaceChildren",
            "requestStorageAccess",
            "startViewTransition",
            "write",
            "writeln",
        ],
        "globalThis" | "window" => WINDOW_FUNCS,
        "history" => &["back", "forward", "go", "pushState", "replaceState"],
        "indexedDB" => &["databases", "deleteDatabase", "open"],
        "localStorage" | "sessionStorage" => STORAGE_FUNCS,
        "location" => &["assign", "reload", "replace"],
        "Math" => &["random"],
        "navigation" => &[
            "back",
            "forward",
            "navigate",
            "reload",
            "traverseTo",
            "updateCurrentEntry",
        ],
        "navigator" => &[
            "canShare",
            "getBattery",
            "getGamepads",
            "getUserMedia",
            "javaEnabled",
            "registerProtocolHandler",
            "requestMediaKeySystemAccess",
            "requestMIDIAccess",
            "sendBeacon",
            "share",
            "vibrate",
        ],
        "Notification" => &["requestPermission"],
        "Object" => &[
            "assign",
            "defineProperties",
            "defineProperty",
            "freeze",
            "preventExtensions",
            "seal",
            "setPrototypeOf",
        ],
        "performance" => &[
            "clearMarks",
            "clearMeasures",
            "getEntries",
            "getEntriesByName",
            "getEntriesByType",
            "mark",
            "measure",
            "now",
        ],
        "process" => &[
            "abort",
            "chdir",
            "cpuUsage",
            "emitWarning",
            "exit",
            "hrtime",
            "kill",
            "memoryUsage",
            "nextTick",
            "send",
            "stderr",
            "stdout",
        ],
        "Reflect" => &[
            "defineProperty",
            "deleteProperty",
            "preventExtensions",
            "set",
            "setPrototypeOf",
        ],
        "scheduler" => &["postTask", "yield"],
        "URL" => &["createObjectURL", "revokeObjectURL"],
        _ => return None,
    })
}

const IMPURE_CTORS: &[&str] = &[
    "Audio",
    "AudioContext",
    "BroadcastChannel",
    "Date",
    "EventSource",
    "FileReader",
    "Image",
    "IntersectionObserver",
    "MediaRecorder",
    "MediaSource",
    "MediaStream",
    "MessageChannel",
    "MutationObserver",
    "Notification",
    "OfflineAudioContext",
    "PerformanceObserver",
    "ReportingObserver",
    "ResizeObserver",
    "RTCPeerConnection",
    "SharedWorker",
    "WebSocket",
    "Worker",
    "XMLHttpRequest",
];

/// Objects of upstream's `PURE_FUNCS` catalog that are not in `IMPURE_FUNCS`.
const PURE_ONLY_OBJECTS: &[&str] = &["Array", "BigInt", "JSON", "Number", "String"];

fn is_impure(object: &str, property: &str) -> bool {
    impure_funcs(object).is_some_and(|l| l.contains(&property))
}

fn is_catalog_object(name: &str) -> bool {
    impure_funcs(name).is_some() || PURE_ONLY_OBJECTS.contains(&name)
}

struct Builtin {
    object: String,
    property: Option<String>,
}

/// `Extract.getMemberChain` restricted to identifier segments: the root
/// identifier and the property names, or `None` if any segment isn't an
/// identifier.
fn member_chain<'b, 'a>(
    m: &'b MemberExpression<'a>,
) -> Option<(&'b IdentifierReference<'a>, Vec<&'b str>)> {
    let mut names = Vec::new();
    let mut cur = Un::Member(m);
    loop {
        match cur {
            Un::Member(m) => {
                names.push(member_property_ident(m)?);
                cur = unwrap(m.object());
            }
            Un::Expr(Expression::Identifier(id)) => {
                names.reverse();
                return Some((id, names));
            }
            _ => return None,
        }
    }
}

fn resolve_member(
    sem: &Semantic<'_>,
    m: &MemberExpression<'_>,
    seen: &mut HashSet<String>,
) -> Option<Builtin> {
    let (root, names) = member_chain(m)?;
    let resolved = resolve_object_name(sem, root, seen)?;
    let mut object = resolved.property.unwrap_or(resolved.object);
    if is_catalog_object(&object) {
        for name in &names[..names.len() - 1] {
            if !is_catalog_object(name) {
                break;
            }
            object = name.to_string();
        }
    }
    Some(Builtin {
        object,
        property: names.last().map(|s| s.to_string()),
    })
}

/// `resolveBuiltinObjectName`.
fn resolve_object_name(
    sem: &Semantic<'_>,
    id: &IdentifierReference<'_>,
    seen: &mut HashSet<String>,
) -> Option<Builtin> {
    let name = id.name.as_str();
    if !seen.insert(name.to_string()) {
        return None;
    }
    let global = || {
        Some(Builtin {
            object: name.to_string(),
            property: None,
        })
    };
    let Some(rid) = id.reference_id.get() else {
        return global();
    };
    let Some(sym) = sem.scoping().get_reference(rid).symbol_id() else {
        return global();
    };
    let AstKind::VariableDeclarator(d) = sem.nodes().kind(sem.scoping().symbol_declaration(sym))
    else {
        return None;
    };
    let init = d.init.as_ref()?;
    let resolved = match unwrap(init) {
        Un::Expr(Expression::Identifier(i)) => resolve_object_name(sem, i, seen),
        Un::Member(m) => resolve_member(sem, m, seen),
        _ => None,
    }?;
    match &d.id {
        BindingPattern::ObjectPattern(pat) => {
            let key = pat.properties.iter().find_map(|p| {
                let target = match &p.value {
                    BindingPattern::AssignmentPattern(ap) => &ap.left,
                    v => v,
                };
                let BindingPattern::BindingIdentifier(b) = target else {
                    return None;
                };
                if b.symbol_id.get() != Some(sym) {
                    return None;
                }
                // getPropertyName(property, "max")
                Some(match &p.key {
                    PropertyKey::StaticIdentifier(i) if !p.computed => Some(i.name.to_string()),
                    PropertyKey::Identifier(i) if !p.computed => Some(i.name.to_string()),
                    PropertyKey::StringLiteral(s) => s.value.as_str().map(str::to_string),
                    PropertyKey::TemplateLiteral(t) if t.expressions.is_empty() => t
                        .quasis
                        .first()
                        .and_then(|q| {
                            q.value
                                .cooked
                                .as_ref()
                                .and_then(|c| c.as_str())
                                .or(Some(q.value.raw.as_str()))
                        })
                        .map(str::to_string),
                    _ => None,
                })
            })??;
            Some(Builtin {
                object: resolved.property.unwrap_or(resolved.object),
                property: Some(key),
            })
        }
        BindingPattern::ArrayPattern(_) => None,
        _ => Some(resolved),
    }
}

pub struct Purity;

impl Rule for Purity {
    fn name(&self) -> &'static str {
        "@eslint-react/purity"
    }
    fn validate(&self, options: &[Value], _: &Value) -> Result<(), String> {
        no_options(options)
    }
    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        let sem = ctx.semantic;
        let nodes = sem.nodes();
        // (enclosing function, node span)
        let mut entries: Vec<(NodeId, Span)> = Vec::new();
        let nearest_fn = |id: NodeId| find_parent(nodes, id, |_, k| as_func(k).is_some(), no_stop);
        for node in nodes.iter() {
            match node.kind() {
                AstKind::CallExpression(call) => {
                    let mut seen = HashSet::new();
                    let impure = match unwrap(&call.callee) {
                        Un::Expr(Expression::Identifier(id)) => {
                            match resolve_object_name(sem, id, &mut seen) {
                                Some(Builtin {
                                    object,
                                    property: Some(p),
                                }) => is_impure(&object, &p),
                                Some(Builtin {
                                    object,
                                    property: None,
                                }) => is_impure("globalThis", &object),
                                None => false,
                            }
                        }
                        Un::Member(m) if member_property_ident(m).is_some() => {
                            match resolve_member(sem, m, &mut seen) {
                                Some(Builtin {
                                    object,
                                    property: Some(p),
                                }) => is_impure(&object, &p),
                                _ => false,
                            }
                        }
                        _ => false,
                    };
                    if impure && let Some(f) = nearest_fn(node.id()) {
                        entries.push((f, call.span));
                    }
                }
                AstKind::NewExpression(new) => {
                    let Un::Expr(Expression::Identifier(id)) = unwrap(&new.callee) else {
                        continue;
                    };
                    let Some(r) = resolve_object_name(sem, id, &mut HashSet::new()) else {
                        continue;
                    };
                    let ctor = match r.property {
                        Some(p) if is_catalog_object(&r.object) => p,
                        _ => r.object,
                    };
                    if !IMPURE_CTORS.contains(&ctor.as_str())
                        || (ctor == "Date" && !new.arguments.is_empty())
                    {
                        continue;
                    }
                    if let Some(f) = nearest_fn(node.id()) {
                        entries.push((f, new.span));
                    }
                }
                _ => {}
            }
        }
        if entries.is_empty() {
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
        let comps: HashSet<NodeId> = collected.component_ids().collect();
        let hooks: HashSet<NodeId> = collected.hook_ids().collect();
        let client = ctx
            .program
            .directives
            .iter()
            .any(|d| d.directive.as_str() == "use client");
        let mut reports = Vec::new();
        for (f, span) in entries {
            let is_comp = comps.contains(&f);
            if !is_comp && !hooks.contains(&f) {
                continue;
            }
            if !client && is_comp && as_func(nodes.kind(f)).is_some_and(Func::is_async) {
                continue;
            }
            reports.push((span, format!(
                "Do not call '{}' during render. Components and hooks must be pure. Move this call into an event handler, effect, or state initializer.",
                ctx.source_text(span)
            )));
        }
        for (s, m) in reports {
            ctx.report(s, m);
        }
    }
}
