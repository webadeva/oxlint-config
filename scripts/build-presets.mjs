#!/usr/bin/env node
// Builds presets/*.json: flat oxlint configs (no `extends` between presets,
// because oxlint's `extends` drops `env`, `settings` and `ignorePatterns`).
// Dev-time only: the published package contains the generated JSON, no JS.
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { addonRules, PLUGIN_FILES } from "./addon-rules.mjs";
import * as tables from "./preset-rules.mjs";

const OUT = join(dirname(fileURLToPath(import.meta.url)), "..", "presets");

// Same as the ESLint config's ignores. oxlint skips dot-directories by
// default; the negations keep `.storybook` linted, as ESLint did.
const ignorePatterns = [
  "**/__tests__/**",
  "**/__mocks__/**",
  "dist",
  "*.config.*",
  "node_modules/**/*.*",
  "dist/**/*.*",
  "!.storybook",
  "!**/.storybook/**",
];

// ESLint: globals.browser + globals.node, on top of the latest ES globals.
const env = { builtin: true, browser: true, node: true };

// Only the rules listed here run: every category is turned off.
const categories = Object.fromEntries(
  [
    "correctness",
    "suspicious",
    "pedantic",
    "perf",
    "style",
    "restriction",
    "nursery",
  ].map((c) => {
    return [c, "off"];
  }),
);

const STORIES = [
  "**/*.stories.{ts,tsx,js,jsx,mjs,cjs}",
  "**/*.story.{ts,tsx,js,jsx,mjs,cjs}",
];
const STORYBOOK_MAIN = [
  ".storybook/main.{js,cjs,mjs,ts}",
  "**/.storybook/main.{js,cjs,mjs,ts}",
];

/** @typedef {{ native: Record<string, unknown>, rust: Record<string, unknown> }} Layer */

/**
 * @param {...Layer} layers
 * @returns {Layer}
 */
const merge = (...layers) => {
  const native = {};
  const rust = {};
  for (const layer of layers) {
    Object.assign(native, layer.native);
    Object.assign(rust, layer.rust);
  }
  return { native, rust };
};

/**
 * @param {object} options
 * @param {string[]} options.plugins
 * @param {Layer[]} options.layers
 * @param {Record<string, unknown>} [options.settings]
 * @param {{ files: string[], rules: Record<string, unknown> }[]} [options.overrides] native rules
 * @param {{ files: string[], rules: Record<string, unknown> }[]} [options.rustOverrides] rust-table rules
 */
const preset = ({
  plugins,
  layers,
  settings,
  overrides = [],
  rustOverrides = [],
}) => {
  const { native, rust } = merge(...layers);
  /** @type {Set<string>} */
  const namespaces = new Set();
  const rules = { ...native, ...addonRules(rust, namespaces) };
  const extraOverrides = rustOverrides.map(({ files, rules: r }) => {
    return { files, rules: addonRules(r, namespaces) };
  });
  return {
    plugins,
    // The rules oxlint lacks, implemented in Rust and loaded through a thin
    // JS plugin. Paths are relative to this file, so they resolve when the
    // preset is extended from node_modules.
    jsPlugins: [...namespaces].sort().map((name) => {
      return { name, specifier: `../plugins/${PLUGIN_FILES[name]}.mjs` };
    }),
    categories,
    options: { typeAware: true },
    env,
    ignorePatterns,
    rules,
    overrides: [
      // `env` inside an override survives oxlint's `extends`; top-level `env`
      // doesn't. Keep both so the preset works extended or used directly.
      { files: ["**/*"], env },
      ...overrides,
      ...extraOverrides,
    ],
    ...(settings ? { settings } : {}),
  };
};

const presets = {
  base: preset({
    plugins: ["eslint", "typescript"],
    layers: [tables.base],
  }),
  react: preset({
    plugins: ["eslint", "typescript", "react", "jsx-a11y"],
    layers: [tables.base, tables.react],
  }),
  next: preset({
    plugins: ["eslint", "typescript", "react", "jsx-a11y", "nextjs"],
    layers: [tables.base, tables.react, tables.next],
  }),
  storybook: preset({
    plugins: ["eslint", "typescript", "react", "jsx-a11y"],
    layers: [tables.base, tables.react],
    // eslint-plugin-storybook's flat/recommended turns rules-of-hooks off in
    // stories, where `render` functions call hooks.
    overrides: [{ files: STORIES, rules: { "react/rules-of-hooks": "off" } }],
    rustOverrides: [
      { files: STORIES, rules: tables.stories.rust },
      { files: STORYBOOK_MAIN, rules: tables.storybookMain.rust },
    ],
  }),
};

// Formatter preset, the oxfmt equivalent of the ESLint config's Prettier
// setup (.prettierrc + prettier-plugin-organize-imports).
const oxfmt = {
  printWidth: 80,
  tabWidth: 2,
  useTabs: false,
  singleQuote: false,
  bracketSpacing: true,
  // Prettier never reordered package.json.
  sortPackageJson: false,
  sortImports: {
    ignoreCase: true,
    newlinesBetween: false,
    // organize-imports order: package imports, then relative ones, each A-Z.
    groups: [
      ["builtin", "external", "internal", "unknown"],
      ["parent", "sibling", "index"],
    ],
  },
};

mkdirSync(OUT, { recursive: true });
for (const [name, config] of Object.entries(presets)) {
  writeFileSync(
    join(OUT, `${name}.json`),
    `${JSON.stringify(config, null, 2)}\n`,
  );
}
writeFileSync(join(OUT, "oxfmt.json"), `${JSON.stringify(oxfmt, null, 2)}\n`);
console.log(`presets written to ${OUT}`);
