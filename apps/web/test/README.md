# Web Tests

`apps/web/test` follows the same control-plane domain split as the Elixir
source tree.

- `kyuubiki_web/api/`
  Router- and HTTP-level integration tests.
- `kyuubiki_web/jobs/`
  Job state, store, and watchdog tests.
- `kyuubiki_web/playground/`
  Agent-runtime boundary tests such as agent client, pool, and registry.
- `kyuubiki_web/workers/`
  Transitional worker adapter tests.

Keep tests close to the source-domain they protect, even when they exercise
multiple modules together.

## PostgreSQL Recovery Tests

Use a disposable database, never an installed runtime's active store. Set
`KYUUBIKI_STORAGE_BACKEND=postgres` and `DATABASE_URL` before starting Mix.
`KYUUBIKI_TEST_DATABASE_URL` enables the PostgreSQL migration and typed-workflow
tests; each creates and removes its own random schema. These tests cover numeric
identity, result compare-and-swap, checkpoint rollback and current repository
lease fencing. Ordinary support loading does not seed the opt-in live fixtures.

The Rust `headless_live` test target has three ignored `real_typed_postgres`
tests. Explicitly provide `KYUUBIKI_HEADLESS_LIVE_TEST_POSTGRES_URL` and run this
filter with `--ignored --test-threads=1`. Each owned workflow round uses a separate
schema, reused only across its Orchestra restart and removed afterwards. Capture
files are atomically published; they do not fabricate solver results or receipts.
The remote fixture allows bounded connection-pool warm-up without changing
production lease/query timeouts. Provisioning and retiring the disposable
database and any SSH tunnel remain the test operator's responsibility.

## Collection Outage Controls

`kyuubiki_web/api/analysis_collection_outage_test.exs` covers job/result lists, workload
discovery, database export and project-bundle download through actual store
stop/restart. It checks opaque unavailable responses without partial attachments,
unchanged restored records, genuine empty collections, authorization before reads,
private recovery exclusion and unchanged project-not-found semantics. Run on
SQLite or an isolated JSON directory; PostgreSQL requires a disposable database.
## SQL Export Snapshot Controls

`kyuubiki_web/storage/export_read_snapshot_test.exs` commits actual writes on a
second repository connection between export queries. Project bundles, database
exports and workload catalogs must retain one generation across model/version
edits, project removal and joint job/result deletion; the next read sees the
committed changes. Database-export security events share that same snapshot.
`kyuubiki_web/storage/read_snapshot_test.exs` checks response shapes, rollback on
query failure, exception visibility, ambient-transaction refusal and dynamic-repo
selection. PostgreSQL additionally verifies repeatable-read/read-only settings
and rejected writes without leaked connection state. Run on SQLite or disposable
PostgreSQL. SQL controls skip on JSON; they do not qualify JSON cross-store
consistency, full backups, external artifacts, large exports or installed GUIs.
