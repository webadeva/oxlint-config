import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { createProject, linesOfCode, oxlint, SOURCE } from "./helpers.mjs";

// Long enough for webadeva/max-lines, and has a console call for no-console.
const LONG = `${SOURCE}${linesOfCode(400)}`;
const IGNORED = [
  "dist/index.ts",
  "packages/app/dist/index.ts",
  "vite.config.ts",
  "src/tailwind.config.mjs",
  "__tests__/a.ts",
  "src/__tests__/b.ts",
  "src/__mocks__/c.ts",
];
const LINTED = [
  "src/index.ts",
  "src/distance.ts",
  "src/config.ts",
  ".storybook/main.ts",
];
const files = Object.fromEntries(
  [...IGNORED, ...LINTED].map((f) => {
    return [f, LONG];
  }),
);

describe("ignorePatterns", { concurrency: true }, () => {
  it("are respected by native and add-on rules", async () => {
    const root = createProject({ preset: "base", files });
    const { results, files: count } = await oxlint(root);
    assert.equal(count, LINTED.length);
    assert.deepEqual(Object.keys(results).sort(), [...LINTED].sort());
    for (const file of LINTED) {
      const ids = results[file]?.ruleIds ?? [];
      assert.ok(ids.includes("no-console"), file);
      assert.ok(ids.includes("webadeva/max-lines"), file);
    }
  });
});
