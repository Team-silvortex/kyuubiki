# Workflow failure persistence without unintended replay

Date: 2026-10-13. Source overlay on `c2f97f47`, daji 3.6.0.

Checkpoint note: the later [successful final-publication follow-up](workflow-completion-retry-20261013.md)
extends this live-runner wait to whole-graph completion. Results and limits below
describe the earlier failure-only checkpoint; rejected intermediate checkpoints
still cannot be promoted to success.

## Reproduction and scope

The graph runner executes ordered nodes sequentially. This change does not assume
parallel branch execution or certify concurrent branch shutdown.

Ordinary failure ignored every rejected terminal write. Native, solve and branch
checkpoint blocking paths also returned immediately after a rejected write. The
registered runner then exited normally while durable recovery still said running.
Its coordinator processed the exit as runner loss, allowing a later recovery scan
to reclaim/replay the job under its existing policy instead of retaining the known
failure. During persistent storage rejection, recovery could instead leave the
job active with no runner. Neither behavior preserved the pending failure reason.

Four red integration regressions, seed 68199, reproduced premature normal runner
exit for ordinary failure and the three blocking paths: four tests, four failures.
The failure was observed before the injected write rejection was removed.

## Repair

- All four paths retain the registered runner and original reason while selected
  persistence/snapshot conflicts retry after 250 ms. The small runner-local helper
  reuses the existing deadline retry classification. It does not create another
  scheduler, background service, storage schema or module.
- Every attempt calls the existing coordinator transaction again with the original
  claim. Job status, generation, attempt, owner session, snapshot and Orchestra
  lease are rechecked. Only terminal persistence retries; computation does not.
- Recognized coordinator call exits become unavailable responses so a brief
  coordinator restart need not lose the waiter. Other exits are not swallowed.
- Competing terminal decisions and changed claims stop the stale finalizer.
  Actual lease/storage authority loss still uses existing ownership fencing and
  may shut down tracked runners. It is not classified as permission to continue
  writing without an owner.
- There is no finite retry count for this local wait while the selected retryable
  conditions persist and the runner survives. External watchdog policy remains
  in force. The separate cancellation delivery budget remains unchanged.
- A successful solve whose checkpoint commit failed remains recovery-blocked;
  its uncommitted output is neither published nor silently re-executed by this
  waiter. Native and branch block reason classes are also preserved.

No solver algorithm, Agent protocol, engine/operator boundary, public user-cancel
behavior, dependency or version was changed. Ordinary-failure cancellation delivery
is still separate work.

## Verification

Thirteen new runner integration cases use the real admission, graph execution,
coordinator and persistence services with a controlled solve client. The three
block-reason routing cases inject structured client errors; an additional case
causes an actual successful-solve checkpoint write to fail. They do not execute
a physical Rust solver or simulate numerical convergence.

- The initial four regression cases pass after the repair, seed 68200.
- Focused SQLite selection, seed 68201: 59 tests, zero failures, 15.7 seconds,
  warnings denied. Includes deadline, native evidence retention and storage outage.
- SQLite result-update triggers and isolated memory persistence faults exercise
  rollback. During rejection, the original runner remains registered and an
  explicit recovery scan does not redispatch its solver.
- Clearing the fault retains the original generation/attempt and failure class.
  Restart after committed retirement cannot re-execute the solver.
- Generation, attempt and owner-session replacement each fence the waiting old
  finalizer. Authorized runner-loss recovery can start new work, which succeeds
  without inheriting the stale failure.
- Competing completed/failed/cancelled joint job/result commits remain unchanged
  through the stale waiter's exit and a coordinator restart. These simulate
  terminal storage decisions, not end-to-end user-cancellation delivery.
- A brief coordinator restart retains the waiter and original failure without
  solver redispatch. Lease takeover instead stops the waiter and leaves both
  durable snapshots unchanged.

- Expanded SQLite selection, seed 68202: 3,252 total, 3,179 passed, 73 conditional
  skips, zero failures, 81.8 seconds, warnings denied. Covers workflow operators,
  public API, native task/dispatch paths, recovery, Agent clients, library, jobs
  and storage.
- Isolated memory/json selection, seed 68203: 810 total, 721 passed, 89 conditional
  skips, zero failures, 41.9 seconds, warnings denied. Covers all Orchestra
  workflow tests, jobs and storage. Its selection differs from the SQLite run.
- All thirteen new cases ran on both storage backends; their only conditional
  skip is live PostgreSQL, which was not run here.

Formatting, whitespace, documentation inventory, the 26-file HTML book, tensor
validation/self-test and project organization checks passed. The runner is 308
lines and the new test file 245 lines; source/document limits remain 800/2,000
with zero tracked debt. The tensor adds scoped contract/recovery evidence, not a
maturity promotion: zero structural gaps, four maturity gaps, 20 evidence-grade
gaps and 15 P0 gaps; the release gate remains blocked. The isolated memory-test
directory was removed. No native build, installation, commit or push was made.

Expanded SQLite reproduction from `apps/web`:

```text
mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api \
  test/kyuubiki_web/orchestra/workflow_*test.exs \
  test/kyuubiki_web/orchestra/operator_task*test.exs \
  test/kyuubiki_web/orchestra/operator_dispatch*test.exs \
  test/kyuubiki_web/orchestra/headless_native_recovery_gate_test.exs \
  test/kyuubiki_web/orchestra/distributed_recovery*test.exs \
  test/kyuubiki_web/playground test/kyuubiki_web/library \
  test/kyuubiki_web/storage test/kyuubiki_web/jobs \
  --seed 68202 --warnings-as-errors
```

The memory run sets `KYUUBIKI_STORAGE_BACKEND=memory` and a fresh temporary
`KYUUBIKI_DATA_DIR`, then selects `test/kyuubiki_web/orchestra/workflow_*test.exs`,
`test/kyuubiki_web/storage` and `test/kyuubiki_web/jobs` with seed 68203.

## Limits

The pending failure lives in the runner until a successful commit. If that runner
or the entire process is lost first, the old recovery policy still applies; this
is not a durable terminal-intent log or exactly-once execution guarantee. A watchdog
decision may still retire a stalled job. This change does not bypass watchdogs,
retain released Agent request identities, or prove remote execution termination.

Successful parent-completion retry semantics and generalized durable failure or
user-cancel delivery are not upgraded. Live PostgreSQL, installed desktop,
physical power loss, remote Linux and large-scale retry performance are not
qualified by these local tests. Tensor maturity and release gaps remain open.
