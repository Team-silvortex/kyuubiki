# Storage Outage Containment And Recovery

Date: 2026-09-27. Source line: daji 3.4.0. Local development evidence only.

## Failure Mode

Workflow recovery scans, watchdog scans, and watchdog health snapshots read
their backing store directly. Temporarily removing the memory state process or
the SQL repository raised a process exit or repository lookup exception in the
caller. The watchdog could terminate, and the recovery coordinator could restart
into the same failing startup scan. The memory lease facade also propagated
lease-process absence instead of returning its documented availability error.

Before production changes, two focused watchdog/health cases failed on memory
and both failed on SQLite. The faults stopped actual supervised storage children,
not a mocked return value. They ran only against disposable test data.

## Changed Boundaries

`Storage.FailureBoundary` converts known connection/driver errors, Ecto repository
lookup failure, and availability exits from the two named memory stores into
bounded error replies. It preserves successful replies and domain errors.
Ordinary exceptions, unrelated process exits, programming crashes inside a store,
and thrown domain control flow are not swallowed. Internal error details are not
copied into the public availability reason.

The boundary wraps outside SQL transaction handling. It neither retries writes
nor assumes a timed-out write was rolled back. Existing atomic workflow/solver
publication boundaries still control multi-record commits.

Workflow recovery behavior:

- A failed list/read returns an unavailable summary, with unknown active-job
  count rather than an empty queue, and does not count a completed recovery pass.
- Read/ownership failures leave the same coordinator process alive, move it to
  standby, stop its tracked runners, and clear stale progress/activity tracking.
- The existing `lease_retry_ms` timer retries acquisition. After storage returns,
  the normal envelope, claim, attempt limit, and replay-policy checks still apply.
  There is no tight retry loop or blanket retry of non-idempotent operations.
- Idempotent running workflows can resume with a new execution generation.
  Checkpoint-required work without a valid checkpoint remains blocked, retaining
  its recovery envelope instead of being executed speculatively.
- Delayed acquire/renew messages do not discard the current heartbeat or retry
  timer reference. Retry state is kept in the existing process, not a new service.
- Recovery batch traversal is extracted into `WorkflowRecoveryScan`; the main
  coordinator stays under the 800-line source limit.

Watchdog and visibility behavior:

- A failed read or progress write does not terminate the watchdog. Scheduled
  scans continue at the configured interval after the current scan returns.
- A storage/write failure stops the current scan rather than repeatedly trying
  to fail unrelated jobs. Counters describe committed writes, not attempted ones.
  Snapshot conflicts and deleted jobs remain ordinary concurrent-write misses.
- `status_snapshot` distinguishes current readable counts from `last_scan`.
  Unreadable active/stalled/timed-out totals are `null`, not zero. A failed scan
  remains visible until a subsequent successful scan replaces it.
- `/api/health` remains HTTP 200 for a live control plane but returns
  `status: degraded` when job storage is unreadable or the latest scan failed.
  A successful read alone cannot clear a failed scan. Readiness consumers must
  inspect the payload, not infer readiness from HTTP 200 alone.
- Frontend health types accept the explicit unknown values, and Workbench job
  counters show `--` when unavailable/missing instead of inventing zero totals.

Existing configuration remains authoritative: `lease_retry_ms` defaults to
1000 ms, and `scan_interval_ms` defaults to 5000 ms. Storage call timeouts still
apply; these changes do not impose a new whole-scan deadline or paging strategy.

## Retained Tests

Twenty-five new tests cover the following boundaries on the configured backend:

- Manual and scheduled watchdog reads during real storage-child absence;
  unchanged watchdog PID and automatic scanning after restoration.
- Recovery read, dispatch, scheduled recovery, and runner-loss notification
  during storage absence; unchanged coordinator PID, honest counters, and retry.
- Progress, final publication, failure, cancellation, and initialization reject
  storage absence without changing the retained workflow/job record.
- Lease acquire, renew, release, lookup, and guarded callbacks reject an absent
  lease backend; callbacks cannot run without confirmed ownership.
- Two running workflow chains exercise ownership withdrawal: an idempotent chain
  resumes in generation/attempt 2, while a checkpoint-required chain remains in
  generation/attempt 1 and is durably blocked without another solver invocation.
- Stale lease messages preserve the current timer reference in owner and standby
  states.
- SQLite UPDATE abort and memory filesystem write faults leave two unrelated
  jobs unchanged; removal of the fault permits the watchdog to process both.
- A Plug health request remains reachable, exposes null counts/degraded state,
  retains the failed scan after reads recover, and returns healthy only after
  a successful scan.
- Boundary unit cases retain domain replies, bound SQL driver errors, and do not
  hide unrelated exceptions/exits. Timeout-shaped exits are synthetic unit
  cases, not a slow-storage or commit-ambiguity qualification.

Sources are `jobs/watchdog_outage_test.exs`,
`orchestra/workflow_storage_outage_test.exs`,
`storage/failure_boundary_test.exs`, and `api/storage_outage_health_test.exs`,
under `apps/web/test/kyuubiki_web`. Helpers are under `apps/web/test/support`.
The running workflow chains use a deliberately blocking/successful runtime
client fixture, not a physical Rust solver. The health test is in-process Plug
transport, not a live socket or desktop WebView test.

To reproduce from `apps/web`, configure a fresh disposable `KYUUBIKI_DATA_DIR`.
For SQLite, also set `KYUUBIKI_STORAGE_BACKEND=sqlite` and a disposable
`SQLITE_DATABASE_PATH`; for memory, set `KYUUBIKI_STORAGE_BACKEND=memory`.
Never target installed runtime or retained research data: tests reset stores and
deliberately stop/restart their supervised children.

```text
mix test --warnings-as-errors --seed 42 \
  test/kyuubiki_web/jobs/watchdog_outage_test.exs \
  test/kyuubiki_web/orchestra/workflow_storage_outage_test.exs \
  test/kyuubiki_web/storage/failure_boundary_test.exs \
  test/kyuubiki_web/api/storage_outage_health_test.exs
```

## Verification

Final verification with warnings treated as errors:

- Full SQLite suite: 1098 tests, zero failures, 20 backend/platform/optional skips.
- Full memory suite: 1098 tests, zero failures, 48 backend/platform/optional skips.
- New focused suite with seeds 1, 42, and 1337 on each backend: 25 tests per run,
  zero failures and no skips, using separate disposable storage for every run.
- Elixir compilation/formatting, frontend type checking, diff whitespace, and
  project organization checks pass within 800 source / 2000 documentation lines.
- Tensor self-tests, structure, and commands pass. Its maturity gate remains
  blocked: four maturity gaps, 16 evidence-grade gaps, and 11 P0 gaps.

One preliminary multi-seed retry reused the same SQLite database across BEAM
instances and produced 18 fixture-setup failures behind the previous instance's
unexpired lease. Independent per-run databases fixed that test isolation error;
no production lease checks or expiry rules were weakened. Fixture teardown is
registered before the ownership assertion so failed setup also restores its
observer configuration.

## Scope And Remaining Work

This is local storage-outage containment evidence, not distributed failover, PostgreSQL, power-loss, or numerical qualification.

Stopping/restarting a SQLite repository is not killing its database server or
corrupting its filesystem. Memory/JSON restart tests use retained snapshots;
they do not establish shared-memory lease durability or prevent every lease-token
incarnation/ABA case after the volatile lease process restarts. Sustained latency,
timeouts with uncertain commit outcomes, and cross-host partitions need separate
qualification. Cancelling a local workflow runner does not establish that a
remote Agent or external numerical process has stopped computing.

Only the selected recovery/watchdog paths and their health projection gain these
availability boundaries. General CRUD APIs and other services are not thereby
certified outage-safe. No numerical kernel, operator interface, schema, backup
history, installed data, or release version is changed. The tensor evidence is
limited to local `contract` and `recovery` for `orchestra-control-plane`; it does
not raise a remote operational grade or physical trust level.

See the [atomic workflow commit report](workflow-atomic-commit-20260927.md) for
the earlier multi-record rollback and stale-claim guarantees.
