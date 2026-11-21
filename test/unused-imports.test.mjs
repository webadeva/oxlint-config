// The original removed unused imports and sorted them through
// prettier-plugin-organize-imports (run by `prettier/prettier --fix`). Here
// oxlint's no-unused-vars removes them on plain --fix (the presets set
// `fix.imports: "safe-fix"`) and oxfmt's sortImports (presets/oxfmt.json,
// which users copy to .oxfmtrc.json) orders them.
import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { createProject, oxfmt, oxlint, read } from "./helpers.mjs";

const UNUSED =
  'import { readFileSync, writeFileSync } from "node:fs";\nimport { join } from "node:path";\n\nexport const read = (path: string): string => {\n  return readFileSync(path, "utf8");\n};\n';

const UNSORTED =
  'import { z } from "./local";\nimport zod from "zod";\nimport path from "node:path";\nimport { y } from "../parent";\nimport React from "react";\nimport { Alpha } from "Alpha";\n\nexport const v = [z, zod, path, y, React, Alpha];\n';
// What organize-imports (TypeScript's organizeImports) produces: bare
// specifiers before relative ones, each sorted case-insensitively, one block.
const SORTED =
  'import { Alpha } from "Alpha";\nimport path from "node:path";\nimport React from "react";\nimport zod from "zod";\nimport { y } from "../parent";\nimport { z } from "./local";\n\nexport const v = [z, zod, path, y, React, Alpha];\n';

describe("unused imports and import order", { concurrency: true }, () => {
  it("oxlint reports unused imports", async () => {
    const root = createProject({
      preset: "base",
      files: { "src/read.ts": UNUSED },
    });
    const { raw } = await oxlint(root);
    const messages = raw.diagnostics
      .map((d) => {
        return d.message;
      })
      .join("\n");
    assert.match(messages, /'writeFileSync' is imported but never used/);
    assert.match(messages, /'join' is imported but never used/);
  });

  it("are removed by plain `oxlint --fix`", async () => {
    const root = createProject({
      preset: "base",
      files: { "src/read.ts": UNUSED },
    });
    await oxlint(root, ["--fix"]);
    const output = read(root, "src/read.ts");
    assert.match(output, /^import \{ readFileSync \} from "node:fs";\n/);
    assert.doesNotMatch(output, /writeFileSync|node:path/);
  });

  it("oxfmt orders imports like prettier-plugin-organize-imports", async () => {
    const root = createProject({
      preset: "base",
      files: { "src/order.ts": UNSORTED },
    });
    // Picks up the .oxfmtrc.json copied from presets/oxfmt.json.
    await oxfmt(root, ["src/order.ts"]);
    assert.equal(read(root, "src/order.ts"), SORTED);
  });

  it(
    "named imports are sorted like prettier-plugin-organize-imports",
    // oxfmt's sortImports orders declarations only; the presets' sort-imports
    // rule (ignoreDeclarationSort) sorts specifiers on --fix.
    async () => {
      const root = createProject({
        preset: "base",
        files: {
          "src/named.ts":
            'import { b, A, c } from "zod";\n\nexport const v = [b, A, c];\n',
        },
      });
      await oxlint(root, ["--fix"]);
      await oxfmt(root, ["src/named.ts"]);
      assert.match(
        read(root, "src/named.ts"),
        /^import \{ A, b, c \} from "zod";/,
      );
    },
  );

  it("`oxlint --fix` + `oxfmt` match the original's `eslint --fix` output", async () => {
    const root = createProject({
      preset: "base",
      files: {
        "src/read.ts":
          'import { join } from "node:path";\nimport { readFileSync, writeFileSync } from "node:fs";\nimport { helper } from "./helper";\n\nexport const read = (path: string): string => {\n  return readFileSync(path, "utf8");\n};\n',
      },
    });
    await oxlint(root, ["--fix"]);
    await oxfmt(root, []);
    assert.equal(
      read(root, "src/read.ts"),
      'import { readFileSync } from "node:fs";\n\nexport const read = (path: string): string => {\n  return readFileSync(path, "utf8");\n};\n',
    );
  });
});
