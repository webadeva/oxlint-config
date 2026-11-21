// Maps the rule tables' rules implemented in Rust (original ESLint ids) to
// the add-on's rule ids and plugin entry files.

/**
 * The add-on's rule id for a rule implemented in Rust (the `rust` tables
 * use the original ESLint ids). Names oxlint reserves for its own plugins
 * get another namespace, as in plugins/bridge.mjs: ESLint core rules go to
 * `eslint-js/`, eslint-plugin-react-hooks to `react-hooks-js/`.
 * @param {string} id
 * @returns {[string, string]} [namespace, rule id]
 */
export const addonId = (id) => {
  if (!id.includes("/")) return ["eslint-js", `eslint-js/${id}`];
  const namespace = id.slice(0, id.lastIndexOf("/"));
  if (namespace === "react-hooks") {
    return ["react-hooks-js", id.replace(/^react-hooks\//, "react-hooks-js/")];
  }
  return [namespace, id];
};

/** Plugin entry file (in plugins/) for each add-on namespace. */
export const PLUGIN_FILES = {
  webadeva: "webadeva",
  "eslint-js": "eslint-js",
  "eslint-comments": "eslint-comments",
  "prefer-arrow-functions": "prefer-arrow-functions",
  "@eslint-react": "eslint-react",
  storybook: "storybook",
  "react-hooks-js": "react-hooks-js",
};

/**
 * @param {Record<string, unknown>} rules rust-table rules
 * @param {Set<string>} namespaces collects the namespaces used
 */
export const addonRules = (rules, namespaces) => {
  return Object.fromEntries(
    Object.entries(rules).map(([id, value]) => {
      const [namespace, addon] = addonId(id);
      namespaces.add(namespace);
      return [addon, value];
    }),
  );
};
