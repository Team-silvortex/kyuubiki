# Atomic Solver Publication Reliability

Date: 2026-09-27. Source line: daji 3.4.0. Local development evidence only.

## Defect And Contract

The asynchronous solver runner previously checked whether a job was terminal,
assigned its worker, saved the result, and applied completed progress separately.
Cancellation after the check could leave a new result on a cancelled job. A
failure between those writes could leave a result without completion, or change
worker ownership even when publication failed. Individual job compare-and-swap
did not make that multi-record sequence atomic.

The runner now calls `Store.complete_with_result/3`. Its publication boundary:

- Publishes the worker, JSON result object, and completed progress together.
- Rejects missing or terminal jobs without changing any stored fields.
- Refuses to replace an already stored result, including duplicate completions.
- Preserves iteration, residual, execution origin, and job metadata; the final
  timestamp does not regress behind an accepted future progress timestamp.
- Returns a storage error on failed publication. The runner can then record a
  failure, if the job is still active and storage remains available.

This is first-writer-wins publication, not exactly-once computation. A duplicate
submission receives `job_already_terminal`; callers can read the retained receipt.
An existing active-job result produces `result_already_exists`, not silent replacement.

## Storage Implementation

SQLite and PostgreSQL use a single Ecto transaction. SQLite reserves its writer
before reading (`mode: :immediate`). The conditional job update validates the
whole snapshot and serializes competing terminal transitions; the result insert
is in that same transaction. Errors roll back job fields and worker assignment.
There is no schema migration or compensating result deletion.

The lightweight memory/JSON backend now has one supervised `AnalysisMemoryState`
Agent and one digest-verified generation containing jobs and results. The public
job and result facades still exist, but no longer own separate memory processes.
One write commits both collections; a failed write leaves live state unchanged.
Strict state equality preserves integer-to-float result changes.

The shared file is `analysis-state.json` inside `KYUUBIKI_DATA_DIR`, with payload
schema `kyuubiki.analysis-memory-state/v1`. It uses the existing bounded `.next`,
`.previous`, `.corrupt`, and `.recovery.json` persistence mechanism, not an
unbounded history. A corrupt or missing primary may recover the previous whole
generation, never independently mix a job generation with a result generation.

On first startup only, legacy `jobs.json` and `results.json` are imported. They
are not rewritten or removed automatically; this avoids destructive migration
of existing data. Once shared-state artifacts exist, damaged shared state must
not reimport stale legacy copies. Unrecoverable shared/legacy state fails startup
instead of silently inventing an empty database. Legacy data inconsistencies
are not automatically repaired or certified by migration.

This remains a small-data development backend: a changed snapshot serializes
both collections. It is not a large-result performance optimization or a
multi-process/multi-host JSON database; production SQLite/PostgreSQL retain
row-level storage. No installed runtime data was migrated during this run.

## Regression Evidence

The new tests cover:

- Completion receipts, all three pre-existing terminal states, missing jobs,
  malformed arguments, duplicate results, and retained diagnostics.
- Result encoding failure, an injected SQLite insert abort, and clean retries.
- Eight competing publishers: exactly one matching worker/result wins.
- Concurrent failure/cancellation versus completion: one terminal winner, no
  newly published result when failure/cancellation wins.
- A real publisher process killed after its job UPDATE but before result INSERT:
  the transaction rolls back and another publisher can complete cleanly.
- A blocked in-flight SQL commit followed by cancellation cannot partially commit.
- Memory restart, legacy migration, corrupt primary recovery, an interrupted
  rename boundary, unrecoverable generations, and an injected filesystem failure.
- HTTP/TCP final result rejection reports failure and a subsequent solver request
  completes normally. Scripted TCP Agents are transport fixtures, not Rust solvers.

Sources are `jobs/atomic_completion_test.exs`, `jobs/atomic_completion_sql_test.exs`,
`storage/analysis_memory_state_test.exs`, and the extended
`api/solver_completion_receipt_api_test.exs`, under `apps/web/test/kyuubiki_web`.
Fault probes and delayed Agents live under `apps/web/test/support`; production
code has no test-only timing callbacks.

Final results with warnings treated as errors:

- Full SQLite suite: 1038 tests, zero failures, 16 skips (eight existing,
  eight memory-only).
- Full memory suite: 1038 tests, zero failures, 37 backend/platform/optional skips.
- Focused publication suite, seeds 1, 42, and 1337: 37 tests per seed/backend,
  zero failures; SQLite skips eight memory-only cases, memory skips three SQL
  cases and one SQLite-only API fault case.
- Compilation, formatting, diff whitespace, and the 800-line source / 2000-line
  documentation audit pass. Tensor structure/commands pass; its maturity gate
  remains blocked (four maturity gaps, 16 evidence-grade gaps, 11 P0 gaps).

To reproduce from `apps/web`, use a fresh disposable SQLite database via
`KYUUBIKI_STORAGE_BACKEND=sqlite` plus `SQLITE_DATABASE_PATH`, or a fresh disposable
directory via `KYUUBIKI_STORAGE_BACKEND=memory` plus `KYUUBIKI_DATA_DIR`, then run:

```text
mix test --warnings-as-errors test/kyuubiki_web/jobs/atomic_completion_test.exs test/kyuubiki_web/jobs/atomic_completion_sql_test.exs test/kyuubiki_web/storage/analysis_memory_state_test.exs test/kyuubiki_web/api/solver_completion_receipt_api_test.exs
```

Tests reset shared stores; never target installed runtime or retained research data.

## Scope And Remaining Work

This is local atomic solver-publication evidence, not PostgreSQL, distributed recovery, or numerical qualification.

PostgreSQL uses the same SQL implementation but was not run against a live
PostgreSQL service here. SQLite process-loss tests kill the publishing process,
not the database server or host. Memory rename/corruption tests exercise coherent
generation recovery, not hardware power-loss durability or filesystem barriers.
Recovering a previous generation can lose the latest completion as a whole; it
does not promise that computation will never be repeated.

Only asynchronous solver finalization uses the new joint publication API in this
change. Workflow checkpoint/lease commits, general job/result CRUD, ABA reuse of
a job ID, and consistent snapshots across multiple separate reader calls remain
separate work. Persistent storage outage can prevent writing a failure receipt,
but must not be reported as successful completion. No numerical trust level or
remote operational grade is promoted by this evidence.

Follow-up: the [workflow commit report](workflow-atomic-commit-20260927.md) extends
the joint boundary to existing workflow runtime/recovery records. The counts and
scope above describe the earlier solver-only checkpoint, not that later change.
