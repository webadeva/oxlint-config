//! Walks files, resolves the rules that apply to each, parses once with oxc,
//! and runs the rules in parallel.

use std::fs;
use std::path::{Path, PathBuf};

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use ignore::{Match, WalkBuilder};
use rayon::prelude::*;
use serde_json::Value;

use crate::config::{Config, RuleSetting, parse_rule_value};
use crate::diagnostic::{Diagnostic, LineIndex, Located, locate};
use crate::directives;
use crate::engine::{self, ActiveRule};
use crate::rule::Rule;

pub const EXTENSIONS: &[&str] = &["js", "mjs", "cjs", "jsx", "ts", "mts", "cts", "tsx"];

pub struct Linter<'r> {
    pub config: Config,
    pub rules: &'r [Box<dyn Rule>],
    ignore: Gitignore,
}

pub struct FileResult {
    pub diagnostics: Vec<Located>,
}

impl<'r> Linter<'r> {
    pub fn new(config: Config, rules: &'r [Box<dyn Rule>]) -> Result<Self, String> {
        // Validate every configured rule once, up front.
        let known = |name: &str| rules.iter().find(|r| r.name() == name);
        let check =
            |map: &serde_json::Map<String, Value>, settings: &Value| -> Result<(), String> {
                for (name, value) in map {
                    let rule = known(name).ok_or_else(|| {
                        format!("webadeva-lint: unknown rule {name:?} in settings.webadeva.rules")
                    })?;
                    let setting = parse_rule_value(value)
                        .map_err(|e| format!("webadeva-lint: {name}: {e}"))?;
                    if setting.severity.is_some() {
                        rule.validate(&setting.options, settings)
                            .map_err(|e| format!("webadeva-lint: {name}: {e}"))?;
                    }
                }
                Ok(())
            };
        check(&config.rules, &config.settings)?;
        for o in &config.overrides {
            let mut settings = config.settings.clone();
            if !o.settings.is_null() {
                crate::config::deep_merge(&mut settings, &o.settings);
            }
            // Base rules must also be valid under an override's settings.
            let mut merged = config.rules.clone();
            merged.extend(o.rules.clone());
            check(&merged, &settings)?;
        }

        let mut builder = GitignoreBuilder::new(&config.root);
        for p in &config.ignore_patterns {
            builder
                .add_line(None, p)
                .map_err(|e| format!("webadeva-lint: invalid ignore pattern {p:?}: {e}"))?;
        }
        let ignore = builder.build().map_err(|e| e.to_string())?;
        Ok(Self {
            config,
            rules,
            ignore,
        })
    }

    fn rel(&self, path: &Path) -> String {
        let rel = path.strip_prefix(&self.config.root).unwrap_or(path);
        rel.to_string_lossy().replace('\\', "/")
    }

    /// Ignored by `ignorePatterns`; dot-directories are skipped unless a
    /// negated pattern (e.g. `!.storybook`) re-includes them, like oxlint.
    fn is_ignored(&self, path: &Path, is_dir: bool) -> bool {
        let rel = path.strip_prefix(&self.config.root).unwrap_or(path);
        match self.ignore.matched_path_or_any_parents(rel, is_dir) {
            Match::Ignore(_) => true,
            Match::Whitelist(_) => false,
            Match::None => rel.components().any(|c| {
                let s = c.as_os_str().to_string_lossy();
                s.starts_with('.') && s != "." && s != ".."
            }),
        }
    }

    pub fn collect_files(&self, paths: &[PathBuf]) -> Vec<PathBuf> {
        let mut files = Vec::new();
        for path in paths {
            let path = if path.is_absolute() {
                path.clone()
            } else {
                self.config.root.join(path)
            };
            if path.is_file() {
                // Explicitly named files are linted unless ignored.
                if !self.is_ignored(&path, false) {
                    files.push(path);
                }
                continue;
            }
            let walker = WalkBuilder::new(&path)
                .hidden(false)
                .git_ignore(true)
                .git_exclude(true)
                .require_git(false)
                .filter_entry({
                    let root = self.config.root.clone();
                    move |e| {
                        let name = e.file_name().to_string_lossy();
                        !(name == "node_modules" || name == ".git") || e.path() == root
                    }
                })
                .build();
            for entry in walker.flatten() {
                let p = entry.path();
                let is_file = entry.file_type().is_some_and(|t| t.is_file());
                if !is_file {
                    continue;
                }
                let ext_ok = p
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| EXTENSIONS.contains(&e));
                if ext_ok && !self.is_ignored(p, false) {
                    files.push(p.to_path_buf());
                }
            }
        }
        files.sort();
        files.dedup();
        files
    }

    pub fn lint_files(&self, files: &[PathBuf], fix: bool) -> Vec<FileResult> {
        files.par_iter().map(|f| self.lint_file(f, fix)).collect()
    }

    /// The rules enabled for a file, resolved before reading or parsing it.
    fn active_rules(&self, rel: &str) -> (Vec<(&dyn Rule, RuleSetting)>, Value) {
        let (rule_values, settings) = self.config.resolve(rel);
        let mut active = Vec::new();
        for rule in self.rules {
            let Some(value) = rule_values.get(rule.name()) else {
                continue;
            };
            let Ok(setting) = parse_rule_value(value) else {
                continue;
            };
            if setting.severity.is_some() && rule.applies_to(rel) {
                active.push((rule.as_ref(), setting));
            }
        }
        (active, settings)
    }

    /// Lints a file; with `fix`, applies autofixes the way `eslint --fix`
    /// does (re-linting until nothing changes, up to 10 passes) and writes
    /// the file when it changed. Reports what's left.
    pub fn lint_file(&self, path: &Path, fix: bool) -> FileResult {
        let rel = self.rel(path);
        let (active, settings) = self.active_rules(&rel);
        let empty = FileResult {
            diagnostics: Vec::new(),
        };
        if active.is_empty() {
            return empty;
        }
        let Ok(original) = fs::read_to_string(path) else {
            return empty;
        };

        let mut source = original.clone();
        let mut diagnostics = self.lint_source(path, &source, &active, &settings);
        if fix {
            for _ in 0..crate::fix::MAX_PASSES {
                let fixes: Vec<&crate::fix::Fix> =
                    diagnostics.iter().filter_map(|d| d.fix.as_ref()).collect();
                let Some(next) = crate::fix::apply(&source, &fixes) else {
                    break;
                };
                source = next;
                diagnostics = self.lint_source(path, &source, &active, &settings);
            }
        }
        if source != original && fs::write(path, &source).is_err() {
            eprintln!("webadeva-lint: could not write fixes to {rel}");
        }

        let index = LineIndex::new(&source);
        let mut located: Vec<Located> = diagnostics
            .iter()
            .map(|d| locate(&rel, &index, d))
            .collect();
        located.sort_by(|a, b| (a.line, a.column, &a.rule).cmp(&(b.line, b.column, &b.rule)));
        FileResult {
            diagnostics: located,
        }
    }

    /// Runs the active rules on `source`, dropping diagnostics silenced by
    /// disable directives (as ESLint does). Diagnostics keep their fixes.
    fn lint_source(
        &self,
        path: &Path,
        source: &str,
        active: &[(&dyn Rule, RuleSetting)],
        settings: &Value,
    ) -> Vec<Diagnostic> {
        let active: Vec<ActiveRule<'_>> = active
            .iter()
            .map(|(rule, setting)| ActiveRule {
                rule: *rule,
                severity: setting.severity.expect("active rules are on"),
                options: &setting.options,
            })
            .collect();
        let mut linted = engine::run(path, &self.config.root, source, &active, settings);
        if !linted.diagnostics.is_empty() {
            let index = LineIndex::new(source);
            linted.diagnostics.retain(|d| {
                !directives::is_suppressed(&linted.directives, &index, d.rule, d.start as usize)
            });
        }
        linted.diagnostics
    }
}
