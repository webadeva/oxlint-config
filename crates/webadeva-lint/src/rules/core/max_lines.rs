//! `webadeva/max-lines` and `webadeva/max-lines-warning`: port of the
//! config's own `src/plugin/rules/max-lines.ts`, configured through
//! `settings.webadeva.maxLines`. Because oxlint `overrides` can't carry
//! `settings`, both rules also take the same object (or `false`) as their
//! first option, layered over the setting key by key, for per-path limits.

use std::collections::HashSet;

use serde_json::Value;

use super::util::is_js_space;
use crate::diagnostic::LineIndex;
use crate::rule::{Ctx, Rule};

const SETTINGS_ERROR: &str = "@webadeva/oxlint-config: the max-lines config (`settings.webadeva.maxLines` \
     or the `webadeva/max-lines` rule options) must be `false` or \
     `{ enabled?, warn?, error?, skipBlankLines?, skipComments? }`";

#[derive(Debug, Clone, Copy)]
struct MaxLinesSettings {
    warn: Option<u64>,
    error: Option<u64>,
    skip_blank_lines: bool,
    skip_comments: bool,
}

/// JS `Number#toString` for the integers we print.
fn js_number(v: &Value) -> String {
    match v.as_f64() {
        Some(f) if f.fract() == 0.0 && f.abs() < 1e21 => format!("{}", f as i64),
        Some(f) => format!("{f}"),
        None => v.to_string(),
    }
}

/// `number | false`; `None` inside means `false`.
fn limit(v: &Value) -> Option<Option<u64>> {
    match v {
        Value::Bool(false) => Some(None),
        Value::Number(n) => {
            let f = n.as_f64()?;
            (f.fract() == 0.0 && f >= 0.0).then_some(Some(f as u64))
        }
        _ => None,
    }
}

/// `settings` with the rule's first option (an object or `false`) layered
/// over `settings.webadeva.maxLines`.
fn effective_settings(options: &[Value], settings: &Value) -> Value {
    let Some(option) = options.first() else {
        return settings.clone();
    };
    let max_lines = match option {
        Value::Object(overlay) => {
            let mut merged = match settings.get("webadeva").and_then(|w| w.get("maxLines")) {
                Some(Value::Object(base)) => base.clone(),
                _ => serde_json::Map::new(),
            };
            merged.extend(overlay.clone());
            Value::Object(merged)
        }
        other => other.clone(),
    };
    serde_json::json!({ "webadeva": { "maxLines": max_lines } })
}

/// `readSettings`: `Ok(None)` when the rules are turned off.
fn read_settings(settings: &Value) -> Result<Option<MaxLinesSettings>, String> {
    let max_lines = settings.get("webadeva").and_then(|w| w.get("maxLines"));
    if matches!(max_lines, Some(Value::Bool(false))) {
        return Ok(None);
    }
    // `{ ...DEFAULTS, ...maxLines }`: only an object overrides keys.
    let get = |key: &str, default: Value| -> Value {
        match max_lines {
            Some(Value::Object(m)) => m.get(key).cloned().unwrap_or(default),
            _ => default,
        }
    };
    let enabled = get("enabled", Value::Bool(true));
    let warn = get("warn", Value::from(150));
    let error = get("error", Value::from(175));
    let skip_blank = get("skipBlankLines", Value::Bool(true));
    let skip_comments = get("skipComments", Value::Bool(true));

    let Value::Bool(enabled) = enabled else {
        return Err(format!("{SETTINGS_ERROR}; `enabled` must be a boolean."));
    };
    if !enabled {
        return Ok(None);
    }
    let (Some(w), Some(e)) = (limit(&warn), limit(&error)) else {
        return Err(format!(
            "{SETTINGS_ERROR}; `warn` and `error` must be non-negative integers or `false`."
        ));
    };
    let (Value::Bool(skip_blank_lines), Value::Bool(skip_comments)) = (skip_blank, skip_comments)
    else {
        return Err(format!(
            "{SETTINGS_ERROR}; `skipBlankLines` and `skipComments` must be booleans."
        ));
    };
    if let (Some(wv), Some(ev)) = (w, e)
        && wv >= ev
    {
        return Err(format!(
            "{SETTINGS_ERROR}; `warn` ({}) must be lower than `error` ({}).",
            js_number(&warn),
            js_number(&error)
        ));
    }
    Ok(Some(MaxLinesSettings {
        warn: w,
        error: e,
        skip_blank_lines,
        skip_comments,
    }))
}

fn count_lines(ctx: &Ctx<'_>, settings: &MaxLinesSettings, index: &LineIndex<'_>) -> usize {
    let mut lines: HashSet<usize> = HashSet::new();
    let add = |lines: &mut HashSet<usize>, start: u32, end: u32| {
        let (a, b) = (index.line_of(start as usize), index.line_of(end as usize));
        for l in a..=b {
            lines.insert(l);
        }
    };
    for token in ctx.tokens {
        // The lexer's end-of-file token is not an ESTree token.
        if token.kind() == oxc_parser::Kind::Eof {
            continue;
        }
        add(&mut lines, token.start(), token.end());
    }
    if !settings.skip_comments {
        for c in &ctx.program.comments {
            add(&mut lines, c.span.start, c.span.end);
        }
        // ESLint keeps a hashbang as a `Shebang` comment.
        if let Some(h) = &ctx.program.hashbang {
            add(&mut lines, h.span.start, h.span.end);
        }
    }
    if !settings.skip_blank_lines {
        let count = index.line_count();
        // The empty string after a trailing newline is not a line of its own.
        let last = if index.line_text(count).is_empty() {
            count - 1
        } else {
            count
        };
        for l in 1..=last {
            if index.line_text(l).chars().all(is_js_space) {
                lines.insert(l);
            }
        }
    }
    lines.len()
}

pub struct MaxLines {
    warning: bool,
}

pub const MAX_LINES: MaxLines = MaxLines { warning: false };
pub const MAX_LINES_WARNING: MaxLines = MaxLines { warning: true };

impl Rule for MaxLines {
    fn name(&self) -> &'static str {
        if self.warning {
            "webadeva/max-lines-warning"
        } else {
            "webadeva/max-lines"
        }
    }

    fn validate(&self, options: &[Value], settings: &Value) -> Result<(), String> {
        read_settings(&effective_settings(options, settings)).map(|_| ())
    }

    fn needs_tokens(&self) -> bool {
        true
    }

    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        let Ok(Some(settings)) = read_settings(&effective_settings(ctx.options, ctx.settings))
        else {
            return;
        };
        let max = if self.warning {
            settings.warn
        } else {
            settings.error
        };
        let Some(max) = max else { return };
        // The warning hands over to the error once the hard limit is passed.
        let upper = if self.warning { settings.error } else { None };

        let index = LineIndex::new(ctx.source);
        let count = count_lines(ctx, &settings, &index) as u64;
        if count <= max || upper.is_some_and(|u| count > u) {
            return;
        }
        let message = if self.warning {
            format!(
                "File has {count} lines of code, over the soft limit of {max}. Consider splitting it into smaller modules."
            )
        } else {
            format!(
                "File has {count} lines of code, over the limit of {max}. Split it into smaller modules."
            )
        };
        let end = ctx.source.len() as u32;
        ctx.report(oxc_span::Span::new(0, end), message);
    }
}

#[cfg(test)]
mod option_tests {
    use super::{effective_settings, read_settings};
    use serde_json::json;

    #[test]
    fn option_layers_over_settings() {
        let settings = json!({ "webadeva": { "maxLines": { "warn": 200, "error": 300 } } });
        let s = read_settings(&effective_settings(&[json!({ "error": 400 })], &settings))
            .unwrap()
            .unwrap();
        assert_eq!((s.warn, s.error), (Some(200), Some(400)));
        assert!(
            read_settings(&effective_settings(&[json!(false)], &settings))
                .unwrap()
                .is_none()
        );
        let s = read_settings(&effective_settings(&[], &json!(null)))
            .unwrap()
            .unwrap();
        assert_eq!((s.warn, s.error), (Some(150), Some(175)));
    }
}
