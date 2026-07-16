# Workbench Language Packs

Workbench language packs let `moxi 2.x` grow beyond the built-in `en`,
`zh`, `ja`, and `es` copy sets without hard-wiring every future language into the
repo first.

This is intentionally a local-first workflow today:

- download a template from `System -> Config -> Language packs`
- edit a JSON pack locally
- import it into the current browser workspace
- let the imported override tree patch the built-in copy dictionary

The same surface is also the planned home for future remote language-pack
delivery.

## Shipped support packs

The repo now keeps distributable support packs under
[`language-packs`](../language-packs/). As of `moxi 2.0.x`, the shipped
catalog covers 30 mainstream translated core locales for both Workbench and Hub:

- target contract:
  [`config/localization/mainstream-language-pack-locales.json`](../config/localization/mainstream-language-pack-locales.json)
- catalog: [`language-packs/catalog.json`](../language-packs/catalog.json)
- Workbench packs: `ar`, `bn`, `cs`, `da`, `de`, `el`, `fa`, `fi`, `fr`,
  `he`, `hi`, `id`, `it`, `ko`, `ms`, `nl`, `no`, `pl`, `pt-BR`, `ro`, `ru`,
  `sv`, `sw`, `ta`, `th`, `tr`, `uk`, `ur`, `vi`, `zh-TW`
- Hub packs: the same 30 locale tags, packaged as separate Hub override
  envelopes

These files are release-line assets rather than built-in copy branches. Import
them from the existing local language-pack panels today; future download-source
flows can consume the same catalog and pack envelopes.

The non-built-in packs now ship translated core UI coverage for the current
language-pack contract: navigation, primary surfaces, shell actions, system
language-pack controls, and high-traffic workflow labels are translated per
locale. Deeper product-copy translation can still expand incrementally without
changing the pack format.

Workbench also mirrors the catalog metadata in its System page so operators can
install the shipped support packs from the built-in catalog even before remote
download sources are wired.

Validate the shipped pack set with:

```sh
make check-language-packs
```

The retained Node entrypoint `node ./scripts/validate-language-packs.mjs` is
kept for local compatibility, but the Make target is the release gate and uses
the native script runner.

## Hub alignment

Hub is now wired to the same local-first copy philosophy through a lightweight
override registry:

- storage key: `hub.copy-overrides.v1`
- merge rule: deep partial override on top of the built-in Hub dictionary
- current scope: built-in Hub locales still ship in-tree, while overrides act as
  the compatibility seam for future pack delivery
- current operator entry: `Home -> Guides -> Localization overrides`

This means Workbench already has operator-facing import and export flows, and
Hub now has the underlying override contract ready without having to fork the
entire built-in copy tree first.

Hub currently accepts two local JSON entry shapes from that panel:

- a single language-pack envelope with `language` plus `overrides`
- a full Hub override registry with `defaults` and/or `languages`

Single-pack imports are merged into the current Hub override registry and kept
visible as pack metadata. Full-registry imports replace the current registry
snapshot in one go.

## Current contract

- schema: [language-pack.schema.json](../schemas/language-pack.schema.json)
- schema version: `kyuubiki.language-pack/v1`

Each pack is a JSON object with:

- `schema_version`
- `id`
- `language`
- optional `targetSurface` (`workbench` or `hub`; omit only for legacy local packs)
- `name`
- `version`
- optional `versionLine`
- optional `targetAppVersion`
- `source`
- `updatedAt`
- optional `description`
- `overrides`

`overrides` is a deep partial tree that patches the built-in workbench copy.
That means a pack does not need to redefine the whole dictionary; it only needs
to replace the keys it cares about.

## Example

```json
{
  "schema_version": "kyuubiki.language-pack/v1",
  "id": "fr-custom-pack",
  "language": "fr",
  "targetSurface": "workbench",
  "name": "French custom pack",
  "version": "2.0.0",
  "versionLine": "moxi 2.x",
  "targetAppVersion": "2.0.0",
  "source": "imported",
  "updatedAt": "2026-05-21T00:00:00.000Z",
  "description": "Overrides a few high-traffic labels first.",
  "overrides": {
    "title": "Établi d'analyse structurelle",
    "rail": {
      "study": "Étude",
      "model": "Modèle",
      "library": "Historique",
      "system": "Système"
    },
    "assistantOpen": "Ouvrir l'assistant"
  }
}
```

## What the current implementation guarantees

- packs are stored locally in browser storage
- imported packs can override the active language immediately
- built-in copy remains the fallback for keys not supplied by the pack
- `targetSurface` prevents Workbench packs and Hub packs from being imported into
  the wrong UI surface once the pack declares a surface
- installed packs can be exported again from the same UI surface
- packs may declare a product-line target and a shipped app-version target so
  operators can see whether a pack was prepared for the current Workbench line
- shipped packs, catalog entries, Workbench local imports, and Hub override
  imports reject text that looks like HTML, JavaScript URLs, inline event
  handlers, browser-storage access, or script evaluation before it can enter UI
  copy state

## What it does not do yet

- no remote catalog download flow yet
- no signature or provenance chain yet
- no full schema validator dependency in the browser yet; the importer currently
  does lightweight structural and unsafe-text checks, then relies on the
  override merge path

That last point is deliberate for now. The schema exists so the format can
stabilize early, even before the remote delivery and validation stack is wired
in.

## Recommended packaging posture

- keep `versionLine` aligned with the active line such as `moxi 2.x`
- set `targetSurface` to `workbench` or `hub` for every newly generated pack
- set `targetAppVersion` when a pack is prepared against a specific shipped UI
  build such as `2.0.0`
- treat packs without either field as generic overrides, not as audited release
  assets
