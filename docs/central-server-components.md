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
- Publish readiness: `GET /api/v1/central/publish-readiness`
- Database policy: `GET /api/v1/central/database-policy`
- Provenance policy: `GET /api/v1/central/provenance-policy`
- Database status: `GET /api/v1/central/database-status`
- Backing module: `KyuubikiWeb.CentralStore`
- Router module: `KyuubikiWeb.CentralStoreRouter`
- Catalog schema: `schemas/central-store-catalog.schema.json`
- Contract-check schema: `schemas/central-store-contract-check.schema.json`
- Session schema: `schemas/central-session-policy.schema.json`
- Publish schema: `schemas/central-publish-policy.schema.json`
- Publish readiness schema: `schemas/central-publish-readiness.schema.json`
- Database schema: `schemas/central-database-policy.schema.json`
- Provenance schema: `schemas/central-provenance-policy.schema.json`
- Database status schema: `schemas/central-database-status.schema.json`
- Readiness report schema: `schemas/central-readiness-report.schema.json`
- Contract-check config: `config/architecture/central-store-contract.json`

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

The publish readiness endpoint turns those blockers into a machine-readable
matrix. It lists global blockers, per-resource readiness, provenance
attestations, installer checks, storage tables, and the next unlocks required
before any write-side upload API can be enabled.

## Provenance Policy

The provenance policy is also read-only. It defines the download and artifact
verification contract before central uploads exist: accepted digest algorithms,
immutable artifact metadata, required attestations, detached signature posture,
yank/security-recall behavior, and installer verification rules. This lets the
Installer, Workbench store, and headless SDKs agree on supply-chain checks
without storing signing keys or release credentials in the repository.

## Database Policy

The database policy exposes the active storage backend, supported server
backends, schema-ready preview central-server persistence domains, table specs,
and smoke commands for a Postgres deployment. The database status endpoint is a
read-only runtime view of the same contract: backend, repo module, managed table
count, and per-domain table coverage. The current preview still relies on
startup schema setup for existing runtime and central tables. Central-server
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

Use `make build-central-readiness-report` when CI, release notes, Hub docs, or
LLM ingestion need one machine-readable summary. The report is written under
`tmp/central-readiness-report.json` by default, with a compact Markdown summary
at `tmp/central-readiness-report.md`. It combines DB readiness, central API
endpoint coverage, schema file coverage, storage table-contract presence, and
central contract config coverage, plus safe runbook commands. It intentionally
does not store credentials or live database connection strings. Its schema is
`schemas/central-readiness-report.schema.json`.

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
