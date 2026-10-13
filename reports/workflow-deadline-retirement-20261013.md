# Durable workflow runner deadline retirement

Date: 2026-10-13. Source overlay on `c2f97f47`, daji 3.6.0.

Checkpoint note: the later [failure persistence follow-up](workflow-failure-retirement-20261013.md)
adds selected terminal-write retries for ordinary and checkpoint-blocked failures.
The deadline results and scope below describe this earlier checkpoint; the later
change does not add ordinary-failure cancellation delivery.

## Reproduction

The prior [durable watchdog checkpoint](workflow-durable-cancellation-20261013.md)
covered periodic scans, not the execution runner's own deadline. That runner
captured a live cancellation snapshot, killed its linked local transport, sent
best-effort cancellation and only afterwards tried to save workflow failure.
An interruption or rejected failure write could therefore lose its request identity
without retaining a cancellation intent.

The native pipeline regression now requires a saved original RPC ID at deadline
retirement. On the old path, seed 68193 selected this case and failed because
`cancellation_intent` was absent: one test, one failure, ten excluded.

## Changed behavior

- The runner now requests a dedicated claim-checked expiry transaction. The job
  must still be active, the running recovery claim must match generation, attempt
  and owner session, and the current Orchestra lease must authorize the write.
- The existing failure finalizer retains uncertain native evidence and captures
  the gate read-only. Failure and bounded cancellation intent commit together,
  reusing the watchdog's schema, storage, observation and exact-target delivery.
  The caller is not killed inside the coordinator callback. After the commit
  returns, the runner fences local reservations and stops its own linked task.
- Selected write/snapshot conflicts, unavailable gate and unavailable coordinator
  responses retry after 250 ms while retaining the waiter. Each request re-reads
  current state and authority, rather than blindly replaying an unchecked write.
  There is no finite retry count for this local commit wait while these conditions
  persist. The separate remote-delivery budget remains three reserved attempts.
- A deadline remains a deadline once observed. Late task output is discarded at
  the parent runner and cannot publish successful parent completion while failure
  persistence is pending. Earlier completion returned by the initial timed wait
  retains the ordinary completion path.
- Terminal jobs and stale claims cannot be rewritten by expiry. A rejected or
  unowned runner stops only its local task, not arbitrary Agent work sharing the
  same job ID. Actual storage/lease unavailability still uses the existing
  ownership-loss fencing, which may stop tracked runners; it is not treated as a
  harmless retry permitting an unowned computation controller to continue.
- A committed intent survives loss before delivery. Startup/explicit recovery
  sends only its original request and bound execution generation. A later
  same-job dispatch is not added. Cancellation acknowledgements neither settle
  uncertain native dispatch evidence nor authorize solver replay.

Ordinary failure and explicit user cancellation retain their existing behavior.
No new scheduler, database, migration, backup, runtime dependency or engine/operator
coupling is introduced. Coordinator additions are delegation only; its source
remains below the 800-line limit.

## Verification

Added 13 deadline-specific tests and one storage-outage operation case, and
strengthened the existing native deadline pipeline test. All tests use local
source services and controlled TCP Agents, not physical Rust solver execution.

- Focused combined SQLite regression, seed 68196: 74 tests, zero failures,
  9.5 seconds, warnings denied.
- Expanded SQLite selection, seed 68198: 3,239 total, 3,166 passed, 73
  conditional skips, zero failures, 84.4 seconds, warnings denied.
- Isolated memory selection, seed 68197: 221 total, 196 passed, 25 conditional
  skips, zero failures, 13.6 seconds, warnings denied. Its selection is not
  identical to the preceding checkpoint's memory selection.
- Delivery is deliberately occupied while a real managed runner expires. The
  failed parent retains a pending original-request intent after the local runner
  exits. A later same-job request is then started; coordinator restart cancels
  only the old request and the later request returns normally.
- SQLite-trigger and memory-file write faults cross the configured deadline.
  While writes are rejected, the waiter and original gate reservation remain
  alive, the parent remains running and no cancellation is sent. Removing the
  fault permits failed retirement and exact cancellation. A separate late-success
  case finishes transport during the write fault and still cannot complete the
  parent successfully after storage repair.
- Changed claim generation, attempt and owner session are rejected before
  snapshot/cancellation effects. Terminal completed/failed/cancelled jobs remain
  unchanged. Unavailable gate, unavailable coordinator, unavailable persistence
  and revoked lease cases retain explicit failures rather than invented success.
- A stale actual runner stops without sending cancellation. The existing restart
  guard subsequently marks the uncertain native dispatch recovery-blocked while
  retaining the newer generation and unchanged dispatch evidence, rather than
  permitting an old timeout to rewrite it or restarting a solver.

Formatting, whitespace, tensor validation/self-test, documentation inventory,
the 26-file HTML book and project organization checks passed. The old tensor
anchor for stop-before-persist was updated to the claim-checked expiry entry;
historical checkpoint results remain unchanged. Tensor maturity still reports
four maturity gaps, 20 evidence-grade gaps and 15 P0 gaps, with zero structural
gaps and the release gate blocked. Source/document limits remain 800/2,000 lines
with zero tracked debt. The isolated memory-test directory was removed. No native
binary was rebuilt or installed, and no version bump, commit or push was made.

Reproduce the expanded SQLite selection from `apps/web`:

```text
mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api \
  test/kyuubiki_web/orchestra/workflow_*test.exs \
  test/kyuubiki_web/orchestra/operator_task*test.exs \
  test/kyuubiki_web/orchestra/operator_dispatch*test.exs \
  test/kyuubiki_web/orchestra/headless_native_recovery_gate_test.exs \
  test/kyuubiki_web/orchestra/distributed_recovery*test.exs \
  test/kyuubiki_web/playground test/kyuubiki_web/library \
  test/kyuubiki_web/storage test/kyuubiki_web/jobs \
  --seed 68198 --warnings-as-errors
```

## Remaining limits

Failed commits cannot establish durable intent. A transport/owner lost before a
successful capture can still leave an unidentified remote execution; dispatch-time
durable execution identity and Agent orphan reconciliation are separate work.
Preserving the waiter during a write fault is not proof that the remote solver has
stopped or that no downstream local work ran before durable fencing. The guarantee
here is no late successful parent publication, not distributed atomic cessation.

The inherited sweep remains bounded: 16 pending jobs, a 15-second delivery-start
budget and no queue for concurrent kicks. Remaining intents need another startup
or explicit recovery sweep. No continuously draining cancellation service, automatic
ordinary-failure/user-cancel upgrade, historical repair, exactly-once execution,
remote termination proof or operator-package cleanup is claimed.

This is not live PostgreSQL, physical power-loss, installed desktop, remote Linux,
large-scale cancellation-latency or numerical qualification. The broader release
and distributed-recovery maturity gaps remain open.
