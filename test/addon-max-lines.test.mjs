// webadeva/max-lines and max-lines-warning: root settings, and the same keys
// as rule options (which, unlike settings, work in overrides).
import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  createProject,
  fatalErrors,
  linesOfCode,
  oxlint,
  ruleIds,
} from "./helpers.mjs";

const FILES = {
  "tsconfig.json": JSON.stringify({ include: ["src", "legacy"] }),
  "src/a.ts": linesOfCode(250),
  "legacy/b.ts": linesOfCode(250),
  "src/c.ts": linesOfCode(80),
  "src/d.ts": linesOfCode(120),
};

/**
 * Lints FILES with the base preset plus `config`.
 * @param {import("./support/project.mjs").ExtraConfig} config
 */
const lint = async (config) => {
  const run = await oxlint(
    createProject({ preset: "base", config, files: FILES }),
  );
  assert.deepEqual(fatalErrors(run.results), []);
  return run;
};

describe("max-lines configuration", { concurrency: true }, () => {
  it("takes limits per path as rule options in overrides", async () => {
    const { results } = await lint({
      overrides: [
        {
          files: ["legacy/**"],
          rules: { "webadeva/max-lines": ["error", { error: 300 }] },
        },
      ],
    });
    assert.deepEqual(ruleIds(results, "src/a.ts"), ["webadeva/max-lines"]);
    assert.ok(!ruleIds(results, "legacy/b.ts").includes("webadeva/max-lines"));
  });

  it("warns up to a limit raised on max-lines alone", async () => {
    // The two rules are one setting: options on either apply to both, so
    // raising the error limit also moves the warning's upper bound.
    const { results } = await lint({
      overrides: [
        {
          files: ["legacy/**"],
          rules: { "webadeva/max-lines": ["error", { error: 300 }] },
        },
      ],
    });
    assert.deepEqual(ruleIds(results, "legacy/b.ts"), [
      "webadeva/max-lines-warning",
    ]);
  });

  it("reads root settings.webadeva.maxLines", async () => {
    const { results } = await lint({
      settings: { webadeva: { maxLines: { warn: 50, error: 100 } } },
    });
    assert.deepEqual(ruleIds(results, "src/c.ts"), [
      "webadeva/max-lines-warning",
    ]);
    assert.deepEqual(ruleIds(results, "src/d.ts"), ["webadeva/max-lines"]);
  });

  it("is turned off by a `false` option", async () => {
    const { results } = await lint({
      rules: {
        "webadeva/max-lines": ["error", false],
        "webadeva/max-lines-warning": ["warn", false],
      },
    });
    assert.deepEqual(results, {});
  });
});
