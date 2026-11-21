//! Inline `eslint-*` / `oxlint-*` disable directives, so our rules can be
//! silenced the same way oxlint's native rules are.

use crate::diagnostic::LineIndex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Disable,
    Enable,
    DisableLine,
    DisableNextLine,
}

#[derive(Debug, Clone)]
pub struct Directive {
    pub kind: Kind,
    /// Byte offset of the comment start (block directives apply from here).
    pub start: usize,
    /// Byte offset of the comment end.
    pub end: usize,
    /// Rules named by the directive; empty means every rule.
    pub rules: Vec<String>,
}

/// One parsed directive comment, also used by `eslint-comments/require-description`.
#[derive(Debug, Clone)]
pub struct ParsedComment<'s> {
    /// The keyword, e.g. `eslint-disable-next-line` or `oxlint-disable`.
    pub keyword: &'s str,
    /// The text after the keyword, before ` -- `.
    pub value: &'s str,
}

const KEYWORDS: &[&str] = &[
    // Longest first, so `eslint-disable-next-line` isn't read as `eslint-disable`.
    "eslint-disable-next-line",
    "eslint-disable-line",
    "eslint-disable",
    "eslint-enable",
    "eslint-env",
    "eslint",
    "exported",
    "globals",
    "global",
    "oxlint-disable-next-line",
    "oxlint-disable-line",
    "oxlint-disable",
    "oxlint-enable",
];

/// Parses a comment's inner text (without `//` or `/* */`) as a directive.
/// `line_comment` restricts which keywords are valid, like ESLint: only the
/// `-line`/`-next-line` forms work in `//` comments, everything works in blocks.
pub fn parse_comment(text: &str, line_comment: bool) -> Option<ParsedComment<'_>> {
    let trimmed = text.trim_start();
    let keyword = KEYWORDS.iter().copied().find(|k| {
        trimmed.starts_with(k)
            && trimmed[k.len()..]
                .chars()
                .next()
                .is_none_or(|c| c.is_whitespace())
    })?;
    if line_comment
        && !matches!(
            keyword,
            "eslint-disable-line"
                | "eslint-disable-next-line"
                | "oxlint-disable-line"
                | "oxlint-disable-next-line"
        )
    {
        return None;
    }
    let rest = &trimmed[keyword.len()..];
    // ESLint splits the description at `--` surrounded by whitespace
    // (`/\s-{2,}\s/`), and a trailing `--` with nothing after also counts.
    let (value, _) = split_description(rest);
    Some(ParsedComment {
        keyword,
        value: value.trim(),
    })
}

fn split_description(rest: &str) -> (&str, Option<&str>) {
    let bytes = rest.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_whitespace() || bytes[i] == b'\n' {
            let mut j = i + 1;
            let mut dashes = 0;
            while j < bytes.len() && bytes[j] == b'-' {
                dashes += 1;
                j += 1;
            }
            if dashes >= 2 && (j == bytes.len() || bytes[j].is_ascii_whitespace()) {
                return (&rest[..i], Some(rest[j..].trim()));
            }
        }
        i += 1;
    }
    (rest, None)
}

pub fn collect(comments: &[(usize, usize, bool, &str)]) -> Vec<Directive> {
    let mut out = Vec::new();
    for &(start, end, line_comment, text) in comments {
        let Some(parsed) = parse_comment(text, line_comment) else {
            continue;
        };
        let kind = match parsed.keyword {
            "eslint-disable" | "oxlint-disable" => Kind::Disable,
            "eslint-enable" | "oxlint-enable" => Kind::Enable,
            "eslint-disable-line" | "oxlint-disable-line" => Kind::DisableLine,
            "eslint-disable-next-line" | "oxlint-disable-next-line" => Kind::DisableNextLine,
            _ => continue,
        };
        let rules = parsed
            .value
            .split(',')
            .map(|r| r.trim().to_string())
            .filter(|r| !r.is_empty())
            .collect();
        out.push(Directive {
            kind,
            start,
            end,
            rules,
        });
    }
    out
}

fn names(d: &Directive, rule: &str) -> bool {
    d.rules.is_empty() || d.rules.iter().any(|r| r == rule)
}

/// Whether a diagnostic of `rule` starting at byte `offset` is suppressed.
pub fn is_suppressed(
    directives: &[Directive],
    index: &LineIndex<'_>,
    rule: &str,
    offset: usize,
) -> bool {
    let line = index.line_of(offset);
    let mut disabled_all = false;
    let mut disabled: Vec<&str> = Vec::new();
    let mut enabled_after_all: Vec<&str> = Vec::new();
    for d in directives {
        match d.kind {
            Kind::DisableLine => {
                if index.line_of(d.start) == line && names(d, rule) {
                    return true;
                }
            }
            Kind::DisableNextLine => {
                if index.line_of(d.end) + 1 == line && names(d, rule) {
                    return true;
                }
            }
            Kind::Disable | Kind::Enable if d.start <= offset => {
                let disable = d.kind == Kind::Disable;
                if d.rules.is_empty() {
                    disabled_all = disable;
                    disabled.clear();
                    enabled_after_all.clear();
                } else {
                    for r in &d.rules {
                        if disable {
                            disabled.push(r);
                            enabled_after_all.retain(|e| e != r);
                        } else {
                            disabled.retain(|e| e != r);
                            if disabled_all {
                                enabled_after_all.push(r);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    (disabled_all && !enabled_after_all.contains(&rule)) || disabled.contains(&rule)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_directives() {
        let p = parse_comment(" eslint-disable-next-line no-console -- logging", true).unwrap();
        assert_eq!(p.keyword, "eslint-disable-next-line");
        assert_eq!(p.value, "no-console");
        let p = parse_comment(" oxlint-disable foo", false).unwrap();
        assert_eq!(p.keyword, "oxlint-disable");
        assert_eq!(p.value, "foo");
        assert!(parse_comment(" eslint-disable foo", true).is_none());
        assert!(parse_comment(" eslint-disabled", false).is_none());
    }
}
