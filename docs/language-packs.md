# Workbench Language Packs

Workbench language packs let `tamamono 1.x` grow beyond the built-in `en`,
`zh`, and `ja` copy sets without hard-wiring every future language into the
repo first.

This is intentionally a local-first workflow today:

- download a template from `System -> Config -> Language packs`
- edit a JSON pack locally
- import it into the current browser workspace
- let the imported override tree patch the built-in copy dictionary

The same surface is also the planned home for future remote language-pack
delivery.

## Current contract

- schema: [language-pack.schema.json](../schemas/language-pack.schema.json)
- schema version: `kyuubiki.language-pack/v1`

Each pack is a JSON object with:

- `schema_version`
- `id`
- `language`
- `name`
- `version`
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
  "name": "French custom pack",
  "version": "1.5.0",
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
- installed packs can be exported again from the same UI surface

## What it does not do yet

- no remote catalog download flow yet
- no signature or provenance chain yet
- no schema validator dependency in the browser yet; the importer currently
  does lightweight structural checks and then relies on the override merge path

That last point is deliberate for now. The schema exists so the format can
stabilize early, even before the remote delivery and validation stack is wired
in.
