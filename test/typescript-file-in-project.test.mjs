// webadeva/typescript-file-in-project: tsgolint types every file, so files
// outside every tsconfig are flagged by this rule instead (see
// typescript-projects.test.mjs).
import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  createProject,
  fatalErrors,
  NOT_IN_PROJECT_RULE,
  oxlint,
  ruleIds,
  SOURCE,
} from "./helpers.mjs";

/**
 * Files of `files` the rule reports, after a plain `oxlint` run.
 * @param {import("./support/project.mjs").ProjectOptions} options
 * @returns {Promise<string[]>}
 */
const flagged = async (options) => {
  const { results } = await oxlint(
    createProject({ preset: "base", ...options }),
  );
  assert.deepEqual(fatalErrors(results), []);
  return Object.keys(options.files)
    .filter((file) => {
      return ruleIds(results, file).includes(NOT_IN_PROJECT_RULE);
    })
    .sort();
};

describe(
  "TypeScript files outside every tsconfig",
  { concurrency: true },
  () => {
    it("reports TypeScript files outside every tsconfig, except root-level ones", async () => {
      const files = await flagged({
        typescript: true,
        files: {
          "tsconfig.json": JSON.stringify({ include: ["src"] }),
          "src/main.ts": SOURCE,
          "scripts/tool.ts": SOURCE,
          // Root-level files were typed through the original's default
          // project.
          "setup.ts": SOURCE,
          // JS is out of scope for the rule.
          "scripts/build.js": SOURCE,
        },
      });
      assert.deepEqual(files, ["scripts/tool.ts"]);
    });

    it("follows solution-style references when looking for a file's tsconfig", async () => {
      const files = await flagged({
        typescript: true,
        files: {
          "tsconfig.json": JSON.stringify({
            files: [],
            references: [{ path: "./tsconfig.app.json" }],
          }),
          "tsconfig.app.json": JSON.stringify({
            compilerOptions: { composite: true },
            include: ["src"],
          }),
          "src/main.ts": SOURCE,
          "scripts/tool.ts": SOURCE,
        },
      });
      assert.deepEqual(files, ["scripts/tool.ts"]);
    });

    it("without a tsconfig, reports every TypeScript file but root-level ones", async () => {
      // Like the original: only root-level files are in the default project.
      const files = await flagged({
        files: { "src/main.ts": SOURCE, "index.ts": SOURCE },
      });
      assert.deepEqual(files, ["src/main.ts"]);
    });
  },
);
