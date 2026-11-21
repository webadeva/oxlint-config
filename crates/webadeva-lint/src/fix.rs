//! Autofixes, applied the way `eslint --fix` applies them: per pass, fixes
//! sorted by position, every fix that doesn't overlap an earlier one is
//! applied, and the file is re-linted until nothing changes (10 passes max).

use oxc_span::Span;

/// One text replacement in the original source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub start: u32,
    pub end: u32,
    pub text: String,
}

impl Edit {
    pub fn replace(span: Span, text: impl Into<String>) -> Self {
        Self {
            start: span.start,
            end: span.end,
            text: text.into(),
        }
    }

    pub fn insert_before(offset: u32, text: impl Into<String>) -> Self {
        Self {
            start: offset,
            end: offset,
            text: text.into(),
        }
    }
}

/// A fix: one or more edits applied together, like an ESLint fixer that
/// returns several fixes (ESLint merges those into one fix covering their
/// whole range, so that range is what overlap checks use).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fix {
    pub edits: Vec<Edit>,
}

impl Fix {
    pub fn new(mut edits: Vec<Edit>) -> Self {
        edits.sort_by_key(|e| (e.start, e.end));
        Self { edits }
    }

    fn range(&self) -> (u32, u32) {
        let start = self.edits.iter().map(|e| e.start).min().unwrap_or(0);
        let end = self.edits.iter().map(|e| e.end).max().unwrap_or(0);
        (start, end)
    }
}

/// ESLint's maximum number of fix passes per file.
pub const MAX_PASSES: usize = 10;

/// Applies the non-overlapping subset of `fixes` to `source`, in ESLint's
/// order (by range start, then end). Returns `None` when nothing applied.
pub fn apply(source: &str, fixes: &[&Fix]) -> Option<String> {
    let mut sorted: Vec<&Fix> = fixes
        .iter()
        .copied()
        .filter(|f| !f.edits.is_empty())
        .collect();
    sorted.sort_by_key(|f| f.range());

    let mut out = String::with_capacity(source.len());
    let mut cursor = 0usize;
    // ESLint's source-code-fixer: a fix is skipped this pass when
    // `lastPos >= start`, so even a fix that only touches the previous one
    // waits for the next pass.
    let mut last_end: Option<u32> = None;
    let mut applied = false;
    for fix in sorted {
        let (start, end) = fix.range();
        if let Some(last) = last_end
            && start <= last
        {
            continue;
        }
        if end as usize > source.len() {
            continue;
        }
        for edit in &fix.edits {
            let (s, e) = (edit.start as usize, edit.end as usize);
            if s < cursor {
                continue;
            }
            out.push_str(&source[cursor..s]);
            out.push_str(&edit.text);
            cursor = e;
        }
        last_end = Some(end);
        applied = true;
    }
    if !applied {
        return None;
    }
    out.push_str(&source[cursor..]);
    (out != source).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fix(start: u32, end: u32, text: &str) -> Fix {
        Fix::new(vec![Edit::replace(Span::new(start, end), text)])
    }

    #[test]
    fn applies_non_overlapping_in_order() {
        let a = fix(0, 1, "A");
        let b = fix(2, 3, "C");
        let overlapping = fix(0, 2, "zz");
        assert_eq!(
            apply("abc", &[&b, &overlapping, &a]).as_deref(),
            Some("AbC")
        );
    }

    #[test]
    fn touching_fix_waits_for_next_pass() {
        let a = fix(0, 1, "A");
        let b = fix(1, 2, "B");
        assert_eq!(apply("abc", &[&a, &b]).as_deref(), Some("Abc"));
    }

    #[test]
    fn multi_edit_fix_is_atomic() {
        let f = Fix::new(vec![
            Edit::insert_before(0, "<"),
            Edit::insert_before(3, ">"),
        ]);
        assert_eq!(apply("abc", &[&f]).as_deref(), Some("<abc>"));
    }

    #[test]
    fn nothing_applied() {
        assert_eq!(apply("abc", &[]), None);
    }
}
