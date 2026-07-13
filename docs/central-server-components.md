# Central Server Components

`tamamono 1.x` now has a preview contract for the future Kyuubiki central
server. The central server is not the solver engine and not the desktop Hub. It
is the distribution and identity plane that can later back hosted stores,
publisher accounts, language-pack delivery, and signed downloads.

## Current Preview Surface

- API catalog: `GET /api/v1/central/catalog`
- API fetch: `GET /api/v1/central/catalog/:kind/:entry_id`
- Session policy: `GET /api/v1/central/session-policy`
- Publish policy: `GET /api/v1/central/publish-policy`
- Database policy: `GET /api/v1/central/database-policy`
- Backing module: `KyuubikiWeb.CentralStore`
- Router module: `KyuubikiWeb.CentralStoreRouter`
- Catalog schema: `schemas/central-store-catalog.schema.json`
- Session schema: `schemas/central-session-policy.schema.json`
- Publish schema: `schemas/central-publish-policy.schema.json`
- Database schema: `schemas/central-database-policy.schema.json`

## Store Kinds

- `operator`: operator packages and built-in operator descriptors.
- `workflow_template`: workflow templates and compound simulation flows.
- `frontend_dsl_template`: wasm-python/frontend automation starter templates.
- `language_pack`: shipped Workbench and Hub language packs.

The preview catalog currently reuses the local `AssetStore` plus
`language-packs/catalog.json`. This gives Hub, Workbench, Installer, and
headless SDKs one future-facing shape before a hosted service exists.

## Publish Policy

The preview publish policy is intentionally read-only. It defines required
resource kinds, manifest schemas, review stages, and evidence requirements, but
does not accept uploads yet. Write paths should wait until publisher accounts,
token scopes, signing keys, and provenance checks are in place.

## Database Policy

The database policy exposes the active storage backend, supported server
backends, schema-ready preview central-server persistence domains, table specs,
and smoke commands for a Postgres deployment. The current preview still relies
on startup schema setup for existing runtime and central tables. Central-server
tables should move to versioned migrations before write-side publishing is
enabled.

Use `make check-central-database-readiness` before local checks. For a server
or cloud rehearsal, pass `MODE=cloud BACKEND=postgres` and provide
`DATABASE_URL` in the environment; the command only validates configuration and
contracts, then the focused Elixir API smoke tests can exercise the real DB.

Use `make test-central-database-smoke` as the safe server rehearsal wrapper. It
is a dry-run unless `RUN_DB_SMOKE=1` is set. On a Postgres host, run it with
`RUN_DB_SMOKE=1 DATABASE_URL=... MODE=cloud BACKEND=postgres` to execute the
focused central-store and asset-store API tests against the configured DB.

Use `make remote-central-database-smoke REMOTE=kyuubiki-lab` when the rehearsal
should run on the Ubuntu lab host instead of the Mac. The remote wrapper syncs
the current source tree to a relative scratch directory, excludes generated
build artifacts, and runs the same readiness/smoke pair there. It does not
store SSH credentials, server config, or `DATABASE_URL` in the repository; a
real DB-backed run still requires `RUN_DB_SMOKE=1 DATABASE_URL=...` in the
current process environment.

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
