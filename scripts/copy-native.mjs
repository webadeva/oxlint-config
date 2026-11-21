#!/usr/bin/env node
// Copies the add-on cargo built into native/webadeva.node, where
// plugins/bridge.mjs looks first (a development checkout of the package).
import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const RELEASE = join(ROOT, "crates/webadeva-lint/target/release");
/** @type {Record<string, string>} */
const LIBRARY = {
  linux: "libwebadeva_lint.so",
  darwin: "libwebadeva_lint.dylib",
  win32: "webadeva_lint.dll",
};

const library = LIBRARY[process.platform];
if (!library) throw new Error(`unsupported platform ${process.platform}`);
mkdirSync(join(ROOT, "native"), { recursive: true });
copyFileSync(join(RELEASE, library), join(ROOT, "native/webadeva.node"));
console.log("native/webadeva.node updated");
