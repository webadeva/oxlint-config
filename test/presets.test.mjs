import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  createProject,
  fatalErrors,
  linesOfCode,
  oxlint,
  PRESETS,
  REACT_PRESETS,
  readPreset,
  ruleIds,
  SOURCE,
} from "./helpers.mjs";

const TSCONFIG = JSON.stringify({ include: ["src"] });
// no-console (native oxlint) and eslint-js/camelcase (add-on).
const SAMPLE =
  "export const greet = (name: string): string => {\n  const my_greeting = `Hello, ${name}`;\n  console.log(my_greeting);\n  return my_greeting;\n};\n";
const LOGO =
  'export const Logo = () => {\n  return <img src="logo.png" />;\n};\n';

describe("presets", { concurrency: true }, () => {
  for (const preset of PRESETS) {
    it(`oxlint runs native and add-on rules with the ${preset} preset`, async () => {
      const root = createProject({
        preset,
        files: { "tsconfig.json": TSCONFIG, "src/sample.ts": SAMPLE },
      });
      const { results, files } = await oxlint(root);
      assert.equal(files, 1);
      assert.deepEqual(fatalErrors(results), []);
      const ids = ruleIds(results, "src/sample.ts");
      assert.ok(ids.includes("no-console"), ids.join(", "));
      assert.ok(ids.includes("eslint-js/camelcase"), ids.join(", "));
    });

    it(`enables both file length rules in the ${preset} preset`, () => {
      const { rules } = readPreset(preset);
      assert.equal(rules["webadeva/max-lines-warning"], "warn");
      assert.equal(rules["webadeva/max-lines"], "error");
    });

    it(`reports long files with the ${preset} preset`, async () => {
      const root = createProject({
        preset,
        files: {
          "tsconfig.json": TSCONFIG,
          "src/short.ts": linesOfCode(20),
          "src/medium.ts": linesOfCode(160),
          "src/long.ts": linesOfCode(400),
        },
      });
      const { results, raw } = await oxlint(root);
      assert.deepEqual(ruleIds(results, "src/short.ts"), []);
      assert.deepEqual(ruleIds(results, "src/medium.ts"), [
        "webadeva/max-lines-warning",
      ]);
      // Past the error limit, only max-lines reports.
      assert.deepEqual(ruleIds(results, "src/long.ts"), ["webadeva/max-lines"]);
      const severities = raw.diagnostics.map((d) => {
        return `${d.filename} ${d.severity}`;
      });
      assert.deepEqual(severities.sort(), [
        "src/long.ts error",
        "src/medium.ts warning",
      ]);
    });
  }

  for (const preset of REACT_PRESETS) {
    it(`reports JSX a11y issues with the ${preset} preset`, async () => {
      const root = createProject({ preset, files: { "src/Logo.tsx": LOGO } });
      const { results } = await oxlint(root);
      assert.deepEqual(fatalErrors(results), []);
      assert.ok(ruleIds(results, "src/Logo.tsx").includes("jsx-a11y/alt-text"));
    });
  }

  it("reports <img> with nextjs/no-img-element only in the next preset", async () => {
    const [next, react] = await Promise.all(
      ["next", "react"].map((preset) => {
        return oxlint(
          createProject({ preset, files: { "src/Logo.tsx": LOGO } }),
        );
      }),
    );
    const rule = "nextjs/no-img-element";
    assert.ok(ruleIds(next.results, "src/Logo.tsx").includes(rule));
    assert.ok(!ruleIds(react.results, "src/Logo.tsx").includes(rule));
  });

  it("the base preset does not load the React rules", async () => {
    const root = createProject({
      preset: "base",
      files: { "src/Logo.tsx": LOGO, "src/run.ts": SOURCE },
    });
    const { results } = await oxlint(root);
    assert.deepEqual(fatalErrors(results), []);
    assert.deepEqual(
      ruleIds(results, "src/Logo.tsx").filter((r) => {
        return /^(react|jsx-a11y|nextjs|@eslint-react|react-hooks-js)\//.test(
          r,
        );
      }),
      [],
    );
  });
});
