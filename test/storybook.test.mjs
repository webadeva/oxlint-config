import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  createProject,
  fatalErrors,
  oxlint,
  readPreset,
  ruleIds,
} from "./helpers.mjs";

// No default export (storybook/default-exports); a "|" hierarchy separator
// would need one, so keep it to rules that need no meta.
const NO_META = 'export const Primary = { args: { label: "hi" } };\n';
const MAIN =
  'const config = {\n  stories: ["../src/**/*.stories.tsx"],\n  addons: ["@storybook/addon-not-installed"],\n};\nconsole.log(config);\nexport default config;\n';
// A hook called from a story's render function.
const HOOK_STORY =
  'import { useState } from "react";\n\nexport default { title: "Counter" };\n\nexport const Primary = {\n  render: () => {\n    const [count] = useState(0);\n    return <p>{count}</p>;\n  },\n};\n';
const FILES = {
  "src/Button.stories.tsx": NO_META,
  "src/Card.story.ts": NO_META,
  "src/Button.tsx": NO_META,
  "src/stories.ts": NO_META,
  ".storybook/main.ts": MAIN,
  ".storybook/preview.ts": MAIN,
};
/**
 * @param {import("./support/tools.mjs").Results} results
 * @param {string} file
 * @returns {string[]}
 */
const storybookRules = (results, file) => {
  return ruleIds(results, file).filter((r) => {
    return r.startsWith("storybook/");
  });
};

/**
 * @param {Omit<import("./support/project.mjs").ProjectOptions, "preset">} options
 * @returns {Promise<import("./support/tools.mjs").Results>}
 */
const lint = async (options) => {
  const { results } = await oxlint(
    createProject({ preset: "storybook", ...options }),
  );
  assert.deepEqual(fatalErrors(results), []);
  return results;
};

describe("storybook preset", { concurrency: true }, () => {
  it("oxlint lints .storybook/main", async () => {
    const results = await lint({ files: FILES });
    assert.ok(ruleIds(results, ".storybook/main.ts").includes("no-console"));
    assert.ok(ruleIds(results, ".storybook/preview.ts").includes("no-console"));
  });

  it("stories rules fire only in *.stories.* / *.story.* files", async () => {
    const results = await lint({ files: FILES });
    assert.deepEqual(storybookRules(results, "src/Button.stories.tsx"), [
      "storybook/default-exports",
    ]);
    assert.deepEqual(storybookRules(results, "src/Card.story.ts"), [
      "storybook/default-exports",
    ]);
    assert.deepEqual(storybookRules(results, "src/Button.tsx"), []);
    assert.deepEqual(storybookRules(results, "src/stories.ts"), []);
  });

  it("lints .storybook/main with storybook/no-uninstalled-addons", async () => {
    const results = await lint({ files: FILES });
    assert.deepEqual(storybookRules(results, ".storybook/main.ts"), [
      "storybook/no-uninstalled-addons",
    ]);
    assert.deepEqual(storybookRules(results, ".storybook/preview.ts"), []);
  });

  it("accepts addons listed in package.json", async () => {
    const results = await lint({
      packageJson: {
        type: "module",
        devDependencies: { "@storybook/addon-not-installed": "^1.0.0" },
      },
      files: { ".storybook/main.ts": MAIN },
    });
    assert.deepEqual(storybookRules(results, ".storybook/main.ts"), []);
  });

  it("turns react/rules-of-hooks off in stories", async () => {
    const preset = readPreset("storybook");
    assert.equal(preset.rules["react/rules-of-hooks"], "error");
    const results = await lint({
      files: {
        "src/Counter.stories.tsx": HOOK_STORY,
        "src/counter.tsx": HOOK_STORY,
      },
    });
    const hooks = "react/rules-of-hooks";
    assert.ok(!ruleIds(results, "src/Counter.stories.tsx").includes(hooks));
    assert.ok(ruleIds(results, "src/counter.tsx").includes(hooks));
  });
});
