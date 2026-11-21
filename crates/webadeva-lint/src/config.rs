//! Reads our section out of the project's `.oxlintrc.json`.
//!
//! Our rules live under `settings.webadeva`, because oxlint rejects rule ids
//! of plugins it doesn't know but passes `settings` through untouched:
//!
//! ```jsonc
//! {
//!   "extends": ["./node_modules/@webadeva/oxlint-config/presets/react.json"],
//!   "settings": {
//!     "webadeva": {
//!       "rules": { "camelcase": "off" },
//!       "maxLines": { "warn": 250, "error": 300 },
//!       "overrides": [
//!         { "files": ["**/*.test.ts"], "settings": { "webadeva": { "maxLines": false } } }
//!       ]
//!     }
//!   }
//! }
//! ```
//!
//! Unlike oxlint (whose `extends` drops `settings` and `ignorePatterns`), we
//! merge everything along `extends`, so presets carry their full config.

use std::fs;
use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};
use serde_json::{Map, Value};

use crate::diagnostic::Severity;

#[derive(Debug, Clone)]
pub struct RuleSetting {
    pub severity: Option<Severity>, // None = off
    pub options: Vec<Value>,
}

#[derive(Debug)]
pub struct Override {
    pub files: GlobSet,
    pub exclude: GlobSet,
    pub rules: Map<String, Value>,
    pub settings: Value,
}

#[derive(Debug)]
pub struct Config {
    pub root: PathBuf,
    pub rules: Map<String, Value>,
    /// The whole merged `settings` object, as ESLint passes it to rules:
    /// `settings.webadeva` holds our settings (minus `rules`/`overrides`),
    /// next to e.g. `settings.react.version`.
    pub settings: Value,
    pub overrides: Vec<Override>,
    pub ignore_patterns: Vec<String>,
}

pub fn parse_rule_value(v: &Value) -> Result<RuleSetting, String> {
    let (sev, options) = match v {
        Value::Array(items) if !items.is_empty() => (&items[0], items[1..].to_vec()),
        other => (other, Vec::new()),
    };
    let severity = match sev {
        Value::String(s) => match s.as_str() {
            "off" | "allow" => None,
            "warn" => Some(Severity::Warning),
            "error" | "deny" => Some(Severity::Error),
            _ => return Err(format!("invalid severity {s:?}")),
        },
        Value::Number(n) => match n.as_u64() {
            Some(0) => None,
            Some(1) => Some(Severity::Warning),
            Some(2) => Some(Severity::Error),
            _ => return Err(format!("invalid severity {n}")),
        },
        other => return Err(format!("invalid severity {other}")),
    };
    Ok(RuleSetting { severity, options })
}

fn read_json(path: &Path) -> Result<Value, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let stripped = json_comments::StripComments::new(raw.as_bytes());
    let value: Value = serde_json::from_reader(stripped)
        .map_err(|e| format!("{}: invalid JSON: {e}", path.display()))?;
    Ok(value)
}

/// Deep-merges `b` into `a` (objects merge recursively; anything else replaces).
pub fn deep_merge(a: &mut Value, b: &Value) {
    match (a, b) {
        (Value::Object(a), Value::Object(b)) => {
            for (k, v) in b {
                match a.get_mut(k) {
                    Some(existing) if existing.is_object() && v.is_object() => {
                        deep_merge(existing, v)
                    }
                    _ => {
                        a.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        (a, b) => *a = b.clone(),
    }
}

fn glob_set(patterns: &[String]) -> Result<GlobSet, String> {
    let mut b = GlobSetBuilder::new();
    for p in patterns {
        // `*.stories.tsx` style patterns match at any depth, like oxlint/ESLint.
        let p = if p.contains('/') {
            p.clone()
        } else {
            format!("**/{p}")
        };
        b.add(Glob::new(&p).map_err(|e| format!("invalid glob {p:?}: {e}"))?);
    }
    b.build().map_err(|e| e.to_string())
}

fn strings(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|s| s.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

struct Raw {
    rules: Map<String, Value>,
    settings: Value,
    overrides: Vec<Override>,
    ignore_patterns: Vec<String>,
}

fn load_into(path: &Path, raw: &mut Raw, depth: usize) -> Result<(), String> {
    if depth > 32 {
        return Err(format!("{}: `extends` nests too deeply", path.display()));
    }
    let json = read_json(path)?;
    let dir = path.parent().unwrap_or(Path::new("."));
    for ext in strings(json.get("extends")) {
        load_into(&dir.join(ext), raw, depth + 1)?;
    }
    raw.ignore_patterns
        .extend(strings(json.get("ignorePatterns")));
    if let Some(Value::Object(settings)) = json.get("settings") {
        let mut settings = settings.clone();
        let ours = settings
            .get_mut("webadeva")
            .and_then(Value::as_object_mut)
            .map(|w| (w.remove("rules"), w.remove("overrides")));
        deep_merge(&mut raw.settings, &Value::Object(settings));
        if let Some((rules, overrides)) = ours {
            if let Some(Value::Object(rules)) = rules {
                for (k, v) in rules {
                    raw.rules.insert(k, v);
                }
            }
            for o in overrides
                .as_ref()
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                raw.overrides.push(Override {
                    files: glob_set(&strings(o.get("files")))?,
                    exclude: glob_set(&strings(o.get("excludeFiles")))?,
                    rules: o
                        .get("rules")
                        .and_then(Value::as_object)
                        .cloned()
                        .unwrap_or_default(),
                    settings: o.get("settings").cloned().unwrap_or(Value::Null),
                });
            }
        }
    }
    Ok(())
}

pub const CONFIG_NAMES: &[&str] = &[".oxlintrc.json", ".oxlintrc.jsonc", "oxlintrc.json"];

pub fn find_config(root: &Path) -> Option<PathBuf> {
    CONFIG_NAMES
        .iter()
        .map(|n| root.join(n))
        .find(|p| p.is_file())
}

pub fn load(config_path: Option<&Path>, root: &Path) -> Result<Config, String> {
    let mut raw = Raw {
        rules: Map::new(),
        settings: Value::Object(Map::new()),
        overrides: Vec::new(),
        ignore_patterns: Vec::new(),
    };
    let path = match config_path {
        Some(p) => Some(p.to_path_buf()),
        None => find_config(root),
    };
    if let Some(path) = path {
        load_into(&path, &mut raw, 0)?;
    }
    Ok(Config {
        root: root.to_path_buf(),
        rules: raw.rules,
        settings: raw.settings,
        overrides: raw.overrides,
        ignore_patterns: raw.ignore_patterns,
    })
}

impl Config {
    /// Effective rule values and settings for one file.
    pub fn resolve(&self, rel_path: &str) -> (Map<String, Value>, Value) {
        let mut rules = self.rules.clone();
        let mut settings = self.settings.clone();
        for o in &self.overrides {
            if o.files.is_match(rel_path) && !o.exclude.is_match(rel_path) {
                for (k, v) in &o.rules {
                    rules.insert(k.clone(), v.clone());
                }
                if !o.settings.is_null() {
                    deep_merge(&mut settings, &o.settings);
                }
            }
        }
        (rules, settings)
    }
}
