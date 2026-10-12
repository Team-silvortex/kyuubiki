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

## JSON Export Snapshot Controls

`kyuubiki_web/storage/memory_read_snapshot_test.exs` covers captured model/version
and job/result views, cross-store validation, ABA content restoration, actual
analysis-store restart, three-attempt conflict limits, rejected writes, nested
reads and one capture per accessed store. Tokens and telemetry carry no payload
archive or durable format changes.
`kyuubiki_web/storage/memory_export_snapshot_test.exs` exercises all three HTTP
composite reads with real writes between captures: model/version publication,
joint job/result deletion, project deletion, bounded conflicts, independent
library/security-store outages and a mid-read analysis-store stop/restart.
Conflicts return opaque 503 JSON without attachment headers. Security events
participate in database export only. Run with `KYUUBIKI_STORAGE_BACKEND=json`
and a fresh temporary `KYUUBIKI_DATA_DIR`; these cases skip on SQL backends.
This verifies live read consistency, not multi-file power-loss recovery or
new library persistence. The lightweight project library remains in memory.

## Independent History Controls

`kyuubiki_web/api/history_read_outage_test.exs` stops the actual project library
or security-event store and checks eight library/receipt routes plus three
security history/export routes. It checks opaque failures, CSV error content type,
authentication before reads, empty/missing records, receipt cache policy, filters
and unchanged execution records. SQL and durable security-event data restore;
the lightweight library's restart is explicitly volatile, not record recovery.
Version-context derivation propagates unavailability without creating a task.

`kyuubiki_web/library/history_read_snapshot_test.exs` commits real second-connection
writes between SQL detail queries, through direct helpers and HTTP routes.
`kyuubiki_web/storage/history_partial_export_test.exs` temporarily renames a table
only in the disposable test database, proving query failure cannot become empty
version history or a successful partial database export. It restores the table
in `after`. `read_snapshot_test.exs` also checks owned nested snapshots, rejected
cross-repository reads and explicit rollback after a contained nested query error.
Run on isolated JSON, SQLite or disposable PostgreSQL; SQL-only cases skip on JSON.
These controls do not qualify installed GUI behavior, a snapshot across multiple
HTTP requests, full backups, or large-history memory/latency.

## Job Record Mutation Controls

`kyuubiki_web/api/job_record_recovery_api_test.exs` distinguishes omitted optional
version metadata from explicit null clearing. Null, empty and whitespace-only
required project/case identities return 422 without changing the job or result.
Confirmed deletion removes only the matching job/result pair; a second deletion
returns 404, and later storage loss cannot undo the committed operation.
`kyuubiki_web/jobs/job_test.exs` also rejects whitespace-only required identities
at creation and update without silently trimming valid identifiers.

Use isolated JSON data or the test runner's disposable SQLite database. These
are real router/store controls, separate from the source browser's mocked-service
tests and not installed, remote, power-loss or exactly-once qualification. See
the [job record recovery results](../../../reports/workbench-job-record-recovery-20261011.md).

## Result Record Mutation Controls

`kyuubiki_web/api/result_administration_api_test.exs` rejects nonobject result
payloads without changing job/result records. Empty and nested objects produce
matching receipts; deletion retains its parent job and other work, and cannot be
repeated. Confirmed save/delete is checked before real storage loss; the later
history read returns opaque 503 rather than empty successful data. Existing
controls protect private workflow recovery metadata, derive public recovery
summaries on the server, reject active-workflow edits and prevent resurrection.

Use isolated JSON data or the test runner's disposable SQLite database. These
are real API/storage controls, not the source browser's mock backend, installed
GUI, remote, power-loss or exactly-once acceptance. See the
[result record recovery results](../../../reports/workbench-result-record-recovery-20261011.md).

## Library Record Mutation Controls

`kyuubiki_web/api/library_record_recovery_api_test.exs` checks real project creation,
metadata edits and project/model/version deletion receipts. Removed identities
return 404 on read, repeated deletion and attempted metadata edits; remaining
parents, unrelated models/projects and execution records stay unchanged. Confirmed
deletion is checked before actual storage loss; the subsequent catalog returns
opaque 503 instead of empty successful data. JSON's library restart is volatile,
so these outage controls do not claim durable memory-store recovery.

Use isolated JSON data or the test runner's disposable SQLite database. These
controls are separate from mocked-service source browser tests and do not qualify
installed GUIs, remote execution, power-loss recovery or exactly-once behavior.
See the [library record recovery results](../../../reports/workbench-library-record-recovery-20261011.md).
