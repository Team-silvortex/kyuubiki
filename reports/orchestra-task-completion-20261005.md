# Orchestra Task Completion and Batch Recovery Verification

Verified on 2026-10-05 against the daji 3.4.7 working tree based on
`2ef0cb4b`. This round moves completion validation into the backend producer,
without moving numerical algorithms into Orchestra or coupling the Rust SDK
to the Elixir implementation.

Initial round: Local Orchestra API verification only; not deployed Agent or scientific qualification.
The follow-ups distinguish test isolation from a real local Rust Agent chain.

## Reproduced Failures

After correcting the test Agent's declared capability/tag routing, all nine
initial API regressions failed before the production changes. The real router
promoted pending or stale Agent acknowledgements to executed, batches counted
attempted/error cases as executed, and zero errors could authorize archival
without complete result coverage. Strict execution also lost skipped recovery
targets when an Agent returned a blocking acknowledgement.

## Execution and Recovery Rules

- Agent completion is bound to task id, digest, operator, and program. Present
  validation/provenance mirrors are checked; malformed, stale, unknown, absent
  result, or contradictory success receipts fail closed. Rejection messages
  expose contract field labels, not rejected receipt values.
- Envelope status reflects executed, blocked, or failed Agent state. The raw
  executor success API returns only completed results; the structured receipt
  API preserves blocked/failed Agent data for the router and batch reporter.
- Batches distinguish attempts, completed results, blocked cases, errors, and
  skipped cases. Executed count equals ok count, not attempted count. Strict
  mode stops at a blocker; independent mode retains healthy results while
  reporting blockers/errors separately. Result values are not rewritten.
- Explicitly malformed readiness cannot fall through legacy normalization.
  Executed readiness must be dispatch-ready and have no remaining blocking
  stage, reason, or repair action. Absent legacy readiness remains supported.
- Checkpoints require unique, matching published results and consistent counts,
  state, digest, and readiness before archival. Mixed failure/blocking/skipping
  uses resolve_incomplete_cases, retaining unresolved targets in batch order,
  readiness repair actions, and existing failure recovery actions after JSON
  reload. Valid successful cases remain outside the recovery target set.
- Completion metadata is checked once per checkpoint, reusing its batch digest
  and set-based case matching. This avoids extra whole-batch digest encoding and
  repeated linear membership scans. No benchmark speedup is claimed.

## Verification

The regression target is automatically included in the normal Web ExUnit suite:

`apps/web/test/kyuubiki_web/api/operator_task_completion_api_test.exs`

It contains 13 tests, including mutation matrices for stale/malformed receipts,
contradictory readiness, failure states, incomplete or duplicated case coverage,
and inconsistent checkpoint completion metadata. The API tests use the real
Plug router, executor, TCP Agent transport, and checkpoint/resume functions.
The controlled Agent replies are deliberately not a real Rust solver.

At the first completion verification, a fresh disposable SQLite database was
selected explicitly using `SQLITE_DATABASE_PATH`. The default development
database was rejected as unversioned by the startup safety gate; it was not
modified or migrated. The follow-up below repairs the default test configuration
so this manual override is no longer required. Never point reset-capable tests
at a user's active database. Local execution needs loopback TCP permission.

- Completion-gate SQLite suite: 1,204 tests, zero failures, 30 conditional skips.
- Focused memory-mode suite: 62 tests, zero failures, zero skips. It includes
  completion API, existing TaskIR API, executor, batch contract, and readiness
  targets, using `KYUUBIKI_STORAGE_BACKEND=memory`.
- Execution/checkpoint/resume JSON schemas and the execution example reflect
  the new counters, blocked row status, and resolve_incomplete_cases action.

### Local Test Runtime Isolation Follow Up

The configuration regression initially failed two of five tests: default test
starts neither owned isolated SQLite storage nor obtained independent paths.
`runtime.exs` now creates a unique test-only directory when SQLite is selected
and no explicit path is supplied. A suite callback stops the application before
removing its owned database/journals and empty directory. It does not recursively
remove unexpected files or claim explicit, development, installed, memory, or
PostgreSQL data. Seven regressions cover path isolation, explicit ownership,
non-test defaults, alternate backends, cleanup, and refusal boundaries.

- Default `mix test`, with no database override: 1,211 tests, zero failures,
  30 conditional skips. The owned temporary directory was gone after the suite.
- Full memory-mode `mix test`: 1,211 tests, zero failures, 63 conditional skips.
  Backend-specific skips are recorded, not counted as executed coverage.
- Native `web-test` entry with isolation/completion targets: 20 tests passed
  without a database override. Changed Elixir formatting, strict Clippy for the
  live Rust target, documentation inventory/book, organization, tensor, and diff
  whitespace checks passed. The tensor still has 14 unresolved P0 gaps.
- Native `headless-test --locked --offline`: 22 tests passed after loopback
  permission was granted. The earlier blocked aggregate attempt remains a
  historical environment failure, not a passing result.
- `headless-live-test --locked --offline`: four tests passed using actual local
  Orchestra HTTP and controlled Agent solver fixtures. The Rust harness now owns
  its child before readiness waiting and owns JSON/SQLite temporary directories,
  so error unwinding and normal fixture release can reap and clean them.

Process and scratch-directory checks found no newly retained local test service
or owned directory. The installed desktop bundle was inspected as 3.4.0 and was
not replaced; its previously stopped services were not restarted. Native doctor
reported the core development tools available; its optional PostgreSQL client
was absent and was not installed for SQLite tests. No installed-app repair or
remote/numerical qualification is claimed by this follow-up.

### Real Local Agent and Orchestra Follow Up

A fresh real-Agent HTTP regression first failed because native TaskIR was
converted to `solve_bar_1d` and lost the Agent execution receipt. Three of four
initial API tests also failed: native TaskIR incorrectly required package
attachment, the API selected the legacy solve RPC, and a stale native task id
escaped receipt validation. A later boundary matrix exposed another mismatch:
Orchestra accepted nonboolean `agent_fetchable` values that Rust rejects.

Native execution now uses the existing Agent TaskIR RPC and completion gate.
Only unbundled native tasks with an explicit boolean false fetch declaration
avoid the external-package readiness constraint. Central fetch tasks and native
tasks with package references keep that constraint. Nonboolean fetch declarations
fail admission before selecting an Agent. Numerical algorithms stay in the Rust
engine; no new operator support is advertised by the control plane.

The live target contains three new cases using the actual Rust Agent binary,
Orchestra HTTP server, native SDK/CLI, and an owned temporary SQLite database:

- A pending external task stays blocked with its repair action. A valid native
  bar executes with Agent-bound identity and independent FL/EA relative error
  below 1e-12. A zero-area failure stops downstream project creation; correcting
  its input and digest allows the same task id to execute. Replaying the original
  valid task retains identical data, and only successful execution may create
  its downstream project. All operations use the same owned Agent process.
- A mixed good/blocked/failed batch retains only the two unfinished cases as
  recovery targets after checkpoint JSON is written and reloaded. The successful
  result remains outside the recovery targets; readiness repair actions persist.
- A truly blocked service execution returns a nonzero native CLI exit while
  writing the requested report and leaving the downstream project absent.

The live fixture configures a real local static endpoint with unknown package
readiness; it does not invent a ready advertisement. Explicitly detached package
readiness is covered by separate API routing tests with controlled replies.
This is not registered/authenticated discovery or package-runtime attachment
recovery; the external placeholder task is not claimed to have completed.

Final SQLite suite: 1,217 tests, zero failures, 30 conditional skips. The live
Rust target passed nine tests: three real-chain cases, three existing workflow
cases with controlled Agent replies, and three lifecycle/cleanup checks. Strict
Clippy passed for the live target. The native SDK suite passed 305 tests
(280 unit and 25 integration tests); the three CLI boundary targets passed
22 tests. Organization, documentation book/inventory, tensor, formatting, and
diff checks passed. The tensor retains 14 P0 gaps and this local evidence is
only verified, not qualified. Full memory-mode rerun was not executed:
three automatic-approval transport failures occurred before process launch.
The earlier 1,211-test memory run remains historical evidence, not a new result.

### Native Failure Relay and Modal HTTP Follow Up

Three new API regressions failed before the relay repair: the transport dropped
the native failure receipt, the batch rewrote its owner/stage/code as a generic
control-plane error, and malformed failure identities were not independently
validated. A Rust HTTP mutation regression also reproduced acceptance of a
failure receipt referring to another task.

The TaskIR transport now preserves the structured failure separately from legacy
generic RPC errors. A task-bound failure is a completed RPC exchange, not a
transport outage: it does not penalize connection health or authorize another
Agent to replay the task. Capacity queuing retains its prior contract. Orchestra
checks schema/owner, task id/digest/operator, optional program id, RPC code and
message, boolean recovery fields, bounded text, and readiness consistency. It
forwards only known failure/recovery fields in a failed execution envelope;
HTTP 200 delivery is not calculation success. Error rejection exposes field
labels, not rejected values. Successful RPC envelopes containing failure data
pass the same gate.

Batch failure rows retain the Agent's owner, engine stage, code, and recovery
action. Their actions survive checkpoint JSON reload. Validated unsafe
continuation stops an independent batch as well as strict execution, preserving
skipped ids. The native Rust client independently checks failure identities,
recovery types, readiness, and outer/nested receipt consistency. Its run summary
derives advice from the typed receipt rather than message matching; its existing
Headless category is runtime_failure, while the exact native code/stage remain
in the step preview. The 4,096-byte message boundary survives report compaction.
Recovery advice never automatically executes or retries a task.

The real local target now covers all three currently advertised native solver
TaskIR routes: axial bar, modal_frame_2d, and modal_frame_3d. This is not all
solver RPCs or all operator library coverage. The new modal target uses 96/100
unit-beam segments in 2D and 100 in 3D. It checks full mode arrays before report
compaction, independent reference roots (relative error below 2e-8), an
independently assembled matrix residual at most 1e-8, finite normalized shapes,
fixed DOFs, and frequency/period consistency. A zero-density 3D task fails with
its bound engine receipt, releases the Agent slot, and succeeds after corrected
input and a fresh digest under the same task id and Agent process.

Final checks for this follow-up:

- SQLite Web suite: 1,222 tests, zero failures, 30 conditional skips.
- Memory Web suite: 1,222 tests, zero failures, 63 conditional skips. This rerun
  executed successfully; the prior approval-transport failures remain historical.
- Native SDK: 308 passing tests (283 unit and 25 integration), none ignored.
- Live target: 11 passing tests, none ignored: five real Agent/Orchestra chain
  cases, three controlled workflow cases, and three lifecycle/cleanup checks.
- CLI boundary targets: 22 passing tests, none ignored. Strict Clippy passed for
  the live CLI target and native SDK tests.
- Formatting, diff whitespace, documentation book/inventory, and the 800-line
  source / 2,000-line documentation organization audit passed. The tensor check
  passed structurally and still reports 14 unresolved P0 gaps; new local claims
  remain verified, not qualified.

The reference models are bounded local fixtures, not a million-node benchmark,
arbitrary-geometry qualification, authenticated deployment, or installed-app
acceptance. Numerical algorithms remain in the Rust Engine; Orchestra only
validates contracts, routes TaskIR, and retains completion/recovery evidence.

### TaskIR Cancellation and Explicit Recovery Follow Up

The precomputation cancellation regression failed before the repair: the RPC
reported cancellation but retained no task-bound failure receipt. The existing
test name incorrectly described a late-result race even though it registered
cancellation before admission; it now names and verifies its actual boundary.
The result-publication cancellation branch also lacked this receipt.

TaskIR failure exits now share one builder for lifecycle evidence, diagnostic
solver checkpoints, and task-bound failure identity. It uses the lifecycle's
final reason/message, including an already-recorded watchdog timeout for the
same execution generation. It rebuilds recovery advice for that reason without
discarding admission diagnostics. Cancellation and watchdog timeout are
non-retryable and require explicit inspection before rerun. The generic legacy
solver RPC contract and numerical algorithms are unchanged.

Four new Rust unit tests cover publication-boundary result disposal and
same-job rerun, watchdog rebinding without stale retry advice, admission
diagnostic preservation, and an actual lifecycle record reused by a late
cancellation reply. The publication test supplies a prepared result directly at
the production publication function, not a real HTTP scheduling race. Neither
this test nor the handler claims to retract an already-started response write.
Cancellation after the final publication check can race with response delivery;
this is not a linearizable cancel-versus-send guarantee.

Three new actual Rust Agent + Orchestra + native SDK HTTP cases cancel an axial
bar before computation, a 100-segment 2D modal frame at sparse matrix
multiplication, and a 100-segment 3D modal frame at final shape normalization.
Each observes the owned execution, submits cancellation to that Agent, retains
its typed task-bound failed receipt and SDK recovery action, counts zero
completed steps, leaves the downstream project store unchanged, and releases
the single execution slot. An explicit rerun with identical task id and digest
on the same Agent then succeeds. Bar displacement is checked against FL/EA;
modal reruns retain the full independent reference and residual checks.

The numerical safe-point checkpoints observed on the Agent remain diagnostic
(`resumable: false`), not continuation snapshots. Their separate RPC diagnostic
is not forwarded by Orchestra's known-field failure contract. Cancellation is
sent directly to the owned Agent; these cases do not qualify Orchestra's
multi-Agent cancel routing or the public asynchronous job cancel surface.
The tests initially accepted either legacy cancellation success or explicit
`cancel_registered` evidence. Unbound jobs normally return `already_released`;
a cleanup error is a conditional bound-package branch, not the default outcome
of a detached package runtime. The follow-up below separates those acknowledgements.

Focused rerun results:

- Agent binary unit target: 156 passed, no failures or ignored tests.
- Modal Agent TCP target: 13 passed, no failures or ignored tests.
- Complete live target: 14 passed, including eight actual Agent/Orchestra
  cases, three controlled workflow cases, and three lifecycle/cleanup checks.
- Native CLI task-completion target: five passed, no failures or ignored tests.
- SQLite Web suite: 1,222 tests, no failures, 30 conditional skips.
- Native SDK: 308 passed (283 unit and 25 integration), none ignored.
- Strict Clippy passed for all CLI and native SDK test targets. Rust formatting,
  diff whitespace, documentation book/inventory, and project organization
  audits passed with the 800-line source / 2,000-line documentation limits.
- Memory Web mode was not rerun in this cancellation follow-up: two automatic
  approval review connections failed before process launch. This is a tool
  transport limitation, not a test failure. The successful 1,222-test memory
  run in the preceding relay follow-up remains historical evidence.

The tensor check passed structurally: 13 modules, 11 paradigms, no structural
gaps, four maturity gaps, 19 evidence-grade gaps, and 14 unresolved P0 gaps.
It registers these local cancellation claims as verified only. They are not
scientific, installed, remote-scale, or authenticated qualification.

### Known Dispatch Cancellation Follow Up

The new public API regression failed before repair: a solver ran on the busy
owner Agent, but cancellation was dispatched to an idle peer from the pool.
This could leave the owner calculating while registering a stray one-shot
cancellation on a different Agent. Cancellation is now routed from job-bound
live capacity leases, not a fresh pool checkout or historical stored worker id.
Queued/reserved work is cancelled locally, and already active capacity is not
released merely because a cancellation acknowledgement was received.

The final local dispatch guard suppresses cancelled successful results and
transport retries without discarding a native TaskIR failure receipt. Server
total-budget cleanup captures targets before local caller shutdown removes its
lease. The public response includes a separate per-target cancellation receipt
with registration counts, partial-delivery status, and
`execution_terminal_confirmed: false`. A cancelled stored job is control-plane
intent, not proof that every remote computation has terminated.

Native cancellation registration is fail-closed if its registry is unavailable.
Once registered, a cache-release error no longer turns cancellation into an RPC
failure: the error remains separately visible in `operator_package_job_release`.
The injected cleanup-failure unit test exercises the production receipt builder
and actual cancellation control; it does not simulate a failed filesystem/cache
activation end-to-end. Existing real package fetch/release tests remain a
separate success-path check. Unbound jobs retain `already_released` cleanup.

A new two-real-Agent HTTP case holds TaskIR predecessors on both Agents,
submits a public asynchronous spring job, switches the owner's hold marker to
the generated job identity, then calls the public cancel endpoint. It checks:

- Only the execution owner is acknowledged as a cancellation target.
- The peer's held TaskIR has no cancellation flag and still completes normally.
- No result is stored for the cancelled job and the owner's slot recovers.
- Reusing that job id directly on the peer succeeds without a stray pending cancel.
- A following public job completes and agrees with `u = F/k = 0.04`, force
  `1,000`, and strain energy `F*u/2 = 20`.

Controlled regressions additionally check multiple known owners, deduplicated
targets, queue/reservation isolation, capacity retention, partial delivery,
typed acknowledgement identity/schema, malformed job ids, and capture before
local caller shutdown. These tests do not establish durable ownership after
Orchestra restart or transport loss, authenticated/remote deployment, all solver
families, or atomic cancellation versus transmission. In particular,
`no_active_dispatch` is not proof that there is no remote execution.

Verification recorded for this follow-up:

- Agent binary unit target: 158 passed.
- Native solver cancellation/transport-loss target: 52 passed. Modal Agent TCP
  target: 13 passed. No failures or ignored tests in either target.
- Bound-package fetch/execute/cache-rotation/release target: one passed when
  explicitly invoked with `--ignored`, using the existing prebuilt template
  dynamic library. The new cancellation registration/cleanup fields were checked.
- Complete Headless live target: 15 passed, including nine real-Agent/Orchestra
  cases, three controlled workflow cases, and three lifecycle/cleanup checks.
- SQLite Web suite: 1,231 tests, no failures, 30 conditional skips.
- Strict CLI Clippy passed with warnings denied.
- Memory Web mode did not launch: automatic approval review disconnected before
  completion. It is an execution-tool limitation, not a test assertion failure.

Do not collapse the rerun history into an unconditional all-green claim. The
first complete live run passed 15 cases in 42.00 seconds. A subsequent run that
overlapped other test work passed 13 and failed two: the 100-segment 3D modal
cancel case did not reach `modal_shape_norm` within its 15-second observation
budget (the Agent remained active at `modal_sweep`, with no cancellation flag
or watchdog failure), and the same-Agent modal repair case hit the native SDK's
30-second synchronous HTTP read timeout. An isolated final full run passed
15/15 in 47.84 seconds, without changing fixture size, numeric assertions, or
request timeouts. Host contention is a plausible cause, not proven root cause.
Long synchronous calculation waiting and load-sensitive test observation remain
reliability work; passing isolated tests does not resolve this budget gap.

These are local verified claims only; they do not upgrade operational or
scientific qualification. No release rebuild or installed-service change occurred.

## Scope

The coverage claim is local verified evidence for Orchestra execution, contract,
and recovery. JSON result consistency is not a cryptographic attestation of
calculation, and neither it nor passing API tests proves numerical correctness.
The suite's conditional skips are not counted as executed coverage. Broader
Agent engine integration beyond the local bar/modal/spring fixtures, external package execution,
authenticated deployed chains, installed acceptance, SDK parity, and scientific
qualification remain separate.

No version bump, commit, push, release rebuild, service restart, or deployment
was performed. Product version remains daji 3.4.7; existing uncommitted Rust work
was preserved.
