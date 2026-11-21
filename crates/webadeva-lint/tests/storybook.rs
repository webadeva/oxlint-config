//! Integration tests for the storybook group. The expected diagnostics are
//! ESLint 10 + eslint-plugin-storybook 10.6.2 (`flat/recommended`, parsed with
//! @typescript-eslint/parser like the original config) on the same fixtures.
#![cfg(feature = "storybook")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Copies the fixture tree, minus the `--fix` fixtures under `fix/`.
fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            if entry.file_name() == "fix" && from.ends_with("tests/fixtures/storybook") {
                continue;
            }
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// A fresh temp dir whose last component is `name` (no-uninstalled-addons
/// prints the cwd's name in its message).
fn temp_dir(test: &str, name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir()
        .join(format!(
            "webadeva-lint-{test}-{}-{nanos}",
            std::process::id()
        ))
        .join(name);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(dir: &Path) -> Vec<String> {
    run_with(dir, &[])
}

fn run_with(dir: &Path, extra: &[&str]) -> Vec<String> {
    let out = Command::new(env!("CARGO_BIN_EXE_webadeva-lint"))
        .args(["-f", "json"])
        .args(extra)
        .current_dir(dir)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "bad JSON ({e}): {stdout}
stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        )
    });
    let mut lines: Vec<String> = json["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| {
            format!(
                "{}:{}:{} {} {}",
                d["file"].as_str().unwrap(),
                d["line"],
                d["column"],
                d["rule"].as_str().unwrap(),
                d["message"].as_str().unwrap()
            )
        })
        .collect();
    lines.sort();
    lines
}

const EXPECTED: &[&str] = &[
    ".storybook/main.ts:10:5 storybook/no-uninstalled-addons The @storybook/addon-interactions/register is not installed in storybook/. Did you forget to install it or is your package.json in a different location?",
    ".storybook/main.ts:12:13 storybook/no-uninstalled-addons The missing-addon-in-object is not installed in storybook/. Did you forget to install it or is your package.json in a different location?",
    ".storybook/main.ts:17:5 storybook/no-uninstalled-addons The @storybook/addon-essentials is not installed in storybook/. Did you forget to install it or is your package.json in a different location?",
    ".storybook/main.ts:22:24 storybook/no-uninstalled-addons The @storybook/addon-viewport is not installed in storybook/. Did you forget to install it or is your package.json in a different location?",
    ".storybook/main.ts:25:24 storybook/no-uninstalled-addons The assigned-missing is not installed in storybook/. Did you forget to install it or is your package.json in a different location?",
    ".storybook/main.ts:7:5 storybook/no-uninstalled-addons The @storybook/addon-essentials is not installed in storybook/. Did you forget to install it or is your package.json in a different location?",
    "src/await.stories.ts:10:5 storybook/await-interactions Interaction should be awaited: findByText",
    "src/await.stories.ts:12:5 storybook/await-interactions Interaction should be awaited: toBeInTheDocument",
    "src/await.stories.ts:15:5 storybook/await-interactions Interaction should be awaited: waitFor",
    "src/await.stories.ts:16:6 storybook/await-interactions Interaction should be awaited: userEvent",
    "src/await.stories.ts:19:5 storybook/await-interactions Interaction should be awaited: userEvent",
    "src/await.stories.ts:27:5 storybook/await-interactions Interaction should be awaited: play",
    "src/await.stories.ts:28:5 storybook/await-interactions Interaction should be awaited: play",
    "src/await.stories.ts:28:5 storybook/context-in-play-function Pass a context when invoking play function of another story",
    "src/await.stories.ts:29:5 storybook/await-interactions Interaction should be awaited: play",
    "src/await.stories.ts:30:5 storybook/await-interactions Interaction should be awaited: wait",
    "src/await.stories.ts:31:5 storybook/await-interactions Interaction should be awaited: findByRole",
    "src/await.stories.ts:8:5 storybook/await-interactions Interaction should be awaited: userEvent",
    "src/context.stories.tsx:10:49 storybook/context-in-play-function Pass a context when invoking play function of another story",
    "src/context.stories.tsx:11:55 storybook/context-in-play-function Pass a context when invoking play function of another story",
    "src/context.stories.tsx:12:46 storybook/context-in-play-function Pass a context when invoking play function of another story",
    "src/context.stories.tsx:13:63 storybook/context-in-play-function Pass a context when invoking play function of another story",
    "src/context.stories.tsx:14:49 storybook/context-in-play-function Pass a context when invoking play function of another story",
    "src/context.stories.tsx:16:54 storybook/context-in-play-function Pass a context when invoking play function of another story",
    "src/directive.stories.tsx:1:1 storybook/default-exports The file should have a default export.",
    "src/edge.stories.ts:10:5 storybook/await-interactions Interaction should be awaited: toBe",
    "src/edge.stories.ts:3:14 storybook/prefer-pascal-case The story should use PascalCase notation: meta",
    "src/edge.stories.ts:6:14 storybook/prefer-pascal-case The story should use PascalCase notation: lower",
    "src/edge.stories.ts:9:5 storybook/await-interactions Interaction should be awaited: userEvent",
    "src/empty-hashbang.story.js:4:1 storybook/default-exports The file should have a default export.",
    "src/empty.stories.ts:2:1 storybook/default-exports The file should have a default export.",
    "src/hier-regex.stories.js:1:18 storybook/hierarchy-separator Deprecated hierarchy separator in title property: /a|b/.",
    "src/hier.stories.ts:3:16 storybook/hierarchy-separator Deprecated hierarchy separator in title property: 'Atoms|Button'.",
    "src/imports.stories.tsx:1:1 storybook/no-renderer-packages Do not import renderer package \"@storybook/react\" directly. Use a framework package instead (e.g. @storybook/nextjs, @storybook/react-vite, @storybook/nextjs-vite, @storybook/react-webpack5, @storybook/react-native-web-vite).",
    "src/imports.stories.tsx:2:1 storybook/use-storybook-testing-library Do not use `@testing-library/react` directly in the story. You should import the functions from `storybook/test` instead.",
    "src/imports.stories.tsx:3:1 storybook/use-storybook-testing-library Do not use `@testing-library/user-event` directly in the story. You should import the functions from `storybook/test` instead.",
    "src/imports.stories.tsx:4:1 storybook/no-renderer-packages Do not import renderer package \"@storybook/vue3\" directly. Use a framework package instead (e.g. @storybook/vue3-vite, @storybook/vue3-webpack5).",
    "src/names.stories.js:10:14 storybook/prefer-pascal-case The story should use PascalCase notation: snake_case_story",
    "src/names.stories.js:10:35 storybook/no-redundant-story-name Named exports should not use the name annotation if it is redundant to the name that would be generated by the export name",
    "src/names.stories.js:11:37 storybook/no-redundant-story-name Named exports should not use the name annotation if it is redundant to the name that would be generated by the export name",
    "src/names.stories.js:14:2 storybook/no-redundant-story-name Named exports should not use the name annotation if it is redundant to the name that would be generated by the export name",
    "src/names.stories.js:3:32 storybook/no-redundant-story-name Named exports should not use the name annotation if it is redundant to the name that would be generated by the export name",
    "src/names.stories.js:4:28 storybook/no-redundant-story-name Named exports should not use the name annotation if it is redundant to the name that would be generated by the export name",
    "src/names.stories.js:7:1 storybook/no-redundant-story-name Named exports should not use the name annotation if it is redundant to the name that would be generated by the export name",
    "src/names.stories.js:8:25 storybook/no-redundant-story-name Named exports should not use the name annotation if it is redundant to the name that would be generated by the export name",
    "src/names.stories.js:9:14 storybook/prefer-pascal-case The story should use PascalCase notation: story1",
    "src/names.stories.js:9:25 storybook/no-redundant-story-name Named exports should not use the name annotation if it is redundant to the name that would be generated by the export name",
    "src/nodefault.stories.ts:3:1 storybook/default-exports The file should have a default export.",
    "src/noimport.stories.js:5:5 storybook/await-interactions Interaction should be awaited: userEvent",
    "src/noimport.stories.js:6:5 storybook/await-interactions Interaction should be awaited: toBeTruthy",
    "src/noimport.stories.js:6:5 storybook/use-storybook-expect Do not use global expect directly in the story. You should import it from `@storybook/test` (preferrably) or `@storybook/jest` instead.",
    "src/noimport.stories.js:7:6 storybook/use-storybook-expect Do not use global expect directly in the story. You should import it from `@storybook/test` (preferrably) or `@storybook/jest` instead.",
    "src/nostories-default-reexport.stories.ts:1:1 storybook/story-exports The file should have at least one story export. Make sure the includeStories/excludeStories you defined are correct, otherwise Storybook will not use any stories for this file.",
    "src/nostories-filter.stories.ts:1:1 storybook/story-exports The file should have at least one story export. Make sure the includeStories/excludeStories you defined are correct, otherwise Storybook will not use any stories for this file.",
    "src/nostories.stories.ts:3:1 storybook/story-exports The file should have at least one story export",
    "src/onlyimports.story.js:1:1 storybook/default-exports The file should have a default export.",
    "src/pascal-array.stories.ts:4:14 storybook/prefer-pascal-case The story should use PascalCase notation: secondary",
    "src/pascal-throw.stories.ts:4:14 storybook/prefer-pascal-case The story should use PascalCase notation: notAStory",
    "src/pascal.stories.js:3:14 storybook/prefer-pascal-case The story should use PascalCase notation: primary",
    "src/pascal.stories.js:9:14 storybook/prefer-pascal-case The story should use PascalCase notation: with_underscore",
    "src/startcase.stories.ts:10:14 storybook/prefer-pascal-case The story should use PascalCase notation: Über",
    "src/startcase.stories.ts:10:23 storybook/no-redundant-story-name Named exports should not use the name annotation if it is redundant to the name that would be generated by the export name",
    "src/startcase.stories.ts:11:14 storybook/prefer-pascal-case The story should use PascalCase notation: lowerSkip",
    "src/startcase.stories.ts:12:14 storybook/prefer-pascal-case The story should use PascalCase notation: $dollar",
    "src/startcase.stories.ts:13:33 storybook/no-redundant-story-name Named exports should not use the name annotation if it is redundant to the name that would be generated by the export name",
    "src/startcase.stories.ts:14:33 storybook/no-redundant-story-name Named exports should not use the name annotation if it is redundant to the name that would be generated by the export name",
    "src/startcase.stories.ts:15:14 storybook/prefer-pascal-case The story should use PascalCase notation: csf4Meta",
    "src/startcase.stories.ts:6:27 storybook/no-redundant-story-name Named exports should not use the name annotation if it is redundant to the name that would be generated by the export name",
    "src/startcase.stories.ts:7:33 storybook/no-redundant-story-name Named exports should not use the name annotation if it is redundant to the name that would be generated by the export name",
    "src/startcase.stories.ts:8:14 storybook/prefer-pascal-case The story should use PascalCase notation: a1B2",
    "src/startcase.stories.ts:8:23 storybook/no-redundant-story-name Named exports should not use the name annotation if it is redundant to the name that would be generated by the export name",
    "src/startcase.stories.ts:9:14 storybook/prefer-pascal-case The story should use PascalCase notation: with$dollar",
    "src/startcase.stories.ts:9:30 storybook/no-redundant-story-name Named exports should not use the name annotation if it is redundant to the name that would be generated by the export name",
    "src/stories-types.stories.ts:1:1 storybook/story-exports The file should have at least one story export",
    "src/storiesof.stories.js:1:1 storybook/no-renderer-packages Do not import renderer package \"@storybook/react\" directly. Use a framework package instead (e.g. @storybook/nextjs, @storybook/react-vite, @storybook/nextjs-vite, @storybook/react-webpack5, @storybook/react-native-web-vite).",
];

#[test]
fn matches_eslint_plugin_storybook() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/storybook");
    let dir = temp_dir("storybook", "storybook");
    copy_dir(&fixture, &dir);
    let actual = run(&dir);
    let mut expected: Vec<String> = EXPECTED.iter().map(|s| s.to_string()).collect();
    expected.sort();
    if actual != expected {
        let missing: Vec<_> = expected.iter().filter(|e| !actual.contains(e)).collect();
        let extra: Vec<_> = actual.iter().filter(|a| !expected.contains(a)).collect();
        panic!(
            "diagnostics differ
missing: {missing:#?}
unexpected: {extra:#?}"
        );
    }
    fs::remove_dir_all(dir.parent().unwrap()).ok();
}

#[test]
fn rules_only_apply_to_story_and_main_files() {
    let dir = temp_dir("storybook-scope", "proj");
    fs::write(
        dir.join(".oxlintrc.json"),
        r#"{ "ignorePatterns": ["!.storybook", "!**/.storybook/**"], "settings": { "webadeva": { "rules": {
            "storybook/default-exports": "error", "storybook/use-storybook-expect": "error",
            "storybook/no-uninstalled-addons": "error" } } } }"#,
    )
    .unwrap();
    fs::write(dir.join("package.json"), "{}").unwrap();
    fs::create_dir_all(dir.join("src/.storybook")).unwrap();
    fs::write(
        dir.join("src/a.ts"),
        "expect(1);
",
    )
    .unwrap();
    fs::write(
        dir.join("src/stories.ts"),
        "expect(1);
",
    )
    .unwrap();
    fs::write(
        dir.join("src/a.stories.mts"),
        "expect(1);
",
    )
    .unwrap();
    fs::write(
        dir.join("src/.storybook/main.ts"),
        "export default { addons: ['x'] };
",
    )
    .unwrap();
    fs::create_dir_all(dir.join(".storybook")).unwrap();
    fs::write(
        dir.join(".storybook/preview.ts"),
        "export default { addons: ['x'] };
",
    )
    .unwrap();
    fs::write(
        dir.join("src/b.story.jsx"),
        "expect(1);
",
    )
    .unwrap();
    assert_eq!(
        run(&dir),
        vec![
            "src/b.story.jsx:1:1 storybook/default-exports The file should have a default export.".to_string(),
            "src/b.story.jsx:1:1 storybook/use-storybook-expect Do not use global expect directly in the story. You should import it from `@storybook/test` (preferrably) or `@storybook/jest` instead.".to_string(),
        ]
    );
    fs::remove_dir_all(dir.parent().unwrap()).ok();
}

#[test]
fn no_uninstalled_addons_reports_unreadable_package_json() {
    let dir = temp_dir("storybook-nopkg", "proj");
    fs::write(
        dir.join(".oxlintrc.json"),
        r#"{ "ignorePatterns": ["!.storybook"], "settings": { "webadeva": { "rules": {
            "storybook/no-uninstalled-addons": ["error", { "packageJsonLocation": "./nested/package.json" }] } } } }"#,
    )
    .unwrap();
    fs::create_dir_all(dir.join(".storybook")).unwrap();
    fs::write(
        dir.join(".storybook/main.js"),
        "module.exports = { addons: ['x'] };
",
    )
    .unwrap();
    let actual = run(&dir);
    assert_eq!(actual.len(), 1, "{actual:?}");
    assert!(actual[0].starts_with(".storybook/main.js:1:28 storybook/no-uninstalled-addons The provided path in your eslintrc.json - "));
    assert!(actual[0].contains("nested/package.json is not a valid path"));

    // With the package.json in place, the relative location shows up in the message.
    fs::create_dir_all(dir.join("nested")).unwrap();
    fs::write(
        dir.join("nested/package.json"),
        r#"{ "dependencies": { "y": "1" } }"#,
    )
    .unwrap();
    assert_eq!(
        run(&dir),
        vec![".storybook/main.js:1:29 storybook/no-uninstalled-addons The x is not installed in proj/nested/package.json. Did you forget to install it or is your package.json in a different location?".to_string()]
    );
    fs::remove_dir_all(dir.parent().unwrap()).ok();
}

/// Every storybook rule except no-uninstalled-addons (main.js only, no fix).
const FIX_CONFIG: &str = r#"{ "settings": { "webadeva": { "rules": {
    "storybook/await-interactions": "error",
    "storybook/context-in-play-function": "error",
    "storybook/default-exports": "error",
    "storybook/hierarchy-separator": "warn",
    "storybook/no-redundant-story-name": "warn",
    "storybook/no-renderer-packages": "error",
    "storybook/prefer-pascal-case": "warn",
    "storybook/story-exports": "error",
    "storybook/use-storybook-expect": "error",
    "storybook/use-storybook-testing-library": "error"
} } } }"#;

/// Fixtures whose remaining messages can't match: upstream await-interactions
/// inserts `async ` before a method's parameters (`playasync (ctx) {`), so the
/// fixed file has `await` in a non-async method. typescript-eslint parses
/// that; oxc reports a syntax error and webadeva-lint then reports nothing
/// (the fixed output itself still matches).
const MESSAGES_DIVERGE: &[&str] = &["await-method.stories.ts"];

/// `--fix` on each fixture under `fix/` must produce ESLint's
/// `verifyAndFix` output (`<name>.fixed.<ext>`) byte for byte, and leave
/// ESLint's remaining messages (`<name>.messages.txt`).
#[test]
fn fix_matches_eslint_verify_and_fix() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/storybook/fix");
    let mut inputs: Vec<PathBuf> = fs::read_dir(&fixtures)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            let name = p.file_name().unwrap().to_str().unwrap();
            !name.ends_with(".messages.txt") && !name.split('.').any(|part| part == "fixed")
        })
        .collect();
    inputs.sort();
    assert!(!inputs.is_empty());
    let mut failures = Vec::new();
    for input in inputs {
        let name = input.file_name().unwrap().to_str().unwrap();
        let (stem, ext) = name.rsplit_once('.').unwrap();
        let expected_output =
            fs::read_to_string(fixtures.join(format!("{stem}.fixed.{ext}"))).unwrap();
        let expected_messages: Vec<String> =
            fs::read_to_string(fixtures.join(format!("{stem}.messages.txt")))
                .unwrap()
                .lines()
                .map(|l| format!("{name}:{l}"))
                .collect();
        let dir = temp_dir("storybook-fix", "storybook");
        fs::write(dir.join(".oxlintrc.json"), FIX_CONFIG).unwrap();
        fs::copy(&input, dir.join(name)).unwrap();
        let mut messages = run_with(&dir, &["--fix"]);
        messages.sort();
        let mut expected_messages = expected_messages;
        expected_messages.sort();
        let output = fs::read_to_string(dir.join(name)).unwrap();
        if output != expected_output {
            failures.push(format!(
                "{name}: output differs\n--- expected\n{expected_output}--- actual\n{output}"
            ));
        }
        if messages != expected_messages && !MESSAGES_DIVERGE.contains(&name) {
            failures.push(format!(
                "{name}: messages differ\nexpected: {expected_messages:#?}\nactual: {messages:#?}"
            ));
        }
        fs::remove_dir_all(dir.parent().unwrap()).ok();
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
