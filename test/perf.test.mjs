// Smoke test: oxlint with the add-on finishes on a 500-file project. Timings are recorded
// as test diagnostics; there is deliberately no threshold.
import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { createProject, fatalErrors, oxlint, writeConfig } from "./helpers.mjs";

const FILES = 500;

const generate = () => {
  const files = {
    "tsconfig.json": JSON.stringify({
      compilerOptions: {
        strict: true,
        jsx: "react-jsx",
        noEmit: true,
        allowJs: true,
      },
      include: ["src"],
    }),
  };
  for (let i = 0; i < FILES; i++) {
    const dir = `src/feature${i % 25}`;
    if (i % 5 === 0) {
      files[`${dir}/Component${i}.tsx`] =
        `import { helper${i + 1} } from "../feature${(i + 1) % 25}/helper${i + 1}";\n\n` +
        `export const Component${i} = ({ label }: { label: string }) => {\n` +
        `  const items = [label, String(helper${i + 1}(2))];\n` +
        `  return (\n    <ul>\n      {items.map((item) => (\n        <li key={item}>{item}</li>\n      ))}\n    </ul>\n  );\n};\n`;
    } else if (i % 5 === 4) {
      files[`${dir}/Component${i}.stories.tsx`] =
        `export default { title: "Feature/Component${i}" };\n\nexport const Primary = { args: { label: "x" } };\n`;
    } else {
      const body = Array.from({ length: 40 }, (_, j) => {
        return `export const value${j} = async (n: number): Promise<number> => n * ${j};`;
      }).join("\n");
      files[`${dir}/helper${i}.ts`] =
        `export const helper${i} = (n: number): number => {\n  void Promise.resolve(n);\n  return n * ${i};\n};\n${body}\n`;
    }
  }
  return files;
};

describe("performance smoke test", () => {
  it(
    `oxlint with the storybook preset finishes on ${FILES} files`,
    { timeout: 600_000 },
    async (t) => {
      const root = createProject({
        preset: "storybook",
        typescript: true,
        reactTypes: true,
        files: generate(),
      });
      const full = await oxlint(root);
      assert.equal(full.files, FILES);
      assert.deepEqual(fatalErrors(full.results), []);
      t.diagnostic(
        `oxlint (type-aware, storybook preset with add-on rules): ${full.ms.toFixed(0)} ms for ${full.files} files`,
      );

      // Without tsgolint, what remains is oxlint plus the add-on.
      writeConfig(root, "storybook", { options: { typeAware: false } });
      const untyped = await oxlint(root);
      assert.equal(untyped.files, FILES);
      t.diagnostic(
        `oxlint (no type-aware, storybook preset with add-on rules): ${untyped.ms.toFixed(0)} ms`,
      );
    },
  );
});
