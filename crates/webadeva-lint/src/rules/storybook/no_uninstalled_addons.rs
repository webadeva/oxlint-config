//! `storybook/no-uninstalled-addons`: addons listed in `.storybook/main.*`
//! must be installed (present in the project's package.json).

use std::path::{Component, MAIN_SEPARATOR, MAIN_SEPARATOR_STR, Path, PathBuf};

use oxc_ast::AstKind;
use oxc_ast::ast::{ArrayExpression, BindingPattern, Declaration, Expression, ObjectExpression};
use oxc_span::{GetSpan, Span};
use serde_json::Value;

use super::{find_ident_prop, is_storybook_main, meta_object, string_literal};
use crate::rule::{Ctx, Rule};

pub struct NoUninstalledAddons;

struct Options {
    package_json_location: String,
    ignore: Vec<String>,
}

/// `context.options.reduce(...)` over the option objects.
fn options(values: &[Value]) -> Options {
    let mut o = Options {
        package_json_location: String::new(),
        ignore: Vec::new(),
    };
    for v in values {
        if let Some(loc) = v
            .get("packageJsonLocation")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        {
            o.package_json_location = loc.to_string();
        }
        if let Some(ignore) = v.get("ignore").and_then(Value::as_array) {
            o.ignore = ignore
                .iter()
                .filter_map(|s| s.as_str().map(str::to_string))
                .collect();
        }
    }
    o
}

/// Lexical `path.resolve(base, p)`.
fn resolve(base: &Path, p: &str) -> PathBuf {
    let joined = if Path::new(p).is_absolute() {
        PathBuf::from(p)
    } else {
        base.join(p)
    };
    let mut out = PathBuf::new();
    for c in joined.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            c => out.push(c),
        }
    }
    out
}

/// `path.relative(from, to)` for absolute, normalised paths.
fn relative(from: &Path, to: &Path) -> String {
    let a: Vec<_> = from.components().collect();
    let b: Vec<_> = to.components().collect();
    let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let mut parts: Vec<String> = vec!["..".into(); a.len() - common];
    parts.extend(
        b[common..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    );
    parts.join(MAIN_SEPARATOR_STR)
}

/// Strips `.js`/`.mjs`/`.cjs`, then `/register`, then `/preset`.
fn addon_package_name(addon: &str) -> &str {
    let mut name = addon;
    for ext in [".mjs", ".cjs", ".js"] {
        if let Some(stripped) = name.strip_suffix(ext) {
            name = stripped;
            break;
        }
    }
    let name = name.strip_suffix("/register").unwrap_or(name);
    name.strip_suffix("/preset").unwrap_or(name)
}

/// Local paths (`./x`, `/x`, `C:...`, `\x`) are not packages. `/\w:.*/`
/// matches a word character followed by `:` anywhere in the string.
fn is_local_addon(addon: &str) -> bool {
    let bytes = addon.as_bytes();
    addon.starts_with('.')
        || addon.starts_with('/')
        || addon.starts_with('\\')
        || bytes
            .windows(2)
            .any(|w| (w[0].is_ascii_alphanumeric() || w[0] == b'_') && w[1] == b':')
}

struct Checker<'o> {
    options: &'o Options,
    cwd: PathBuf,
    /// `Ok(deps)` or `Err(message)` when package.json can't be read;
    /// loaded on first use.
    deps: Option<Result<Vec<String>, String>>,
    reports: Vec<(Span, String)>,
}

impl Checker<'_> {
    fn load_deps(&self) -> Result<Vec<String>, String> {
        let location = if self.options.package_json_location.is_empty() {
            "./package.json"
        } else {
            &self.options.package_json_location
        };
        let path = resolve(&self.cwd, location);
        let parsed = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .ok_or_else(|| {
                format!(
                    "The provided path in your eslintrc.json - {} is not a valid path to a package.json file or your package.json file is not in the same folder as ESLint is running from.",
                    path.display()
                )
            })?;
        let keys = |field: &str| -> Vec<String> {
            parsed
                .get(field)
                .and_then(Value::as_object)
                .map(|m| m.keys().cloned().collect())
                .unwrap_or_default()
        };
        let mut deps = keys("dependencies");
        deps.extend(keys("devDependencies"));
        Ok(deps)
    }

    fn package_json_display(&self) -> String {
        let dir_name = self
            .cwd
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let target = resolve(&self.cwd, &self.options.package_json_location);
        format!("{dir_name}{MAIN_SEPARATOR}{}", relative(&self.cwd, &target))
    }

    /// Upstream `reportUninstalledAddons`.
    fn check_array(&mut self, arr: &ArrayExpression<'_>) {
        if self.deps.is_none() {
            self.deps = Some(self.load_deps());
        }
        let deps = match self.deps.as_ref().expect("loaded") {
            Ok(deps) => deps,
            Err(message) => {
                self.reports.push((arr.span, message.clone()));
                return;
            }
        };
        // String elements first, then `{ name: '...' }` elements.
        let mut addons: Vec<(String, Span)> = Vec::new();
        for el in &arr.elements {
            if let Some(e) = el.as_expression()
                && let Some(s) = string_literal(e)
            {
                addons.push((s.into_owned(), e.without_parentheses().span()));
            }
        }
        for el in &arr.elements {
            if let Some(Expression::ObjectExpression(obj)) =
                el.as_expression().map(Expression::without_parentheses)
                && let Some(prop) = find_ident_prop(obj, "name")
                && let Some(s) = string_literal(&prop.value)
            {
                addons.push((s.into_owned(), prop.value.without_parentheses().span()));
            }
        }
        let package_json = self.package_json_display();
        for (addon, span) in addons {
            let installed = deps.iter().any(|d| d == addon_package_name(&addon));
            if is_local_addon(&addon) || installed || self.options.ignore.contains(&addon) {
                continue;
            }
            self.reports.push((
                span,
                format!(
                    "The {addon} is not installed in {package_json}. Did you forget to install it or is your package.json in a different location?"
                ),
            ));
        }
    }

    /// Upstream `findAddonsPropAndReport`.
    fn check_object(&mut self, obj: &ObjectExpression<'_>) {
        if let Some(prop) = find_ident_prop(obj, "addons")
            && let Expression::ArrayExpression(arr) = prop.value.without_parentheses()
        {
            self.check_array(arr);
        }
    }
}

impl Rule for NoUninstalledAddons {
    fn name(&self) -> &'static str {
        "storybook/no-uninstalled-addons"
    }

    fn applies_to(&self, rel_path: &str) -> bool {
        is_storybook_main(rel_path)
    }

    fn validate(&self, options: &[Value], _settings: &Value) -> Result<(), String> {
        if options.len() > 1 {
            return Err("expected at most one options object".into());
        }
        let Some(opt) = options.first() else {
            return Ok(());
        };
        let obj = opt.as_object().ok_or("options must be an object")?;
        for (key, value) in obj {
            match key.as_str() {
                "packageJsonLocation" if value.is_string() => {}
                "ignore"
                    if value
                        .as_array()
                        .is_some_and(|a| a.iter().all(Value::is_string)) => {}
                "packageJsonLocation" => return Err("packageJsonLocation must be a string".into()),
                "ignore" => return Err("ignore must be an array of strings".into()),
                other => return Err(format!("unknown option {other:?}")),
            }
        }
        Ok(())
    }

    fn run<'a>(&self, ctx: &mut Ctx<'a>) {
        let options = options(ctx.options);
        // Upstream resolves against `process.cwd()`.
        let cwd = std::env::current_dir().unwrap_or_else(|_| ctx.root.to_path_buf());
        let mut checker = Checker {
            options: &options,
            cwd,
            deps: None,
            reports: Vec::new(),
        };
        for node in ctx.semantic.nodes().iter() {
            match node.kind() {
                AstKind::AssignmentExpression(assign) => {
                    if let Expression::ObjectExpression(obj) = assign.right.without_parentheses() {
                        checker.check_object(obj);
                    }
                }
                AstKind::ExportDefaultDeclaration(decl) => {
                    if let Some(meta) = meta_object(ctx.semantic, decl) {
                        checker.check_object(meta);
                    }
                }
                AstKind::ExportDeclaration(export) => {
                    let Declaration::VariableDeclaration(v) = &export.declaration else {
                        continue;
                    };
                    let addons = v.declarations.iter().find(
                        |d| matches!(&d.id, BindingPattern::BindingIdentifier(id) if id.name == "addons"),
                    );
                    if let Some(Expression::ArrayExpression(arr)) = addons
                        .and_then(|d| d.init.as_ref())
                        .map(Expression::without_parentheses)
                    {
                        checker.check_array(arr);
                    }
                }
                _ => {}
            }
        }
        for (span, message) in checker.reports {
            ctx.report(span, message);
        }
    }
}
