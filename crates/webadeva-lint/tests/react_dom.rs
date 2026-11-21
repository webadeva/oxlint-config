#![cfg(feature = "react_dom")]
//! Parity tests for the `react_dom` group. Each fixture in
//! `tests/fixtures/react_dom/` is named `<rule>[.<n>].<ext>` (the rule id
//! without `@eslint-react/`); `<fixture>.expected.json` holds the
//! diagnostics ESLint (with @eslint-react/eslint-plugin 5.28) reports for it,
//! an optional `<fixture>.settings.json` holds the `settings` object and an
//! optional `<fixture>.options.json` the rule's options array.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/react_dom")
}

fn temp_dir(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("webadeva-react-dom-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Diag {
    rule: String,
    line: u64,
    column: u64,
    end_line: u64,
    end_column: u64,
    message: String,
}

fn diag(v: &Value) -> Diag {
    Diag {
        rule: v["rule"].as_str().unwrap().to_string(),
        line: v["line"].as_u64().unwrap(),
        column: v["column"].as_u64().unwrap(),
        end_line: v["endLine"].as_u64().unwrap(),
        end_column: v["endColumn"].as_u64().unwrap(),
        message: v["message"].as_str().unwrap().to_string(),
    }
}

fn run_fixture(file: &str) -> (Vec<Diag>, Vec<Diag>) {
    let dir = fixtures_dir();
    let rule = format!("@eslint-react/{}", file.split('.').next().unwrap());
    let tmp = temp_dir(&file.replace('.', "_"));
    fs::copy(dir.join(file), tmp.join(file)).unwrap();
    let settings_path = dir.join(format!("{file}.settings.json"));
    let mut settings: Value = if settings_path.exists() {
        serde_json::from_str(&fs::read_to_string(settings_path).unwrap()).unwrap()
    } else {
        json!({})
    };
    let options_path = dir.join(format!("{file}.options.json"));
    let mut value = json!(["error"]);
    if options_path.exists() {
        let options: Value =
            serde_json::from_str(&fs::read_to_string(options_path).unwrap()).unwrap();
        value
            .as_array_mut()
            .unwrap()
            .extend(options.as_array().unwrap().iter().cloned());
    }
    settings["webadeva"] = json!({ "rules": { rule.clone(): value } });
    fs::write(
        tmp.join(".oxlintrc.json"),
        serde_json::to_string(&json!({ "settings": settings })).unwrap(),
    )
    .unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_webadeva-lint"))
        .current_dir(&tmp)
        .args(["-f", "json", file])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let report: Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "{file}: bad output ({e}): {stdout}\n{}",
            String::from_utf8_lossy(&out.stderr)
        )
    });
    let mut actual: Vec<Diag> = report["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(diag)
        .collect();
    let expected_raw = fs::read_to_string(dir.join(format!("{file}.expected.json"))).unwrap();
    let mut expected: Vec<Diag> = serde_json::from_str::<Value>(&expected_raw)
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(diag)
        .collect();
    actual.sort();
    expected.sort();
    let _ = fs::remove_dir_all(&tmp);
    (actual, expected)
}

#[test]
fn react_dom_fixtures_match_eslint() {
    let mut files: Vec<String> = fs::read_dir(fixtures_dir())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|f| {
            f.ends_with(".tsx") || f.ends_with(".ts") || f.ends_with(".jsx") || f.ends_with(".js")
        })
        .collect();
    files.sort();
    assert!(files.len() > 30, "fixtures missing");
    let mut failures = Vec::new();
    for file in &files {
        let (actual, expected) = run_fixture(file);
        if actual != expected {
            let missing: Vec<_> = expected.iter().filter(|d| !actual.contains(d)).collect();
            let extra: Vec<_> = actual.iter().filter(|d| !expected.contains(d)).collect();
            failures.push(format!(
                "{file}:\n  missing: {missing:#?}\n  extra: {extra:#?}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} fixture(s) differ from ESLint:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// `jsx-no-key-after-spread` only applies with the automatic runtime, read
/// from the nearest tsconfig (ESLint gets it from the type-aware program).
#[test]
fn key_after_spread_follows_tsconfig_jsx() {
    let run = |name: &str, tsconfigs: &[(&str, &str)]| -> usize {
        let tmp = temp_dir(name);
        fs::create_dir_all(tmp.join("src")).unwrap();
        for (path, content) in tsconfigs {
            fs::write(tmp.join(path), content).unwrap();
        }
        fs::write(
            tmp.join("src/a.tsx"),
            "const a = <div {...props} key=\"1\" />;\n",
        )
        .unwrap();
        fs::write(
            tmp.join(".oxlintrc.json"),
            r#"{ "settings": { "webadeva": { "rules": { "@eslint-react/jsx-no-key-after-spread": "error" } } } }"#,
        )
        .unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_webadeva-lint"))
            .current_dir(&tmp)
            .args(["-f", "json"])
            .output()
            .unwrap();
        let report: Value = serde_json::from_slice(&out.stdout).unwrap();
        let _ = fs::remove_dir_all(&tmp);
        report["diagnostics"].as_array().unwrap().len()
    };
    assert_eq!(run("no-tsconfig", &[]), 1);
    assert_eq!(
        run(
            "preserve",
            &[(
                "tsconfig.json",
                r#"{ "compilerOptions": { "jsx": "preserve" } }"#
            )]
        ),
        0
    );
    assert_eq!(
        run(
            "extends",
            &[
                (
                    "tsconfig.base.json",
                    "{\n  // base\n  \"compilerOptions\": { \"jsx\": \"react-jsx\", },\n}\n"
                ),
                (
                    "src/tsconfig.json",
                    r#"{ "extends": "../tsconfig.base.json", "compilerOptions": {} }"#
                ),
            ]
        ),
        1
    );
    assert_eq!(
        run(
            "extends-classic",
            &[
                (
                    "tsconfig.base.json",
                    r#"{ "compilerOptions": { "jsx": "react" } }"#
                ),
                ("tsconfig.json", r#"{ "extends": "./tsconfig.base" }"#),
            ]
        ),
        0
    );
}

#[test]
fn all_rules_are_registered() {
    let out = Command::new(env!("CARGO_BIN_EXE_webadeva-lint"))
        .arg("--rules")
        .output()
        .unwrap();
    let listed = String::from_utf8_lossy(&out.stdout);
    for rule in [
        "jsx-no-children-prop",
        "jsx-no-children-prop-with-children",
        "jsx-no-key-after-spread",
        "jsx-no-leaked-dollar",
        "jsx-no-leaked-semicolon",
        "jsx-no-namespace",
        "no-missing-key",
        "no-children-count",
        "no-children-for-each",
        "no-children-map",
        "no-children-only",
        "no-children-to-array",
        "dom-no-dangerously-set-innerhtml",
        "dom-no-dangerously-set-innerhtml-with-children",
        "dom-no-flush-sync",
        "dom-no-hydrate",
        "dom-no-render",
        "dom-no-script-url",
        "dom-no-unsafe-iframe-sandbox",
        "dom-no-use-form-state",
        "dom-no-void-elements-with-children",
        "dom-no-find-dom-node",
        "dom-no-render-return-value",
        "dom-no-unknown-property",
        "web-api-no-leaked-event-listener",
        "web-api-no-leaked-fetch",
        "web-api-no-leaked-intersection-observer",
        "web-api-no-leaked-interval",
        "web-api-no-leaked-resize-observer",
        "web-api-no-leaked-timeout",
    ] {
        let id = format!("@eslint-react/{rule}");
        assert!(listed.lines().any(|l| l == id), "{id} not registered");
    }
}

/// `--fix` parity. Each input in `tests/fixtures/react_dom/fix/` has a
/// `<name>.fixed.<ext>` with ESLint's `verifyAndFix` output and a
/// `<input>.expected.json` with the diagnostics left after fixing. Rules come
/// from `<input>.rules.json` (`{ ruleId: config }`) or the file name (plus
/// `<input>.options.json`); `<input>.settings.json` holds `settings`.
#[test]
fn react_dom_fixes_match_eslint() {
    let dir = fixtures_dir().join("fix");
    let read_json = |p: PathBuf| -> Option<Value> {
        p.exists()
            .then(|| serde_json::from_str(&fs::read_to_string(p).unwrap()).unwrap())
    };
    let mut files: Vec<String> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|f| {
            [".tsx", ".ts", ".jsx", ".js"]
                .iter()
                .any(|e| f.ends_with(e))
                && !f.contains(".fixed.")
        })
        .collect();
    files.sort();
    assert!(files.len() >= 10, "fix fixtures missing");
    let mut failures = Vec::new();
    for file in &files {
        let input = dir.join(file);
        let rules = read_json(dir.join(format!("{file}.rules.json"))).unwrap_or_else(|| {
            let mut value = json!(["error"]);
            if let Some(Value::Array(opts)) = read_json(dir.join(format!("{file}.options.json"))) {
                value.as_array_mut().unwrap().extend(opts);
            }
            let rule = format!("@eslint-react/{}", file.split('.').next().unwrap());
            json!({ rule: value })
        });
        let mut settings =
            read_json(dir.join(format!("{file}.settings.json"))).unwrap_or_else(|| json!({}));
        settings["webadeva"] = json!({ "rules": rules });

        let tmp = temp_dir(&format!("fix-{}", file.replace('.', "_")));
        fs::copy(&input, tmp.join(file)).unwrap();
        fs::write(
            tmp.join(".oxlintrc.json"),
            serde_json::to_string(&json!({ "settings": settings })).unwrap(),
        )
        .unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_webadeva-lint"))
            .current_dir(&tmp)
            .args(["--fix", "-f", "json", file])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        let report: Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
            panic!(
                "{file}: bad output ({e}): {stdout}\n{}",
                String::from_utf8_lossy(&out.stderr)
            )
        });
        let fixed = fs::read_to_string(tmp.join(file)).unwrap();
        let _ = fs::remove_dir_all(&tmp);

        let ext = Path::new(file).extension().unwrap().to_string_lossy();
        let stem = &file[..file.len() - ext.len() - 1];
        let expected_fixed = fs::read_to_string(dir.join(format!("{stem}.fixed.{ext}"))).unwrap();
        if fixed != expected_fixed {
            failures.push(format!(
                "{file}: fixed output differs\n--- expected\n{expected_fixed}\n--- actual\n{fixed}"
            ));
        }

        let mut actual: Vec<Diag> = report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .map(diag)
            .collect();
        let mut expected: Vec<Diag> = read_json(dir.join(format!("{file}.expected.json")))
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(diag)
            .collect();
        actual.sort();
        expected.sort();
        if actual != expected {
            failures.push(format!(
                "{file}: remaining diagnostics differ\n  expected: {expected:#?}\n  actual: {actual:#?}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} fix fixture(s) differ from ESLint:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
