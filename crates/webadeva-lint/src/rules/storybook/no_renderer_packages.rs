//! `storybook/no-renderer-packages`: do not import renderer packages
//! directly in stories.

use oxc_ast::ast::ModuleDeclaration;

use super::{is_stories_file, lit_str};
use crate::rule::{Ctx, Rule};

pub struct NoRendererPackages;

const RENDERER_TO_FRAMEWORKS: &[(&str, &[&str])] = &[
    (
        "@storybook/html",
        &["@storybook/html-vite", "@storybook/html-webpack5"],
    ),
    (
        "@storybook/preact",
        &["@storybook/preact-vite", "@storybook/preact-webpack5"],
    ),
    (
        "@storybook/react",
        &[
            "@storybook/nextjs",
            "@storybook/react-vite",
            "@storybook/nextjs-vite",
            "@storybook/react-webpack5",
            "@storybook/react-native-web-vite",
        ],
    ),
    ("@storybook/server", &["@storybook/server-webpack5"]),
    (
        "@storybook/svelte",
        &[
            "@storybook/svelte-vite",
            "@storybook/svelte-webpack5",
            "@storybook/sveltekit",
        ],
    ),
    (
        "@storybook/vue3",
        &["@storybook/vue3-vite", "@storybook/vue3-webpack5"],
    ),
    (
        "@storybook/web-components",
        &[
            "@storybook/web-components-vite",
            "@storybook/web-components-webpack5",
        ],
    ),
];

impl Rule for NoRendererPackages {
    fn name(&self) -> &'static str {
        "storybook/no-renderer-packages"
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
            if let Some((pkg, frameworks)) =
                RENDERER_TO_FRAMEWORKS.iter().find(|(r, _)| *r == source)
            {
                ctx.report(
                    import.span,
                    format!(
                        "Do not import renderer package \"{pkg}\" directly. Use a framework package instead (e.g. {}).",
                        frameworks.join(", ")
                    ),
                );
            }
        }
    }
}
