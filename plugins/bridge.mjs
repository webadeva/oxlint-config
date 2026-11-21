// Bridge between oxlint's JS plugin API and the Rust rules (a Node add-on).
//
// oxlint creates every enabled rule for a file before visiting it. Each rule
// registers itself here; the first one to finish (`Program:exit`) makes one
// native call for all of them (one parse per file, however many rules are
// on), and every rule then reports its own results. Severity, overrides,
// disable comments and `--fix` stay oxlint's job.
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);

const PLATFORMS = {
  "linux-x64": ["linux-x64-gnu", "linux-x64-musl"],
  "linux-arm64": ["linux-arm64-gnu", "linux-arm64-musl"],
  "darwin-x64": ["darwin-x64"],
  "darwin-arm64": ["darwin-arm64"],
  "win32-x64": ["win32-x64"],
  "win32-arm64": ["win32-arm64"],
};

/** @returns {boolean} */
const isMusl = () => {
  const report = /** @type {{ header?: { glibcVersionRuntime?: string } }} */ (
    process.report.getReport()
  );
  return !report.header?.glibcVersionRuntime;
};

/**
 * @typedef {{ name: string, options?: unknown[] }} RuleConfig
 * @typedef {{ start: number, end: number, text: string }} Edit
 * @typedef {{ rule: string, message: string, start: number, end: number, fix?: Edit[] }} Report
 * @typedef {{
 *   rules: () => string[],
 *   lint: (filename: string, source: string, rules: RuleConfig[], settings: unknown, cwd: string) => Report[],
 * }} Native
 * @typedef {{ line: number, column: number }} Position
 * @typedef {{ replaceTextRange: (range: [number, number], text: string) => unknown }} Fixer
 * @typedef {{
 *   filename: string,
 *   cwd: string,
 *   settings: unknown,
 *   options: unknown[],
 *   sourceCode: { text: string, getLocFromIndex: (index: number) => Position },
 *   report: (report: {
 *     message: string,
 *     loc: { start: Position, end: Position },
 *     fix?: (fixer: Fixer) => unknown[],
 *   }) => void,
 * }} Context
 */

/** @returns {Native} */
const loadNative = () => {
  const override = process.env.WEBADEVA_OXLINT_NATIVE;
  if (override) return /** @type {Native} */ (require(override));
  const errors = [];
  // A local build (development checkout of this package).
  try {
    return /** @type {Native} */ (require("../native/webadeva.node"));
  } catch (error) {
    errors.push(error);
  }
  const candidates = PLATFORMS[`${process.platform}-${process.arch}`] ?? [];
  const ordered =
    process.platform === "linux" && isMusl()
      ? [...candidates].reverse()
      : candidates;
  for (const platform of ordered) {
    try {
      return /** @type {Native} */ (
        require(`@webadeva/oxlint-config-${platform}/webadeva.node`)
      );
    } catch (error) {
      errors.push(error);
    }
  }
  throw new Error(
    `@webadeva/oxlint-config: no native rules for ${process.platform}-${process.arch}. ` +
      "Reinstall the package so the matching @webadeva/oxlint-config-<platform> " +
      "package is installed.",
    { cause: errors.at(-1) },
  );
};

const native = loadNative();

/**
 * Rule names oxlint reserves for its own plugins get another namespace:
 * ESLint core rules go to `eslint-js/`, eslint-plugin-react-hooks to
 * `react-hooks-js/`. Everything else keeps its ESLint id.
 * @param {string} name native name
 * @returns {[string, string]} [namespace, rule]
 */
const split = (name) => {
  if (!name.includes("/")) return ["eslint-js", name];
  const slash = name.lastIndexOf("/");
  const namespace = name.slice(0, slash);
  return [
    namespace === "react-hooks" ? "react-hooks-js" : namespace,
    name.slice(slash + 1),
  ];
};

/** @type {Map<string, { rules: RuleConfig[], results: Report[] | undefined, pending: number }>} */
const files = new Map();

/**
 * @param {Context} context
 * @returns {Report[]}
 */
const resultsFor = (context) => {
  const file = files.get(context.filename);
  if (!file) return [];
  file.results ??= native.lint(
    context.filename,
    context.sourceCode.text,
    file.rules,
    context.settings ?? null,
    context.cwd,
  );
  return file.results;
};

/**
 * The oxlint plugin for one namespace, e.g. `storybook`.
 * @param {string} namespace
 */
export const plugin = (namespace) => {
  /** @type {Record<string, object>} */
  const rules = {};
  for (const nativeName of native.rules()) {
    const [ns, rule] = split(nativeName);
    if (ns !== namespace) continue;
    rules[rule] = {
      meta: { type: "problem", fixable: "code", schema: false },
      /** @param {Context} context */
      create: (context) => {
        let file = files.get(context.filename);
        if (!file || file.results) {
          file = { rules: [], results: undefined, pending: 0 };
          files.set(context.filename, file);
        }
        file.rules.push({ name: nativeName, options: context.options });
        file.pending += 1;
        return {
          "Program:exit": () => {
            const { sourceCode } = context;
            for (const report of resultsFor(context)) {
              if (report.rule !== nativeName) continue;
              const edits = report.fix;
              context.report({
                message: report.message,
                loc: {
                  start: sourceCode.getLocFromIndex(report.start),
                  end: sourceCode.getLocFromIndex(report.end),
                },
                fix: edits
                  ? (/** @type {Fixer} */ fixer) => {
                      return edits.map((edit) => {
                        return fixer.replaceTextRange(
                          [edit.start, edit.end],
                          edit.text,
                        );
                      });
                    }
                  : undefined,
              });
            }
            const current = files.get(context.filename);
            if (current && --current.pending === 0)
              files.delete(context.filename);
          },
        };
      },
    };
  }
  return { meta: { name: namespace }, rules };
};
