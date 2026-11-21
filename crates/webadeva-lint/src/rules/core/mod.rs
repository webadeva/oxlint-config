//! Core rules: the config's own `webadeva/*` rules plus ESLint core and
//! small-plugin rules that oxlint lacks or only approximates.

mod camelcase;
mod getter_return;
mod max_lines;
mod no_dupe_args;
mod no_octal;
mod prefer_arrow_functions;
mod require_description;
mod tsconfig;
mod typescript_file_in_project;
mod util;

use crate::rule::Rule;

pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(max_lines::MAX_LINES),
        Box::new(max_lines::MAX_LINES_WARNING),
        Box::new(typescript_file_in_project::TypescriptFileInProject),
        Box::new(require_description::RequireDescription),
        Box::new(camelcase::Camelcase),
        Box::new(no_dupe_args::NoDupeArgs),
        Box::new(no_octal::NoOctal),
        Box::new(prefer_arrow_functions::PreferArrowFunctions),
        Box::new(getter_return::GetterReturn),
    ]
}
