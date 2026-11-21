// Ports the React project setups. The original typed JSX only with React's
// types installed and only inside the default project; tsgolint types every
// file, so `typed` is true throughout (see typescript-projects.test.mjs).
import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  COMPONENT,
  createProject,
  fatalErrors,
  isTyped,
  oxlint,
  REACT_RULE,
  ruleIds,
} from "./helpers.mjs";

/**
 * @param {Omit<import("./support/project.mjs").ProjectOptions, "preset">} options
 * @returns {Promise<import("./support/tools.mjs").Results>}
 */
const lint = async (options) => {
  const { results } = await oxlint(
    createProject({ preset: "react", ...options }),
  );
  assert.deepEqual(fatalErrors(results), []);
  return results;
};

/**
 * @param {import("./support/tools.mjs").Results} results
 * @param {string} file
 * @returns {string[]}
 */
const unsafe = (results, file) => {
  return ruleIds(results, file).filter((r) => {
    return r.includes("unsafe");
  });
};

describe("React project setups", { concurrency: true }, () => {
  it("lints React projects without TypeScript installed", async () => {
    // The original refused to load @eslint-react without TypeScript.
    const results = await lint({ files: { "src/App.jsx": COMPONENT } });
    assert.ok(ruleIds(results, "src/App.jsx").includes(REACT_RULE));
    assert.ok(ruleIds(results, "src/App.jsx").includes("no-console"));
  });

  it("React project with TypeScript but no tsconfig", async () => {
    const results = await lint({
      typescript: true,
      files: { "App.jsx": COMPONENT, "src/Page.jsx": COMPONENT },
    });
    assert.ok(ruleIds(results, "src/Page.jsx").includes(REACT_RULE));
    // Original: both false (JSX without React's types stayed untyped).
    assert.equal(isTyped(results, "App.jsx"), true);
    assert.equal(isTyped(results, "src/Page.jsx"), true);
  });

  it(
    "doesn't report typescript/no-unsafe-* on JSX without React's types",
    {
      todo:
        "tsgolint types JSX as an error type without @types/react, so every " +
        "component returning JSX gets typescript/no-unsafe-return (the original " +
        "kept such files out of the type-aware rules)",
    },
    async () => {
      const results = await lint({
        typescript: true,
        files: { "App.jsx": COMPONENT, "src/Page.tsx": COMPONENT },
      });
      assert.deepEqual(unsafe(results, "App.jsx"), []);
      assert.deepEqual(unsafe(results, "src/Page.tsx"), []);
    },
  );

  it("React project with TypeScript and React types but no tsconfig", async () => {
    const results = await lint({
      typescript: true,
      reactTypes: true,
      files: { "App.jsx": COMPONENT, "src/Page.jsx": COMPONENT },
    });
    assert.equal(isTyped(results, "App.jsx"), true);
    assert.deepEqual(unsafe(results, "App.jsx"), []);
    // Original: false (outside the default project).
    assert.equal(isTyped(results, "src/Page.jsx"), true);
    assert.deepEqual(unsafe(results, "src/Page.jsx"), []);
  });

  it("React project with a tsconfig and React types", async () => {
    const results = await lint({
      typescript: true,
      reactTypes: true,
      files: {
        "tsconfig.json": JSON.stringify({
          compilerOptions: { jsx: "react-jsx", strict: true, noEmit: true },
          include: ["src"],
        }),
        "src/App.tsx": COMPONENT.replace(
          'class="greeting"',
          'className="greeting"',
        ),
      },
    });
    assert.equal(isTyped(results, "src/App.tsx"), true);
    assert.deepEqual(unsafe(results, "src/App.tsx"), []);
    assert.ok(!ruleIds(results, "src/App.tsx").includes(REACT_RULE));
  });
});
