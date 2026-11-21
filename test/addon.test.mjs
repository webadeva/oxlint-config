// The package is an oxlint add-on: users run plain `oxlint`, and add-on
// rules are configured like any oxlint rule in .oxlintrc.json.
import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  createProject,
  fatalErrors,
  oxlint,
  read,
  ruleIds,
} from "./helpers.mjs";

const CAMEL = "eslint-js/camelcase";
const SNAKE = "export const my_value = 1;\n";
const FILES = {
  "tsconfig.json": JSON.stringify({ include: ["src", "legacy"] }),
  "src/a.ts": SNAKE,
  "legacy/b.ts": SNAKE,
};

/**
 * Lints `files` with the base preset plus `config`.
 * @param {import("./support/project.mjs").ExtraConfig} config
 * @param {Record<string, string>} [files]
 */
const lint = async (config, files = FILES) => {
  const run = await oxlint(createProject({ preset: "base", config, files }));
  assert.deepEqual(fatalErrors(run.results), []);
  return run;
};

describe(
  "add-on rules are configured like oxlint rules",
  { concurrency: true },
  () => {
    it("are reported by plain oxlint with the preset's severity", async () => {
      const { results, raw, code } = await lint({});
      assert.equal(code, 1);
      assert.deepEqual(ruleIds(results, "src/a.ts"), [CAMEL]);
      assert.equal(raw.diagnostics[0]?.severity, "error");
    });

    it("can be turned off", async () => {
      const { results, code } = await lint({ rules: { [CAMEL]: "off" } });
      assert.equal(code, 0);
      assert.deepEqual(results, {});
    });

    it("can change severity", async () => {
      const { raw } = await lint({ rules: { [CAMEL]: "warn" } });
      assert.deepEqual(
        raw.diagnostics.map((d) => {
          return d.severity;
        }),
        ["warning", "warning"],
      );
    });

    it("take options", async () => {
      const { results } = await lint({
        rules: { [CAMEL]: ["error", { allow: ["my_value"] }] },
      });
      assert.deepEqual(results, {});
    });

    it("can be changed per path with overrides", async () => {
      const { results } = await lint({
        overrides: [{ files: ["legacy/**"], rules: { [CAMEL]: "off" } }],
      });
      assert.deepEqual(ruleIds(results, "src/a.ts"), [CAMEL]);
      assert.deepEqual(ruleIds(results, "legacy/b.ts"), []);
    });
  },
);

describe("oxlint --fix", { concurrency: true }, () => {
  it("applies add-on fixes", async () => {
    const root = createProject({
      preset: "react",
      typescript: true,
      reactTypes: true,
      files: {
        "tsconfig.json": JSON.stringify({
          compilerOptions: { jsx: "react-jsx" },
          include: ["src"],
        }),
        "src/math.ts":
          "export function double(n: number): number {\n  return n * 2;\n}\n",
        "src/App.tsx":
          'export const App = () => {\n  return <div class="greeting">hi</div>;\n};\n',
      },
    });
    const { results } = await oxlint(root, ["--fix"]);
    assert.deepEqual(results, {});
    assert.equal(
      read(root, "src/math.ts"),
      "export const double = (n: number): number => {\n  return n * 2;\n};\n",
    );
    assert.match(read(root, "src/App.tsx"), /<div className="greeting">/);
  });
});

describe("invalid add-on options", { concurrency: true }, () => {
  const files = {
    "tsconfig.json": JSON.stringify({ include: ["src"] }),
    // Also trips prefer-arrow-functions, to show other add-on rules still run.
    "src/a.ts": `${SNAKE}console.log(my_value);\nexport function legacy() {\n  return 1;\n}\n`,
  };
  /**
   * @param {Record<string, unknown>} rules
   */
  const lintWith = (rules) => {
    return oxlint(createProject({ preset: "base", config: { rules }, files }));
  };
  /**
   * @param {{ raw: { diagnostics: { code?: string, message: string }[] } }} run
   * @param {string} code oxlint code, e.g. `eslint-js(camelcase)`
   */
  const messagesOf = (run, code) => {
    return run.raw.diagnostics
      .filter((d) => {
        return d.code === code;
      })
      .map((d) => {
        return d.message;
      });
  };

  it("are reported by that rule only; the other rules keep running", async () => {
    const run = await lintWith({ [CAMEL]: ["error", { bogus: true }] });
    assert.deepEqual(fatalErrors(run.results), []);
    assert.deepEqual(messagesOf(run, "eslint-js(camelcase)"), [
      'Invalid configuration: unknown option "bogus"',
    ]);
    const ids = ruleIds(run.results, "src/a.ts");
    assert.ok(ids.includes("no-console"));
    assert.ok(ids.includes("prefer-arrow-functions/prefer-arrow-functions"));
  });

  it("report invalid max-lines limits on the rule", async () => {
    const run = await lintWith({
      "webadeva/max-lines": ["error", { error: "lots" }],
    });
    assert.deepEqual(fatalErrors(run.results), []);
    const [message = ""] = messagesOf(run, "webadeva(max-lines)");
    assert.match(message, /^Invalid configuration: .*max-lines config/);
    assert.match(message, /must be non-negative integers or `false`/);
    assert.ok(ruleIds(run.results, "src/a.ts").includes("no-console"));
  });

  it("max-lines ignores unknown keys, like the original's setting", async () => {
    const { results } = await lintWith({
      "webadeva/max-lines": ["error", { bogus: true }],
    });
    assert.deepEqual(fatalErrors(results), []);
    const ids = ruleIds(results, "src/a.ts");
    assert.ok(ids.includes(CAMEL));
    assert.ok(ids.includes("no-console"));
    // Short file, unknown key ignored: no max-lines report or config error.
    assert.ok(!ids.includes("webadeva/max-lines"));
  });
});
