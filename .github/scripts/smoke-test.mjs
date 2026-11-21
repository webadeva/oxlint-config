#!/usr/bin/env node
// Packs the package the way the release does (main package + this machine's
// platform package with the native add-on), installs both tarballs into a temp
// project with each package manager, plus oxlint and oxlint-tsgolint at the
// repo's versions, and runs plain `oxlint` with a .oxlintrc.json extending
// every preset. Checks that native rules and add-on rules both report, that
// no JS plugin fails to load, and that `oxlint --fix` applies an add-on fix.
// CI runs it on every OS and the oldest Node versions in `engines`.
//
//   node .github/scripts/smoke-test.mjs [--addon <path>] [--pm npm,pnpm]
//
// The add-on defaults to native/webadeva.node (`pnpm build:native`).
import {
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { parseArgs } from "node:util";
import { root } from "../../npm/scripts/build-packages.mjs";
import { lint, packPackages, rootPkg, run } from "./smoke-helpers.mjs";

const { values } = parseArgs({
  options: {
    addon: { type: "string", default: join(root, "native/webadeva.node") },
    pm: { type: "string", default: "npm,pnpm" },
  },
});

const FILES = {
  "tsconfig.json": JSON.stringify({
    compilerOptions: { strict: true, jsx: "react-jsx" },
    include: ["*.ts", "*.tsx"],
  }),
  // no-console is native; eslint-js/camelcase comes from the add-on.
  "sample.ts": 'console.log("smoke");\nconst my_var = 1;\nexport { my_var };\n',
  // @eslint-react/no-missing-key, from the add-on, in the React presets.
  "list.tsx":
    "export const List = () => {\n  return <ul>{[1, 2].map((n) => {\n    return <li>{n}</li>;\n  })}</ul>;\n};\n",
  // prefer-arrow-functions/prefer-arrow-functions fixes this into an arrow.
  "fix.ts": "export function one() {\n  return 1;\n}\n",
};

/**
 * @param {string} label
 * @param {boolean} ok
 * @param {unknown} detail
 */
const check = (label, ok, detail) => {
  if (!ok)
    throw new Error(
      `${label}\n${typeof detail === "string" ? detail : JSON.stringify(detail, null, 2)}`,
    );
};

const work = mkdtempSync(join(tmpdir(), "oxlint-config-smoke-"));
try {
  const { key, platformName, mainTgz, platformTgz } = packPackages(
    work,
    values.addon,
  );
  const presets = readdirSync(join(root, "presets")).filter((file) => {
    return file !== "oxfmt.json" && file.endsWith(".json");
  });
  const peers = ["oxlint", "oxlint-tsgolint"].map((name) => {
    return `${name}@${rootPkg.devDependencies[name]}`;
  });

  for (const pm of values.pm.split(",")) {
    try {
      run(pm, ["--version"], work);
    } catch {
      console.log(`skip ${pm}: not runnable on Node ${process.version}`);
      continue;
    }
    const app = join(work, `app-${pm}`);
    mkdirSync(app);
    // Point the optionalDependency at the local tarball, as the registry would.
    const spec = `file:${platformTgz.replaceAll("\\", "/")}`;
    writeFileSync(
      join(app, "package.json"),
      JSON.stringify({
        name: "app",
        private: true,
        overrides: { [platformName]: spec },
      }),
    );
    if (pm === "pnpm")
      writeFileSync(
        join(app, "pnpm-workspace.yaml"),
        `overrides:\n  "${platformName}": "${spec}"\n`,
      );
    run(pm, [pm === "npm" ? "install" : "add", mainTgz, ...peers], app);
    const oxlint = join(app, "node_modules", ".bin", "oxlint");

    for (const preset of presets) {
      const config =
        /** @type {{ ignorePatterns?: string[], jsPlugins?: { name: string }[] }} */ (
          JSON.parse(readFileSync(join(root, "presets", preset), "utf8"))
        );
      writeFileSync(
        join(app, ".oxlintrc.json"),
        JSON.stringify({
          extends: [`./node_modules/${rootPkg.name}/presets/${preset}`],
          ignorePatterns: config.ignorePatterns ?? [],
        }),
      );
      for (const [file, text] of Object.entries(FILES))
        writeFileSync(join(app, file), text);
      const label = `${pm} ${preset}`;
      const isReact = Boolean(
        config.jsPlugins?.some((plugin) => {
          return plugin.name === "@eslint-react";
        }),
      );

      const { stdout, stderr } = lint(
        oxlint,
        ["-f", "json", "sample.ts", "list.tsx"],
        app,
      );
      check(
        `${label}: oxlint printed no JSON report`,
        stdout.trimStart().startsWith("{"),
        `${stdout}${stderr}`,
      );
      const { diagnostics } =
        /** @type {{ diagnostics: { code?: string, message: string }[] }} */ (
          JSON.parse(stdout)
        );
      /** @param {string} code */
      const reported = (code) => {
        return diagnostics.some((diagnostic) => {
          return diagnostic.code?.includes(code);
        });
      };
      check(
        `${label}: a JS plugin failed`,
        !`${stdout}${stderr}`.includes("Error running JS plugin"),
        `${stdout}${stderr}`,
      );
      check(
        `${label}: no native no-console`,
        reported("no-console"),
        diagnostics,
      );
      check(
        `${label}: no add-on eslint-js(camelcase)`,
        reported("eslint-js(camelcase)"),
        diagnostics,
      );
      // Both files are in tsconfig.json's include. A path the add-on
      // mangles (Windows drive letters, once) reports every file instead.
      check(
        `${label}: webadeva(typescript-file-in-project) reported a file the tsconfig includes`,
        !reported("typescript-file-in-project"),
        diagnostics,
      );
      if (isReact)
        check(
          `${label}: no add-on @eslint-react(no-missing-key)`,
          reported("@eslint-react(no-missing-key)"),
          diagnostics,
        );

      lint(oxlint, ["--fix", "fix.ts"], app);
      const fixed = readFileSync(join(app, "fix.ts"), "utf8");
      check(
        `${label}: oxlint --fix did not apply prefer-arrow-functions`,
        fixed.includes("const one = () =>"),
        fixed,
      );
      console.log(
        `ok  ${label}: native + add-on rules report, --fix applies add-on fixes`,
      );
    }
  }
  console.log(`smoke test passed on Node ${process.version} (${key})`);
} finally {
  rmSync(work, { recursive: true, force: true });
}
