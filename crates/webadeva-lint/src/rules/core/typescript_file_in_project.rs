//! `webadeva/typescript-file-in-project`: reports TypeScript files no tsconfig
//! includes. In the ESLint config the rule is only switched on for such files
//! (`src/configs/untyped.ts`); here the rule decides itself, porting
//! `hasTypeInformation` from `src/utils/typescript.ts`. The message deliberately
//! differs from the original: under oxlint, tsgolint still types such files
//! (with default compiler options) instead of skipping type-aware rules.

use oxc_span::Span;

use super::tsconfig;
use crate::rule::{Ctx, Rule};

pub struct TypescriptFileInProject;

/// `/\.[cm]?tsx?$/`
fn is_typescript_file(path: &str) -> bool {
    let p = path.strip_suffix('x').unwrap_or(path);
    p.ends_with(".ts") || p.ends_with(".cts") || p.ends_with(".mts")
}

impl Rule for TypescriptFileInProject {
    fn name(&self) -> &'static str {
        "webadeva/typescript-file-in-project"
    }

    fn applies_to(&self, rel_path: &str) -> bool {
        is_typescript_file(rel_path)
    }

    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        let path = tsconfig::normalize(&ctx.root.join(ctx.path));
        let root = tsconfig::normalize(ctx.root);
        if tsconfig::default_project_files(&root).contains(&path) || tsconfig::is_in_project(&path)
        {
            return;
        }
        ctx.report(
            Span::new(0, 0),
            "This file is not included in any tsconfig, so `tsc` does not type-check it and type-aware lint rules use default compiler options. Add it to a tsconfig's `include`.",
        );
    }
}
