#!/usr/bin/env node
// Builds the per-platform npm packages that carry the Rust rules (the Node
// add-on plugins/bridge.mjs loads), and pins them in the main package's
// optionalDependencies.
//
//   node npm/scripts/build-packages.mjs [--artifacts <dir>] [--out <dir>] [--only <platform,...>]
//
// Input:  <artifacts>/<platform>/webadeva.node (default npm/artifacts): the
//         cdylib cargo built for that target (libwebadeva_lint.so/.dylib or
//         webadeva_lint.dll), renamed.
// Output: <out>/<platform>/{package.json,webadeva.node} (default npm/dist), and
//         package.json#optionalDependencies at the main package's version.
//         Missing add-ons fail the build unless --only limits it to the
//         platforms you have.
import {
  copyFileSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

/**
 * @typedef {{ target: string, os: string, cpu: string, libc?: string }} Platform
 * @typedef {{ name: string, version: string, files: string[], devDependencies: Record<string, string>, optionalDependencies?: Record<string, string>, [field: string]: unknown }} RootPackage
 */

export const ADDON = "webadeva.node";

// Keys match the package suffixes plugins/bridge.mjs requires.
/** @type {Record<string, Platform>} */
export const PLATFORMS = {
  "linux-x64-gnu": {
    target: "x86_64-unknown-linux-gnu",
    os: "linux",
    cpu: "x64",
    libc: "glibc",
  },
  "linux-arm64-gnu": {
    target: "aarch64-unknown-linux-gnu",
    os: "linux",
    cpu: "arm64",
    libc: "glibc",
  },
  "linux-x64-musl": {
    target: "x86_64-unknown-linux-musl",
    os: "linux",
    cpu: "x64",
    libc: "musl",
  },
  "linux-arm64-musl": {
    target: "aarch64-unknown-linux-musl",
    os: "linux",
    cpu: "arm64",
    libc: "musl",
  },
  "darwin-x64": { target: "x86_64-apple-darwin", os: "darwin", cpu: "x64" },
  "darwin-arm64": {
    target: "aarch64-apple-darwin",
    os: "darwin",
    cpu: "arm64",
  },
  "win32-x64": { target: "x86_64-pc-windows-msvc", os: "win32", cpu: "x64" },
  "win32-arm64": {
    target: "aarch64-pc-windows-msvc",
    os: "win32",
    cpu: "arm64",
  },
};

export const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");

/**
 * Writes the platform package for `key` into `dir`, with `addon` as its
 * webadeva.node, and returns its package.json.
 * @param {RootPackage} rootPkg
 * @param {string} key
 * @param {string} addon
 * @param {string} dir
 */
export const buildPlatformPackage = (rootPkg, key, addon, dir) => {
  const { os, cpu, libc } = PLATFORMS[key];
  mkdirSync(dir, { recursive: true });
  copyFileSync(addon, join(dir, ADDON));
  const pkg = {
    name: `${rootPkg.name}-${key}`,
    version: rootPkg.version,
    description: `The ${key} native rules of ${rootPkg.name} (a Node add-on).`,
    homepage: rootPkg.homepage,
    bugs: rootPkg.bugs,
    repository: rootPkg.repository,
    license: rootPkg.license,
    author: rootPkg.author,
    main: ADDON,
    files: [ADDON],
    os: [os],
    cpu: [cpu],
    ...(libc && { libc: [libc] }),
    publishConfig: { access: "public" },
  };
  writeFileSync(join(dir, "package.json"), `${JSON.stringify(pkg, null, 2)}\n`);
  return pkg;
};

/**
 * Pins every platform package in `keys` at the main package's version.
 * @param {RootPackage} rootPkg
 * @param {string[]} keys
 * @returns {RootPackage}
 */
export const pinOptionalDependencies = (rootPkg, keys) => {
  rootPkg.optionalDependencies = Object.fromEntries(
    keys.map((key) => {
      return [`${rootPkg.name}-${key}`, rootPkg.version];
    }),
  );
  return rootPkg;
};

// The platform key of the running machine, as plugins/bridge.mjs picks it.
export const hostPlatform = () => {
  const { arch, platform } = process;
  if (platform !== "linux") return `${platform}-${arch}`;
  const report = /** @type {{ header?: { glibcVersionRuntime?: string } }} */ (
    process.report.getReport()
  );
  return `${platform}-${arch}-${report.header?.glibcVersionRuntime ? "gnu" : "musl"}`;
};

const main = () => {
  const { values } = parseArgs({
    options: {
      artifacts: { type: "string", default: join(root, "npm/artifacts") },
      out: { type: "string", default: join(root, "npm/dist") },
      only: { type: "string" },
    },
  });
  const keys = values.only ? values.only.split(",") : Object.keys(PLATFORMS);
  for (const key of keys) {
    if (!PLATFORMS[key]) throw new Error(`unknown platform ${key}`);
  }

  const pkgPath = join(root, "package.json");
  const rootPkg = /** @type {RootPackage} */ (
    JSON.parse(readFileSync(pkgPath, "utf8"))
  );
  const out = resolve(values.out);
  rmSync(out, { recursive: true, force: true });

  for (const key of keys) {
    const addon = join(resolve(values.artifacts), key, ADDON);
    const pkg = buildPlatformPackage(rootPkg, key, addon, join(out, key));
    console.log(`built ${pkg.name}@${pkg.version}`);
  }

  pinOptionalDependencies(rootPkg, keys);
  writeFileSync(pkgPath, `${JSON.stringify(rootPkg, null, 2)}\n`);
  console.log(
    `pinned ${keys.length} optionalDependencies at ${rootPkg.version}`,
  );
};

if (process.argv[1] === fileURLToPath(import.meta.url)) main();
