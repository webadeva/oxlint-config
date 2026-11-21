//! `storybook/use-storybook-testing-library`: do not use testing-library
//! directly in stories.

use oxc_ast::ast::{ImportDeclaration, ImportDeclarationSpecifier, ModuleDeclaration};
use oxc_span::{GetSpan, Span};

use super::{is_stories_file, lit_str};
use crate::fix::Edit;
use crate::rule::{Ctx, Rule};

pub struct UseStorybookTestingLibrary;

/// JavaScript's `\s`.
fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

/// Upstream `fixSpecifiers`: drop the first `{` and `}`, collapse runs of
/// two or more whitespace characters, trim, and wrap in braces.
fn fix_specifiers(text: &str) -> String {
    let text = text.replacen('{', "", 1).replacen('}', "", 1);
    let mut collapsed = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if is_js_space(c) && chars.peek().is_some_and(|&n| is_js_space(n)) {
            while chars.peek().is_some_and(|&n| is_js_space(n)) {
                chars.next();
            }
            collapsed.push(' ');
        } else {
            collapsed.push(c);
        }
    }
    format!("{{ {} }}", collapsed.trim_matches(is_js_space))
}

/// Upstream `getSpecifiers`: from the first specifier to the last one,
/// extended to a closing `}` found from one character before its end (as
/// long as it lies within the import).
fn specifiers_span(source: &str, import: &ImportDeclaration<'_>) -> Option<Span> {
    let specs = import.specifiers.as_ref()?;
    let start = specs.first()?.span().start;
    let mut end = specs.last()?.span().end;
    let from = end.saturating_sub(1) as usize;
    if let Some(i) = source[from..].find('}') {
        let brace = (from + i) as u32;
        if brace <= import.span.end {
            end = brace + 1;
        }
    }
    Some(Span::new(start, end))
}

impl Rule for UseStorybookTestingLibrary {
    fn name(&self) -> &'static str {
        "storybook/use-storybook-testing-library"
    }

    fn applies_to(&self, rel_path: &str) -> bool {
        is_stories_file(rel_path)
    }

    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        for stmt in &ctx.program.body {
            let Some(ModuleDeclaration::ImportDeclaration(import)) = stmt.as_module_declaration()
            else {
                continue;
            };
            let source = lit_str(&import.source);
            if !source.contains("@testing-library") {
                continue;
            }
            let src = import.source.span;
            let mut edits = vec![Edit::replace(
                Span::new(src.start + 1, src.end - 1),
                "storybook/test",
            )];
            let has_default = import.specifiers.as_ref().is_some_and(|specs| {
                specs
                    .iter()
                    .any(|s| matches!(s, ImportDeclarationSpecifier::ImportDefaultSpecifier(_)))
            });
            if has_default && let Some(span) = specifiers_span(ctx.source, import) {
                edits.push(Edit::replace(span, fix_specifiers(ctx.source_text(span))));
            }
            ctx.report_with_fix(
                import.span,
                format!(
                    "Do not use `{source}` directly in the story. You should import the functions from `storybook/test` instead."
                ),
                edits,
            );
        }
    }
}
