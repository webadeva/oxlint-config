//! Parity tests for the `react_components` group. Expected diagnostics
//! (file, line, column) were produced by ESLint 10 with @eslint-react 5.28.2
//! / eslint-plugin-react-hooks 7.1.1 on the same fixtures
//! (tests/fixtures/react_components).
#![cfg(feature = "react_components")]

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn run(
    rule: &str,
    files: &[&str],
    options: Option<&str>,
    settings: &str,
    expected: &[(&str, usize, usize)],
) {
    let fixtures =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/react_components");
    let dir = std::env::temp_dir().join(format!(
        "webadeva-react-components-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    for f in files {
        std::fs::copy(fixtures.join(f), dir.join(f)).unwrap();
    }
    let value = match options {
        Some(o) => format!(r#"["error", {o}]"#),
        None => r#""error""#.to_string(),
    };
    let mut settings: serde_json::Value = serde_json::from_str(settings).unwrap();
    settings["webadeva"] =
        serde_json::from_str(&format!(r#"{{"rules": {{"{rule}": {value}}}}}"#)).unwrap();
    std::fs::write(
        dir.join(".oxlintrc.json"),
        serde_json::json!({ "settings": settings }).to_string(),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_webadeva-lint"))
        .args(["-f", "json", "."])
        .current_dir(&dir)
        .output()
        .unwrap();
    std::fs::remove_dir_all(&dir).ok();
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "{e}: {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    });
    let mut got: Vec<(String, usize, usize)> = json["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| {
            assert_eq!(d["rule"], rule);
            let file = d["file"]
                .as_str()
                .unwrap()
                .rsplit('/')
                .next()
                .unwrap()
                .to_string();
            (
                file,
                d["line"].as_u64().unwrap() as usize,
                d["column"].as_u64().unwrap() as usize,
            )
        })
        .collect();
    got.sort();
    let mut want: Vec<(String, usize, usize)> = expected
        .iter()
        .map(|(f, l, c)| (f.to_string(), *l, *c))
        .collect();
    want.sort();
    assert_eq!(got, want, "{rule}");
}

#[test]
fn no_access_state_in_setstate() {
    run(
        "@eslint-react/no-access-state-in-setstate",
        &["no-access-state-in-setstate.tsx"],
        None,
        r#"{}"#,
        &[
            ("no-access-state-in-setstate.tsx", 5, 28),
            ("no-access-state-in-setstate.tsx", 8, 35),
            ("no-access-state-in-setstate.tsx", 16, 30),
            ("no-access-state-in-setstate.tsx", 30, 24),
            ("no-access-state-in-setstate.tsx", 31, 24),
        ],
    );
}

#[test]
fn no_component_will_mount() {
    run(
        "@eslint-react/no-component-will-mount",
        &["lifecycle-members.tsx"],
        None,
        r#"{}"#,
        &[
            ("lifecycle-members.tsx", 4, 3),
            ("lifecycle-members.tsx", 16, 3),
            ("lifecycle-members.tsx", 34, 3),
        ],
    );
}

#[test]
fn no_component_will_receive_props() {
    run(
        "@eslint-react/no-component-will-receive-props",
        &["lifecycle-members.tsx"],
        None,
        r#"{}"#,
        &[
            ("lifecycle-members.tsx", 5, 3),
            ("lifecycle-members.tsx", 17, 3),
        ],
    );
}

#[test]
fn no_component_will_update() {
    run(
        "@eslint-react/no-component-will-update",
        &["lifecycle-members.tsx"],
        None,
        r#"{}"#,
        &[
            ("lifecycle-members.tsx", 6, 3),
            ("lifecycle-members.tsx", 24, 3),
        ],
    );
}

#[test]
fn no_unsafe_component_will_mount() {
    run(
        "@eslint-react/no-unsafe-component-will-mount",
        &["lifecycle-members.tsx"],
        None,
        r#"{}"#,
        &[("lifecycle-members.tsx", 7, 3)],
    );
}

#[test]
fn no_unsafe_component_will_receive_props() {
    run(
        "@eslint-react/no-unsafe-component-will-receive-props",
        &["lifecycle-members.tsx"],
        None,
        r#"{}"#,
        &[
            ("lifecycle-members.tsx", 8, 3),
            ("lifecycle-members.tsx", 20, 3),
        ],
    );
}

#[test]
fn no_unsafe_component_will_update() {
    run(
        "@eslint-react/no-unsafe-component-will-update",
        &["lifecycle-members.tsx"],
        None,
        r#"{}"#,
        &[
            ("lifecycle-members.tsx", 9, 3),
            ("lifecycle-members.tsx", 25, 3),
        ],
    );
}

#[test]
fn no_set_state_in_component_did_mount() {
    run(
        "@eslint-react/no-set-state-in-component-did-mount",
        &["set-state-in-lifecycle.tsx"],
        None,
        r#"{}"#,
        &[
            ("set-state-in-lifecycle.tsx", 5, 5),
            ("set-state-in-lifecycle.tsx", 11, 7),
            ("set-state-in-lifecycle.tsx", 34, 5),
            ("set-state-in-lifecycle.tsx", 55, 5),
        ],
    );
}

#[test]
fn no_set_state_in_component_did_update() {
    run(
        "@eslint-react/no-set-state-in-component-did-update",
        &["set-state-in-lifecycle.tsx"],
        None,
        r#"{}"#,
        &[
            ("set-state-in-lifecycle.tsx", 15, 5),
            ("set-state-in-lifecycle.tsx", 37, 5),
        ],
    );
}

#[test]
fn no_set_state_in_component_will_update() {
    run(
        "@eslint-react/no-set-state-in-component-will-update",
        &["set-state-in-lifecycle.tsx"],
        None,
        r#"{}"#,
        &[("set-state-in-lifecycle.tsx", 19, 5)],
    );
}

#[test]
fn no_unused_class_component_members() {
    run(
        "@eslint-react/no-unused-class-component-members",
        &[
            "no-unused-class-component-members.tsx",
            "no-unused-class-component-members-2.tsx",
        ],
        None,
        r#"{}"#,
        &[
            ("no-unused-class-component-members-2.tsx", 8, 3),
            ("no-unused-class-component-members-2.tsx", 9, 7),
            ("no-unused-class-component-members-2.tsx", 12, 7),
            ("no-unused-class-component-members-2.tsx", 14, 11),
            ("no-unused-class-component-members-2.tsx", 34, 3),
            ("no-unused-class-component-members-2.tsx", 42, 3),
            ("no-unused-class-component-members-2.tsx", 51, 3),
            ("no-unused-class-component-members-2.tsx", 59, 11),
            ("no-unused-class-component-members.tsx", 4, 3),
            ("no-unused-class-component-members.tsx", 6, 3),
            ("no-unused-class-component-members.tsx", 7, 3),
            ("no-unused-class-component-members.tsx", 12, 10),
            ("no-unused-class-component-members.tsx", 33, 3),
            ("no-unused-class-component-members.tsx", 43, 11),
            ("no-unused-class-component-members.tsx", 44, 13),
            ("no-unused-class-component-members.tsx", 46, 3),
            ("no-unused-class-component-members.tsx", 64, 3),
            ("no-unused-class-component-members.tsx", 68, 3),
            ("no-unused-class-component-members.tsx", 76, 3),
            ("no-unused-class-component-members.tsx", 78, 7),
            ("no-unused-class-component-members.tsx", 86, 3),
        ],
    );
}

#[test]
fn no_create_ref() {
    run(
        "@eslint-react/no-create-ref",
        &["no-create-ref.tsx", "no-nested-component-definitions-2.jsx"],
        None,
        r#"{}"#,
        &[
            ("no-create-ref.tsx", 4, 15),
            ("no-create-ref.tsx", 9, 15),
            ("no-create-ref.tsx", 14, 15),
            ("no-create-ref.tsx", 18, 24),
            ("no-create-ref.tsx", 40, 15),
            ("no-create-ref.tsx", 46, 13),
            ("no-create-ref.tsx", 51, 13),
        ],
    );
}

#[test]
fn no_context_provider() {
    run(
        "@eslint-react/no-context-provider",
        &["react-19-apis.tsx", "react-18.tsx"],
        None,
        r#"{}"#,
        &[
            ("react-19-apis.tsx", 14, 7),
            ("react-19-apis.tsx", 15, 7),
            ("react-19-apis.tsx", 19, 7),
        ],
    );
}

#[test]
fn no_forward_ref() {
    run(
        "@eslint-react/no-forward-ref",
        &["react-19-apis.tsx", "react-18.tsx"],
        None,
        r#"{}"#,
        &[
            ("react-18.tsx", 4, 7),
            ("react-19-apis.tsx", 24, 7),
            ("react-19-apis.tsx", 25, 7),
            ("react-19-apis.tsx", 28, 28),
            ("react-19-apis.tsx", 30, 1),
            ("react-19-apis.tsx", 31, 15),
            ("react-19-apis.tsx", 32, 16),
            ("react-19-apis.tsx", 33, 7),
            ("react-19-apis.tsx", 34, 7),
        ],
    );
}

#[test]
fn no_use_context() {
    run(
        "@eslint-react/no-use-context",
        &["react-19-apis.tsx", "react-18.tsx"],
        None,
        r#"{}"#,
        &[
            ("react-18.tsx", 6, 13),
            ("react-19-apis.tsx", 8, 17),
            ("react-19-apis.tsx", 9, 17),
            ("react-19-apis.tsx", 10, 17),
            ("react-19-apis.tsx", 11, 17),
        ],
    );
}

#[test]
fn no_context_provider_react_18() {
    run(
        "@eslint-react/no-context-provider",
        &["react-18.tsx"],
        None,
        r#"{"react-x": {"version": "18.3.1"}}"#,
        &[],
    );
}

#[test]
fn no_forward_ref_react_18() {
    run(
        "@eslint-react/no-forward-ref",
        &["react-18.tsx"],
        None,
        r#"{"react-x": {"version": "18.3.1"}}"#,
        &[],
    );
}

#[test]
fn no_use_context_react_18() {
    run(
        "@eslint-react/no-use-context",
        &["react-18.tsx"],
        None,
        r#"{"react-x": {"version": "18.3.1"}}"#,
        &[],
    );
}

#[test]
fn no_nested_component_definitions() {
    run(
        "@eslint-react/no-nested-component-definitions",
        &[
            "no-nested-component-definitions.tsx",
            "no-nested-component-definitions-2.jsx",
        ],
        None,
        r#"{}"#,
        &[
            ("no-nested-component-definitions-2.jsx", 5, 39),
            ("no-nested-component-definitions-2.jsx", 6, 29),
            ("no-nested-component-definitions-2.jsx", 10, 3),
            ("no-nested-component-definitions-2.jsx", 16, 22),
            ("no-nested-component-definitions-2.jsx", 19, 20),
            ("no-nested-component-definitions-2.jsx", 35, 22),
            ("no-nested-component-definitions.tsx", 4, 3),
            ("no-nested-component-definitions.tsx", 15, 35),
            ("no-nested-component-definitions.tsx", 18, 25),
            ("no-nested-component-definitions.tsx", 19, 38),
            ("no-nested-component-definitions.tsx", 20, 32),
            ("no-nested-component-definitions.tsx", 26, 19),
            ("no-nested-component-definitions.tsx", 27, 5),
            ("no-nested-component-definitions.tsx", 36, 5),
            ("no-nested-component-definitions.tsx", 59, 10),
            ("no-nested-component-definitions.tsx", 60, 8),
            ("no-nested-component-definitions.tsx", 69, 13),
            ("no-nested-component-definitions.tsx", 104, 3),
            ("no-nested-component-definitions.tsx", 113, 19),
        ],
    );
}

#[test]
fn no_nested_lazy_component_declarations() {
    run(
        "@eslint-react/no-nested-lazy-component-declarations",
        &["no-nested-lazy-component-declarations.tsx"],
        None,
        r#"{}"#,
        &[
            ("no-nested-lazy-component-declarations.tsx", 7, 18),
            ("no-nested-lazy-component-declarations.tsx", 8, 16),
            ("no-nested-lazy-component-declarations.tsx", 17, 13),
            ("no-nested-lazy-component-declarations.tsx", 22, 7),
            ("no-nested-lazy-component-declarations.tsx", 33, 13),
        ],
    );
}

#[test]
fn no_unnecessary_use_prefix() {
    run(
        "@eslint-react/no-unnecessary-use-prefix",
        &[
            "no-unnecessary-use-prefix.tsx",
            "no-unnecessary-use-prefix-2.ts",
        ],
        None,
        r#"{}"#,
        &[
            ("no-unnecessary-use-prefix-2.ts", 1, 14),
            ("no-unnecessary-use-prefix-2.ts", 12, 1),
            ("no-unnecessary-use-prefix-2.ts", 19, 7),
            ("no-unnecessary-use-prefix-2.ts", 19, 7),
            ("no-unnecessary-use-prefix-2.ts", 20, 7),
            ("no-unnecessary-use-prefix-2.ts", 21, 18),
            ("no-unnecessary-use-prefix-2.ts", 24, 9),
            ("no-unnecessary-use-prefix-2.ts", 26, 1),
            ("no-unnecessary-use-prefix-2.ts", 28, 7),
            ("no-unnecessary-use-prefix-2.ts", 30, 23),
            ("no-unnecessary-use-prefix-2.ts", 37, 3),
            ("no-unnecessary-use-prefix.tsx", 3, 10),
            ("no-unnecessary-use-prefix.tsx", 7, 7),
            ("no-unnecessary-use-prefix.tsx", 40, 10),
            ("no-unnecessary-use-prefix.tsx", 57, 3),
            ("no-unnecessary-use-prefix.tsx", 63, 3),
            ("no-unnecessary-use-prefix.tsx", 68, 10),
            ("no-unnecessary-use-prefix.tsx", 84, 25),
            ("no-unnecessary-use-prefix.tsx", 88, 10),
        ],
    );
}

#[test]
fn use_state() {
    run(
        "@eslint-react/use-state",
        &["use-state.tsx"],
        Some(
            r#"{"enforceAssignment": true, "enforceLazyInitialization": true, "enforceSetterName": true}"#,
        ),
        r#"{}"#,
        &[
            ("use-state.tsx", 6, 13),
            ("use-state.tsx", 7, 15),
            ("use-state.tsx", 8, 9),
            ("use-state.tsx", 9, 9),
            ("use-state.tsx", 11, 9),
            ("use-state.tsx", 13, 10),
            ("use-state.tsx", 14, 14),
            ("use-state.tsx", 15, 3),
            ("use-state.tsx", 16, 32),
            ("use-state.tsx", 18, 32),
            ("use-state.tsx", 22, 33),
            ("use-state.tsx", 22, 49),
            ("use-state.tsx", 24, 36),
            ("use-state.tsx", 25, 40),
            ("use-state.tsx", 40, 5),
            ("use-state.tsx", 42, 14),
        ],
    );
}

#[test]
fn rsc_function_definition() {
    run(
        "@eslint-react/rsc-function-definition",
        &[
            "rsc-function-definition.tsx",
            "rsc-function-definition-default.ts",
            "rsc-function-definition-client.tsx",
            "rsc-function-definition-2.ts",
        ],
        None,
        r#"{}"#,
        &[
            ("rsc-function-definition-2.ts", 9, 16),
            ("rsc-function-definition-2.ts", 12, 23),
            ("rsc-function-definition-2.ts", 16, 1),
            ("rsc-function-definition-2.ts", 21, 8),
            ("rsc-function-definition-client.tsx", 2, 1),
            ("rsc-function-definition-client.tsx", 6, 1),
            ("rsc-function-definition-client.tsx", 11, 11),
            ("rsc-function-definition-default.ts", 3, 17),
            ("rsc-function-definition.tsx", 5, 1),
            ("rsc-function-definition.tsx", 7, 1),
            ("rsc-function-definition.tsx", 11, 8),
            ("rsc-function-definition.tsx", 13, 22),
            ("rsc-function-definition.tsx", 17, 23),
            ("rsc-function-definition.tsx", 19, 1),
            ("rsc-function-definition.tsx", 21, 20),
            ("rsc-function-definition.tsx", 25, 16),
            ("rsc-function-definition.tsx", 27, 1),
            ("rsc-function-definition.tsx", 36, 3),
            ("rsc-function-definition.tsx", 41, 3),
            ("rsc-function-definition.tsx", 45, 3),
            ("rsc-function-definition.tsx", 49, 9),
        ],
    );
}

#[test]
fn naming_convention_context_name() {
    run(
        "@eslint-react/naming-convention-context-name",
        &["naming-convention.tsx"],
        None,
        r#"{}"#,
        &[
            ("naming-convention.tsx", 4, 7),
            ("naming-convention.tsx", 5, 7),
            ("naming-convention.tsx", 6, 20),
            ("naming-convention.tsx", 8, 1),
            ("naming-convention.tsx", 10, 1),
            ("naming-convention.tsx", 11, 7),
            ("naming-convention.tsx", 12, 7),
            ("naming-convention.tsx", 13, 7),
            ("naming-convention.tsx", 14, 7),
            ("naming-convention.tsx", 16, 3),
        ],
    );
}

#[test]
fn naming_convention_id_name() {
    run(
        "@eslint-react/naming-convention-id-name",
        &["naming-convention.tsx"],
        None,
        r#"{}"#,
        &[
            ("naming-convention.tsx", 23, 9),
            ("naming-convention.tsx", 31, 9),
        ],
    );
}

#[test]
fn naming_convention_ref_name() {
    run(
        "@eslint-react/naming-convention-ref-name",
        &["naming-convention.tsx"],
        None,
        r#"{}"#,
        &[
            ("naming-convention.tsx", 27, 9),
            ("naming-convention.tsx", 29, 18),
            ("naming-convention.tsx", 31, 9),
        ],
    );
}

#[test]
fn no_forward_ref_2() {
    run(
        "@eslint-react/no-forward-ref",
        &["parens.tsx"],
        None,
        r#"{}"#,
        &[("parens.tsx", 3, 7)],
    );
}

#[test]
fn no_nested_component_definitions_2() {
    run(
        "@eslint-react/no-nested-component-definitions",
        &["parens.tsx"],
        None,
        r#"{}"#,
        &[("parens.tsx", 9, 19), ("parens.tsx", 24, 20)],
    );
}

#[test]
fn use_state_2() {
    run(
        "@eslint-react/use-state",
        &["parens.tsx"],
        Some(
            r#"{"enforceAssignment": true, "enforceLazyInitialization": true, "enforceSetterName": true}"#,
        ),
        r#"{}"#,
        &[("parens.tsx", 11, 31), ("parens.tsx", 14, 9)],
    );
}

#[test]
fn naming_convention_context_name_2() {
    run(
        "@eslint-react/naming-convention-context-name",
        &["parens.tsx"],
        None,
        r#"{}"#,
        &[("parens.tsx", 5, 7), ("parens.tsx", 6, 7)],
    );
}

#[test]
fn naming_convention_ref_name_2() {
    run(
        "@eslint-react/naming-convention-ref-name",
        &["parens.tsx"],
        None,
        r#"{}"#,
        &[("parens.tsx", 12, 9)],
    );
}

#[test]
fn naming_convention_id_name_2() {
    run(
        "@eslint-react/naming-convention-id-name",
        &["parens.tsx"],
        None,
        r#"{}"#,
        &[("parens.tsx", 13, 9)],
    );
}

#[test]
fn no_access_state_in_setstate_2() {
    run(
        "@eslint-react/no-access-state-in-setstate",
        &["parens.tsx"],
        None,
        r#"{}"#,
        &[("parens.tsx", 20, 27), ("parens.tsx", 21, 25)],
    );
}

#[test]
fn no_set_state_in_component_did_mount_2() {
    run(
        "@eslint-react/no-set-state-in-component-did-mount",
        &["parens.tsx"],
        None,
        r#"{}"#,
        &[("parens.tsx", 20, 5), ("parens.tsx", 21, 5)],
    );
}

#[test]
fn no_unnecessary_use_prefix_2() {
    run(
        "@eslint-react/no-unnecessary-use-prefix",
        &["parens.tsx"],
        None,
        r#"{}"#,
        &[("parens.tsx", 29, 10)],
    );
}

#[test]
fn purity() {
    run(
        "@eslint-react/purity",
        &["purity.tsx", "purity-server.tsx"],
        None,
        r#"{}"#,
        &[
            ("purity-server.tsx", 7, 10),
            ("purity-server.tsx", 11, 16),
            ("purity.tsx", 5, 15),
            ("purity.tsx", 6, 13),
            ("purity.tsx", 7, 13),
            ("purity.tsx", 9, 14),
            ("purity.tsx", 11, 13),
            ("purity.tsx", 12, 13),
            ("purity.tsx", 13, 13),
            ("purity.tsx", 14, 18),
            ("purity.tsx", 17, 3),
            ("purity.tsx", 18, 3),
            ("purity.tsx", 28, 10),
            ("purity.tsx", 37, 3),
            ("purity.tsx", 38, 3),
            ("purity.tsx", 39, 3),
            ("purity.tsx", 40, 3),
            ("purity.tsx", 41, 3),
            ("purity.tsx", 52, 13),
            ("purity.tsx", 57, 13),
            ("purity.tsx", 58, 14),
            ("purity.tsx", 60, 3),
            ("purity.tsx", 62, 3),
            ("purity.tsx", 63, 3),
        ],
    );
}

#[test]
fn error_boundaries() {
    run(
        "@eslint-react/error-boundaries",
        &[
            "overlap-error-boundaries.tsx",
            "lifecycle-members.tsx",
            "naming-convention.tsx",
            "no-access-state-in-setstate.tsx",
            "no-create-ref.tsx",
            "no-nested-component-definitions-2.jsx",
            "no-nested-component-definitions.tsx",
            "no-nested-lazy-component-declarations.tsx",
            "no-unnecessary-use-prefix-2.ts",
            "no-unnecessary-use-prefix.tsx",
            "no-unused-class-component-members-2.tsx",
            "no-unused-class-component-members.tsx",
            "parens.tsx",
            "purity-server.tsx",
            "purity.tsx",
            "react-18.tsx",
            "react-19-apis.tsx",
            "react-compiler.tsx",
            "rsc-function-definition-2.ts",
            "rsc-function-definition-client.tsx",
            "rsc-function-definition-default.ts",
            "rsc-function-definition.tsx",
            "set-state-in-lifecycle.tsx",
            "use-state.tsx",
        ],
        None,
        r#"{}"#,
        &[
            ("overlap-error-boundaries.tsx", 3, 3),
            ("overlap-error-boundaries.tsx", 11, 3),
            ("overlap-error-boundaries.tsx", 18, 3),
            ("overlap-error-boundaries.tsx", 45, 3),
        ],
    );
}

#[test]
fn no_array_index_key() {
    run(
        "@eslint-react/no-array-index-key",
        &[
            "overlap-no-array-index-key.tsx",
            "lifecycle-members.tsx",
            "naming-convention.tsx",
            "no-access-state-in-setstate.tsx",
            "no-create-ref.tsx",
            "no-nested-component-definitions-2.jsx",
            "no-nested-component-definitions.tsx",
            "no-nested-lazy-component-declarations.tsx",
            "no-unnecessary-use-prefix-2.ts",
            "no-unnecessary-use-prefix.tsx",
            "no-unused-class-component-members-2.tsx",
            "no-unused-class-component-members.tsx",
            "parens.tsx",
            "purity-server.tsx",
            "purity.tsx",
            "react-18.tsx",
            "react-19-apis.tsx",
            "react-compiler.tsx",
            "rsc-function-definition-2.ts",
            "rsc-function-definition-client.tsx",
            "rsc-function-definition-default.ts",
            "rsc-function-definition.tsx",
            "set-state-in-lifecycle.tsx",
            "use-state.tsx",
        ],
        None,
        r#"{}"#,
        &[
            ("overlap-no-array-index-key.tsx", 5, 44),
            ("overlap-no-array-index-key.tsx", 6, 45),
            ("overlap-no-array-index-key.tsx", 7, 46),
            ("overlap-no-array-index-key.tsx", 8, 40),
            ("overlap-no-array-index-key.tsx", 9, 47),
            ("overlap-no-array-index-key.tsx", 10, 44),
            ("overlap-no-array-index-key.tsx", 12, 54),
            ("overlap-no-array-index-key.tsx", 13, 64),
            ("overlap-no-array-index-key.tsx", 14, 57),
            ("overlap-no-array-index-key.tsx", 16, 45),
            ("overlap-no-array-index-key.tsx", 17, 57),
            ("overlap-no-array-index-key.tsx", 18, 47),
            ("overlap-no-array-index-key.tsx", 19, 46),
            ("overlap-no-array-index-key.tsx", 20, 41),
        ],
    );
}

#[test]
fn no_clone_element() {
    run(
        "@eslint-react/no-clone-element",
        &[
            "overlap-no-clone-element.tsx",
            "lifecycle-members.tsx",
            "naming-convention.tsx",
            "no-access-state-in-setstate.tsx",
            "no-create-ref.tsx",
            "no-nested-component-definitions-2.jsx",
            "no-nested-component-definitions.tsx",
            "no-nested-lazy-component-declarations.tsx",
            "no-unnecessary-use-prefix-2.ts",
            "no-unnecessary-use-prefix.tsx",
            "no-unused-class-component-members-2.tsx",
            "no-unused-class-component-members.tsx",
            "parens.tsx",
            "purity-server.tsx",
            "purity.tsx",
            "react-18.tsx",
            "react-19-apis.tsx",
            "react-compiler.tsx",
            "rsc-function-definition-2.ts",
            "rsc-function-definition-client.tsx",
            "rsc-function-definition-default.ts",
            "rsc-function-definition.tsx",
            "set-state-in-lifecycle.tsx",
            "use-state.tsx",
        ],
        None,
        r#"{}"#,
        &[
            ("overlap-no-clone-element.tsx", 3, 11),
            ("overlap-no-clone-element.tsx", 4, 11),
            ("overlap-no-clone-element.tsx", 5, 11),
            ("overlap-no-clone-element.tsx", 6, 11),
            ("overlap-no-clone-element.tsx", 7, 11),
            ("overlap-no-clone-element.tsx", 8, 23),
        ],
    );
}

#[test]
fn no_direct_mutation_state() {
    run(
        "@eslint-react/no-direct-mutation-state",
        &[
            "overlap-no-direct-mutation-state.tsx",
            "lifecycle-members.tsx",
            "naming-convention.tsx",
            "no-access-state-in-setstate.tsx",
            "no-create-ref.tsx",
            "no-nested-component-definitions-2.jsx",
            "no-nested-component-definitions.tsx",
            "no-nested-lazy-component-declarations.tsx",
            "no-unnecessary-use-prefix-2.ts",
            "no-unnecessary-use-prefix.tsx",
            "no-unused-class-component-members-2.tsx",
            "no-unused-class-component-members.tsx",
            "parens.tsx",
            "purity-server.tsx",
            "purity.tsx",
            "react-18.tsx",
            "react-19-apis.tsx",
            "react-compiler.tsx",
            "rsc-function-definition-2.ts",
            "rsc-function-definition-client.tsx",
            "rsc-function-definition-default.ts",
            "rsc-function-definition.tsx",
            "set-state-in-lifecycle.tsx",
            "use-state.tsx",
        ],
        None,
        r#"{}"#,
        &[
            ("overlap-no-direct-mutation-state.tsx", 4, 25),
            ("overlap-no-direct-mutation-state.tsx", 4, 43),
            ("overlap-no-direct-mutation-state.tsx", 4, 60),
            ("overlap-no-direct-mutation-state.tsx", 5, 21),
            ("overlap-no-direct-mutation-state.tsx", 6, 32),
            ("overlap-no-direct-mutation-state.tsx", 10, 53),
            ("overlap-no-direct-mutation-state.tsx", 11, 80),
            ("overlap-no-direct-mutation-state.tsx", 15, 16),
            ("overlap-no-direct-mutation-state.tsx", 20, 9),
        ],
    );
}

#[test]
fn set_state_in_render() {
    run(
        "@eslint-react/set-state-in-render",
        &[
            "overlap-set-state-in-render.tsx",
            "lifecycle-members.tsx",
            "naming-convention.tsx",
            "no-access-state-in-setstate.tsx",
            "no-create-ref.tsx",
            "no-nested-component-definitions-2.jsx",
            "no-nested-component-definitions.tsx",
            "no-nested-lazy-component-declarations.tsx",
            "no-unnecessary-use-prefix-2.ts",
            "no-unnecessary-use-prefix.tsx",
            "no-unused-class-component-members-2.tsx",
            "no-unused-class-component-members.tsx",
            "parens.tsx",
            "purity-server.tsx",
            "purity.tsx",
            "react-18.tsx",
            "react-19-apis.tsx",
            "react-compiler.tsx",
            "rsc-function-definition-2.ts",
            "rsc-function-definition-client.tsx",
            "rsc-function-definition-default.ts",
            "rsc-function-definition.tsx",
            "set-state-in-lifecycle.tsx",
            "use-state.tsx",
        ],
        None,
        r#"{}"#,
        &[
            ("overlap-set-state-in-render.tsx", 4, 3),
            ("overlap-set-state-in-render.tsx", 11, 3),
            ("overlap-set-state-in-render.tsx", 16, 3),
            ("overlap-set-state-in-render.tsx", 17, 3),
            ("overlap-set-state-in-render.tsx", 33, 3),
            ("overlap-set-state-in-render.tsx", 38, 24),
            ("overlap-set-state-in-render.tsx", 39, 9),
            ("overlap-set-state-in-render.tsx", 47, 17),
            ("overlap-set-state-in-render.tsx", 52, 3),
            ("overlap-set-state-in-render.tsx", 55, 55),
        ],
    );
}

#[test]
fn static_components() {
    run(
        "@eslint-react/static-components",
        &[
            "overlap-static-components.tsx",
            "lifecycle-members.tsx",
            "naming-convention.tsx",
            "no-access-state-in-setstate.tsx",
            "no-create-ref.tsx",
            "no-nested-component-definitions-2.jsx",
            "no-nested-component-definitions.tsx",
            "no-nested-lazy-component-declarations.tsx",
            "no-unnecessary-use-prefix-2.ts",
            "no-unnecessary-use-prefix.tsx",
            "no-unused-class-component-members-2.tsx",
            "no-unused-class-component-members.tsx",
            "parens.tsx",
            "purity-server.tsx",
            "purity.tsx",
            "react-18.tsx",
            "react-19-apis.tsx",
            "react-compiler.tsx",
            "rsc-function-definition-2.ts",
            "rsc-function-definition-client.tsx",
            "rsc-function-definition-default.ts",
            "rsc-function-definition.tsx",
            "set-state-in-lifecycle.tsx",
            "use-state.tsx",
        ],
        None,
        r#"{}"#,
        &[
            ("no-nested-component-definitions-2.jsx", 5, 21),
            ("no-nested-component-definitions-2.jsx", 21, 11),
            ("no-nested-component-definitions.tsx", 4, 3),
            ("no-nested-component-definitions.tsx", 9, 8),
            ("no-nested-component-definitions.tsx", 15, 35),
            ("no-nested-component-definitions.tsx", 21, 11),
            ("no-nested-component-definitions.tsx", 26, 19),
            ("no-nested-component-definitions.tsx", 30, 13),
            ("no-nested-component-definitions.tsx", 36, 5),
            ("no-nested-component-definitions.tsx", 41, 13),
            ("no-nested-component-definitions.tsx", 104, 3),
            ("no-nested-component-definitions.tsx", 108, 11),
            ("no-nested-component-definitions.tsx", 113, 19),
            ("no-nested-component-definitions.tsx", 114, 13),
            ("no-nested-lazy-component-declarations.tsx", 7, 18),
            ("no-nested-lazy-component-declarations.tsx", 13, 11),
            ("no-nested-lazy-component-declarations.tsx", 33, 13),
            ("no-nested-lazy-component-declarations.tsx", 34, 11),
            ("overlap-static-components.tsx", 3, 17),
            ("overlap-static-components.tsx", 4, 11),
            ("overlap-static-components.tsx", 7, 3),
            ("overlap-static-components.tsx", 8, 11),
            ("overlap-static-components.tsx", 15, 16),
            ("overlap-static-components.tsx", 16, 11),
            ("overlap-static-components.tsx", 19, 16),
            ("overlap-static-components.tsx", 20, 11),
            ("overlap-static-components.tsx", 24, 17),
            ("overlap-static-components.tsx", 25, 11),
            ("overlap-static-components.tsx", 28, 16),
            ("overlap-static-components.tsx", 29, 11),
            ("overlap-static-components.tsx", 33, 19),
            ("overlap-static-components.tsx", 34, 13),
            ("overlap-static-components.tsx", 42, 23),
            ("overlap-static-components.tsx", 43, 11),
        ],
    );
}

#[test]
fn unsupported_syntax() {
    run(
        "@eslint-react/unsupported-syntax",
        &[
            "overlap-unsupported-syntax.tsx",
            "lifecycle-members.tsx",
            "naming-convention.tsx",
            "no-access-state-in-setstate.tsx",
            "no-create-ref.tsx",
            "no-nested-component-definitions-2.jsx",
            "no-nested-component-definitions.tsx",
            "no-nested-lazy-component-declarations.tsx",
            "no-unnecessary-use-prefix-2.ts",
            "no-unnecessary-use-prefix.tsx",
            "no-unused-class-component-members-2.tsx",
            "no-unused-class-component-members.tsx",
            "parens.tsx",
            "purity-server.tsx",
            "purity.tsx",
            "react-18.tsx",
            "react-19-apis.tsx",
            "react-compiler.tsx",
            "rsc-function-definition-2.ts",
            "rsc-function-definition-client.tsx",
            "rsc-function-definition-default.ts",
            "rsc-function-definition.tsx",
            "set-state-in-lifecycle.tsx",
            "use-state.tsx",
        ],
        None,
        r#"{}"#,
        &[
            ("overlap-unsupported-syntax.tsx", 2, 3),
            ("overlap-unsupported-syntax.tsx", 3, 3),
            ("overlap-unsupported-syntax.tsx", 7, 3),
        ],
    );
}

#[test]
fn use_memo() {
    run(
        "@eslint-react/use-memo",
        &[
            "overlap-use-memo.tsx",
            "lifecycle-members.tsx",
            "naming-convention.tsx",
            "no-access-state-in-setstate.tsx",
            "no-create-ref.tsx",
            "no-nested-component-definitions-2.jsx",
            "no-nested-component-definitions.tsx",
            "no-nested-lazy-component-declarations.tsx",
            "no-unnecessary-use-prefix-2.ts",
            "no-unnecessary-use-prefix.tsx",
            "no-unused-class-component-members-2.tsx",
            "no-unused-class-component-members.tsx",
            "parens.tsx",
            "purity-server.tsx",
            "purity.tsx",
            "react-18.tsx",
            "react-19-apis.tsx",
            "react-compiler.tsx",
            "rsc-function-definition-2.ts",
            "rsc-function-definition-client.tsx",
            "rsc-function-definition-default.ts",
            "rsc-function-definition.tsx",
            "set-state-in-lifecycle.tsx",
            "use-state.tsx",
        ],
        None,
        r#"{}"#,
        &[
            ("overlap-use-memo.tsx", 3, 3),
            ("overlap-use-memo.tsx", 4, 22),
            ("overlap-use-memo.tsx", 5, 21),
            ("overlap-use-memo.tsx", 6, 21),
            ("overlap-use-memo.tsx", 6, 21),
            ("overlap-use-memo.tsx", 8, 29),
            ("overlap-use-memo.tsx", 9, 21),
            ("overlap-use-memo.tsx", 11, 21),
            ("overlap-use-memo.tsx", 15, 3),
            ("overlap-use-memo.tsx", 16, 21),
            ("overlap-use-memo.tsx", 18, 20),
            ("overlap-use-memo.tsx", 20, 27),
            ("overlap-use-memo.tsx", 25, 34),
            ("overlap-use-memo.tsx", 25, 48),
            ("overlap-use-memo.tsx", 25, 61),
            ("overlap-use-memo.tsx", 26, 21),
            ("overlap-use-memo.tsx", 27, 19),
        ],
    );
}

#[test]
fn react_hooks_config() {
    run(
        "react-hooks/config",
        &["react-compiler.tsx", "no-nested-component-definitions.tsx"],
        None,
        r#"{}"#,
        &[],
    );
}

#[test]
fn react_hooks_gating() {
    run(
        "react-hooks/gating",
        &["react-compiler.tsx", "no-nested-component-definitions.tsx"],
        None,
        r#"{}"#,
        &[],
    );
}

// --- `--fix` parity: the expected `.fixed.*` files and remaining messages
// come from ESLint's `Linter#verifyAndFix` on tests/fixtures/react_components/fix.

const LIFECYCLE_FIXABLE: &[&str] = &[
    "@eslint-react/no-component-will-mount",
    "@eslint-react/no-component-will-receive-props",
    "@eslint-react/no-component-will-update",
];
const RSC: &[&str] = &["@eslint-react/rsc-function-definition"];

fn run_fix(fixture: &str, rules: &[&str], expected: &[(&str, usize, usize)]) {
    let fixtures =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/react_components/fix");
    let dir = std::env::temp_dir().join(format!(
        "webadeva-react-components-fix-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(fixtures.join(fixture), dir.join(fixture)).unwrap();
    let rules: serde_json::Map<String, serde_json::Value> = rules
        .iter()
        .map(|r| (r.to_string(), serde_json::json!("error")))
        .collect();
    std::fs::write(
        dir.join(".oxlintrc.json"),
        serde_json::json!({ "settings": {
            "react-x": { "version": "19.2.0" },
            "webadeva": { "rules": rules },
        } })
        .to_string(),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_webadeva-lint"))
        .args(["-f", "json", "--fix", "."])
        .current_dir(&dir)
        .output()
        .unwrap();
    let fixed = std::fs::read_to_string(dir.join(fixture)).unwrap();
    std::fs::remove_dir_all(&dir).ok();
    let (stem, ext) = fixture.rsplit_once('.').unwrap();
    let want = std::fs::read_to_string(fixtures.join(format!("{stem}.fixed.{ext}"))).unwrap();
    assert_eq!(fixed, want, "{fixture}");

    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "{e}: {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    });
    let mut got: Vec<(String, usize, usize)> = json["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| {
            (
                d["rule"].as_str().unwrap().to_string(),
                d["line"].as_u64().unwrap() as usize,
                d["column"].as_u64().unwrap() as usize,
            )
        })
        .collect();
    got.sort();
    let mut want: Vec<(String, usize, usize)> = expected
        .iter()
        .map(|(r, l, c)| (r.to_string(), *l, *c))
        .collect();
    want.sort();
    assert_eq!(got, want, "{fixture}");
}

#[test]
fn fix_lifecycle_renames() {
    run_fix("lifecycle.tsx", LIFECYCLE_FIXABLE, &[]);
}

#[test]
fn fix_rsc_exports_in_use_server_file() {
    run_fix("rsc-file.ts", RSC, &[]);
    run_fix("rsc-default.ts", RSC, &[]);
    run_fix("rsc-default-2.ts", RSC, &[]);
}

#[test]
fn fix_rsc_local_use_server() {
    // Getters, setters and constructors are reported without a fix.
    run_fix(
        "rsc-local.tsx",
        RSC,
        &[
            ("@eslint-react/rsc-function-definition", 22, 15),
            ("@eslint-react/rsc-function-definition", 26, 15),
            ("@eslint-react/rsc-function-definition", 43, 14),
            ("@eslint-react/rsc-function-definition", 64, 13),
        ],
    );
}

#[test]
fn fix_multi_rule_passes() {
    let rules: Vec<&str> = LIFECYCLE_FIXABLE
        .iter()
        .chain(RSC)
        .copied()
        .chain([
            "@eslint-react/no-unsafe-component-will-mount",
            "@eslint-react/no-unsafe-component-will-receive-props",
            "@eslint-react/no-unsafe-component-will-update",
            "@eslint-react/no-set-state-in-component-will-update",
            "@eslint-react/no-set-state-in-component-did-mount",
        ])
        .collect();
    run_fix(
        "multi.tsx",
        &rules,
        &[
            ("@eslint-react/no-unsafe-component-will-mount", 5, 3),
            ("@eslint-react/no-unsafe-component-will-update", 9, 3),
            (
                "@eslint-react/no-unsafe-component-will-receive-props",
                16,
                3,
            ),
            ("@eslint-react/no-set-state-in-component-did-mount", 18, 5),
        ],
    );
}

/// @eslint-react 3.0.0 (the rule left @eslint-react in v5) on ESLint 9.39.5,
/// over every fixture of this group: same positions, and the messages were
/// compared by hand, incl. the upstream `'null'` name of anonymous classes.
#[test]
fn component_hook_factories() {
    run(
        "@eslint-react/component-hook-factories",
        &[
            "component-hook-factories.tsx",
            "lifecycle-members.tsx",
            "naming-convention.tsx",
            "no-access-state-in-setstate.tsx",
            "no-create-ref.tsx",
            "no-nested-component-definitions-2.jsx",
            "no-nested-component-definitions.tsx",
            "no-nested-lazy-component-declarations.tsx",
            "no-unnecessary-use-prefix-2.ts",
            "no-unnecessary-use-prefix.tsx",
            "no-unused-class-component-members-2.tsx",
            "no-unused-class-component-members.tsx",
            "overlap-error-boundaries.tsx",
            "overlap-no-array-index-key.tsx",
            "overlap-no-clone-element.tsx",
            "overlap-no-direct-mutation-state.tsx",
            "overlap-set-state-in-render.tsx",
            "overlap-static-components.tsx",
            "overlap-unsupported-syntax.tsx",
            "overlap-use-memo.tsx",
            "parens.tsx",
            "purity-server.tsx",
            "purity.tsx",
            "react-18.tsx",
            "react-19-apis.tsx",
            "react-compiler.tsx",
            "rsc-function-definition-2.ts",
            "rsc-function-definition-client.tsx",
            "rsc-function-definition-default.ts",
            "rsc-function-definition.tsx",
            "set-state-in-lifecycle.tsx",
            "use-state.tsx",
        ],
        None,
        r#"{}"#,
        &[
            ("component-hook-factories.tsx", 12, 3),
            ("component-hook-factories.tsx", 15, 17),
            ("component-hook-factories.tsx", 16, 21),
            ("component-hook-factories.tsx", 17, 42),
            ("component-hook-factories.tsx", 22, 3),
            ("component-hook-factories.tsx", 26, 20),
            ("component-hook-factories.tsx", 27, 26),
            ("component-hook-factories.tsx", 32, 3),
            ("component-hook-factories.tsx", 37, 16),
            ("component-hook-factories.tsx", 42, 24),
            ("component-hook-factories.tsx", 64, 19),
            ("component-hook-factories.tsx", 73, 10),
            ("component-hook-factories.tsx", 79, 3),
            ("no-nested-component-definitions-2.jsx", 10, 3),
            ("no-nested-component-definitions-2.jsx", 16, 22),
            ("no-nested-component-definitions-2.jsx", 19, 20),
            ("no-nested-component-definitions-2.jsx", 35, 22),
            ("no-nested-component-definitions-2.jsx", 47, 17),
            ("no-nested-component-definitions-2.jsx", 52, 24),
            ("no-nested-component-definitions-2.jsx", 55, 41),
            ("no-nested-component-definitions-2.jsx", 60, 20),
            ("no-nested-component-definitions-2.jsx", 65, 17),
            ("no-nested-component-definitions-2.jsx", 72, 16),
            ("no-nested-component-definitions-2.jsx", 74, 15),
            ("no-nested-component-definitions-2.jsx", 75, 20),
            ("no-nested-component-definitions-2.jsx", 76, 17),
            ("no-nested-component-definitions.tsx", 104, 3),
            ("no-nested-component-definitions.tsx", 113, 19),
            ("no-nested-component-definitions.tsx", 15, 35),
            ("no-nested-component-definitions.tsx", 18, 25),
            ("no-nested-component-definitions.tsx", 19, 38),
            ("no-nested-component-definitions.tsx", 20, 32),
            ("no-nested-component-definitions.tsx", 26, 19),
            ("no-nested-component-definitions.tsx", 27, 5),
            ("no-nested-component-definitions.tsx", 36, 5),
            ("no-nested-component-definitions.tsx", 4, 3),
            ("no-nested-component-definitions.tsx", 59, 10),
            ("no-nested-component-definitions.tsx", 60, 8),
            ("no-nested-component-definitions.tsx", 69, 13),
            ("no-nested-component-definitions.tsx", 89, 16),
            ("no-nested-component-definitions.tsx", 95, 18),
            ("no-nested-component-definitions.tsx", 98, 16),
            ("no-unnecessary-use-prefix-2.ts", 21, 29),
            ("no-unnecessary-use-prefix.tsx", 46, 12),
            ("no-unnecessary-use-prefix.tsx", 52, 20),
            ("no-unused-class-component-members.tsx", 77, 5),
            ("overlap-static-components.tsx", 15, 21),
            ("overlap-static-components.tsx", 24, 17),
            ("overlap-static-components.tsx", 33, 19),
            ("overlap-static-components.tsx", 38, 17),
            ("overlap-static-components.tsx", 3, 17),
            ("overlap-static-components.tsx", 42, 23),
            ("overlap-static-components.tsx", 7, 3),
            ("parens.tsx", 24, 20),
            ("parens.tsx", 9, 19),
        ],
    );
}
