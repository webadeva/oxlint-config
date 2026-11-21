//! `eslint-comments/require-description` from
//! @eslint-community/eslint-plugin-eslint-comments: directive comments need a
//! `-- description`. Also applied to `oxlint-*` directives.

use oxc_span::Span;
use serde_json::Value;

use super::util::{is_js_space, js_trim};
use crate::rule::{Ctx, Rule};

pub struct RequireDescription;

const ESLINT_KINDS: &[&str] = &[
    "eslint",
    "eslint-disable",
    "eslint-disable-line",
    "eslint-disable-next-line",
    "eslint-enable",
    "eslint-env",
    "exported",
    "global",
    "globals",
];
const OXLINT_KINDS: &[&str] = &[
    "oxlint-disable",
    "oxlint-disable-line",
    "oxlint-disable-next-line",
    "oxlint-enable",
];

/// `divideDirectiveComment`: `value.split(/\s-{2,}\s/u)`; the text is the
/// first part, the description the second (both trimmed).
fn divide(value: &str) -> (&str, Option<&str>) {
    let mut parts: Vec<&str> = Vec::new();
    let chars: Vec<(usize, char)> = value.char_indices().collect();
    let mut last = 0;
    let mut i = 0;
    while i < chars.len() {
        let (pos, c) = chars[i];
        if pos >= last && is_js_space(c) {
            // Greedy `-{2,}` then `\s`, backtracking over the dashes.
            let mut j = i + 1;
            while j < chars.len() && chars[j].1 == '-' {
                j += 1;
            }
            let mut end = None;
            let mut k = j;
            while k >= i + 3 {
                if k < chars.len() && is_js_space(chars[k].1) {
                    end = Some(k);
                    break;
                }
                k -= 1;
            }
            if let Some(k) = end {
                parts.push(&value[last..pos]);
                last = chars[k].0 + chars[k].1.len_utf8();
                i = k + 1;
                continue;
            }
        }
        i += 1;
    }
    parts.push(&value[last..]);
    (js_trim(parts[0]), parts.get(1).map(|d| js_trim(d)))
}

/// The directive kind matched at the start of `text` (followed by
/// whitespace or the end), like `DIRECTIVE_PATTERN`.
fn directive_kind(text: &str, additional: &[String]) -> Option<(String, bool)> {
    let followed_ok = |k: &str| text[k.len()..].chars().next().is_none_or(is_js_space);
    // `eslint(?:-env|-enable|-disable(?:(?:-next)?-line)?)?|exported|globals?`
    // plus the oxlint equivalents.
    let mut candidates: Vec<&str> = ESLINT_KINDS.iter().chain(OXLINT_KINDS).copied().collect();
    candidates.sort_by_key(|k| std::cmp::Reverse(k.len()));
    for k in candidates {
        if text.starts_with(k) && followed_ok(k) {
            return Some((k.to_string(), false));
        }
    }
    for k in additional {
        if text.starts_with(k.as_str()) && followed_ok(k) {
            return Some((k.clone(), true));
        }
    }
    None
}

fn string_array(opt: &Value, key: &str) -> Vec<String> {
    opt.get(key)
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

impl Rule for RequireDescription {
    fn name(&self) -> &'static str {
        "eslint-comments/require-description"
    }

    fn validate(&self, options: &[Value], _settings: &Value) -> Result<(), String> {
        let Some(opt) = options.first() else {
            return Ok(());
        };
        let Value::Object(map) = opt else {
            return Err("options must be an object".into());
        };
        for (k, v) in map {
            match k.as_str() {
                "ignore" => {
                    let ok = v.as_array().is_some_and(|a| {
                        a.iter().all(|x| {
                            x.as_str().is_some_and(|s| {
                                ESLINT_KINDS.contains(&s) || OXLINT_KINDS.contains(&s)
                            })
                        })
                    });
                    if !ok {
                        return Err(format!(
                            "`ignore` must be an array of {:?}",
                            [ESLINT_KINDS, OXLINT_KINDS].concat()
                        ));
                    }
                }
                "additionalDirectives" => {
                    if !v.as_array().is_some_and(|a| a.iter().all(Value::is_string)) {
                        return Err("`additionalDirectives` must be an array of strings".into());
                    }
                }
                other => return Err(format!("unknown option {other:?}")),
            }
        }
        Ok(())
    }

    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        let opt = ctx.option().clone();
        let ignore = string_array(&opt, "ignore");
        let additional = string_array(&opt, "additionalDirectives");
        let source = ctx.source;
        let mut reports = Vec::new();
        for comment in &ctx.program.comments {
            let value =
                &source[comment.content_span().start as usize..comment.content_span().end as usize];
            let (text, description) = divide(value);
            let Some((kind, is_additional)) = directive_kind(text, &additional) else {
                continue;
            };
            let line_ok = is_additional
                || matches!(
                    kind.as_str(),
                    "eslint-disable-line" | "eslint-disable-next-line"
                )
                || kind.starts_with("oxlint-");
            if comment.is_line() && !line_ok {
                continue;
            }
            if (kind == "eslint-disable-line" || kind == "oxlint-disable-line")
                && source[comment.span.start as usize..comment.span.end as usize]
                    .contains(['\n', '\r', '\u{2028}', '\u{2029}'])
            {
                // disable-line comments must not span lines.
                continue;
            }
            // `ignore: ["eslint-disable"]` also covers `oxlint-disable`.
            let eslint_equivalent = kind.replacen("oxlint-", "eslint-", 1);
            if ignore.iter().any(|i| *i == kind || *i == eslint_equivalent) {
                continue;
            }
            if description.is_none_or(str::is_empty) {
                // ESLint reports column 0 of the comment's line; the closest
                // we can express is the line start.
                let line_start = source[..comment.span.start as usize]
                    .rfind(['\n', '\r', '\u{2028}', '\u{2029}'])
                    .map_or(0, |i| {
                        i + source[i..].chars().next().map_or(1, char::len_utf8)
                    });
                reports.push(Span::new(line_start as u32, comment.span.end));
            }
        }
        for span in reports {
            ctx.report(
                span,
                "Unexpected undescribed directive comment. Include descriptions to explain why the comment is necessary.",
            );
        }
    }
}
