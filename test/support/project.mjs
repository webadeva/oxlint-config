// Builds throwaway projects that consume the package the way users do:
// `pnpm add -D @webadeva/oxlint-config oxlint oxlint-tsgolint oxfmt`, then an
// `.oxlintrc.json` extending a preset and a copy of presets/oxfmt.json.
import {
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { after } from "node:test";
import {
  NATIVE,
  OXFMT_DIR,
  OXLINT_DIR,
  readPreset,
  REPO,
  repoModule,
  TSGOLINT_DIR,
} from "./env.mjs";

/** @type {string[]} */
const roots = [];
after(() => {
  if (process.env.KEEP_PROJECTS) return;
  for (const root of roots) rmSync(root, { recursive: true, force: true });
});

/**
 * @param {string} target
 * @param {string} path
 */
const link = (target, path) => {
  mkdirSync(dirname(path), { recursive: true });
  symlinkSync(target, path);
};

/**
 * @typedef {{ rules?: Record<string, unknown>, overrides?: object[], settings?: object, [key: string]: unknown }} ExtraConfig
 */

/**
 * Writes the documented setup: `.oxlintrc.json` extending the preset (with
 * its ignorePatterns copied, since `extends` drops them) plus `extra`, and
 * `.oxfmtrc.json` copied from presets/oxfmt.json.
 * @param {string} root
 * @param {string} preset
 * @param {ExtraConfig} [extra]
 */
export const writeConfig = (root, preset, extra = {}) => {
  const config = {
    extends: [`./node_modules/@webadeva/oxlint-config/presets/${preset}.json`],
    ignorePatterns: readPreset(preset).ignorePatterns,
    ...extra,
  };
  writeFileSync(
    join(root, ".oxlintrc.json"),
    JSON.stringify(config, null, 2) + "\n",
  );
  copyFileSync(join(REPO, "presets/oxfmt.json"), join(root, ".oxfmtrc.json"));
};

/**
 * @typedef {object} ProjectOptions
 * @property {Record<string, string>} files
 * @property {string} [preset]
 * @property {ExtraConfig} [config] Extra `.oxlintrc.json` keys.
 * @property {boolean} [typescript]
 * @property {boolean} [reactTypes]
 * @property {Record<string, unknown>} [packageJson]
 */

/**
 * Links what a user would install into node_modules (single packages only:
 * a linked node_modules dir would make tsgolint crawl it).
 * @param {string} nm
 * @param {{ typescript: boolean, reactTypes: boolean }} options
 */
const install = (nm, { typescript, reactTypes }) => {
  // Only what the published package ships (package.json "files"); the repo's
  // crates/**/fixtures hold nested .oxlintrc.json files oxlint would pick up.
  const pkg = join(nm, "@webadeva/oxlint-config");
  for (const entry of ["package.json", "presets", "plugins"]) {
    link(join(REPO, entry), join(pkg, entry));
  }
  // bridge.mjs resolves ../native/webadeva.node from the real plugins dir.
  link(NATIVE, join(pkg, "native/webadeva.node"));
  link(OXLINT_DIR, join(nm, "oxlint"));
  link(TSGOLINT_DIR, join(nm, "oxlint-tsgolint"));
  link(OXFMT_DIR, join(nm, "oxfmt"));
  // oxlint looks for tsgolint in node_modules/.bin.
  link(join(OXLINT_DIR, "bin/oxlint"), join(nm, ".bin/oxlint"));
  link(join(TSGOLINT_DIR, "bin/tsgolint.js"), join(nm, ".bin/tsgolint"));
  link(join(OXFMT_DIR, "bin/oxfmt"), join(nm, ".bin/oxfmt"));
  if (typescript) link(repoModule("typescript"), join(nm, "typescript"));
  if (reactTypes) link(repoModule("@types/react"), join(nm, "@types/react"));
};

/**
 * Writes `files` into a fresh temp dir outside the repo, installs the
 * packages and, with `preset`, writes the config files.
 * @param {ProjectOptions} options
 * @returns {string} The project root.
 */
export const createProject = ({
  files,
  preset,
  config,
  typescript = false,
  reactTypes = false,
  packageJson = { type: "module" },
}) => {
  const root = realpathSync(mkdtempSync(join(tmpdir(), "oxlint-config-e2e-")));
  roots.push(root);

  /** @type {Record<string, string>} */
  const all = {
    "package.json": JSON.stringify(packageJson, null, 2) + "\n",
    ...files,
  };
  for (const [path, content] of Object.entries(all)) {
    mkdirSync(dirname(join(root, path)), { recursive: true });
    writeFileSync(join(root, path), content);
  }
  install(join(root, "node_modules"), { typescript, reactTypes });
  if (preset) writeConfig(root, preset, config);
  return root;
};
