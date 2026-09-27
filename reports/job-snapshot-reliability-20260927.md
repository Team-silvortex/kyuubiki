# Job Snapshot Reliability

Date: 2026-09-27. Source line: daji 3.4.0. Local development evidence only.

## Reproduced Failures

The initial eight deterministic SQL interleaving tests all failed before this
repair. Progress validation used one job snapshot, but a second read was used
to write every field from that older snapshot. Consequences included:

- Completed, failed, or cancelled jobs could become active again.
- Concurrently increased progress could be replaced by a lower value.
- Worker assignment could revert a completion and its solver diagnostics.
- Progress and terminal replays could overwrite newer metadata or ownership.
- Watchdog decisions could fail a job after a same-progress heartbeat arrived.

Additional boundary tests found that whole-second timestamps accepted by the
domain model raised during SQL encoding into `utc_datetime_usec` columns.

## Write Contract

The shared SQLite/PostgreSQL implementation now uses a conditional SQL UPDATE
matching every persisted field from the snapshot used for validation, including
nullable values, ownership, metadata, timing budgets, and solver diagnostics.
Matching only `updated_at` is insufficient: two events can have the same time.
Only changed fields are written. No schema migration or backup copy is needed.

Ordinary progress, metadata, and worker assignment get at most four attempts.
Each conflict rereads and revalidates the current job before trying again.
Progress events are parsed once, so an implicit timestamp is never refreshed
during retries. Remaining contention returns `{:job_write_conflict, job_id}`;
deletion detected on reread returns `{:job_not_found, job_id}`. Neither inserts a
replacement job. An unchanged terminal replay performs no update.

`Store.apply_progress_if_current/2` is deliberately different: the decision is
bound to the supplied snapshot and is never rebased onto newer work. A mismatch
returns `{:stale_job_snapshot, job_id}` (or not-found after deletion). The memory
backend performs the comparison and mutation within one Agent callback.

The watchdog uses this conditional API. A heartbeat, terminal receipt, worker or
metadata change, or queued-to-running transition invalidates its old decision.
Only a successful conditional write contributes to timeout/stall counters.
Skipped jobs are assessed again on the next scan rather than failed using stale
evidence. A deleted job does not crash the watchdog or prevent another stale job
from being processed.

Shared domain helpers validate required metadata before persistence and keep
worker/metadata mutation timestamps at least as new as the existing job clock.
SQL encoding preserves whole-second instants while providing the six-digit
precision required by Ecto. This does not correct clock skew between machines.

## Regression Coverage

- `progress_concurrency_test.exs`: 19 SQL tests, including synchronous telemetry
  interleavings after SELECT, equal-time progress/stage changes, completion
  races, retained metadata, bounded retry exhaustion, and 16 concurrent writers.
- `progress_race_test.exs`: two deletion races, including watchdog survival,
  honest counters, and a subsequent healthy job.
- `snapshot_test.exs`: ten backend-independent tests, including eight competing
  conditional writers with exactly one winner, stale/deleted snapshots,
  identity validation, invalid metadata, and timestamp precision/order.
- Existing job/domain/watchdog and application tests remain in the regression.

Query instrumentation is test-only. It changes storage between a SELECT and
the caller's write, without sleeps or production fault-injection hooks. Both
the returned receipt and the persisted state are checked. The retry-limit test
keeps changing metadata at the same timestamp to reject time-only locking.

Executed locally on macOS using disposable SQLite and memory/JSON directories:

- Full Web suite: 992 tests, zero failures, eight existing skips.
- Focused SQLite job suite: 49 tests, zero failures for each seed 1, 42, and 1337.
- Memory job/domain/store/snapshot/watchdog suite: 28 tests, zero failures, one
  existing SQL-only foreign-key test skipped.
- Compilation and the above test runs pass with warnings treated as errors.
- Formatting, diff whitespace, and the 800-line source / 2000-line documentation
  organization audit pass. The coverage tensor structure and command checks pass;
  its overall maturity gate remains blocked, not promoted by these local tests.

For a focused run, from `apps/web`, set `SQLITE_DATABASE_PATH` to a database in a
new disposable directory, set `KYUUBIKI_STORAGE_BACKEND=sqlite`, then run
`mix test test/kyuubiki_web/jobs`. For memory parity, set `KYUUBIKI_DATA_DIR` to a
different disposable directory and `KYUUBIKI_STORAGE_BACKEND=memory`; run
`job_test.exs`, `store_test.exs`, `snapshot_test.exs`, and `watchdog_test.exs` from
that same test directory. These suites reset job/result stores: never point them
at an installed runtime or a database containing work to retain.

## Evidence Boundary

This is bounded local job-snapshot evidence, not PostgreSQL or distributed recovery qualification.

The SQL implementation is shared, but this run did not start a PostgreSQL
instance or exercise remote network partitions, process loss, sustained load,
or whole-workflow exactly-once execution. Result artifacts and job status are
not made one cross-store transaction by this change. Arbitrary CRUD races and
delete/recreate of an identical job id and snapshot are outside this contract;
the snapshot is not a persistent incarnation/revision token. The changes do
not qualify physical solvers or eliminate all possible storage exceptions.

This follows the deleted-row crash fix in
[the named-input report](workflow-named-input-contract-20260927.md). That
report's earlier stale-changeset mechanism is superseded by conditional UPDATE;
its recorded test results and limitations remain historical evidence.
