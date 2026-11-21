//! Ports of the `@storybook/csf` helpers upstream uses (`storyNameFromExport`,
//! `isExportStory`) and of the plugin's `getDescriptor`.

use oxc_ast::ast::{ArrayExpressionElement, Expression, ObjectExpression, ObjectPropertyKind};

use super::{key_ident_name, lit_str};

// ---------------------------------------------------------------------------
// storyNameFromExport / toStartCaseStr
// ---------------------------------------------------------------------------

/// Applies a global regex replace of a fixed-width pattern made of
/// single-character classes, like `str.replace(/(a)(b)(c)/g, f)`.
fn replace_classes(
    input: &[char],
    classes: &[&dyn Fn(char) -> bool],
    out: &mut Vec<char>,
    sep_after: usize,
) {
    let n = classes.len();
    let mut i = 0;
    while i < input.len() {
        if i + n <= input.len() && classes.iter().enumerate().all(|(k, c)| c(input[i + k])) {
            for (k, ch) in input[i..i + n].iter().enumerate() {
                if k == sep_after {
                    out.push(' ');
                }
                out.push(*ch);
            }
            i += n;
        } else {
            out.push(input[i]);
            i += 1;
        }
    }
}

fn is_js_space(c: char) -> bool {
    c.is_whitespace() || c == '\u{feff}'
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

pub(crate) fn to_start_case(s: &str) -> String {
    let mut cur: Vec<char> = s
        .chars()
        .map(|c| {
            if c == '_' || c == '-' || c == '.' {
                ' '
            } else {
                c
            }
        })
        .collect();
    let upper = |c: char| c.is_ascii_uppercase();
    let lower = |c: char| c.is_ascii_lowercase();
    let alpha = |c: char| c.is_ascii_alphabetic();
    let digit = |c: char| c.is_ascii_digit();
    let not_nl = |c: char| c != '\n';

    let mut next = Vec::with_capacity(cur.len() + 8);
    // /([^\n])([A-Z])([a-z])/g -> "$1 $2$3"
    replace_classes(&cur, &[&not_nl, &upper, &lower], &mut next, 1);
    std::mem::swap(&mut cur, &mut next);
    next.clear();
    // /([a-z])([A-Z])/g -> "$1 $2"
    replace_classes(&cur, &[&lower, &upper], &mut next, 1);
    std::mem::swap(&mut cur, &mut next);
    next.clear();
    // /([a-z])([0-9])/gi
    replace_classes(&cur, &[&alpha, &digit], &mut next, 1);
    std::mem::swap(&mut cur, &mut next);
    next.clear();
    // /([0-9])([a-z])/gi
    replace_classes(&cur, &[&digit, &alpha], &mut next, 1);
    std::mem::swap(&mut cur, &mut next);
    next.clear();
    // /(\s|^)(\w)/g -> "$1" + upper("$2")
    let mut i = 0;
    while i < cur.len() {
        if is_js_space(cur[i]) && i + 1 < cur.len() && is_word(cur[i + 1]) {
            next.push(cur[i]);
            next.push(cur[i + 1].to_ascii_uppercase());
            i += 2;
        } else if i == 0 && is_word(cur[0]) {
            next.push(cur[0].to_ascii_uppercase());
            i += 1;
        } else {
            next.push(cur[i]);
            i += 1;
        }
    }
    // / +/g -> " "
    let mut out = String::with_capacity(next.len());
    for c in next {
        if c == ' ' && out.ends_with(' ') {
            continue;
        }
        out.push(c);
    }
    out.trim_matches(is_js_space).to_string()
}

pub(crate) fn story_name_from_export(key: &str) -> String {
    to_start_case(key)
}

// ---------------------------------------------------------------------------
// includeStories / excludeStories descriptors
// ---------------------------------------------------------------------------

/// A JS value an ESTree `Literal` evaluates to.
#[derive(Debug)]
pub(crate) enum LitValue {
    Str(String),
    Num(f64),
    Bool(bool),
    Null,
    BigInt(String),
    /// `None` when the JS pattern has no Rust `regex` equivalent
    /// (lookaround, backreferences); such a filter then matches nothing.
    Regex(Option<regex::Regex>),
}

#[derive(Debug)]
pub(crate) enum Descriptor {
    Array(Vec<LitValue>),
    Value(LitValue),
}

impl Descriptor {
    /// JS truthiness (an array is always truthy).
    pub(crate) fn truthy(&self) -> bool {
        match self {
            Descriptor::Array(_) => true,
            Descriptor::Value(v) => match v {
                LitValue::Str(s) => !s.is_empty(),
                LitValue::Num(n) => *n != 0.0 && !n.is_nan(),
                LitValue::Bool(b) => *b,
                LitValue::Null => false,
                LitValue::BigInt(s) => !s.trim_start_matches('0').is_empty(),
                LitValue::Regex(_) => true,
            },
        }
    }

    /// Upstream `matches(storyKey, arrayOrRegex)`: `includes` for arrays,
    /// else `storyKey.match(value)` (which turns a non-RegExp into one).
    fn matches(&self, key: &str) -> bool {
        match self {
            Descriptor::Array(items) => items
                .iter()
                .any(|i| matches!(i, LitValue::Str(s) if s == key)),
            Descriptor::Value(v) => {
                let source = match v {
                    LitValue::Regex(r) => return r.as_ref().is_some_and(|r| r.is_match(key)),
                    LitValue::Str(s) => s.clone(),
                    LitValue::Num(n) => js_number_to_string(*n),
                    LitValue::Bool(b) => b.to_string(),
                    LitValue::BigInt(s) => s.clone(),
                    LitValue::Null => "null".into(),
                };
                compile_js_regex(&source, "").is_some_and(|r| r.is_match(key))
            }
        }
    }
}

fn js_number_to_string(n: f64) -> String {
    if n.is_nan() {
        "NaN".into()
    } else if n.is_infinite() {
        if n > 0.0 {
            "Infinity".into()
        } else {
            "-Infinity".into()
        }
    } else if n.fract() == 0.0 && n.abs() < 1e21 {
        format!("{}", n as i128)
    } else {
        format!("{n}")
    }
}

/// Compiles a JS regex for the Rust `regex` engine (close enough for the
/// patterns people put in `includeStories` / `excludeStories`).
pub(crate) fn compile_js_regex(pattern: &str, flags: &str) -> Option<regex::Regex> {
    let pattern = if flags.contains('y') {
        format!("^(?:{pattern})")
    } else {
        pattern.to_string()
    };
    regex::RegexBuilder::new(&pattern)
        .case_insensitive(flags.contains('i'))
        .multi_line(flags.contains('m'))
        .dot_matches_new_line(flags.contains('s'))
        .build()
        .ok()
}

fn literal_value(e: &Expression<'_>) -> Option<LitValue> {
    Some(match e.without_parentheses() {
        Expression::StringLiteral(s) => LitValue::Str(lit_str(s).into_owned()),
        Expression::NumericLiteral(n) => LitValue::Num(n.value),
        Expression::BooleanLiteral(b) => LitValue::Bool(b.value),
        Expression::NullLiteral(_) => LitValue::Null,
        Expression::BigIntLiteral(b) => LitValue::BigInt(b.value.to_string()),
        Expression::RegExpLiteral(r) => LitValue::Regex(compile_js_regex(
            r.regex.pattern.text.as_str(),
            &r.regex.flags.to_string(),
        )),
        _ => return None,
    })
}

/// Upstream `getDescriptor(meta, name)`: `Ok(None)` when the property is
/// absent, `Err(())` where upstream throws (non-literal values).
pub(crate) fn get_descriptor(
    meta: &ObjectExpression<'_>,
    name: &str,
) -> Result<Option<Descriptor>, ()> {
    let prop = meta.properties.iter().find_map(|p| match p {
        ObjectPropertyKind::ObjectProperty(p) if key_ident_name(&p.key) == Some(name) => Some(p),
        _ => None,
    });
    let Some(prop) = prop else { return Ok(None) };
    match prop.value.without_parentheses() {
        Expression::ArrayExpression(arr) => {
            let mut items = Vec::with_capacity(arr.elements.len());
            for el in &arr.elements {
                let value = match el {
                    ArrayExpressionElement::Elision(_)
                    | ArrayExpressionElement::SpreadElement(_) => None,
                    el => el.as_expression().and_then(literal_value),
                };
                items.push(value.ok_or(())?);
            }
            Ok(Some(Descriptor::Array(items)))
        }
        e => literal_value(e)
            .map(|v| Some(Descriptor::Value(v)))
            .ok_or(()),
    }
}

#[derive(Default, Debug)]
pub(crate) struct NonStoryExportsConfig {
    pub include_stories: Option<Descriptor>,
    pub exclude_stories: Option<Descriptor>,
}

impl NonStoryExportsConfig {
    /// `{ excludeStories: getDescriptor(...), includeStories: getDescriptor(...) }`,
    /// or `None` where upstream's `try` swallows a throw.
    pub(crate) fn from_meta(meta: &ObjectExpression<'_>) -> Option<Self> {
        let exclude_stories = get_descriptor(meta, "excludeStories").ok()?;
        let include_stories = get_descriptor(meta, "includeStories").ok()?;
        Some(Self {
            include_stories,
            exclude_stories,
        })
    }

    pub(crate) fn has_filter(&self) -> bool {
        self.include_stories
            .as_ref()
            .is_some_and(Descriptor::truthy)
            || self
                .exclude_stories
                .as_ref()
                .is_some_and(Descriptor::truthy)
    }
}

/// `isExportStory(key, { includeStories, excludeStories })`.
pub(crate) fn is_export_story(key: &str, config: &NonStoryExportsConfig) -> bool {
    key != "__esModule"
        && config
            .include_stories
            .as_ref()
            .filter(|d| d.truthy())
            .is_none_or(|d| d.matches(key))
        && config
            .exclude_stories
            .as_ref()
            .filter(|d| d.truthy())
            .is_none_or(|d| !d.matches(key))
}

#[cfg(test)]
mod tests {
    use super::to_start_case;

    #[test]
    fn start_case() {
        assert_eq!(to_start_case("PrimaryButton"), "Primary Button");
        assert_eq!(to_start_case("primary_button"), "Primary Button");
        assert_eq!(to_start_case("Primary"), "Primary");
        assert_eq!(to_start_case("ABCWord"), "ABC Word");
        assert_eq!(to_start_case("story1"), "Story 1");
        assert_eq!(to_start_case("Story1A"), "Story 1 A");
        assert_eq!(to_start_case("a-b.c"), "A B C");
        assert_eq!(to_start_case("__private"), "Private");
    }
}
