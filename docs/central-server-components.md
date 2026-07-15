# Central Server Components

`tamamono 1.x` now has a preview contract for the future Kyuubiki central
server. The central server is not the solver engine and not the desktop Hub. It
is the distribution and identity plane that can later back hosted stores,
publisher accounts, language-pack delivery, and signed downloads.

The website service belongs to this central-server surface. It is a deployable
web/API face of the same `apps/web` control-plane workload, not a separate
top-level module and not an independent product shell. Hosted Kyuubiki and
self-hosted research deployments should consume the same contracts so a lab can
run its own store, login/session policy, language-pack distribution, and signed
download index without forking the architecture.

Architecture boundary: the website service is not a separate top-level module.

## Current Preview Surface

- API catalog: `GET /api/v1/central/catalog`
- API fetch: `GET /api/v1/central/catalog/:kind/:entry_id`
- Session policy: `GET /api/v1/central/session-policy`
- Publish policy: `GET /api/v1/central/publish-policy`
- Publisher policy: `GET /api/v1/central/publisher-policy`
- Publish readiness: `GET /api/v1/central/publish-readiness`
- Database policy: `GET /api/v1/central/database-policy`
- Provenance policy: `GET /api/v1/central/provenance-policy`
- Artifact admission policy: `GET /api/v1/central/artifact-admission-policy`
- Publish pipeline: `GET /api/v1/central/publish-pipeline`
- Database status: `GET /api/v1/central/database-status`
- Backing module: `KyuubikiWeb.CentralStore`
- Router module: `KyuubikiWeb.CentralStoreRouter`
- Catalog schema: `schemas/central-store-catalog.schema.json`
- Contract-check schema: `schemas/central-store-contract-check.schema.json`
- Session schema: `schemas/central-session-policy.schema.json`
- Publish schema: `schemas/central-publish-policy.schema.json`
- Publisher schema: `schemas/central-publisher-policy.schema.json`
- Publish readiness schema: `schemas/central-publish-readiness.schema.json`
- Database schema: `schemas/central-database-policy.schema.json`
- Provenance schema: `schemas/central-provenance-policy.schema.json`
- Artifact admission schema:
  `schemas/central-artifact-admission-policy.schema.json`
- Publish pipeline schema: `schemas/central-publish-pipeline.schema.json`
- Database status schema: `schemas/central-database-status.schema.json`
- Readiness report schema: `schemas/central-readiness-report.schema.json`
- Contract-check config: `config/architecture/central-store-contract.json`
- Module topology service surface: `central-web-service` under
  `orchestra-control-plane`

## Self-Hosted Website Service Boundary

The central website service is an internal service surface of
`orchestra-control-plane`.

It should:

- publish central store catalogs, language packs, release indexes, and read-only
  policy surfaces through stable API contracts.
- support self-hosted deployment with the same schema and readiness checks as a
  future hosted Kyuubiki service.
- keep credentials, database URLs, signing keys, and host-local deployment
  details outside the repository.
- remain callable by Hub, Workbench, Installer, and headless SDK clients without
  importing GUI code.

It should not:

- become a separate top-level module with its own path ownership.
- replace Hub or Workbench UI responsibilities.
- execute solver workloads or agent tasks directly.
- bypass installer-managed deployment, provenance, or integrity checks.

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

The publisher policy is the read-only account and token-scope contract for that
future write side. It keeps account creation and token issuance disabled, but
defines the publisher lifecycle, planned login modes, token scopes, fingerprint
storage table, rotation/revocation posture, and the rule that raw tokens are
never stored by the central service.

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

The artifact admission policy is the companion preflight contract for future
write-side publishing. It keeps uploads disabled, but exposes the artifact
envelope, resource-specific evidence, token scopes, review queue stages,
signature/SBOM expectations, and blocking reasons that must be cleared before a
real upload endpoint can exist. This gives SDKs and self-hosted deployments one
machine-readable target without prematurely accepting packages.

The publish pipeline endpoint ties the separate policies into one ordered
read-only workflow: publisher identity, artifact envelope, detached signature,
review queue, catalog indexing, yank/security recall, and installer download
verification. It deliberately reports `accepting_writes=false` until those
stages have real backing services, but gives Hub, Workbench, Installer, and
headless SDKs one stable sequence for UI guidance and self-host readiness.

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
central contract config coverage, self-hosted website service-surface coverage,
plus safe runbook commands. It intentionally does not store credentials or live
database connection strings. Its schema is
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
