# Kyuubiki Language Packs

This directory contains distributable UI language support packs for the
`moxi 2.x` line. The catalog currently ships 30 mainstream packs for both
Workbench and Hub, so the pack contract is exercised across desktop, webview,
and future remote-store delivery paths.

The authoritative locale target lives in
`config/localization/mainstream-language-pack-locales.json`. The shipped catalog
and UI catalog tests must match that target exactly.

Language packs are intentionally kept outside the built-in UI dictionaries.
Workbench and Hub can import these JSON envelopes as local packs today, while
future store or update-source flows can use the same files as downloadable
assets.

## Layout

- `catalog.json`: release-line catalog for shipped support packs.
- `workbench/*.json`: 30 Workbench language-pack envelopes.
- `hub/*.json`: 30 Hub language-pack envelopes.

## Validate

```sh
make check-language-packs
```

The native validator checks the catalog, target surfaces, version line, app
version, pack ids, timestamps, discovered pack files, unique language tags, and
the minimum 30-pack mainstream target for each UI surface. It also checks
structural UI-key coverage:

- Workbench: 30 languages x 32 required override paths = 960/960.
- Hub: 30 languages x 17 required override paths = 510/510.

For a machine-readable coverage report:

```sh
node scripts/report-language-pack-coverage.mjs
```

It also rejects paths that are not repository-relative and text that looks like
HTML, JavaScript URLs, inline event handlers, browser-storage access, or script
evaluation. Workbench and Hub local import paths mirror the same unsafe-text
rule before a pack or override registry can enter UI copy state.

The retained Node entrypoint `node ./scripts/validate-language-packs.mjs` is
kept for local compatibility, but the Make target is the release gate.
