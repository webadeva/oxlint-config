# @webadeva/oxlint-config

[![npm version](https://img.shields.io/npm/v/@webadeva/oxlint-config)](https://www.npmjs.com/package/@webadeva/oxlint-config)
[![Release](https://github.com/webadeva/oxlint-config/actions/workflows/release.yml/badge.svg)](https://github.com/webadeva/oxlint-config/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/npm/l/@webadeva/oxlint-config)](LICENSE)

The rules of [`@webadeva/eslint-config`](https://github.com/webadeva/eslint-config), running on [Oxlint](https://oxc.rs/docs/guide/usage/linter.html) instead of ESLint. Same problems reported, same fixes applied, a lot less waiting.

```
$ pnpm lint        # a Next.js starter, 33 files

  before   ESLint + Prettier               19–25 s   1.4 GB
  after    oxlint + this config + oxfmt      8.3 s   0.6 GB
           (1.2 s without the Tailwind ESLint plugin, see below)
```

It's an add-on for Oxlint, not a replacement for it. You still run plain `oxlint` and `oxlint --fix`, and every rule is configured in `.oxlintrc.json` like any other Oxlint rule. The package brings two things:

1. Presets (`base`, `react`, `next`, `storybook`) that turn on the Oxlint rules the ESLint config used, with the same options.
2. The 86 rules Oxlint doesn't have, or has with different behavior: `@eslint-react/*`, `storybook/*`, `prefer-arrow-functions`, a few ESLint core rules, and this package's own `webadeva/*` rules. They're written in Rust and load into Oxlint as a JS plugin, so nothing from the ESLint config was dropped.

Formatting moves to [oxfmt](https://oxc.rs/docs/guide/usage/formatter.html), which takes over from Prettier and `prettier-plugin-organize-imports`.

## Getting started

You'll need Node.js `^20.19.0 || >=22.12.0`. Install the config and the three Oxc tools:

```bash
pnpm add -D @webadeva/oxlint-config oxlint oxlint-tsgolint oxfmt
```

For the React, Next.js, and Storybook presets, also install `@types/react` if you haven't already. TypeScript itself isn't needed: the type-aware rules run on `oxlint-tsgolint`, which ships its own TypeScript 7, so whatever version your project uses is fine.

Then create `.oxlintrc.json`. Pick a preset and copy its `ignorePatterns` next to it:

```json
{
  "extends": ["./node_modules/@webadeva/oxlint-config/presets/react.json"],
  "ignorePatterns": [
    "**/__tests__/**",
    "**/__mocks__/**",
    "dist",
    "*.config.*",
    "node_modules/**/*.*",
    "dist/**/*.*",
    "!.storybook",
    "!**/.storybook/**"
  ]
}
```

The copy is there because Oxlint's `extends` leaves out a config's `ignorePatterns`, `env`, and `settings`. The presets already work around this for `env`; for the ignores, you keep your own list. If a new version of the preset changes them, update yours.

oxfmt has no `extends`, so copy [`presets/oxfmt.json`](presets/oxfmt.json) to `.oxfmtrc.json`. Last, the scripts:

```json
{
  "scripts": {
    "lint": "oxlint",
    "lint:fix": "oxlint --fix",
    "format": "oxfmt"
  }
}
```

In CI, run `oxlint` and `oxfmt --check`. Type-aware linting is on in every preset, so you don't need `--type-aware`.

### Presets

| Preset    | Extend                                                          | What's in it            |
| --------- | --------------------------------------------------------------- | ----------------------- |
| Base      | `./node_modules/@webadeva/oxlint-config/presets/base.json`      | TypeScript              |
| React     | `./node_modules/@webadeva/oxlint-config/presets/react.json`     | Base + React + JSX a11y |
| Next.js   | `./node_modules/@webadeva/oxlint-config/presets/next.json`      | React + Next.js         |
| Storybook | `./node_modules/@webadeva/oxlint-config/presets/storybook.json` | React + Storybook       |

Each preset is a complete config. It switches off Oxlint's default rule categories and turns on only the rules picked here, so you won't get a second, different set of opinions on top. It also loads the add-on, so there's nothing else to set up.

The add-on comes prebuilt for Linux (x64 and arm64, glibc and musl), macOS (Intel and Apple silicon), and Windows (x64 and arm64). Your package manager downloads only the one for your machine.

## Fixing

`oxlint --fix` applies fixes from Oxlint's rules and the add-on's rules alike. Among other things it removes unused imports and sorts the names inside each import. `oxfmt` then formats the file and orders the import statements. Run both and you get what `eslint --fix` used to give you.

One difference: Oxlint makes one round of fixes per run, where ESLint kept going for up to 10. When two fixes touch the same code, one has to wait, so once in a while a second `oxlint --fix` finds a little more to do. In practice this mostly shows up with ESLint plugins that stack several fixes on one string, like the Tailwind plugin below.

## Customizing

Add-on rules are ordinary Oxlint rules. Turn them off, change their severity, or set them per path with `overrides`:

```json
{
  "extends": ["./node_modules/@webadeva/oxlint-config/presets/react.json"],
  "ignorePatterns": ["…copied from the preset…"],
  "rules": {
    "no-console": "off",
    "eslint-js/camelcase": "off",
    "@eslint-react/no-array-index-key": "error"
  },
  "overrides": [
    {
      "files": ["**/*.test.ts"],
      "rules": { "prefer-arrow-functions/prefer-arrow-functions": "off" }
    }
  ],
  "settings": {
    "webadeva": { "maxLines": { "warn": 250, "error": 300 } }
  }
}
```

`maxLines` is the file length limit. Out of the box a file warns past 150 lines of code and errors past 175. Like the ignores, the setting goes in your own `.oxlintrc.json`. To use different limits for some paths, pass the same keys as options to `webadeva/max-lines` inside an `overrides` entry. [docs/rules.md](docs/rules.md) has every option.

Disable comments work for every rule, with either prefix. Each one needs a reason after `--`:

```ts
// eslint-disable-next-line no-console, eslint-js/camelcase -- the CLI prints its result
console.log(user_name);
```

### Bringing your own ESLint plugins

Oxlint can run most ESLint plugins through `jsPlugins`, and they work next to this config. For example, [`eslint-plugin-better-tailwindcss`](https://github.com/schoero/eslint-plugin-better-tailwindcss):

```json
{
  "jsPlugins": [
    {
      "name": "better-tailwindcss",
      "specifier": "eslint-plugin-better-tailwindcss"
    }
  ],
  "rules": {
    "better-tailwindcss/no-duplicate-classes": "warn",
    "better-tailwindcss/enforce-consistent-class-order": "warn"
  },
  "settings": {
    "better-tailwindcss": { "entryPoint": "src/styles/globals.css" }
  }
}
```

JS plugins run at JavaScript speed, though. In the Next.js starter measured at the top, the Tailwind plugin took about 7 of the 8.3 seconds, and nearly all of that was one rule, `enforce-canonical-classes`, waiting on Tailwind's own compiler. If you can live without that rule, turning it off brings the whole lint down to about 2.5 s.

## Coming from @webadeva/eslint-config

Most rule ids are what you'd expect from Oxlint. Add-on rules keep their ESLint ids, except where Oxlint has already claimed the plugin name. You'll need to update disable comments and rule overrides that mention these:

| In `@webadeva/eslint-config`                                             | Here                    |
| ------------------------------------------------------------------------ | ----------------------- |
| `camelcase`, `getter-return`, `no-dupe-args`, `no-octal`, `no-redeclare` | `eslint-js/<rule>`      |
| `react-hooks/config`, `react-hooks/gating`                               | `react-hooks-js/<rule>` |
| other `react-hooks/<rule>`                                               | `react/<rule>`          |
| `react-refresh/<rule>`                                                   | `react/<rule>`          |
| `@typescript-eslint/<rule>`                                              | `typescript/<rule>`     |
| `jsx-a11y-x/<rule>`                                                      | `jsx-a11y/<rule>`       |
| `@next/next/<rule>`                                                      | `nextjs/<rule>`         |
| other core rules, such as `no-console`                                   | unchanged               |
| `@eslint-react/*`, `storybook/*`, `webadeva/*`                           | unchanged               |
| `prefer-arrow-functions/*`, `eslint-comments/*`                          | unchanged               |

In its output Oxlint writes the plugin in parentheses, as in `eslint-js(camelcase)` or `@eslint-react(no-missing-key)`.

A few other things change:

- Formatting is its own step. `prettier/prettier` used to be a lint rule, so ESLint reported formatting problems and fixed them. Now `oxfmt` formats and `oxfmt --check` reports, and it flags whole files rather than single lines. oxfmt also lays out a few things differently from Prettier; long union types, for one, get one member per line.
- Every file is type-checked, not only the ones a tsconfig includes. More on that below.
- TypeScript is never loaded, so any version works, 7 included.
- As in the ESLint version, a handful of React Compiler checks run twice, once as `react/<rule>` and once as `@eslint-react/<rule>` (`purity`, `static-components`, `use-memo`, `set-state-in-render`, `error-boundaries`, `unsupported-syntax`).

On a real project the results line up closely. On a test file full of deliberate mistakes, 37 of ESLint's findings came back identical (same rule, line, and severity), and `--fix` left the files byte-for-byte the same as `eslint --fix` with Prettier. The add-on's rules are each tested against ESLint's output on the same files, including the odd bugs of the plugins they port. The known differences are small:

- `eslint-comments/require-description` reports column 1 of the comment's line; ESLint said column 0.
- `eslint-js/no-octal` fires in sloppy-mode code, where ESLint's TypeScript parser gave up with a parse error.
- `storybook/*` rules can't use `includeStories`/`excludeStories` regexes with lookaround or backreferences. A filter like that matches nothing.
- Oxlint's `prefer-const` and `no-unreachable` put some reports at a different position.
- Oxlint's `typescript/no-inferrable-types` also flags literal-type annotations.
- Oxlint's `sort-imports` reorders lists already sorted by code point (`{ Zed, alpha }`), and wants inline `type` specifiers where their name sorts instead of last.

## Type-aware linting

The type-aware rules run on [oxlint-tsgolint](https://github.com/oxc-project/tsgolint), which is built on the Go port of TypeScript. Unlike typescript-eslint, it types every file, even files no tsconfig includes. Those get default compiler options, so the rules still run, but the types may not be quite what a tsconfig would have given.

Since `tsc` never checks a file that's outside every tsconfig, `webadeva/typescript-file-in-project` reports one. Up to 8 root-level files, such as config files and scripts, are let through, as before.

Two things to watch for:

- Install `@types/react` in React projects. Without it JSX has the `error` type, and the `typescript/no-unsafe-*` rules complain about it.
- An invalid tsconfig (for example `composite` with `allowJs` but no `outDir`) shows up as `typescript/tsconfig-error`.

## Speed

On a 313-file project, ESLint took 37.5 s and `oxlint` with this config about 3.5 s, most of it type-aware linting. The add-on's rules cost about 0.15 s per 1,000 files, and handing files from Oxlint to the add-on another 0.4–0.5 s per 1,000. Oxlint spreads files over your CPU cores, so more cores means faster runs.

## Accessibility

The React preset, and so the Next.js and Storybook ones, turns on Oxlint's [`jsx-a11y` rules](https://oxc.rs/docs/guide/usage/linter/rules.html): the same 31 rules and options as the `recommended` config of [`eslint-plugin-jsx-a11y-x`](https://github.com/es-tooling/eslint-plugin-jsx-a11y-x) that the ESLint version uses. Add more, or tune one, under `rules`:

```json
{
  "rules": {
    "jsx-a11y/prefer-tag-over-role": "error",
    "jsx-a11y/no-autofocus": "off"
  }
}
```

A linter only sees the JSX you wrote, not the page that renders. To check the rendered DOM, run [`@axe-core/react`](https://github.com/dequelabs/axe-core-npm/tree/develop/packages/react) in development. It's a runtime tool, so this package doesn't install it:

```bash
pnpm add -D @axe-core/react
```

```js
import React from "react";
import ReactDOM from "react-dom";

if (process.env.NODE_ENV !== "production") {
  const { default: axe } = await import("@axe-core/react");
  await axe(React, ReactDOM, 1000);
}
```

It logs problems to the browser console. Note that `@axe-core/react` doesn't support React 18 or later, and needs Node.js `>=22.12`.

## Good to know

- `dist`, `*.config.*`, `**/__tests__/**`, and `**/__mocks__/**` are ignored. `.storybook` is linted, even though Oxlint skips other dot-directories.
- The React rules read your React version from the installed `react` package, monorepos included.
- Some rules are opinionated. `no-console` warns, and arrow functions are preferred over `function`.

## What's inside

From Oxlint itself: the `eslint`, `typescript` (typescript-eslint's recommended and stylistic rules, type-checked through oxlint-tsgolint), `react` (with the react-hooks and react-refresh rules), `jsx-a11y`, and `nextjs` plugins.

From the add-on: the rules of [`@eslint-react`](https://eslint-react.xyz/), [Storybook](https://github.com/storybookjs/eslint-plugin-storybook), [prefer-arrow-functions](https://github.com/JamieMason/eslint-plugin-prefer-arrow-functions), and [eslint-comments](https://github.com/eslint-community/eslint-plugin-eslint-comments) that the ESLint version uses, the ESLint core and react-hooks rules Oxlint lacks or does differently, and [this package's own rules](docs/rules.md).

## Contributing

Issues and pull requests are welcome. [CONTRIBUTING.md](CONTRIBUTING.md) explains the setup, and [docs/development.md](docs/development.md) walks through how the add-on works.

## License

[MIT](LICENSE)
