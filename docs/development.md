# Development

This page is for anyone changing the package itself. If you only want to use it, the [README](../README.md) is all you need.

The package is two things glued together. The presets are plain JSON configs for Oxlint. The add-on is a small Oxlint JS plugin that hands each file to rules written in Rust, which Node loads as a native add-on. Users never run anything of ours directly; they run `oxlint`, and Oxlint calls into us.

Here's where everything lives:

```
crates/webadeva-lint/          the rules (Rust, built on oxc)
  src/rules/<group>/           rules, one Cargo feature per group; registered in the group's mod.rs
  src/rule.rs                  the `Rule` trait
  src/engine.rs                parses a file once and runs the enabled rules on it
  src/napi.rs                  the Node add-on (`napi` feature): `rules()` and `lint()`
  src/main.rs, config.rs,      a test harness binary (`cli` feature) that runs the rules the way
    linter.rs, fix.rs            ESLint would, with disable comments and the --fix loop; not shipped
  tests/                       parity tests per group, fixtures in tests/fixtures/
plugins/bridge.mjs             the JS plugin: one native call per file, then each rule reports
plugins/<namespace>.mjs        one entry per rule namespace, all built by bridge.mjs
scripts/preset-rules.mjs       rule tables per preset: `native` (Oxlint) and `rust` (the add-on)
scripts/addon-rules.mjs        maps the `rust` tables' ESLint ids to add-on ids and plugin files
scripts/build-presets.mjs      writes presets/*.json and presets/oxfmt.json from the tables
scripts/copy-native.mjs        copies the built library to native/webadeva.node
presets/                       generated; never edit by hand
native/                        the local add-on build (dev only, not published)
npm/scripts/                   the platform-package build script
test/                          end-to-end node:test suites
```

The rule groups are Cargo features, all on by default, so each group builds and tests on its own:

| Feature            | Rules                                                                                            |
| ------------------ | ------------------------------------------------------------------------------------------------ |
| `core`             | `webadeva/*`, `eslint-comments/require-description`, `prefer-arrow-functions`, ESLint core rules |
| `react_dom`        | `@eslint-react` DOM, JSX, Web API, and `Children` rules                                          |
| `react_components` | other `@eslint-react` rules, `react-hooks/config`, `react-hooks/gating`                          |
| `storybook`        | `storybook/*`                                                                                    |

The `napi` feature builds the add-on and `cli` (on by default) the harness.

### How a file is linted

Oxlint creates every enabled rule for a file before visiting it. Each add-on rule registers with `plugins/bridge.mjs`, and the first to reach `Program:exit` calls `native.lint(filename, source, rules, settings, cwd)` once for all of them: one oxc parse, however many rules are on. Each rule then reports its own results, with their fixes, through `context.report`. Severity, overrides, disable comments, and `--fix` stay Oxlint's job.

Inside the add-on, rules use their ESLint ids (`camelcase`, `react-hooks/config`). The bridge and `scripts/addon-rules.mjs` map the ones whose plugin name Oxlint reserves: ESLint core rules to `eslint-js/`, `react-hooks/` to `react-hooks-js/`.

The bridge loads the add-on from `WEBADEVA_OXLINT_NATIVE` if set, then from `native/webadeva.node` (a local build), then from the `@webadeva/oxlint-config-<platform>` package.

## Scripts

| Script                | Does                                                                                       |
| --------------------- | ------------------------------------------------------------------------------------------ |
| `pnpm build:presets`  | Generate `presets/` from the rule tables, and copy the formatter preset to `.oxfmtrc.json` |
| `pnpm build:native`   | Build the add-on in release mode to `native/webadeva.node`                                 |
| `pnpm build`          | Both of the above                                                                          |
| `pnpm lint`           | Lint the repo with `oxlint` and its own base preset (`.oxlintrc.json`)                     |
| `pnpm format`         | Format with oxfmt (`format:check` only checks)                                             |
| `pnpm test:rust`      | Run `cargo test`                                                                           |
| `pnpm test`           | Run `cargo test`, then the end-to-end tests; needs `pnpm build` first                      |
| `pnpm check`          | Build, lint, and test, in that order; also runs on publish                                 |
| `pnpm build:packages` | Build the per-platform npm packages (see below)                                            |

You need a stable Rust toolchain (`cargo`) besides Node.js and pnpm.

## Tests

- `cargo test` runs the unit tests and the parity tests in `crates/webadeva-lint/tests/`, one file per group. Each lints fixtures with the harness binary and compares every diagnostic (rule, line, column, message) and every fix to what ESLint produced on the same fixtures with the original plugins.
- `test/*.test.mjs` build throwaway projects outside the repo the way users set them up (an `.oxlintrc.json` extending a preset, a copy of `presets/oxfmt.json`), then run `oxlint` and `oxfmt` there:
  - `presets.test.mjs`: every preset lints, with the add-on's rules on.
  - `typescript-projects.test.mjs`, `react-projects.test.mjs`, and `typescript-file-in-project.test.mjs`: ports of the ESLint version's project setups, with tsgolint typing every file.
  - `storybook.test.mjs`, `directives.test.mjs`, and `ignore-patterns.test.mjs`: Storybook overrides, disable comments for add-on rules, and ignores.
  - `unused-imports.test.mjs`: `oxlint --fix` and `oxfmt` give the original's `eslint --fix` output.
  - `perf.test.mjs`: `oxlint` with the add-on finishes on a 500-file project, and the timings are printed.

## Adding a rule

The goal is parity: a rule reports exactly what the ESLint version of this config reports, at the same position, with the same message and fix. That includes the bugs of the original plugin. Only add a rule to the add-on when Oxlint lacks it or its native version behaves differently.

1. If Oxlint has the rule and matches ESLint, add it to the `native` table of a layer in `scripts/preset-rules.mjs` under its Oxlint id, and run `pnpm build:presets`.
2. Otherwise, port it in Rust to the group it belongs to under `crates/webadeva-lint/src/rules/<group>/`, implementing the `Rule` trait with the original ESLint id, and register it in the group's `mod.rs`. The bridge picks it up from `native.rules()`; there is no JS to write. A rule in a new namespace also needs a `plugins/<namespace>.mjs` entry and a line in `PLUGIN_FILES` in `scripts/addon-rules.mjs`.
3. Add fixtures to `crates/webadeva-lint/tests/fixtures/<group>/`. Generate the expected diagnostics and fixes by running ESLint with the original plugin on them, and assert them in the group's test file. Note the plugin versions in the test file.
4. Add the rule to the `rust` table of a layer in `scripts/preset-rules.mjs`, with the ESLint version's severity and options, and run `pnpm build`.
5. Document it in [rules.md](rules.md), and list any difference from ESLint in the README.

## CI, releases, and distribution

### How the add-on reaches users

The Rust rules ship as a Node native add-on (the crate's `cdylib`, built with napi-rs), the way oxlint, Biome, and esbuild ship their native code: one npm package per platform holding only `webadeva.node`, and the main package listing all of them as `optionalDependencies`. Each platform package sets `os`, `cpu`, and on Linux `libc`, so npm, pnpm, and Yarn download just the one that matches.

| Package                                    | Rust target                  |
| ------------------------------------------ | ---------------------------- |
| `@webadeva/oxlint-config-linux-x64-gnu`    | `x86_64-unknown-linux-gnu`   |
| `@webadeva/oxlint-config-linux-arm64-gnu`  | `aarch64-unknown-linux-gnu`  |
| `@webadeva/oxlint-config-linux-x64-musl`   | `x86_64-unknown-linux-musl`  |
| `@webadeva/oxlint-config-linux-arm64-musl` | `aarch64-unknown-linux-musl` |
| `@webadeva/oxlint-config-darwin-x64`       | `x86_64-apple-darwin`        |
| `@webadeva/oxlint-config-darwin-arm64`     | `aarch64-apple-darwin`       |
| `@webadeva/oxlint-config-win32-x64`        | `x86_64-pc-windows-msvc`     |
| `@webadeva/oxlint-config-win32-arm64`      | `aarch64-pc-windows-msvc`    |

There is no `bin` and no install script. `plugins/bridge.mjs`, the oxlint JS plugin, loads the add-on from `WEBADEVA_OXLINT_NATIVE` if set, then from `native/webadeva.node` (a development checkout, written by `pnpm build:native`), then from `@webadeva/oxlint-config-<platform>/webadeva.node`, trying musl first on musl Linux. The platform package is a dependency of the main package, so it resolves from `plugins/` under every package manager's layout.

The add-on is built with `--lib --no-default-features --features napi,core,react_dom,react_components,storybook`, which leaves out the parity-test harness (`cli` feature). `crates/webadeva-lint/build.rs` runs `napi_build::setup()` for the `napi` feature, which on macOS links Node's symbols with dynamic lookup.

`npm/scripts/build-packages.mjs` (`pnpm build:packages`) builds the platform packages into `npm/dist/<platform>/` (`package.json` with `main: "webadeva.node"`, and `webadeva.node`) from `npm/artifacts/<platform>/webadeva.node`, and pins them, at the main package's version, in `package.json#optionalDependencies`. The pins exist only at publish time. The committed `package.json` has none, so the lockfile never lists the platform packages. `--only <platform,...>` builds a subset.

### Workflows

- `.github/workflows/ci.yml` runs on pull requests. It checks commit messages, self-tests the version bump script, and runs `cargo fmt --check`, `cargo clippy --all-targets -D warnings` (default features) and a clippy of the add-on build, `cargo test`, `pnpm build` (presets and add-on), `pnpm lint`, the node tests, and the format check. The `smoke` job runs `.github/scripts/smoke-test.mjs` on Linux x64 and arm64, macOS, and Windows, each on Node 20.19 and 22.12: it packs the main package and the runner's platform package, installs them with npm and pnpm next to `oxlint` and `oxlint-tsgolint` (the repo's versions), writes a `.oxlintrc.json` extending each preset, and checks that `oxlint` reports a native rule (`no-console`) and add-on rules (`eslint-js/camelcase`, and `@eslint-react/no-missing-key` in the React presets), that no JS plugin fails, and that `oxlint --fix` applies an add-on fix (`prefer-arrow-functions`).
- `.github/workflows/release.yml` runs on pushes to `main`. It tests (as CI, plus the smoke test), picks the version bump (`.github/scripts/next-version.mjs`), and sets that version in `package.json`, `Cargo.toml`, and `Cargo.lock` (`.github/scripts/set-version.mjs`). It builds the eight add-ons (Linux with `cargo zigbuild`: gnu against glibc 2.17, musl with `RUSTFLAGS=-C target-feature=-crt-static` since a `cdylib` cannot link musl statically; macOS and Windows on their own runners), checks that the ones the runners can load list their rules, builds the platform packages, publishes them and then the main package with trusted publishing (provenance included), commits the version, tags, and creates a GitHub release with the add-ons attached as `webadeva-<platform>.node`. Commits starting with `no-release:` only run the tests.

Trusted publishing is configured per package on npmjs.com. Each new platform package needs one manual first publish and a trusted publisher entry (with `npm publish` allowed) before the workflow can publish it.

To try the packaging locally:

```sh
pnpm build
node .github/scripts/smoke-test.mjs            # npm and pnpm, this machine's platform
node .github/scripts/smoke-test.mjs --pm npm   # one package manager; --addon <path> for another build
```
