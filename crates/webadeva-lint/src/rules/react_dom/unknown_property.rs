//! `@eslint-react/dom-no-unknown-property` (eslint-plugin-react-dom).

use std::borrow::Cow;

use oxc_ast::AstKind;
use oxc_ast::ast::*;
use oxc_span::{GetSpan, Span};
use serde_json::Value;

use super::helpers::{react_version, version_lt};
use super::unknown_property_tables::*;
use crate::fix::Edit;
use crate::rule::{Ctx, Rule};

fn lookup<'t>(table: &[(&str, &'t str)], key: &str) -> Option<&'t str> {
    table.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

struct Versioned {
    dom_property_names: Vec<Cow<'static, str>>,
    react_19: bool,
    react_19_3: bool,
}

impl Versioned {
    fn new(version: &str) -> Self {
        let ge = |min: &str| !version_lt(version, min);
        let mut names: Vec<Cow<'static, str>> = DOM_PROPERTY_NAMES_TWO_WORDS
            .iter()
            .chain(DOM_PROPERTY_NAMES_ONE_WORD)
            .map(|s| Cow::Borrowed(*s))
            .collect();
        let push_all = |names: &mut Vec<Cow<'static, str>>, list: &[&'static str]| {
            names.extend(list.iter().map(|s| Cow::Borrowed(*s)));
        };
        let react_19 = ge("19.0.0-rc.0");
        let react_19_3 = ge("19.3.0");
        if !version_lt("16.1.0", version) {
            // version <= 16.1.0
            names.push(Cow::Borrowed("allowTransparency"));
        } else {
            if ge("16.4.0") {
                push_all(&mut names, REACT_ON_PROPS);
            }
            if react_19 {
                push_all(&mut names, REACT_19_PROPS);
            } else {
                names.extend(REACT_19_PROPS.iter().map(|p| Cow::Owned(p.to_lowercase())));
            }
            if react_19_3 {
                push_all(&mut names, REACT_19_3_PROPS);
            }
        }
        Self {
            dom_property_names: names,
            react_19,
            react_19_3,
        }
    }

    fn allowed_tags(&self, name: &str) -> Option<&'static [&'static str]> {
        let find = |t: &'static [(&'static str, &'static [&'static str])]| {
            t.iter().find(|(k, _)| *k == name).map(|(_, v)| *v)
        };
        let mut found = find(ATTRIBUTE_TAGS_MAP);
        if self.react_19
            && let Some(v) = find(REACT_19_ATTRIBUTE_TAGS_MAP)
        {
            found = Some(v);
        }
        if self.react_19_3
            && let Some(v) = find(REACT_19_3_ATTRIBUTE_TAGS_MAP)
        {
            found = Some(v);
        }
        found
    }

    fn standard_name(&self, name: &str) -> Option<Cow<'static, str>> {
        if let Some(v) = lookup(DOM_ATTRIBUTE_NAMES, name) {
            return Some(Cow::Borrowed(v));
        }
        if self.react_19_3
            && let Some(v) = lookup(REACT_19_3_SVGDOM_ATTRIBUTE_NAMES, name)
        {
            return Some(Cow::Borrowed(v));
        }
        if let Some(v) = lookup(SVGDOM_ATTRIBUTE_NAMES, name) {
            return Some(Cow::Borrowed(v));
        }
        let lower = name.to_lowercase();
        self.dom_property_names
            .iter()
            .find(|e| e.to_lowercase() == lower)
            .cloned()
    }
}

fn normalize_attribute_case(name: &str) -> String {
    let lower = name.to_lowercase();
    DOM_PROPERTIES_IGNORE_CASE
        .iter()
        .find(|e| e.to_lowercase() == lower)
        .map_or_else(|| name.to_string(), |e| (*e).to_string())
}

/// `!/^data-xml/i.test(name) && /^data-[^:]*$/.test(name)`
fn is_valid_data_attribute(name: &str) -> bool {
    let lower_prefix = name.get(..8).map(str::to_ascii_lowercase);
    if lower_prefix.as_deref() == Some("data-xml") {
        return false;
    }
    name.strip_prefix("data-")
        .is_some_and(|rest| !rest.contains(':'))
}

/// `/^[a-z][^-]*$/`
fn is_html_tag_name(name: &str) -> bool {
    name.chars().next().is_some_and(|c| c.is_ascii_lowercase()) && !name.contains('-')
}

pub struct NoUnknownProperty;

impl Rule for NoUnknownProperty {
    fn name(&self) -> &'static str {
        "@eslint-react/dom-no-unknown-property"
    }

    fn validate(&self, options: &[Value], _settings: &Value) -> Result<(), String> {
        let Some(opt) = options.first() else {
            return Ok(());
        };
        let obj = opt.as_object().ok_or("options must be an object")?;
        for (k, v) in obj {
            match k.as_str() {
                "ignore" => {
                    if !v.as_array().is_some_and(|a| a.iter().all(Value::is_string)) {
                        return Err("`ignore` must be an array of strings".into());
                    }
                }
                "requireDataLowercase" => {
                    if !v.is_boolean() {
                        return Err("`requireDataLowercase` must be a boolean".into());
                    }
                }
                other => return Err(format!("unknown option {other:?}")),
            }
        }
        Ok(())
    }

    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        if !ctx.source.contains('<') {
            return;
        }
        let opt = ctx.option();
        let ignore: Vec<String> = opt
            .get("ignore")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let require_data_lowercase = opt
            .get("requireDataLowercase")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let mut versioned: Option<Versioned> = None;
        let sem = ctx.semantic;
        let src = ctx.source;
        let mut out: Vec<(Span, String, Vec<Edit>)> = Vec::new();
        for node in sem.nodes().iter() {
            let AstKind::JSXOpeningElement(o) = node.kind() else {
                continue;
            };
            if matches!(o.name, JSXElementName::MemberExpression(_)) {
                continue;
            }
            let tag_name: Option<&str> = match &o.name {
                JSXElementName::Identifier(i) => Some(i.name.as_str()),
                JSXElementName::IdentifierReference(i) => Some(i.name.as_str()),
                JSXElementName::ThisExpression(_) => Some("this"),
                _ => None,
            };
            let valid_html_tag = tag_name.is_some_and(is_html_tag_name)
                && !o.attributes.iter().any(|a| {
                    matches!(a, JSXAttributeItem::Attribute(a)
                        if matches!(&a.name, JSXAttributeName::Identifier(i) if i.name == "is"))
                });
            for item in &o.attributes {
                let JSXAttributeItem::Attribute(attr) = item else {
                    continue;
                };
                let name_span = attr.name.span();
                let actual = &src[name_span.start as usize..name_span.end as usize];
                if ignore.iter().any(|i| i == actual) {
                    continue;
                }
                let name = normalize_attribute_case(actual);
                if is_valid_data_attribute(&name) {
                    if require_data_lowercase && name.to_lowercase() != name {
                        out.push((
                            attr.span,
                            format!(
                                "React does not recognize data-* props with uppercase characters on a DOM element. Found '{actual}', use '{}' instead",
                                actual.to_lowercase()
                            ),
                            Vec::new(),
                        ));
                    }
                    continue;
                }
                if ARIA_PROPERTIES.contains(&name.as_str()) {
                    continue;
                }
                if matches!(tag_name, Some("fbt" | "fbs")) {
                    continue;
                }
                if !valid_html_tag {
                    continue;
                }
                let v = versioned.get_or_insert_with(|| Versioned::new(&react_version(ctx)));
                if let (Some(tag), Some(allowed)) = (tag_name, v.allowed_tags(&name)) {
                    if !allowed.contains(&tag) {
                        out.push((
                            attr.span,
                            format!(
                                "Invalid property '{actual}' found on tag '{tag}', but it is only allowed on: {}",
                                allowed.join(", ")
                            ),
                            Vec::new(),
                        ));
                    }
                    continue;
                }
                match v.standard_name(&name) {
                    Some(std) if std == name => {}
                    // Upstream fixes by replacing the attribute name.
                    Some(std) => out.push((
                        attr.span,
                        format!("Unknown property '{actual}' found, use '{std}' instead"),
                        vec![Edit::replace(name_span, std.as_ref())],
                    )),
                    None => out.push((
                        attr.span,
                        format!("Unknown property '{actual}' found"),
                        Vec::new(),
                    )),
                }
            }
        }
        for (s, m, edits) in out {
            ctx.report_with_fix(s, m, edits);
        }
    }
}
