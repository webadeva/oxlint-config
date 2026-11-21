// Conventional Commits plus the release prefixes this repo uses:
// `major:` / `minor:` pick the version bump, `patch:` (or any other type) is a patch,
// and `no-release:` skips the release.
export default {
  extends: ["@commitlint/config-conventional"],
  // The default header pattern only allows word characters in the type, which rejects `no-release`.
  parserPreset: {
    parserOpts: { headerPattern: /^([\w-]+)(?:\((.*)\))?!?: (.*)$/ },
  },
  rules: {
    "type-enum": [
      2,
      "always",
      [
        "major",
        "minor",
        "patch",
        "no-release",
        "build",
        "chore",
        "ci",
        "docs",
        "feat",
        "fix",
        "perf",
        "refactor",
        "revert",
        "style",
        "test",
      ],
    ],
  },
};
