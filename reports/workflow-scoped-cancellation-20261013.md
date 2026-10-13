# Captured-request workflow cancellation

Date: 2026-10-13. Source overlay on `c2f97f47`, daji 3.6.0.

Checkpoint note: the later [durable watchdog intent follow-up](workflow-durable-cancellation-20261013.md)
adds bounded restart compensation for periodic watchdog retirement only. Results
and limits below describe this earlier captured-request checkpoint.

## Reproduction and scope

Before adding restart compensation, inspection found that workflow deadline and
periodic watchdog cleanup still sent `cancel_job`. The captured snapshot retained
only endpoints, not the original requests. A delayed job-wide cancellation could
affect a later execution sharing the same job ID or create a pending cancellation
on an Agent which no longer had the original request.

Seed 68180 reproduced five failures in 19 tests: two missing-request-identity
snapshot assertions and three controlled TCP paths which expected scoped
`cancel_execution` but received `cancel_job`. The latter cover runner deadline,
periodic execution timeout and periodic stall retirement.

## Implementation

- The existing capacity gate captures original RPC IDs with their endpoint and
  job identities before local owner shutdown. It still cancels queued/reserved
  work locally and does not prematurely release dispatched capacity.
- Workflow automatic cancellation uses only that frozen request set. It checks
  that each original endpoint identity/address/session is still configured,
  observes that endpoint once, and matches both request ID and job ID. The exact
  observed process instance and generation become the `cancel_execution` target.
- Later requests, including another request with the same job ID, are not added
  to the snapshot. Missing targets, invalid/duplicate observations, changed
  endpoint sessions and unavailable descriptions never trigger a job-wide or
  peer fallback. A target which changes after observation is not reselected.
- The generic Agent control layer now owns observation, target validation,
  endpoint fingerprints and acknowledgement verification. Journal-bound operator
  inspection/cancellation reuse it; engines and physical operators do not acquire
  workflow lifecycle responsibilities. Existing journal bytes/fingerprints are
  unchanged.
- A snapshot is limited to 128 requests and 32 endpoint identities. Oversize,
  duplicate or malformed snapshots are rejected before network activity. A
  15-second delivery-start budget prevents starting another probe/cancel after
  expiry; each transport also has its existing bounded connect/read/frame limits.
  This is not a hard wall-clock deadline including DNS, scheduling and transport
  overhead. Several requests on one endpoint share one bounded description.
  A failed probe is isolated from other captured endpoints; the maximum valid
  128-request/32-endpoint snapshot is tested without truncation.
- Receipts distinguish requested, partial, not-requested and unknown outcomes,
  retain false terminal/replay/pending-cancellation claims, and omit endpoint
  credentials and raw transport diagnostics. A lost acknowledgement is unknown,
  never proof that cancellation was rejected. No health penalty or journal
  mutation is introduced by observation/cancellation.

Explicit user-requested `AgentClient.cancel_job` behavior is unchanged. Only the
automatic `WorkflowJobRunner` deadline/failed-runner path uses the new scoped
delivery. A cancellation acknowledgement is not proof of termination.

## Verification

Added 14 scoped cancellation tests and strengthened five existing tests.
The controlled TCP fixture implements Agent description and exact-generation
cancellation; it is not a physical Rust Agent or a numerical solver.

- Initial scoped/operator regression, seed 68181: 25 tests, zero failures.
- Focused SQLite regression, seed 68182: 55 tests, zero failures, warnings denied.
- Expanded SQLite, final seed 68185, max_cases 16: 3,204 total, 3,131 passed,
  73 conditional skips, zero failures, 65.2 seconds, warnings denied. An earlier
  expanded seed 68183 passed 3,201 tests including 73 conditional skips before
  the final three boundary tests were added.
- Final isolated memory selection, seed 68186: 196 total, 171 passed, 25
  conditional skips, zero failures, 7.2 seconds, warnings denied. The earlier
  seed 68184 passed 193 tests including 25 skips before the final three additions.
- A captured old request and a later same-job request coexist on one controlled
  endpoint. Only the old request is targeted; the later request returns normally.
- Negative controls cover moved/removed/re-sessioned endpoints, missing or
  duplicate requests, foreign jobs, malformed process/generation fields,
  oversized descriptions/snapshots, stale target acknowledgements, transport
  exits/timeouts, forged acknowledgements, no-work snapshots and budget expiry.
- Native watchdog/deadline tests retain the original blocked recovery evidence
  and restart the journal/coordinator without a new solver request. These tests
  assert exact cancellation method, request, process and generation.

Formatting and whitespace checks, tensor validation/self-test, the 26-file HTML
book, documentation inventory and project organization checks passed. Tensor
maturity is unchanged: zero structural gaps, four maturity gaps, 20 evidence-grade
gaps, 15 P0 gaps, release gate blocked. Sources remain under 800 lines and documents
under 2,000 lines, with zero tracked debt. No new binaries are built or installed;
the dedicated memory-test data directory is removed after verification.

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
  --seed 68185 --warnings-as-errors
```

The memory selection uses the two Playground cancellation files, native pipeline,
watchdog failure, native failure retention, storage outage, operator cancellation,
cancellation API, storage failure boundary and Jobs tests, setting
`KYUUBIKI_STORAGE_BACKEND=memory` and a fresh temporary `KYUUBIKI_DATA_DIR`.

## Remaining limits

Durable restart cancellation delivery remains open. The captured request set and
acknowledgements are not persisted, and this change adds no startup cancellation
scanner. Loss after failure commit can still interrupt delivery. A request not
yet visible in an Agent description is not cancelled speculatively; later
reconciliation and Agent orphan policy remain necessary. The observed process
identity is acquired during cleanup, not durably recorded before execution.

There is no retry after ambiguous delivery, remote termination proof, automatic
operator-package cleanup, exactly-once claim, historical-record repair or release
promotion. New evidence is local service/control-protocol testing, not live
PostgreSQL, real Rust Agent, remote Linux, installed desktop, numerical-accuracy
or physical power-loss qualification. Prior intermittent lease-admission failures
are not claimed resolved by this independent cancellation change.

The public job/recovery durability rules from the
[watchdog report](workflow-watchdog-finalization-20261013.md) remain unchanged.
No new database bucket, migration, backup or runtime dependency is added.
