# Kyuubiki Language Packs

This directory contains distributable UI language support packs for the
`moxi 2.x` line. The catalog currently ships 30 mainstream starter packs
for both Workbench and Hub, so the pack contract is already exercised across
desktop, webview, and future remote-store delivery paths.

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
node ./scripts/validate-language-packs.mjs
```

The validator checks the catalog, target surfaces, version line, app version,
pack ids, timestamps, discovered pack files, unique language tags, and the
minimum 30-pack mainstream target for each UI surface.
