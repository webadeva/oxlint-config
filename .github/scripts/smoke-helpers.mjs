// Helpers for smoke-test.mjs: running commands and packing the packages the
// way the release publishes them.
import { execFileSync } from "node:child_process";
import { cpSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import {
  buildPlatformPackage,
  hostPlatform,
  pinOptionalDependencies,
  root,
} from "../../npm/scripts/build-packages.mjs";

const isWindows = process.platform === "win32";
// Never let a local build stand in for the installed platform package.
const env = { ...process.env };
delete env.WEBADEVA_OXLINT_NATIVE;

/**
 * Runs a command and returns its stdout. .cmd shims on Windows need a shell.
 * @param {string} cmd
 * @param {string[]} args
 * @param {string} cwd
 * @returns {string}
 */
export const run = (cmd, args, cwd) => {
  return execFileSync(cmd, args, {
    cwd,
    env,
    encoding: "utf8",
    shell: isWindows,
    stdio: ["ignore", "pipe", "pipe"],
  });
};

/**
 * Like run, but exit code 1 (problems reported) is a result, not a failure.
 * Returns stdout and stderr together.
 * @param {string} cmd
 * @param {string[]} args
 * @param {string} cwd
 * @returns {{ stdout: string, stderr: string }}
 */
export const lint = (cmd, args, cwd) => {
  try {
    return { stdout: run(cmd, args, cwd), stderr: "" };
  } catch (error) {
    const failure =
      /** @type {{ status: number | null, stdout: string, stderr: string }} */ (
        error
      );
    if (failure.status === 1)
      return { stdout: failure.stdout, stderr: failure.stderr };
    throw new Error(
      `${cmd} ${args.join(" ")} failed (${failure.status}):\n${failure.stdout}${failure.stderr}`,
      {
        cause: error,
      },
    );
  }
};

/** @type {import("../../npm/scripts/build-packages.mjs").RootPackage} */
export const rootPkg = JSON.parse(
  readFileSync(join(root, "package.json"), "utf8"),
);

/**
 * Packs the main package and this machine's platform package (from `addon`)
 * into `<work>/tarballs`, leaving the repo untouched.
 * @param {string} work
 * @param {string} addon
 * @returns {{ key: string, platformName: string, mainTgz: string, platformTgz: string }}
 */
export const packPackages = (work, addon) => {
  const key = hostPlatform();
  const platformName = `${rootPkg.name}-${key}`;
  const tarballs = join(work, "tarballs");
  mkdirSync(tarballs);
  /**
   * @param {string} dir
   * @returns {string} the tarball's path
   */
  const pack = (dir) => {
    const file = run(
      "npm",
      ["pack", "--silent", "--pack-destination", tarballs],
      dir,
    )
      .trim()
      .split("\n")
      .at(-1);
    if (!file) throw new Error(`npm pack printed nothing in ${dir}`);
    return join(tarballs, file);
  };

  buildPlatformPackage(rootPkg, key, addon, join(work, "platform"));
  const platformTgz = pack(join(work, "platform"));

  const stage = join(work, "main");
  mkdirSync(stage);
  for (const file of [...rootPkg.files, "LICENSE", "README.md"]) {
    cpSync(join(root, file), join(stage, file), { recursive: true });
  }
  const stagedPkg = pinOptionalDependencies({ ...rootPkg }, [key]);
  delete stagedPkg.scripts;
  delete stagedPkg.devDependencies;
  writeFileSync(
    join(stage, "package.json"),
    JSON.stringify(stagedPkg, null, 2),
  );
  return { key, platformName, mainTgz: pack(stage), platformTgz };
};
