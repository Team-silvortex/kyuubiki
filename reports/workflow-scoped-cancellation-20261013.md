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

## Live Directory Follow-Up

Date: 2026-10-26. Package metadata remains 3.7.1. This section updates the
immediate `AgentDispatchCancellation` path used by failed-runner cleanup, not
the separately implemented durable expiry sweep. Earlier counts above remain
historical and are not added to these checks.

The old implementation captured configured fingerprints only once, before
starting its delivery budget. It then probed and cancelled using the captured
endpoint object. Removing or replacing that endpoint while a description was
in flight, or after the first cancellation in a batch, could still authorize a
later send. Waiting for initial discovery also did not consume the budget.
The initial 21 new regressions produced 20 failures and one unchanged-directory
control pass before repair. They are boundary cases, not 20 unrelated defects.

The repair reuses the existing fingerprint and exact execution contract:

- Resolve current membership before each request observation and immediately
  before each exact send. Cached descriptions retain process/request/job/generation
  identity, not permission to contact an endpoint removed from the directory.
  Current matching endpoint metadata replaces the captured object for delivery.
- Removal, changed ID/address/port/session and unavailable directory reads cannot
  fall back to captured addresses, substitute peers, send job-wide cancellation
  or select a later same-job request. Registry mode reads current registration
  even if the pool still holds a matching static endpoint.
- Directory failure is recorded per target as `endpoint_directory_unavailable`,
  without collapsing earlier successful or unknown deliveries into a global
  error or fabricated not-sent result. A healthy captured peer remains independent
  when another owner is removed. Queued-only cleanup needs no directory access.
- Start the 15-second delivery-start budget before discovery. The pool's existing
  zero-argument lookup remains compatible; an optional timeout caps each wait to
  the smaller of five seconds and the remaining budget. Recheck time after lookup
  and fingerprint search, before allowing a probe or send. No budget is reset.
- Preserve one description per endpoint, 128-request/32-endpoint limits, strict
  acknowledgement validation and false termination/replay/pending-cancel claims.
  No numerical operator, execution engine or durable journal format changes.

The new isolated test module contains 27 regressions and controls. Two use real
loopback TCP observations and assert that removing or re-sessioning an endpoint
prevents a cancellation packet. Other tests cover changes between cached targets,
directory process loss before/after observation or sending, lost acknowledgements,
current metadata, independent peers, empty work, live registry changes and budget
consumption at both lookup boundaries. A suspended pool checks that only 50 ms of
remaining budget is passed to its wait instead of the default five seconds.

The registry session fixture explicitly unregisters the original session before
registering its replacement and verifies both operations. The registry correctly
refuses an arbitrary replacement of a still-active session; a separate control
proves that a rejected replacement leaves legitimate original cancellation usable.
This distinction prevents an unsuccessful fault injection from being mistaken for
a production failure.

Completed local checks, with warnings denied:

| Scope | Result | Seed |
| --- | --- | --- |
| Initial focused cancellation/recovery selection | 94 passed, zero failures | 599192 |
| Expanded Playground, dispatch/API and selected workflow recovery selection | 681 passed, zero failures | 261026 |
| Isolated memory-backend cancellation/recovery selection | 100 passed, zero failures | 261027 |

The larger selections overlap. No skips occurred in these runs; the 27 new cases
are included in both final selections, not additional coverage to sum. The memory
run uses a fresh temporary data directory removed after verification. Existing
maximum-capacity and exact same-job-bystander tests remain in the passing selection.

Formatting, diff whitespace, the 26-file HTML book, documentation inventory and
project organization checks pass. Source/config and document limits remain 800
and 2,000 lines with zero tracked debt. Tensor validation and self-test pass:
13 modules, 11 paradigms, zero structural gaps, two maturity gaps, 23 evidence-grade
gaps and 18 P0 gaps. The release gate stays blocked; this local claim does not
promote distributed or installed qualification.

These are local control-plane and controlled-transport checks. They do not test
native numerical execution, real Agent authentication/credential rotation, live
PostgreSQL, remote Linux, installed desktop behavior or physical power loss.
Membership can still change after the last check; no registry/network atomicity
or initial-process attestation is claimed. The independent
`WorkflowCancellationDelivery` durable sweep was not repaired or qualified by
this immediate-path checkpoint. Its per-job endpoint snapshot is subsequently
addressed in the [durable directory follow-up](workflow-durable-cancellation-20261013.md#durable-directory-handoff-follow-up),
with its own tests and durable authority fences. Explicit public cancellation is
likewise unchanged. Both paths now share `AgentCancellationDirectory`; this
refactoring does not extend the historical verification counts above.
No commit, push, version bump, package rebuild or installation is performed.
