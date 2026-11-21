// Ports the TypeScript project setups of @webadeva/eslint-config. There,
// typescript-eslint's project service only typed files in a tsconfig (plus
// root-level files via the default project). tsgolint instead types every
// file, inside a tsconfig or not, so where the original asserted
// `typed: false` these assert `true`. Files outside every tsconfig are still
// flagged, by webadeva/typescript-file-in-project
// (typescript-file-in-project.test.mjs).
import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  createProject,
  fatalErrors,
  isTyped,
  oxlint,
  ruleIds,
  SOURCE,
} from "./helpers.mjs";

/**
 * @param {Omit<import("./support/project.mjs").ProjectOptions, "preset">} options
 * @returns {Promise<import("./support/tools.mjs").Results>}
 */
const lint = async (options) => {
  const root = createProject({ preset: "base", ...options });
  const { results } = await oxlint(root);
  assert.deepEqual(fatalErrors(results), []);
  return results;
};

describe("TypeScript project setups", { concurrency: true }, () => {
  it("lints projects without TypeScript installed", async () => {
    // The original refused to run; tsgolint bundles its own TypeScript.
    const results = await lint({
      files: { "index.js": SOURCE, "src/util.ts": SOURCE },
    });
    for (const file of ["index.js", "src/util.ts"]) {
      assert.ok(ruleIds(results, file).includes("no-console"), file);
      assert.equal(isTyped(results, file), true, file);
    }
  });

  it("TypeScript installed without a tsconfig", async () => {
    const results = await lint({
      typescript: true,
      files: { "index.js": SOURCE, "src/util.js": SOURCE },
    });
    assert.equal(isTyped(results, "index.js"), true);
    // Original: false (outside the default project). tsgolint types it.
    assert.equal(isTyped(results, "src/util.js"), true);
  });

  it("tsconfig with allowJs", async () => {
    const results = await lint({
      typescript: true,
      files: {
        "tsconfig.json": JSON.stringify({
          compilerOptions: { allowJs: true, noEmit: true },
          include: ["src"],
        }),
        "src/util.js": SOURCE,
        "src/main.ts": SOURCE,
        "scripts/build.js": SOURCE,
      },
    });
    assert.equal(isTyped(results, "src/util.js"), true);
    assert.equal(isTyped(results, "src/main.ts"), true);
    // Original: false (outside the tsconfig). tsgolint types it.
    assert.equal(isTyped(results, "scripts/build.js"), true);
  });

  it("tsconfig without allowJs", async () => {
    const results = await lint({
      typescript: true,
      files: {
        "tsconfig.json": JSON.stringify({ include: ["src"] }),
        "src/util.js": SOURCE,
        "src/main.ts": SOURCE,
      },
    });
    // Original: false (JS not part of the program). tsgolint types it.
    assert.equal(isTyped(results, "src/util.js"), true);
    assert.equal(isTyped(results, "src/main.ts"), true);
  });

  it("solution-style tsconfig with project references", async () => {
    const results = await lint({
      typescript: true,
      files: {
        "tsconfig.json": JSON.stringify({
          files: [],
          references: [{ path: "./tsconfig.app.json" }],
        }),
        "tsconfig.app.json": JSON.stringify({
          // Without outDir, emitting src/util.js would overwrite itself;
          // tsgolint reports that as typescript/tsconfig-error and then types
          // nothing (typescript-eslint never checked it).
          compilerOptions: { composite: true, allowJs: true, outDir: "out" },
          include: ["src"],
        }),
        "src/util.js": SOURCE,
        "src/main.ts": SOURCE,
      },
    });
    assert.equal(isTyped(results, "src/util.js"), true);
    assert.equal(isTyped(results, "src/main.ts"), true);
  });

  it("reports an invalid tsconfig", async () => {
    const root = createProject({
      preset: "base",
      files: {
        "tsconfig.json": JSON.stringify({
          compilerOptions: { composite: true, allowJs: true },
          include: ["src"],
        }),
        "src/util.js": SOURCE,
      },
    });
    const { results } = await oxlint(root);
    assert.ok(
      ruleIds(results, "tsconfig.json").includes("typescript/tsconfig-error"),
    );
  });

  it("types TypeScript files outside every tsconfig", async () => {
    const results = await lint({
      typescript: true,
      files: {
        "tsconfig.json": JSON.stringify({ include: ["src"] }),
        "src/main.ts": SOURCE,
        "scripts/tool.ts": SOURCE,
        "setup.ts": SOURCE,
      },
    });
    assert.equal(isTyped(results, "src/main.ts"), true);
    // Original: false. tsgolint types files outside every tsconfig too.
    assert.equal(isTyped(results, "scripts/tool.ts"), true);
    assert.equal(isTyped(results, "setup.ts"), true);
  });
});
