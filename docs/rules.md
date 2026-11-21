# Rules

Oxlint runs the rules it implements natively. This package's add-on runs the rest: the rules Oxlint lacks or implements differently from ESLint, and this package's own `webadeva/*` rules. This page covers the add-on.

## Configuring add-on rules

Add-on rules are Oxlint rules. They go in `.oxlintrc.json` under `rules` and `overrides`, take the same severities (`"off"`, `"warn"`, `"error"`, or `0`, `1`, `2`) and `[severity, options]` arrays, and follow disable comments (`eslint-disable…` and `oxlint-disable…`) and `--fix` like native rules:

```json
{
  "extends": ["./node_modules/@webadeva/oxlint-config/presets/react.json"],
  "rules": {
    "eslint-js/camelcase": "off",
    "@eslint-react/no-array-index-key": "error"
  },
  "overrides": [
    {
      "files": ["**/*.test.ts"],
      "rules": { "prefer-arrow-functions/prefer-arrow-functions": "off" }
    }
  ]
}
```

Their ids are the ESLint ids, except where Oxlint reserves the plugin name: ESLint's core rules are `eslint-js/<rule>` and `eslint-plugin-react-hooks`' are `react-hooks-js/<rule>`. An invalid option is reported by that rule as `Invalid configuration: …`, on every file it would lint; the other rules keep working.

## `webadeva/max-lines` and `webadeva/max-lines-warning`

These rules limit how long a file can get, measured in lines of code. Each report covers the whole file.

| Lines of code | Result  |
| ------------- | ------- |
| up to 150     | nothing |
| 151–175       | warning |
| over 175      | error   |

A rule has one severity, so the limit is two rules: `max-lines-warning` set to `warn` and `max-lines` set to `error`. Both read one shared setting, so you configure them in one place. Put it in your own `.oxlintrc.json`, because Oxlint's `extends` drops `settings`:

```json
{ "settings": { "webadeva": { "maxLines": { "warn": 250, "error": 300 } } } }
```

Any key you leave out keeps its default:

| Key              | Default | Meaning                                                                                              |
| ---------------- | ------- | ---------------------------------------------------------------------------------------------------- |
| `enabled`        | `true`  | `false` turns both rules off                                                                         |
| `warn`           | `150`   | Warn while `warn < LOC <= error`. `false` turns only the warning off.                                |
| `error`          | `175`   | Error when `LOC > error`. `false` turns only the error off, and the warning then has no upper bound. |
| `skipBlankLines` | `true`  | Don't count blank lines                                                                              |
| `skipComments`   | `true`  | Don't count comment-only lines                                                                       |

`maxLines: false` is shorthand for `{ enabled: false }`. `warn` must be lower than `error`. An invalid setting is reported by both rules as `Invalid configuration: …`.

Oxlint's `overrides` can't change `settings`, so the rules also take the same keys (or `false`) as an option, layered over the setting key by key. That gives limits per path. The two rules are one setting, so an option on either applies to both (if both have one, the warning's applies first, then the error's):

```json
{
  "overrides": [
    {
      "files": ["**/*.test.ts"],
      "rules": {
        "webadeva/max-lines": ["error", { "warn": 400, "error": 500 }]
      }
    }
  ]
}
```

## `webadeva/typescript-file-in-project`

This rule reports a TypeScript file (`.ts`, `.tsx`, `.mts`, `.cts`) that no tsconfig includes. `tsc` doesn't type-check such a file, and type-aware rules run on it with default compiler options instead of your project's. The rule looks for the nearest `tsconfig.json` above the file and follows its `include`, `exclude`, `files`, and project references. As in the ESLint version, up to 8 root-level files, such as config files and scripts, are exempt.

To fix the report, add the file to a tsconfig's `include`, or turn the rule off with `"webadeva/typescript-file-in-project": "off"`.

## `eslint-comments/require-description`

This rule reports a directive comment without a description, such as `// eslint-disable-next-line no-console` with no `-- reason` after it. It is a port of the rule from [`@eslint-community/eslint-plugin-eslint-comments`](https://eslint-community.github.io/eslint-plugin-eslint-comments/rules/require-description.html), with the same `ignore` and `additionalDirectives` options. It also checks `oxlint-disable`, `oxlint-disable-line`, `oxlint-disable-next-line`, and `oxlint-enable` comments, and `ignore: ["eslint-disable"]` covers `oxlint-disable` too. Write the reason after `--`:

```ts
// oxlint-disable-next-line no-console -- the CLI prints its result
console.log(result);
```

## All add-on rules

Every rule keeps the ESLint version's severity and options. The options are in [`scripts/preset-rules.mjs`](../scripts/preset-rules.mjs) and the generated `presets/*.json`.

### Every preset

| Rule                                            | Severity | Notes                                                                                                                    |
| ----------------------------------------------- | -------- | ------------------------------------------------------------------------------------------------------------------------ |
| `eslint-js/camelcase`                           | `error`  | Not in Oxlint                                                                                                            |
| `eslint-js/getter-return`                       | `error`  | Oxlint's native version differs from ESLint's                                                                            |
| `eslint-js/no-dupe-args`                        | `error`  | Not in Oxlint                                                                                                            |
| `eslint-js/no-octal`                            | `error`  | Not in Oxlint                                                                                                            |
| `eslint-js/no-redeclare`                        | `error`  | Oxlint's native version reports the first declaration instead of the later ones, and skips TypeScript declaration merges |
| `prefer-arrow-functions/prefer-arrow-functions` | `warn`   |                                                                                                                          |
| `eslint-comments/require-description`           | `error`  | [Above](#eslint-commentsrequire-description)                                                                             |
| `webadeva/max-lines`                            | `error`  | [Above](#webadevamax-lines-and-webadevamax-lines-warning)                                                                |
| `webadeva/max-lines-warning`                    | `warn`   | [Above](#webadevamax-lines-and-webadevamax-lines-warning)                                                                |
| `webadeva/typescript-file-in-project`           | `error`  | [Above](#webadevatypescript-file-in-project)                                                                             |

### React, Next.js, and Storybook

The [`@eslint-react`](https://eslint-react.xyz/) rules, all prefixed `@eslint-react/`:

| Group      | `error`                                                                                                                                                                                                                                                                                       | `warn`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Components | `no-access-state-in-setstate`, `no-component-will-mount`, `no-component-will-receive-props`, `no-component-will-update`, `no-create-ref`, `no-direct-mutation-state`, `no-missing-key`, `no-nested-component-definitions`, `no-nested-lazy-component-declarations`, `rsc-function-definition` | `no-array-index-key`, `no-children-count`, `no-children-for-each`, `no-children-map`, `no-children-only`, `no-children-to-array`, `no-clone-element`, `no-context-provider`, `no-forward-ref`, `no-set-state-in-component-did-mount`, `no-set-state-in-component-did-update`, `no-set-state-in-component-will-update`, `no-unnecessary-use-prefix`, `no-unsafe-component-will-mount`, `no-unsafe-component-will-receive-props`, `no-unsafe-component-will-update`, `no-unused-class-component-members`, `no-use-context`, `use-state` |
| Compiler   | `error-boundaries`, `set-state-in-render`, `static-components`, `unsupported-syntax`, `use-memo`                                                                                                                                                                                              | `purity`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| JSX        | `jsx-no-children-prop-with-children`, `jsx-no-key-after-spread`, `jsx-no-namespace`                                                                                                                                                                                                           | `jsx-no-children-prop`, `jsx-no-leaked-dollar`, `jsx-no-leaked-semicolon`                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| DOM        | `dom-no-dangerously-set-innerhtml-with-children`, `dom-no-find-dom-node`, `dom-no-flush-sync`, `dom-no-hydrate`, `dom-no-render`, `dom-no-render-return-value`, `dom-no-unknown-property`, `dom-no-use-form-state`, `dom-no-void-elements-with-children`                                      | `dom-no-dangerously-set-innerhtml`, `dom-no-script-url`, `dom-no-unsafe-iframe-sandbox`                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| Web API    |                                                                                                                                                                                                                                                                                               | `web-api-no-leaked-event-listener`, `web-api-no-leaked-fetch`, `web-api-no-leaked-intersection-observer`, `web-api-no-leaked-interval`, `web-api-no-leaked-resize-observer`, `web-api-no-leaked-timeout`                                                                                                                                                                                                                                                                                                                              |
| Naming     |                                                                                                                                                                                                                                                                                               | `naming-convention-context-name`, `naming-convention-id-name`, `naming-convention-ref-name`                                                                                                                                                                                                                                                                                                                                                                                                                                           |

The compiler rules also run natively in Oxlint as `react/<rule>`. The ESLint version had both `react-hooks/<rule>` and `@eslint-react/<rule>` on, so this keeps the same double coverage.

`react-hooks-js/config` and `react-hooks-js/gating` (`error`) are on too. They are ESLint's `react-hooks/config` and `react-hooks/gating`. As in ESLint, they only report problems with React Compiler options, and the preset sets none, so they never fire.

The Next.js preset adds no add-on rules.

### Off in every preset

`@eslint-react/component-hook-factories` reports components and hooks defined inside another function. A factory like that gives them a new identity on every call, so React remounts them and loses their state. It's a port of the rule from @eslint-react 3, which v5 dropped, so the presets (like the ESLint version on v5) leave it off. Configs that came from an older @eslint-react can turn it on in the React presets:

```json
{
  "rules": { "@eslint-react/component-hook-factories": "error" }
}
```

### Storybook

On story files (`*.stories.*` and `*.story.*`):

| Rule                                      | Severity |
| ----------------------------------------- | -------- |
| `storybook/await-interactions`            | `error`  |
| `storybook/context-in-play-function`      | `error`  |
| `storybook/default-exports`               | `error`  |
| `storybook/hierarchy-separator`           | `warn`   |
| `storybook/no-redundant-story-name`       | `warn`   |
| `storybook/no-renderer-packages`          | `error`  |
| `storybook/prefer-pascal-case`            | `warn`   |
| `storybook/story-exports`                 | `error`  |
| `storybook/use-storybook-expect`          | `error`  |
| `storybook/use-storybook-testing-library` | `error`  |

On `.storybook/main.*`: `storybook/no-uninstalled-addons` (`error`).

As in the ESLint version, the Storybook preset also turns `react/rules-of-hooks` off in story files.
