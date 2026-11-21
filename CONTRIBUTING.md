# Contributing

Thanks for helping out! Bug reports, rule suggestions, and pull requests are all welcome.

## Setup

You need Node.js `>=20.19`, pnpm, and a stable Rust toolchain. The repo pins its pnpm version, so `corepack enable` picks the right one.

```bash
git clone https://github.com/webadeva/oxlint-config.git
cd oxlint-config
pnpm install
pnpm check
```

`pnpm check` builds the presets and the native add-on, lints, and runs the Rust and end-to-end tests. It's the same check that runs before publishing.

## Making a change

1. Edit the rule tables in `scripts/preset-rules.mjs` or the Rust source in `crates/webadeva-lint/`. Never edit `presets/`; it's generated.
2. Add or update tests: fixtures with ESLint's expected output in `crates/webadeva-lint/tests/`, end-to-end tests in `test/`.
3. If behavior changes, update [README.md](README.md) or [docs/rules.md](docs/rules.md).
4. Run `pnpm check` and `pnpm format`, and make sure both pass. CI also runs `cargo fmt --check` and `cargo clippy`.

[docs/development.md](docs/development.md) covers the source layout, the scripts, the test suites, and how to add a rule.

## Commit messages

Commits follow [Conventional Commits](https://www.conventionalcommits.org/), e.g. `fix: handle empty tsconfig`. A git hook checks every commit message, and CI checks every commit in a pull request.

## Releases

Every merge to `main` publishes a new version to npm, unless its message starts with `no-release:`. The commit message on `main` picks the version bump:

| Message starts with | Bump  | Example                     |
| ------------------- | ----- | --------------------------- |
| `major:`            | major | `major: drop Node.js 20`    |
| `minor:`            | minor | `minor: add a Vue preset`   |
| `no-release:`       | none  | `no-release: update docs`   |
| anything else       | patch | `fix: count JSX text lines` |

When squash-merging, set the merge commit's title accordingly. A `no-release:` commit is still tested, just not published. Release commits (`chore(release): ...`) don't trigger another release.

## Pull requests

`main` only accepts pull requests. A git hook blocks direct pushes to it and runs `pnpm check` before every push.

- Keep each pull request to one change, and explain why it's needed.
- Rule changes affect everyone who uses the config, so say what each one catches or allows, ideally with a code example.
- This package follows [`@webadeva/eslint-config`](https://github.com/webadeva/eslint-config). A rule change should land in both, so the two report the same problems.
- Keep files short. The repo lints itself, so the [file-length limits](docs/rules.md) apply here too.

## Reporting issues

Open an issue with your Oxlint, oxlint-tsgolint, and package versions, your platform, the preset you use, and a minimal file that shows the problem. If the ESLint version reports something different on that file, say what it reports.
