#!/usr/bin/env node
// Usage: node set-version.mjs <version>
// Sets the version of the npm package and of the webadeva-lint crate (and its
// Cargo.lock entry), so the native add-on and its platform packages carry the
// npm release's version.
import { readFileSync, writeFileSync } from "node:fs";

const version = process.argv[2];
if (!/^\d+\.\d+\.\d+$/.test(version ?? "")) {
  console.error(`usage: set-version.mjs <x.y.z>, got ${version}`);
  process.exit(1);
}

/**
 * @param {string} path
 * @param {RegExp} pattern
 * @param {string} replacement
 */
const edit = (path, pattern, replacement) => {
  const text = readFileSync(path, "utf8");
  if (!pattern.test(text)) throw new Error(`${path}: no version to replace`);
  writeFileSync(path, text.replace(pattern, replacement));
};

edit("package.json", /("version":\s*)"[^"]*"/, `$1"${version}"`);
// The first `version =` in Cargo.toml is the [package] one.
edit(
  "crates/webadeva-lint/Cargo.toml",
  /^version = "[^"]*"/m,
  `version = "${version}"`,
);
edit(
  "crates/webadeva-lint/Cargo.lock",
  /(name = "webadeva-lint"\r?\nversion = )"[^"]*"/,
  `$1"${version}"`,
);
console.log(`version set to ${version}`);
