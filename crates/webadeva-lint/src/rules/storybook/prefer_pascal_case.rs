//! `storybook/prefer-pascal-case`: stories should use PascalCase.

use oxc_ast::AstKind;

use super::csf::{NonStoryExportsConfig, is_export_story};
use super::{exported_first_var, has_stories_of_import, is_stories_file, meta_object};
use crate::rule::{Ctx, Rule};

pub struct PreferPascalCase;

/// `/^[A-Z]+([a-z0-9]?)+/`: starts with an ASCII capital.
fn is_pascal_case(name: &str) -> bool {
    name.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
}

impl Rule for PreferPascalCase {
    fn name(&self) -> &'static str {
        "storybook/prefer-pascal-case"
    }

    fn applies_to(&self, rel_path: &str) -> bool {
        is_stories_file(rel_path)
    }

    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        if has_stories_of_import(ctx.program) {
            return;
        }
        let mut config = NonStoryExportsConfig::default();
        let mut named_exports = Vec::new();
        for node in ctx.semantic.nodes().iter() {
            match node.kind() {
                AstKind::ExportDefaultDeclaration(decl) => {
                    if let Some(meta) = meta_object(ctx.semantic, decl)
                        && let Some(c) = NonStoryExportsConfig::from_meta(meta)
                    {
                        config = c;
                    }
                }
                AstKind::ExportDeclaration(export) => {
                    if let Some((id, _)) = exported_first_var(&export.declaration) {
                        named_exports.push(id);
                    }
                }
                _ => {}
            }
        }
        for id in named_exports {
            let name = id.name.as_str();
            if !is_export_story(name, &config) || name == "__namedExportsOrder" {
                continue;
            }
            if !name.starts_with('_') && !is_pascal_case(name) {
                ctx.report(
                    id.span,
                    format!("The story should use PascalCase notation: {name}"),
                );
            }
        }
    }
}
