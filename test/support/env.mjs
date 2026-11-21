// Where the tools live and the fixtures shared by the suites.
import { existsSync, readFileSync, realpathSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

export const REPO = realpathSync(
  fileURLToPath(new URL("../..", import.meta.url)),
);
export const REPO_MODULES = join(REPO, "node_modules");

/**
 * Path of a package in the repo's node_modules, symlinks resolved.
 * @param {string} name
 * @returns {string}
 */
export const repoModule = (name) => {
  return realpathSync(join(REPO_MODULES, name));
};

export const OXLINT_DIR = repoModule("oxlint");
export const TSGOLINT_DIR = repoModule("oxlint-tsgolint");
export const OXFMT_DIR = repoModule("oxfmt");
// Stands in for the @webadeva/oxlint-config-<platform> package.
export const NATIVE = join(REPO, "native/webadeva.node");
if (!existsSync(NATIVE)) {
  throw new Error(`${NATIVE} is missing; run pnpm build:native first`);
}

export const PRESETS = ["base", "react", "next", "storybook"];
export const REACT_PRESETS = ["react", "next", "storybook"];

// A floating promise is only reported with type information, while
// `no-console` is reported either way.
export const SOURCE =
  "export const run = () => {\n  console.log(Promise.resolve(1));\n  Promise.resolve(2);\n};\n";
export const COMPONENT =
  'export const App = () => {\n  console.log(Promise.resolve(1));\n  Promise.resolve(2);\n  return <div class="greeting">hi</div>;\n};\n';
export const TYPED_RULE = "typescript/no-floating-promises";
// Only reported when the React rules are active.
export const REACT_RULE = "@eslint-react/dom-no-unknown-property";
export const NOT_IN_PROJECT_RULE = "webadeva/typescript-file-in-project";

/**
 * `n` lines of code, each its own statement.
 * @param {number} n
 * @returns {string}
 */
export const linesOfCode = (n) => {
  return (
    Array.from({ length: n }, (_, i) => {
      return `export const value${i} = ${i};`;
    }).join("\n") + "\n"
  );
};

/**
 * @typedef {{ files: string[], rules?: Record<string, unknown>, [key: string]: unknown }} Override
 * @typedef {{
 *   ignorePatterns?: string[],
 *   rules: Record<string, unknown>,
 *   overrides?: Override[],
 *   [key: string]: unknown,
 * }} Preset
 */

/**
 * @param {string} preset
 * @returns {Preset}
 */
export const readPreset = (preset) => {
  return /** @type {Preset} */ (
    JSON.parse(readFileSync(join(REPO, "presets", `${preset}.json`), "utf8"))
  );
};
