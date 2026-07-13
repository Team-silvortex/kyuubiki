# Central Server Components

`tamamono 1.x` now has a preview contract for the future Kyuubiki central
server. The central server is not the solver engine and not the desktop Hub. It
is the distribution and identity plane that can later back hosted stores,
publisher accounts, language-pack delivery, and signed downloads.

## Current Preview Surface

- API catalog: `GET /api/v1/central/catalog`
- API fetch: `GET /api/v1/central/catalog/:kind/:entry_id`
- Session policy: `GET /api/v1/central/session-policy`
- Backing module: `KyuubikiWeb.CentralStore`
- Router module: `KyuubikiWeb.CentralStoreRouter`
- Catalog schema: `schemas/central-store-catalog.schema.json`
- Session schema: `schemas/central-session-policy.schema.json`

## Store Kinds

- `operator`: operator packages and built-in operator descriptors.
- `workflow_template`: workflow templates and compound simulation flows.
- `frontend_dsl_template`: wasm-python/frontend automation starter templates.
- `language_pack`: shipped Workbench and Hub language packs.

The preview catalog currently reuses the local `AssetStore` plus
`language-packs/catalog.json`. This gives Hub, Workbench, Installer, and
headless SDKs one future-facing shape before a hosted service exists.

## Auth Boundary

The current auth mode remains local orchestra token auth. The central session
policy explicitly marks hosted login as preview/planned instead of pretending a
real account system exists.

Planned providers:

- OIDC for Hub, Workbench, and SDK login.
- Device-code login for CLI, remote agents, and headless SDK flows.
- Personal access tokens for CI, installer, and automation.

Credential storage stays client-owned: platform keychain where available, or
memory-only for sensitive active-session material.

## Non-Goals For This Layer

- It does not execute solver workloads.
- It does not replace agent-orchestra authority rules.
- It does not make each agent mirror the whole operator library.
- It does not introduce publisher trust without signatures and provenance.
