# Kyuubiki Language Packs

This directory contains distributable UI language support packs for the
`tamamono 1.x` line.

Language packs are intentionally kept outside the built-in UI dictionaries.
Workbench and Hub can import these JSON envelopes as local packs today, while
future store or update-source flows can use the same files as downloadable
assets.

## Layout

- `catalog.json`: release-line catalog for shipped support packs.
- `workbench/*.json`: Workbench language-pack envelopes.
- `hub/*.json`: Hub language-pack envelopes.

## Validate

```sh
node ./scripts/validate-language-packs.mjs
```

The validator checks the catalog, target surfaces, version line, app version,
pack ids, timestamps, and discovered pack files.
