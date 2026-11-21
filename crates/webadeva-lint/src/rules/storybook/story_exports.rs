//! `storybook/story-exports`: a story file must contain at least one story
//! export.

use oxc_ast::AstKind;
use oxc_ast::ast::{Declaration, ExportSpecifier, FunctionType};

use super::csf::{NonStoryExportsConfig, is_export_story};
use super::{
    exported_first_var, first_non_import_span, has_stories_of_import, is_stories_file, meta_object,
    module_export_ident,
};
use crate::rule::{Ctx, Rule};

pub struct StoryExports;

fn specifier_names<'a>(specs: &'a [ExportSpecifier<'a>], out: &mut Vec<&'a str>) {
    out.extend(
        specs
            .iter()
            .filter_map(|s| module_export_ident(&s.exported)),
    );
}

impl Rule for StoryExports {
    fn name(&self) -> &'static str {
        "storybook/story-exports"
    }

    fn applies_to(&self, rel_path: &str) -> bool {
        is_stories_file(rel_path)
    }

    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        if has_stories_of_import(ctx.program) {
            return;
        }
        let mut config = NonStoryExportsConfig::default();
        let mut has_meta = false;
        let mut named_exports: Vec<&str> = Vec::new();
        for node in ctx.semantic.nodes().iter() {
            match node.kind() {
                AstKind::ExportDefaultDeclaration(decl) => {
                    if let Some(meta) = meta_object(ctx.semantic, decl) {
                        has_meta = true;
                        if let Some(c) = NonStoryExportsConfig::from_meta(meta) {
                            config = c;
                        }
                    } else {
                        has_meta = false;
                    }
                }
                // Upstream `getAllNamedExports`.
                AstKind::ExportDeclaration(export) => match &export.declaration {
                    Declaration::FunctionDeclaration(f)
                        if f.r#type == FunctionType::FunctionDeclaration =>
                    {
                        named_exports.extend(f.id.as_ref().map(|id| id.name.as_str()));
                    }
                    decl => named_exports
                        .extend(exported_first_var(decl).map(|(id, _)| id.name.as_str())),
                },
                AstKind::ExportNamedDeclaration(export) => {
                    specifier_names(&export.specifiers, &mut named_exports)
                }
                AstKind::ExportFromDeclaration(export) => {
                    specifier_names(&export.specifiers, &mut named_exports)
                }
                _ => {}
            }
        }
        if !has_meta
            || named_exports
                .iter()
                .any(|name| is_export_story(name, &config) && *name != "__namedExportsOrder")
        {
            return;
        }
        let message = if config.has_filter() {
            "The file should have at least one story export. Make sure the includeStories/excludeStories you defined are correct, otherwise Storybook will not use any stories for this file."
        } else {
            "The file should have at least one story export"
        };
        ctx.report(first_non_import_span(ctx.program), message);
    }
}
