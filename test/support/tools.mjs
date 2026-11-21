// Runs oxlint and oxfmt in a project, as users do, and collects the results.
import { execFile } from "node:child_process";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { promisify } from "node:util";
import { TYPED_RULE } from "./env.mjs";

const execFileAsync = promisify(execFile);

/**
 * @typedef {{ code: number, stdout: string, stderr: string, ms: number }} RunResult
 * @typedef {{ ruleIds: string[], fatal: string[] }} FileResult
 * @typedef {Record<string, FileResult>} Results
 * @typedef {{ filename: string, code?: string, severity: string, message: string }} OxlintDiagnostic
 * @typedef {{ diagnostics: OxlintDiagnostic[], number_of_files: number }} OxlintReport
 * @typedef {{ results: Results, files: number, ms: number, code: number, raw: OxlintReport }} LintRun
 */

/**
 * Runs a project's node_modules/.bin tool.
 * @param {string} root
 * @param {string} tool
 * @param {string[]} args
 * @returns {Promise<RunResult>}
 */
const run = async (root, tool, args) => {
  const started = performance.now();
  try {
    const { stdout, stderr } = await execFileAsync(
      join(root, "node_modules/.bin", tool),
      args,
      {
        cwd: root,
        maxBuffer: 256 * 1024 * 1024,
        env: { ...process.env, NO_COLOR: "1" },
      },
    );
    return { code: 0, stdout, stderr, ms: performance.now() - started };
  } catch (error) {
    const failure =
      /** @type {{ code?: unknown, stdout: string, stderr: string }} */ (error);
    if (typeof failure.code !== "number") throw error;
    return {
      code: failure.code,
      stdout: failure.stdout,
      stderr: failure.stderr,
      ms: performance.now() - started,
    };
  }
};

const PLUGIN_NAMES = new Map([
  ["typescript-eslint", "typescript"],
  ["next", "nextjs"],
  // oxlint shows react/rules-of-hooks and react/exhaustive-deps this way.
  ["react-hooks", "react"],
]);

/**
 * oxlint's code to the id used in config: `eslint(no-console)` ->
 * `no-console`, `typescript-eslint(x)` -> `typescript/x`,
 * `eslint-plugin-react(x)` -> `react/x`, `webadeva(x)` -> `webadeva/x`.
 * @param {string} code
 * @returns {string}
 */
export const normalizeCode = (code) => {
  const m = /^([^(]+)\((.+)\)$/.exec(code);
  if (!m) return code;
  if (m[1] === "eslint") return m[2];
  const plugin = m[1].replace(/^eslint-plugin-/, "");
  return `${PLUGIN_NAMES.get(plugin) ?? plugin}/${m[2]}`;
};

/**
 * Runs `oxlint -f json`; returns per-file `{ ruleIds, fatal }` (diagnostics
 * without a code, e.g. parse errors or "Error running JS plugin").
 * @param {string} root
 * @param {string[]} [args]
 * @returns {Promise<LintRun>}
 */
export const oxlint = async (root, args = []) => {
  const res = await run(root, "oxlint", ["-f", "json", ...args]);
  if (res.code > 1 || !res.stdout.startsWith("{")) {
    throw new Error(`oxlint exited ${res.code}\n${res.stderr}\n${res.stdout}`);
  }
  const report = /** @type {OxlintReport} */ (JSON.parse(res.stdout));
  /** @type {Results} */
  const results = {};
  for (const d of report.diagnostics) {
    const r = (results[d.filename] ??= { ruleIds: [], fatal: [] });
    if (d.code) r.ruleIds.push(normalizeCode(d.code));
    else r.fatal.push(d.message);
  }
  return {
    results,
    files: report.number_of_files,
    ms: res.ms,
    code: res.code,
    raw: report,
  };
};

/**
 * Every fatal message of a run, prefixed with its file.
 * @param {Results} results
 * @returns {string[]}
 */
export const fatalErrors = (results) => {
  return Object.entries(results).flatMap(([file, r]) => {
    return r.fatal.map((message) => {
      return `${file}: ${message}`;
    });
  });
};

/**
 * @param {string} root
 * @param {string[]} args
 * @returns {Promise<RunResult>}
 */
export const oxfmt = async (root, args) => {
  const res = await run(root, "oxfmt", args);
  if (res.code !== 0)
    throw new Error(`oxfmt exited ${res.code}\n${res.stderr}\n${res.stdout}`);
  return res;
};

/**
 * @param {string} root
 * @param {string} file
 * @returns {string}
 */
export const read = (root, file) => {
  return readFileSync(join(root, file), "utf8");
};

/**
 * @param {Results} results
 * @param {string} file
 * @returns {string[]}
 */
export const ruleIds = (results, file) => {
  return results[file]?.ruleIds ?? [];
};

/**
 * Whether type-aware rules ran on the file.
 * @param {Results} results
 * @param {string} file
 * @returns {boolean}
 */
export const isTyped = (results, file) => {
  return ruleIds(results, file).includes(TYPED_RULE);
};
