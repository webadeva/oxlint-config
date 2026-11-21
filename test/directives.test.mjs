// oxlint applies inline directives to add-on rules like to its own, so users
// silence them the same way.
import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  createProject,
  fatalErrors,
  linesOfCode,
  oxlint,
  ruleIds,
} from "./helpers.mjs";

const LONG = linesOfCode(400);
const RULE = "webadeva/max-lines";
const CAMEL = "eslint-js/camelcase";
const SNAKE = "export const my_value = 1;\n";

describe("disable directives", { concurrency: true }, () => {
  it("eslint-disable and oxlint-disable silence add-on rules", async () => {
    const root = createProject({
      preset: "base",
      files: {
        "tsconfig.json": JSON.stringify({ include: ["src"] }),
        "src/plain.ts": LONG,
        "src/eslint.ts": `/* eslint-disable ${RULE} -- generated */\n${LONG}`,
        "src/oxlint.ts": `/* oxlint-disable ${RULE} -- generated */\n${LONG}`,
        "src/all.ts": `/* eslint-disable -- generated */\n${LONG}`,
        "src/other.ts": `/* eslint-disable no-console -- generated */\n${LONG}`,
        "src/snake.ts": SNAKE,
        "src/next-line.ts": `// eslint-disable-next-line ${CAMEL} -- legacy name\n${SNAKE}`,
        "src/ox-line.ts": `// oxlint-disable-next-line ${CAMEL} -- legacy name\n${SNAKE}`,
      },
    });
    const { results } = await oxlint(root);
    assert.deepEqual(fatalErrors(results), []);
    assert.deepEqual(ruleIds(results, "src/plain.ts"), [RULE]);
    assert.deepEqual(ruleIds(results, "src/eslint.ts"), []);
    assert.deepEqual(ruleIds(results, "src/oxlint.ts"), []);
    assert.deepEqual(ruleIds(results, "src/all.ts"), []);
    assert.deepEqual(ruleIds(results, "src/other.ts"), [RULE]);
    assert.deepEqual(ruleIds(results, "src/snake.ts"), [CAMEL]);
    assert.deepEqual(ruleIds(results, "src/next-line.ts"), []);
    assert.deepEqual(ruleIds(results, "src/ox-line.ts"), []);
  });

  it("eslint-comments/require-description reports directives without a reason", async () => {
    const root = createProject({
      preset: "base",
      files: {
        "tsconfig.json": JSON.stringify({ include: ["src"] }),
        "src/bare.ts": `// eslint-disable-next-line ${CAMEL}\n${SNAKE}`,
      },
    });
    const { results } = await oxlint(root);
    assert.deepEqual(ruleIds(results, "src/bare.ts"), [
      "eslint-comments/require-description",
    ]);
  });
});
