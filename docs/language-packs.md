# Workbench Language Packs

Workbench language packs let `tamamono 1.x` grow beyond the built-in `en`,
`zh`, `ja`, and `es` copy sets without hard-wiring every future language into the
repo first.

This is intentionally a local-first workflow today:

- download a template from `System -> Config -> Language packs`
- edit a JSON pack locally
- import it into the current browser workspace
- let the imported override tree patch the built-in copy dictionary

The same surface is also the planned home for future remote language-pack
delivery.

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
  "version": "1.6.0",
  "versionLine": "tamamono 1.x",
  "targetAppVersion": "1.11.0",
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

## What it does not do yet

- no remote catalog download flow yet
- no signature or provenance chain yet
- no schema validator dependency in the browser yet; the importer currently
  does lightweight structural checks and then relies on the override merge path

That last point is deliberate for now. The schema exists so the format can
stabilize early, even before the remote delivery and validation stack is wired
in.

## Recommended packaging posture

- keep `versionLine` aligned with the active line such as `tamamono 1.x`
- set `targetSurface` to `workbench` or `hub` for every newly generated pack
- set `targetAppVersion` when a pack is prepared against a specific shipped UI
  build such as `1.11.0`
- treat packs without either field as generic overrides, not as audited release
  assets
