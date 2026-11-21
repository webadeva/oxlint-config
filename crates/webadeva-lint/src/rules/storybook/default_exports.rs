//! `storybook/default-exports`: story files should have a default export.

use oxc_ast::ast::{Expression, ImportDeclarationSpecifier, ModuleDeclaration, Program, Statement};

use super::{first_non_import_span, has_stories_of_import, is_stories_file, member_parts};
use crate::fix::Edit;
use crate::rule::{Ctx, Rule};

pub struct DefaultExports;

/// Upstream `getComponentName`: the file's base name up to the first `.`,
/// when some import from `./<name>...` binds a local of that name.
fn component_name<'a>(program: &Program<'_>, path: &'a std::path::Path) -> Option<&'a str> {
    let base = path.file_name()?.to_str()?;
    let name = base.split('.').next().unwrap_or(base);
    let prefix = format!("./{name}");
    program
        .body
        .iter()
        .any(|s| match s {
            Statement::ImportDeclaration(d) => {
                d.source.value.starts_with(prefix.as_str())
                    && d.specifiers.as_ref().is_some_and(|specs| {
                        specs.iter().any(|sp| {
                            let local = match sp {
                                ImportDeclarationSpecifier::ImportSpecifier(sp) => &sp.local,
                                ImportDeclarationSpecifier::ImportDefaultSpecifier(sp) => &sp.local,
                                ImportDeclarationSpecifier::ImportNamespaceSpecifier(sp) => {
                                    &sp.local
                                }
                            };
                            local.name == name
                        })
                    })
            }
            _ => false,
        })
        .then_some(name)
}

impl Rule for DefaultExports {
    fn name(&self) -> &'static str {
        "storybook/default-exports"
    }

    fn applies_to(&self, rel_path: &str) -> bool {
        is_stories_file(rel_path)
    }

    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        let program = ctx.program;
        // `export default` can only appear at the top level.
        let has_default_export = program.body.iter().any(|s| {
            matches!(
                s.as_module_declaration(),
                Some(ModuleDeclaration::ExportDefaultDeclaration(_))
            )
        });
        if has_default_export || has_stories_of_import(program) {
            return;
        }
        // CSF4: a top-level `const meta = preview.meta({...})`.
        let is_csf4 = program.body.iter().any(|s| {
            let Statement::VariableDeclaration(v) = s else { return false };
            v.declarations.iter().any(|d| {
                matches!(
                    d.init.as_ref().map(Expression::without_parentheses),
                    Some(Expression::CallExpression(c)) if matches!(member_parts(&c.callee), Some((_, Some("meta"))))
                )
            })
        });
        if is_csf4 {
            return;
        }
        let span = first_non_import_span(program);
        let meta = match component_name(program, ctx.path) {
            Some(name) => format!("export default {{ component: {name} }}\n"),
            None => "export default {}\n".to_string(),
        };
        ctx.report_with_fix(
            span,
            "The file should have a default export.",
            vec![Edit::insert_before(span.start, meta)],
        );
    }
}
