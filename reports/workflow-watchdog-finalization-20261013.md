# Periodic watchdog workflow finalization

Date: 2026-10-13. Scope: periodic queue/execution timeout and stalled-job decisions.

Follow-up: [captured-request cancellation](workflow-scoped-cancellation-20261013.md)
replaces automatic job-wide delivery with observed request/process/generation
targets. The original results below remain this checkpoint's evidence; durable
restart compensation is still open.

## Reproduced failures

The periodic watchdog previously updated only the job row. Its workflow recovery
record remained `running`, and a stalled local runner could continue waiting even
though the public job had failed. A native task already accepted by an Agent was
left with unresolved evidence but no matching blocked recovery state.

Four red tests (seed 68171, 13 total) demonstrated this: direct local workflow
expiry, a held runner, and controlled native TCP stall/execution-timeout paths.
Each expected failed or blocked recovery but observed `running` instead.

Expanded fault injection found a second bug: a database repo may disappear from
Ecto's registry between name resolution and ETS lookup. The resulting
`ArgumentError` escaped the storage boundary and restarted the watchdog during
repo recovery. A deterministic dead-repo-PID test reproduced it (seed 68176,
10 tests, one failure); unrelated ETS errors are explicit negative controls.

## Changes and authority

`Jobs.Watchdog` submits its exact observed job to the recovery coordinator.
Retirement requires the current Orchestra lease and an unchanged full job snapshot.
For workflows, the recovery envelope and dispatch evidence are checked before
failure and the job/recovery states commit in one transaction. Unresolved native
attempts retain `recovery_blocked` and their original evidence. Pure local and
confirmed native failures retain normal cleanup. Queued pending work ends without
inventing an execution generation.

Fresh heartbeats, changed metadata, concurrent terminal jobs or already terminal
recovery records cannot be rewritten by a stale scan. Malformed private recovery
values are not treated as plain jobs. Legacy/non-workflow result maps participate
in the same conditional transaction without altering their payloads; jobs with
no result retain their conditional job update. Plain-job retirement now also
requires coordinator availability and authority. Standby/unavailable states report
an unavailable scan with bounded reasons, not successful failure counts.

Only a successful durable retirement permits local runner shutdown. Actual Agent
targets are captured before losing local dispatch ownership, then cancellation is
requested asynchronously. A rejected database write leaves job and recovery
unchanged and the runner alive; after repairing storage, a new scan succeeds.
Cancellation acknowledgement is not execution termination. A separate request on
the same controlled Agent remains alive and completes normally, and late replies
cannot publish results into the failed parent.

SQL snapshot-only writes now use the existing atomic-change boundary too. This
rolls back rejected writes as persistence failures rather than misclassifying them
as lease failures. The repo-restart exception guard matches the Ecto registry
lookup stack and PID precisely; direct/unrelated ETS and programming errors still
raise. No new protocol, backup, dependency, storage bucket or migration is introduced.

## Verification

Added 20 tests: 15 watchdog finalization cases, two controlled TCP pipelines and
three storage-boundary checks. Existing snapshot concurrency, storage outage,
native evidence retention and targeted cancellation regressions remain included.

- Focused SQLite, seed 68177: 172 total, 170 passed, two conditional skips, zero
  failures, warnings denied.
- Focused memory, seed 68178: 172 total, 147 passed, 25 conditional skips, zero
  failures, warnings denied, in an isolated temporary data directory.
- Both native pipeline cases restart the journal/coordinator, retain the original
  blocked evidence, and observe no new execution request. These are actual local
  service components with controlled TCP fixtures, not Rust Agent calculations.

Expanded SQLite regression (seed 68179, max_cases 16): **3,190 total, 3,117 passed,
73 conditional skips, zero failures**, 59.4 seconds, warnings denied. This covers
workflow, API, native/batch/dispatch recovery, headless recovery admission,
distributed recovery, Playground, Library, Storage and Jobs selections, not the
entire repository test inventory. The earlier intermittent lease-admission failure
did not recur in this run; its root cause is still not claimed resolved.

Formatting, whitespace, tensor validation/self-test, the 26-file HTML book,
documentation inventory and project organization checks passed. Source and document
limits remain 800 and 2,000 lines, with no tracked debt; the coordinator is 790
lines. Tensor maturity is unchanged: zero structural gaps, four maturity gaps,
20 evidence-grade gaps, 15 P0 gaps, release gate blocked. The isolated memory
test directory is removed after validation; no release is built or installed.

An intermediate run also exposed a single-row SQL rejection being flattened into
a lease error after retirement was moved under ownership. The atomic-change
boundary fixes this and preserves the existing immediate repaired-retry test;
no timing relaxation or retry assertion was substituted.

Reproduce the focused selection from `apps/web`:

```text
mix test test/kyuubiki_web/orchestra/workflow_watchdog_failure_test.exs \
  test/kyuubiki_web/orchestra/workflow_native_dispatch_pipeline_test.exs \
  test/kyuubiki_web/orchestra/workflow_native_failure_retention_test.exs \
  test/kyuubiki_web/orchestra/workflow_storage_outage_test.exs \
  test/kyuubiki_web/jobs test/kyuubiki_web/storage/failure_boundary_test.exs \
  test/kyuubiki_web/playground/agent_job_cancellation_test.exs \
  --seed 68177 --warnings-as-errors
```

Memory coverage sets `KYUUBIKI_STORAGE_BACKEND=memory` and an isolated
`KYUUBIKI_DATA_DIR`. No installed services or user research data are used.

## Remaining limits

- Historical failed-job/running-recovery mismatches are not silently migrated.
  Existing terminal recovery records are not rewritten as new watchdog failures.
- Retirement commit and external cancellation are not one atomic operation.
  Durable cancellation delivery, acknowledgement persistence and proof of remote
  termination remain open. A process loss after commit can still interrupt delivery.
- An unknown native result is retained and blocked, not reconstructed or replayed.
  Exactly-once execution and atomic Agent-to-database handoff remain unqualified.
- This is local SQLite/memory and controlled TCP evidence, not new live PostgreSQL,
  Rust Agent, remote Linux, installed desktop, numerical accuracy or physical
  power-loss acceptance. Earlier intermittent lease-admission failures are not
  claimed fixed by this unrelated deterministic registry-race repair.
- Module maturity and release readiness are not promoted on this basis.
