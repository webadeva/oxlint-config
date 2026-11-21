//! `storybook/use-storybook-expect`: use `expect` from `storybook/test`.

use oxc_ast::AstKind;
use oxc_ast::ast::{Expression, ImportDeclarationSpecifier, ModuleDeclaration};

use super::{is_stories_file, lit_str, module_export_ident};
use crate::rule::{Ctx, Rule};

pub struct UseStorybookExpect;

impl Rule for UseStorybookExpect {
    fn name(&self) -> &'static str {
        "storybook/use-storybook-expect"
    }

    fn applies_to(&self, rel_path: &str) -> bool {
        is_stories_file(rel_path)
    }

    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        let imports_expect = ctx.program.body.iter().any(|s| {
            let Some(ModuleDeclaration::ImportDeclaration(d)) = s.as_module_declaration() else {
                return false;
            };
            matches!(
                &*lit_str(&d.source),
                "@storybook/jest" | "@storybook/test" | "storybook/test"
            ) && d.specifiers.as_ref().is_some_and(|specs| {
                specs.iter().any(|sp| {
                    matches!(sp, ImportDeclarationSpecifier::ImportSpecifier(sp)
                            if module_export_ident(&sp.imported) == Some("expect"))
                })
            })
        });
        if imports_expect {
            return;
        }
        for node in ctx.semantic.nodes().iter() {
            if let AstKind::CallExpression(call) = node.kind()
                && let Expression::Identifier(id) = call.callee.without_parentheses()
                && id.name == "expect"
            {
                ctx.report(id.span, "Do not use global expect directly in the story. You should import it from `@storybook/test` (preferrably) or `@storybook/jest` instead.");
            }
        }
    }
}
