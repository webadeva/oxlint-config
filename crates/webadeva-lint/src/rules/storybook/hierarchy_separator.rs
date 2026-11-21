//! `storybook/hierarchy-separator`: deprecated `|` separator in meta titles.

use oxc_ast::ast::{ModuleDeclaration, ObjectPropertyKind};
use oxc_span::GetSpan;

use super::{is_literal, is_stories_file, key_ident_name, meta_object};
use crate::fix::Edit;
use crate::rule::{Ctx, Rule};

pub struct HierarchySeparator;

impl Rule for HierarchySeparator {
    fn name(&self) -> &'static str {
        "storybook/hierarchy-separator"
    }

    fn applies_to(&self, rel_path: &str) -> bool {
        is_stories_file(rel_path)
    }

    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        for stmt in &ctx.program.body {
            let Some(ModuleDeclaration::ExportDefaultDeclaration(decl)) =
                stmt.as_module_declaration()
            else {
                continue;
            };
            let Some(meta) = meta_object(ctx.semantic, decl) else {
                continue;
            };
            let title = meta.properties.iter().find_map(|p| match p {
                ObjectPropertyKind::ObjectProperty(p)
                    if key_ident_name(&p.key) == Some("title") =>
                {
                    Some(p)
                }
                _ => None,
            });
            let Some(title) = title else { continue };
            if !is_literal(&title.value) {
                continue;
            }
            let value_span = title.value.without_parentheses().span();
            let raw = ctx.source_text(value_span);
            if raw.contains('|') {
                ctx.report_with_fix(
                    title.span,
                    format!("Deprecated hierarchy separator in title property: {raw}."),
                    vec![Edit::replace(value_span, raw.replace('|', "/"))],
                );
            }
        }
    }
}
