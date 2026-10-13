# Successful workflow result publication through transient write failure

Date: 2026-10-13. Source overlay on `c2f97f47`, daji 3.6.0.

## Reproduction

After the entire graph succeeded, the runner shaped its result and called the
completion transaction once. Any non-stale-claim error immediately entered
failure finalization. With the preceding failure-retirement fix, a persistent
write fault retained a waiter, but that waiter retried failure, not the successful
result. A repaired disk could therefore leave a successfully computed workflow
marked failed without its final output.

A controlled solver completes a real admitted graph. A test-only OTP system
debug handler pauses the first final-completion call before its transaction,
after all earlier checkpoints and node progress have completed. A SQLite result
UPDATE trigger or isolated memory snapshot fault then rejects the final write.
The probe releases automatically if its test owner exits and is removed on cleanup;
there is no production fault-injection hook or runtime dependency.

Seed 68205 reproduced the missing successful-commit retry: one test, one failure.
The production runner retried failure instead. An earlier probe-cleanup defect
was repaired before this clean red run and is not counted as a product defect.

## Repair

- The existing runner-local failure wait is now named `await_terminal_commit` and
  also wraps successful final publication. Shape the output once, retain it in the
  runner and retry only the original commit after selected transient errors. No
  solver, operator, graph execution or result shaping is repeated by this waiter.
- Every attempt invokes the existing joint job/result transaction. Current job,
  runtime snapshot, execution generation/attempt/owner session and Orchestra lease
  are checked again. A stale runner cannot overwrite newer work, another terminal
  decision or a deleted job. Already terminal jobs end the old completion waiter
  without attempting to rewrite them as failed.
- Recognized coordinator call exits use the existing unavailable response and
  retry policy. A brief coordinator restart can preserve pending successful output
  without redispatch. Actual authority loss continues to stop tracked runners.
- JSON encoding, unsupported encoding protocols, Ecto schema errors and Ecto query
  cast/dump errors receive the shared nested `invalid_payload` classification on
  both storage backends. The outer `completion_persistence_failed` tag remains.
  This classification is non-retryable, rather than guessed from exception text.
  Its stable bounded message omits result contents and query details. Other write
  failures retain the existing error shape. No extra JSON pass/copy is added to
  the successful hot path.
- This changes only final publication after a successful graph. Earlier native,
  solve and branch checkpoint rejection still blocks the graph; it must not be
  reinterpreted as successful output merely because one solver returned a value.

No new production module, scheduler, disk backup, storage schema, dependency,
solver algorithm, Agent protocol or engine/operator coupling was introduced.

## Verification

Ten new runner integration cases use real local admission, graph execution,
coordinator and persistence with a controlled solve client. A separate shared
error-classification test was added, and existing workflow/plain-job encoding
rollback tests now require the non-retryable classification.

- Focused SQLite regression, seed 68207: 73 tests, zero failures, 21.0 seconds,
  warnings denied. Includes prior failure/deadline retirement and atomic completion.
- Rejected writes preserve both durable snapshots and the original runner.
  A retry carries the same claim and complete shaped result, including small
  floating-point values, lineage and execution metrics. After repair, publication
  succeeds with the original generation/attempt and no second solver dispatch.
- Coordinator restart preserves that output. Competing completed, failed and
  cancelled joint storage decisions retain their own result unchanged. These are
  terminal-store interleavings, not end-to-end user-cancellation qualification.
- Generation, attempt and owner-session changes reject old publication. Existing
  recovery can create a newer runner and reuse the already committed solve
  checkpoint. Its completed generation proves the old finalizer did not publish;
  checkpoint reuse avoids a second solver dispatch.
- Lease takeover stops the waiter without publishing. Deletion during retry does
  not recreate either job or result. Existing unencodable-result tests confirm
  rollback and successful subsequent valid publication on the same job.

- Expanded SQLite regression, seed 68208: 3,304 total, 3,227 passed, 77 conditional
  skips, zero failures, 82.2 seconds, warnings denied. In addition to the prior
  workflow/API/Agent/storage selection, this run includes result administration
  because both shared persistence paths now classify known invalid payloads.
- Isolated memory/json regression, seed 68209: 862 total, 765 passed, 97 conditional
  skips, zero failures, 46.1 seconds, warnings denied. Covers Orchestra workflow,
  storage, jobs and result administration; it is not the same selection as SQLite.
- All ten new runner cases and the error-classification test ran on both backends.
  The runner cases skip live PostgreSQL, which was not run here.

Formatting, whitespace, documentation inventory, the 26-file HTML book, tensor
validation/self-test and project organization checks passed. The shared runner
is 314 lines, the new runner tests 222 lines and the test-only probe 52 lines.
Source/document limits remain 800/2,000 with zero tracked debt. Tensor evidence
was extended without a maturity promotion: zero structural gaps, four maturity
gaps, 20 evidence-grade gaps and 15 P0 gaps; the release gate remains blocked.
The isolated memory-test directory was removed. No native build, installation,
version bump, commit or push was made.

Expanded SQLite reproduction from `apps/web`:

```text
mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api \
  test/kyuubiki_web/orchestra/workflow_*test.exs \
  test/kyuubiki_web/orchestra/operator_task*test.exs \
  test/kyuubiki_web/orchestra/operator_dispatch*test.exs \
  test/kyuubiki_web/orchestra/headless_native_recovery_gate_test.exs \
  test/kyuubiki_web/orchestra/distributed_recovery*test.exs \
  test/kyuubiki_web/playground test/kyuubiki_web/library \
  test/kyuubiki_web/storage test/kyuubiki_web/jobs test/kyuubiki_web/results \
  --seed 68208 --warnings-as-errors
```

The memory run sets `KYUUBIKI_STORAGE_BACKEND=memory` and a fresh temporary
`KYUUBIKI_DATA_DIR`, then selects `test/kyuubiki_web/orchestra/workflow_*test.exs`,
`test/kyuubiki_web/storage`, `test/kyuubiki_web/jobs` and `test/kyuubiki_web/results`
with seed 68209.

## Follow-up

This report records the earlier in-memory retry increment. The subsequent
[durable publication checkpoint](workflow-completion-checkpoint-20261013.md)
adds a separate staging commit and actual independent-BEAM SIGKILL coverage.
The historical test counts in this report are not new power-loss qualification.
Changed pending claims now block rather than falling back to a new graph claim.

## Limits (Original Increment)

Pending success remains in runner memory until a successful commit. This is not
a durable outbox, crash-proof final-result handoff or exactly-once execution.
Runner/process loss before commit still uses existing recovery policy and bounded
checkpoints. Watchdog policy is unchanged and can win during a long write outage.
The live result remains resident during retries; no new large-result memory cap,
retry-backlog budget or large-scale latency qualification is claimed.

Local terminal-commit waiting still retries at 250 ms while selected conditions
persist; it has no finite retry count independent of claim/lease/watchdog fencing.
The cancellation-delivery retry budget is separate and unchanged. A call timeout
is not proof of rollback; another attempt must still pass the active-job/claim
transaction, so already committed or competing terminal state cannot be replaced.
Actual 30-second lost-ack timing and physical process-loss experiments are not
part of this test increment.

Known payload errors are classified, not a complete taxonomy of every possible
backend failure. Live PostgreSQL, remote Rust solver execution, installed desktop,
power loss and large-scale performance are not qualified by these local source
tests. Tensor maturity and release gaps remain open.
