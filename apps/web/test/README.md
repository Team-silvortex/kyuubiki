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

## Native Diagnostic Catalog Controls

The follow-up [native thermal catalog controls](../../../reports/thermal-catalog-native-recovery-20261012.md)
repair the three diagnostic catalog failures found during the expanded receipt
sweep. `WorkflowDiagnosticAgent` scripts explicit solver/diagnostic sessions,
validates task binding and checks complete consumption. Its default diagnostic
adapter is a test reference, not native execution. Set
`KYUUBIKI_WORKFLOW_DIAGNOSTIC_AGENT_PORT` to an isolated local Rust Agent port to
execute real diagnostics; solver outputs remain fixtures. The thermal catalog
tests cover stale receipts, pending acknowledgements, missing required flux and
explicit new-job recovery. No automatic fallback to mock mode is permitted when
a live port is configured. Stop the owned Agent after the run.

## Cross-Domain Decision Recovery

`workflow_domain_decision_integrity_test.exs` covers eight guard/pair families,
including incomplete evidence, malformed rules, convergence markers, label
collisions, valid zeros and numeric overflow. `api/workflow_decision_recovery_api_test.exs`
executes rejected inputs and explicit fresh-job controls through the Router and
asynchronous coordinator, checking both report publication and retained failures.
These are controlled summaries, not solver runs. See the
[initial decision admission evidence](../../../reports/domain-decision-integrity-20261012.md).

`workflow_metric_integrity_test.exs` exercises checked resolution, corrupt aliases,
incomplete derived samples, valid zeros, stable norms and native peak tie rules.
`workflow_cfd_diagnostic_integrity_test.exs` checks complete node/element evidence,
mixed-alias reductions, first/middle/last corrupt rows and checked arithmetic.
The API suite also covers alias/sample rejection and explicit fresh-job recovery
across all eight domain families. See the
[checked metrics and CFD follow-up](../../../reports/checked-metric-cfd-recovery-20261012.md)
for commands, counts and remaining legacy-reducer gaps.

`workflow_electromagnetic_diagnostic_integrity_test.exs` protects optional versus
partial static electric/magnetic groups, selected aliases, source mappings,
convergence and numeric boundaries. `workflow_electromagnetic_diagnostic_recovery_test.exs`
executes controlled raw-result graphs through registered local reducers, guards
and JSON export under both `fail` and `skip`. These are direct GraphRunner tests,
not new API jobs or live native dispatch. See the
[electromagnetic evidence](../../../reports/electromagnetic-diagnostic-integrity-20261012.md).

`workflow_thermal_diagnostic_integrity_test.exs` extends checked raw admission to
thermal/thermo temperature, heat loads, gradients, flux, displacement, stress and
strain. It covers signed reductions, selected aliases, component fallbacks,
numeric extremes and complete secondary groups beside otherwise healthy data.
`workflow_thermal_diagnostic_recovery_test.exs` checks direct GraphRunner failure
isolation and corrected fresh runs through thermal guards and JSON export under
both error policies. It is not live-Agent or durable recovery qualification.
See the [thermal follow-up](../../../reports/thermal-diagnostic-integrity-20261012.md)
for native mapping alignment, commands and remaining scoring/admission work.

`workflow_quality_integrity_test.exs` checks all nine domain quality scorers:
selected terms, every declared override, convergence, damaged versus absent
metrics, zero weights, small positive targets and checked arithmetic.
`workflow_quality_recovery_test.exs` follows those scorers through composite
objective and JSON export with both `fail` and `skip`. Rejected inputs cannot
publish, corrected fresh runs recover, and genuinely missing metrics export only
a blocking assessment. The decision matrix and asynchronous API recovery suite
now include thermal guards and coupled-heat pairs. These controlled evidence
checks do not qualify independent composite-objective/candidate-ranking input,
live physical solves or general durable recovery. See the
[quality and thermal decision evidence](../../../reports/quality-and-thermal-decision-integrity-20261012.md).

`workflow_objective_admission_test.exs` and `workflow_quality_round_admission_test.exs`
check independently supplied objective/ranking inputs, checked numeric boundaries,
validation evidence, retained metadata/blockers and incomplete-ranking gates.
`workflow_objective_recovery_test.exs` exercises both error policies and export
isolation; `api/workflow_objective_recovery_api_test.exs` tests actual asynchronous
service jobs with the shared `support/quality_recovery.exs` graph fixtures.
Partial rankings are successful evaluations but cannot become completed research
decisions. Corrected submissions preserve earlier failure/replan records.
The 76 new tests use controlled summaries, not physical solves. See the
[objective and ranking evidence](../../../reports/quality-objective-ranking-recovery-20261012.md).

`workflow_sweep_admission_test.exs` protects planning budgets, deferred ranges,
strict counts, unambiguous path assignment and metadata retention.
`workflow_quality_batch_admission_test.exs` rejects partial/empty cases and invalid
batch configuration, and checks JSON exchange rather than only in-memory tuples.
`workflow_sweep_recovery_test.exs` and `api/workflow_sweep_recovery_api_test.exs`
use `support/quality_sweep_recovery.exs` for plan/materialize/expand/batch/export
graphs. Decoded batches reach real local preparation and execution, with analytic
thermal-shock results checked against independent formulas. Blocked scans cannot
publish tasks; fresh submissions preserve earlier failures. These are not live
Agent, FEM-accuracy or automatic durable-resumption tests. See the
[sweep and batch evidence](../../../reports/quality-sweep-batch-recovery-20261012.md).

`workflow_sweep_result_admission_test.exs` checks exact keyed joins, whole-batch
positional inputs, stale-summary removal and rejected/unmatched result accounting.
`workflow_sweep_evaluation_test.exs` checks complete selected metrics, provenance
protection, numeric boundaries and hard feasibility ahead of score. Missing-data
tables require explicit diagnostic mode and cannot enter scoring.
`workflow_sweep_result_recovery_test.exs` and
`api/workflow_sweep_result_recovery_api_test.exs` use
`support/sweep_result_recovery.exs` for join/summary/score/export recovery. They
also collect shuffled results from actual local thermal-shock batches with an
explicit metric projection, not a generic extraction adapter or live Agent.
Fresh successful submissions cannot rewrite failed jobs; all-infeasible completed
jobs have no actionable winner. See the
[result-selection evidence](../../../reports/sweep-result-selection-recovery-20261012.md).

`workflow_sweep_score_contract_test.exs` runs the same 25 scoring fixtures as the
native Engine tests from `tests/fixtures/workflow-sweep-score-contract.json` at
the repository root. Valid controls assert selected output fields, including
hard-limit breakdowns; invalid controls require rejection. This is bounded
scoring parity, not full workflow equivalence or live Agent dispatch. The
[native selection follow-up](../../../reports/native-sweep-selection-recovery-20261012.md)
documents fixed-reference quality costs, native graph recovery and remaining
aggregate-only evidence limits.

`workflow_sweep_collection_contract_test.exs` runs 41 cases from the shared
`tests/fixtures/workflow-sweep-collection-contract.json` fixture and checks custom
summary cleanup and graph-level recovery. Native companion controls live under
`workers/rust/crates/engine/src/tests/workflow_sweep_collection_*`. Final solver
markers must be checked before field filtering, and provenance containers cannot
be repurposed as result summaries. See the
[collection follow-up](../../../reports/native-sweep-collection-recovery-20261012.md)
for sixteen native and four service controlled graphs, copy avoidance and limits.

`workflow_quality_decision_contract_test.exs` runs the same 49 cases as native
`workflow_quality_decision_contract.rs`: composition domains/types, validation
evidence, candidate identity, ranking accounting and next-round admission.
Native `workflow_quality_decision_recovery.rs` adds thirty controlled graph runs,
distinguishing incomplete evaluation from valid repair searches. See the
[quality-decision follow-up](../../../reports/native-quality-decision-recovery-20261012.md)
for fresh-run recovery and the remaining non-equivalence boundaries.

`workflow_quality_ranking_integrity_test.exs` runs 31 shared ordering/score-copy
cases plus canonical permutation and JSON-exchange controls. Its recovery test
uses sixteen controlled service graphs and observes exports; the native
`workflow_quality_ranking_recovery.rs` uses sixteen graphs through sweep expansion.
See [ranking consistency evidence](../../../reports/quality-ranking-consistency-recovery-20261012.md)
for rejection, corrected fresh execution and physical-optimality limits.

## Project Creation Recovery

`kyuubiki_web/library/project_request_test.exs` covers opt-in creation identities,
identical/conflicting concurrent requests, original receipts after edits, deletion
tombstones, invalid IDs and authentication before creation or replay. Run on the
default disposable SQLite store or with `KYUUBIKI_STORAGE_BACKEND=memory`.
`project_request_transaction_test.exs` additionally injects a real receipt-insert
failure, uses independent SQLite connections, and closes/reopens a private database
for committed and deleted receipts. Memory Library receipts are process-local,
not durable recovery. These tests do not simulate a physical network disconnect.

`storage/project_request_migration_test.exs` verifies an explicit revision-2 to
revision-3 candidate upgrade, immutable old ledger rows, rollback and future-ledger
refusal. `sqlite_lifecycle_test.exs` retains independent revision-1 and revision-2
snapshot verification. No active installed database is migrated by these tests.
See the [results and remaining gaps](../../../reports/project-create-receipts-20261012.md).

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
