//! The Node add-on oxlint loads through our JS plugin (`plugins/bridge.mjs`).
//!
//! oxlint creates every enabled plugin rule for a file before visiting it,
//! so the bridge collects them and calls [`lint`] once per file: one oxc parse
//! however many rules are on. Offsets in the result are UTF-16 code units,
//! the unit of JavaScript strings (oxc works in UTF-8 bytes).

use std::path::Path;
use std::sync::OnceLock;

use napi_derive::napi;
use serde_json::Value;

use crate::diagnostic::Severity;
use crate::engine::{self, ActiveRule};
use crate::rule::Rule;

fn registry() -> &'static [Box<dyn Rule>] {
    static RULES: OnceLock<Vec<Box<dyn Rule>>> = OnceLock::new();
    RULES.get_or_init(crate::rules::all)
}

/// A rule the bridge wants run, by its native name (e.g. `camelcase`,
/// `@eslint-react/no-missing-key`), with its configured options.
#[napi(object)]
pub struct RuleConfig {
    pub name: String,
    pub options: Option<Vec<Value>>,
}

#[napi(object)]
pub struct Edit {
    pub start: u32,
    pub end: u32,
    pub text: String,
}

#[napi(object)]
pub struct Report {
    /// The rule's native name, as passed in.
    pub rule: String,
    pub message: String,
    pub start: u32,
    pub end: u32,
    /// The autofix, if any: edits applied together by `oxlint --fix`.
    pub fix: Option<Vec<Edit>>,
}

/// Maps UTF-8 byte offsets to UTF-16 offsets.
struct Utf16 {
    /// For each non-ASCII char: its byte offset, its UTF-16 offset, and its
    /// length in UTF-8 and UTF-16. Empty for ASCII-only sources.
    marks: Vec<(u32, u32, u32, u32)>,
}

impl Utf16 {
    fn new(source: &str) -> Self {
        if source.is_ascii() {
            return Self { marks: Vec::new() };
        }
        let mut marks = Vec::new();
        let mut utf16 = 0u32;
        for (byte, ch) in source.char_indices() {
            if !ch.is_ascii() {
                marks.push((
                    byte as u32,
                    utf16,
                    ch.len_utf8() as u32,
                    ch.len_utf16() as u32,
                ));
            }
            utf16 += ch.len_utf16() as u32;
        }
        Self { marks }
    }

    fn convert(&self, offset: u32) -> u32 {
        // The last non-ASCII char starting at or before `offset`; everything
        // between its end and `offset` is ASCII, one unit per byte.
        let i = self.marks.partition_point(|&(b, ..)| b <= offset);
        if i == 0 {
            return offset;
        }
        let (byte, utf16, len8, len16) = self.marks[i - 1];
        if offset < byte + len8 {
            utf16
        } else {
            utf16 + len16 + (offset - byte - len8)
        }
    }
}

/// Every rule's native name, e.g. `camelcase` or `@eslint-react/no-missing-key`.
#[napi]
pub fn rules() -> Vec<String> {
    registry().iter().map(|r| r.name().to_string()).collect()
}

/// Lints one file with the given rules.
#[napi]
pub fn lint(
    filename: String,
    source: String,
    rules: Vec<RuleConfig>,
    settings: Option<Value>,
    cwd: String,
) -> napi::Result<Vec<Report>> {
    let settings = settings.unwrap_or(Value::Null);
    let registry = registry();
    let mut options: Vec<Vec<Value>> = rules
        .iter()
        .map(|r| r.options.clone().unwrap_or_default())
        .collect();
    share_max_lines_options(&rules, &mut options);
    let path = Path::new(&filename);
    let root = Path::new(&cwd);
    let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");

    let mut config_errors = Vec::new();
    let mut active = Vec::with_capacity(rules.len());
    for (config, options) in rules.iter().zip(&options) {
        let Some(rule) = registry.iter().find(|r| r.name() == config.name) else {
            return Err(napi::Error::from_reason(format!(
                "@webadeva/oxlint-config: unknown rule {:?}",
                config.name
            )));
        };
        // A misconfigured rule reports its own error; the other rules keep
        // running (they share this one call per file).
        if let Err(e) = rule.validate(options, &settings) {
            config_errors.push(Report {
                rule: config.name.clone(),
                message: format!("Invalid configuration: {e}"),
                start: 0,
                end: 0,
                fix: None,
            });
            continue;
        }
        if rule.applies_to(&rel) {
            active.push(ActiveRule {
                rule: rule.as_ref(),
                // oxlint owns severities; ours only feeds the harness.
                severity: Severity::Error,
                options,
            });
        }
    }

    let linted = engine::run(path, root, &source, &active, &settings);
    let utf16 = Utf16::new(&source);
    Ok(config_errors
        .into_iter()
        .chain(linted.diagnostics.into_iter().map(|d| Report {
            rule: d.rule.to_string(),
            message: d.message,
            start: utf16.convert(d.start),
            end: utf16.convert(d.end),
            fix: d.fix.map(|f| {
                f.edits
                    .into_iter()
                    .map(|e| Edit {
                        start: utf16.convert(e.start),
                        end: utf16.convert(e.end),
                        text: e.text,
                    })
                    .collect()
            }),
        }))
        .collect())
}

const MAX_LINES: &str = "webadeva/max-lines";
const MAX_LINES_WARNING: &str = "webadeva/max-lines-warning";

/// `webadeva/max-lines` and `webadeva/max-lines-warning` are one setting
/// (`settings.webadeva.maxLines`) split in two rules only because a rule has
/// one severity. Options given to either apply to both, so e.g. raising the
/// error limit in an override doesn't leave the warning stopping short of it.
/// When both have options, the warning's are applied first, then the error's.
fn share_max_lines_options(rules: &[RuleConfig], options: &mut [Vec<Value>]) {
    let mut shared: Option<Value> = None;
    for name in [MAX_LINES_WARNING, MAX_LINES] {
        let Some(i) = rules.iter().position(|r| r.name == name) else {
            continue;
        };
        let Some(option) = options[i].first() else {
            continue;
        };
        shared = Some(match (shared, option) {
            (Some(Value::Object(mut acc)), Value::Object(next)) => {
                acc.extend(next.clone());
                Value::Object(acc)
            }
            (_, next) => next.clone(),
        });
    }
    let Some(shared) = shared else { return };
    for (rule, opts) in rules.iter().zip(options.iter_mut()) {
        if rule.name == MAX_LINES || rule.name == MAX_LINES_WARNING {
            *opts = vec![shared.clone()];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Utf16;

    #[test]
    fn converts_byte_offsets_to_utf16() {
        let src = "a\u{e9}b\u{1F600}c";
        let map = Utf16::new(src);
        for (i, ch) in src.char_indices() {
            let expected = src[..i].encode_utf16().count() as u32;
            assert_eq!(map.convert(i as u32), expected, "at {ch:?}");
        }
        assert_eq!(
            map.convert(src.len() as u32),
            src.encode_utf16().count() as u32
        );
        assert_eq!(Utf16::new("abc").convert(2), 2);
    }
}
