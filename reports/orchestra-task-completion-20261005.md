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

### Synchronous TaskIR Wait Budget Follow Up

The 2026-10-06 follow-up addresses the specific mismatch retained above: native
Headless HTTP stopped waiting at 30 seconds while Orchestra allowed separate
120-second queue and execution windows. The SDK now sends a validated
`kyuubiki.operator-task-request-budget/v1` envelope outside the signed task.
Default HTTP waiting is 250 seconds under one monotonic deadline: queue plus
execution plus 10 seconds of connection/framing allowance. DNS/connect retry,
write and read share it; slow-drip bytes cannot renew it. Metadata and job status
polling keep their existing policies. Task response framing is bounded at
64 MiB separately from the smaller status-response cap.

The server validates and echoes the budget, forwards queue/RPC limits, and
preserves the original task digest. A short-budget regression initially showed
that legacy solver idempotency replayed the timed-out request on an idle peer.
Budgeted requests now impose `checkpoint_required` after dispatch; receive/send
failures stop instead of multiplying work across candidates. Connect failures
before dispatch may still fail over. Legacy requests without this envelope keep
their existing routing policy.

Native transport failures remain `operator_task_outcome_unknown`, nonretryable,
and halt downstream actions. This is conservative observation failure, not proof
of remote cancellation or nonexecution. A successful response still passes all
task identity, readiness and completion gates. These limits do not terminate
Orchestra-local solver CPU work, cover a whole batch, or establish a global
scheduler deadline across pre-dispatch candidate failures.

Modal safe-point observation now uses the same default 120-second execution
budget instead of an independent 15-second limit. Fixtures, independent reference
roots, residual checks and numerical tolerances are unchanged.

Verified locally:

- New native budget tests: 9 passed, including a genuinely silent 31-second
  response, a bounded stalled request with no replay/downstream action,
  slow-drip exhaustion, malformed policy rejection, per-step override isolation,
  and full results larger than the job status limit.
- Complete Rust Headless SDK: 292 unit tests plus 25 integration tests passed.
- Focused Web API/client regression: 46 tests passed. The complete SQLite Web
  suite passed 1,236 tests with 30 conditional skips.
- Complete real-Agent/Orchestra Headless live target: 15 passed in 40.53 seconds.
  It includes unchanged 2D/3D modal numerical assertions and same-task recovery;
  modal result cases additionally check the budget echoed by the real server.
- Strict Headless SDK/CLI Clippy passed with warnings denied. Formatting,
  documentation book/inventory, TaskIR examples and the 800/2,000-line
  organization audit passed. The tensor accepted the new local verified claim;
  all 14 P0 qualification gaps remain open.

The original failed loaded-host history remains above. These local regressions
resolve the identified wait-policy mismatch; they do not prove load-independent
latency, remote/installed qualification, or durable recovery of remote ownership
after a disconnected synchronous caller. Memory-mode Web was not rerun in this
follow-up. No version bump, App rebuild or installed-service change occurred.

## Retained Dispatch Observation Follow Up

On 2026-10-06, added bounded pre-RPC TaskIR dispatch observations and a read-only
`inspect-dispatch` API, exposed by native Rust
`ServiceHeadlessExecutor.inspect_operator_task_dispatch`. Attempts retain only
identity, a configured endpoint/session fingerprint, timestamps and observed
response state. The existing task-bound completion gate validates observations;
raw RPC success alone cannot become retained success. Caller exit and lost
transport remain unknown. The journal stores no input model, output result or
credential and does not authorize retries, cancellation or publication.

The runtime sidecar uses one small file per attempt under the existing data root,
not an unversioned SQL table or a whole-state snapshot. It caps total records at
512, per-file bytes at 4,096, and confirmed-response history at 128. Pending and
unknown attempts are not silently evicted; saturation blocks new TaskIR
dispatch. Damaged, symlinked or incomplete generations fail closed instead of
rolling back. A checksum is corruption detection, not a signature. One Orchestra
process owns each directory; shared-directory and power-loss qualification remain
open. Test-owned directories are isolated and cleaned without deleting unrelated
files. The live harness now explicitly supplies its own temporary data root.

Inspection accepts only task ID/digest and checks current configuration for the
same original fingerprint. It sends only `describe_agent`, at most once per
target and to at most four targets, with 1.5-second connection limits, 2-second
RPC waits and 1-MiB frames.
It returns at most 128 attempts with truncation metadata. Replaced or missing
targets, missing active requests and absent history remain unknown and do not
grant replay. Current process identity is diagnostic, not a pre-dispatch boot
pin. The result is not a terminal-result cache.

Verified locally at this follow-up:

- Complete SQLite Web suite: 1,257 tests passed with 30 conditional skips.
- Complete native Rust Headless SDK: 296 unit tests and 25 integration tests passed.
- Complete Headless live target: 16 tests passed in 45.61 seconds, including the
  two-real-Agent disconnect/restart case and unchanged bar/modal correctness checks.
- The new real chain observes an active request, times out it under the explicit
  budget, restarts only its owned temporary Orchestra using the same journal
  directory, and retains exactly the original attempt. Idle observation remains
  unknown. No peer execution occurs before the test's explicit rerun. A later
  explicit rerun has a distinct attempt and independent FL/EA numerical checks;
  it does not rewrite the first unknown attempt as successful.
- Additional boundaries include strict oversized-frame rejection, empty-registry
  no-fallback observation, preventing pruning of damaged history, and rejecting
  oversized escaped metadata without poisoning healthy dispatches. Ordinary
  tests ignore inherited production data roots; only the owned live harness
  explicitly supplies a stable restart directory. A typed native cancellation
  is retained as a failed dispatch observation, not a cached result.
- Strict Headless SDK/CLI Clippy passed with warnings denied. Documentation,
  API surface, topology, tensor, formatting and the 800/2,000-line organization
  audit passed. The tensor still reports 14 P0 qualification gaps; this local
  claim does not promote deployed or scientific maturity.
- Memory-mode execution was requested but automatic approval review disconnected
  before launch. It was not run or bypassed, and is not claimed as tested.

This adds local observation continuity, not durable remote cancellation ownership,
exactly-once execution, post-disconnect final-result retrieval, authenticated
deployed qualification or installed acceptance. No installed services, version,
App package, Git commit or push were changed. Original failed histories and prior
qualification limitations remain intact.

## Original Attempt Result Retrieval Follow Up

The native Rust SDK now exposes `fetch_operator_task_result(&task, attempt_id)`.
It validates TaskIR locally and uses a read-authorized Orchestra API to retrieve
one original Agent computation response, without replay, job/project publication,
pool-health changes or rewriting the dispatch journal. The selected attempt must
match retained task ID/digest and the original currently configured endpoint/session
fingerprint. RPC request, attempt, operator and program identities are checked;
the recovered response passes the existing task completion/failure gate in both
Orchestra and the Rust SDK. Current Agent process identity/generation are returned,
not invented historical boot attestation.

Agent retention is memory-only and visible in its descriptor: 64 reservations,
8 MiB per encoded response, 32 MiB total retained encoded responses, 10 minutes
from reservation creation. Metadata and transient JSON allocations are additional.
Expiry is reclaimed lazily; reads do not renew age. Count pressure can evict pending
reservations, byte pressure evicts old retained responses, and late completion
cannot recreate an evicted entry. Oversized results are not cached, without
blocking or truncating original execution. Reusing a retained attempt makes it
ambiguous rather than returning an older generation. No result spool, input copy
or backup is written to disk. Agent restart clears the cache.

Verification at this follow-up:

- Web SQLite regression: 1,263 total tests, zero failures, 30 conditional skips.
- Native Agent binary: 163 unit tests passed, including five retention boundaries
  and the unchanged cancellation, watchdog and reply-delivery tests.
- Protocol: 111 unit tests and seven integration tests passed.
- Full owned Headless live target: 17 tests passed in 45.97 seconds, including
  the existing bar, modal, cancellation, mixed-batch and CLI chains.
- Rust Headless SDK: 299 unit tests and 25 integration tests passed; the new
  retrieval validator also rejects a missing or mismatched nested Agent receipt.
- The real bar chain times out a held request, releases it before the first
  heartbeat, retrieves the original completed response, and independently checks
  FL/EA. Restarting the owned temporary Orchestra still retrieves that receipt;
  only one execution occurred and the peer stayed untouched. Restarting the
  owned Agent returns `not_retained` / `unknown` without any new execution.
- The separate retained-dispatch chain explicitly cancels its owned request
  after disconnect and Orchestra restart, then retrieves the final typed failed
  receipt with reason `cancelled`. The original unknown journal observation is
  unchanged. The first run of this new assertion incorrectly assumed timeout
  implies cancellation; it was corrected to explicit cancellation, not by
  weakening computation or receipt validation.
- The initial Web failures were strict old RPC parameter-shape assertions. They
  now explicitly validate the random dispatch attempt ID while still comparing
  the entire remaining TaskIR/mode/job payload unchanged. Wait budgets remain
  outside signed TaskIR and solver parameters.
- Strict Headless SDK/CLI Clippy, formatting, documentation book/inventory,
  API surface, topology and the 800/2,000-line organization audit passed. The
  tensor recognizes this separate local verified claim while retaining 14 P0
  qualification gaps and blocked daji release status; deployed/scientific grades
  were not promoted.

This is local original-receipt retrieval, not durable result publication, Agent
restart/power-loss recovery, exactly-once execution, pre-dispatch boot pinning,
cryptographic provenance or authenticated deployed qualification. Recovery APIs
always retain `automatic_replay_authorized: false` and `publication_performed:
false`. No installed-service restart, App rebuild, version bump or Git operation
was performed. Existing failed histories and broader qualification gaps remain.

## Result Publication Race Follow Up

Three deterministic regressions first reproduced incorrect success replies:
heartbeat shutdown requesting cancellation, watchdog termination during shutdown
before the solver cancellation flag propagates, and legacy solver result
serialization requesting cancellation after the old final check. The native
publication gate now runs after heartbeat shutdown and requires both an
uncancelled solver control and the current active watchdog generation. Legacy
solver RPCs recheck after result encoding and heartbeat shutdown as well.

The TaskIR regression checks the retained original-attempt response, not just
the immediate reply: it contains the exact failed RPC and typed `publish_result`
receipt, with no computed success value or replay permission. The watchdog test
keeps cancellation false and verifies the existing terminal reason/generation,
rather than masking the race by eagerly setting the solver flag. Explicit rerun
is not poisoned. The legacy RPC test also rejects success progress frames.

Verification at this follow-up:

- Complete native Agent binary: 166 unit tests passed, including all three new
  regressions and the existing reply-delivery and generation-fencing tests.
- Complete owned Headless live target: 17 tests passed in 45.13 seconds. This
  reruns original-receipt retrieval, explicit cancellation/recovery, two-Agent
  owner isolation, temporary Orchestra/Agent restart, modal residual checks,
  mixed batches and CLI workflow chains.
- Complete Rust Headless SDK: 299 unit tests and 25 integration tests passed;
  the unit target finished in 31.33 seconds.
- Strict CLI/Headless SDK Clippy passed with warnings denied. Documentation
  book/inventory, API surface, topology, tensor, formatting and the 800/2,000-line
  organization audit passed. The tensor retains 14 P0 qualification gaps and
  blocked daji release status. The full standalone Web suite was not rerun in
  this follow-up; the owned live target exercises its service chain.

The shutdown callback exists only in test code and runs after the heartbeat
stop flag changes; no sleeps or scheduling guesses select the failure window.
It does not exercise a real in-flight heartbeat socket failure. The legacy
fixture exercises result serialization, not remote artifact-upload rollback.
Neither this fence nor cache retrieval grants atomic cancel-versus-send semantics,
durable publication, exactly-once execution or installed/deployed qualification.
Cancellation or termination after the last gate can still race with delivery.
No installed-service restart, App rebuild, version bump or Git operation was
performed. The tensor's local evidence is extended without promoting release
readiness or deployed/scientific grades.

## Terminal Failure Ownership Follow Up

The watchdog's 16-entry recent-failure list previously also owned terminal-cause
deduplication. Two new regressions first reproduced failure replacement after
diagnostic eviction: an explicit failure became a later transport error, and a
watchdog timeout became a later cancellation. Original job/method/timing fields
were lost and the same execution was counted again.

Each admitted watchdog execution now shares an `Arc<OnceLock<FailureReport>>`
between its record and guard clones. Explicit failure and timeout scans pin one
original cause. A late callback reads that cause without reinserting diagnostic
history or incrementing failed counters. The active generation of a newly reused
request ID is not touched. The diagnostic view stays bounded at 16 entries;
terminal-cause cells remain only while execution references retain them and are
freed after the last guard drops. There is no unbounded archive, spool or backup.
Failure metadata remains additional memory outside the TaskIR cache byte budget.

Five new isolated unit regressions cover eight concurrent first-failure callbacks,
eight concurrent late callbacks after eviction and request-ID reuse, a real
watchdog scan with exact timeout timing, weak-reference verification of final-guard
memory release, and preservation of an already-known failure after local
watchdog-lock poisoning. Poisoned state still rejects new admission. Existing
tests were moved into the dedicated `cli/src/tests/agent_watchdog.rs` module
without changing their assertions.

Verification at this follow-up:

- Complete native Agent binary: 171 unit tests passed, including all five new
  boundaries and the unchanged reply-delivery and TaskIR publication tests.
- Complete owned Headless live target: 17 tests passed in 43.68 seconds, including
  original-result retrieval, explicit recovery, two-Agent isolation and temporary
  process restart.
- Complete real solver cancellation target: 52 tests passed in 59.83 seconds.
  These cover heat assembly, PCG iterations, dense pivots, scaling, postprocessing,
  disconnected orphan capacity release, job isolation and healthy replay with
  independent analytical temperature checks, plus harness readiness boundaries.
- Strict CLI/Headless SDK Clippy passed with warnings denied. Formatting,
  documentation book/inventory, API surface, topology, tensor and the 800/2,000-line
  organization audit passed. The tensor keeps 14 P0 qualification gaps and blocked
  daji release status. The standalone Web and Headless SDK suites were not rerun
  in this follow-up; the owned live target exercises the cross-process SDK chain.

The eviction/poisoning fixtures are unit boundaries, not a live transport-fault
or deployed qualification. Process restart still loses watchdog memory; this
does not add exactly-once execution, durable publication or atomic cancel/send.
No installed-service restart, App rebuild, version bump or Git operation was
performed. New evidence is local/verified only, not a release-grade promotion.

## Legacy Failure Reply Classification Follow Up

Two deterministic regressions first reproduced contradictory legacy solver
failure replies. A terminal watchdog timeout remained in the detailed report,
but the outer RPC code was hardcoded to `invalid_params` after decoding failed
or `result_transport_failed` after encoding failed. Both paths now use the
original report's reason code and message consistently. The post-admission
execution body is separated from admission without adding a public entrypoint
or changing reply-writer ownership, TaskIR, solver parameters or runtime protocols.

Four new unit tests cover those two late-failure boundaries and the corresponding
ordinary errors without a prior terminal failure. The former compare the entire
original report, reject success values/progress, then explicitly rerun the same
request/job IDs under a fresh generation. The latter retain their original error
classification. Fixtures pin failure at the admitted-execution boundary before
solver cancellation propagates; they do not change global watchdog policies,
wait on arbitrary sleeps or qualify remote artifact-upload rollback.

The first complete live run had 16 passes and one failure in the retained-dispatch
restart fixture. That fixture sent a job-wide cancellation after Orchestra restart,
assuming the disconnected execution was still active. A failed heartbeat could
already have terminated it; the late cancellation then deliberately registered
the existing one-shot pre-admission cancellation and poisoned the explicit rerun.
The fixture now keeps computation held until the closed transport cancels its
owner and the lifecycle reports zero active executions. It checks the failed
receipt's original generation, then explicitly reruns. It does not weaken the
unknown journal, peer-isolation, failed-receipt or independent FL/EA assertions,
and does not retry a failed rerun until one happens to succeed. The earlier
explicit-cancellation fixture described above is superseded by this sequencing.

Verification at this follow-up:

- Complete native Agent binary: 175 unit tests passed, including four new
  classification boundaries and the existing watchdog, cancellation and delivery
  tests. Two new tests were observed failing before the classification fix.
- Corrected restart fixture: one focused test passed in 4.46 seconds, followed
  by the complete owned Headless live target with 17 passes in 13.83 seconds.
- Complete real solver cancellation target: 52 tests passed in 9.69 seconds,
  covering heat assembly, iterative/dense solver boundaries, scaling, postprocessing,
  orphan capacity release, job isolation and healthy rerun with independent
  analytical temperature checks, plus harness readiness tests.
- Complete Rust Headless SDK: 299 unit tests and 25 integration tests passed;
  the unit target finished in 31.33 seconds. Its receipt validation, timeout and
  recovery tests remain unchanged.
- Strict CLI/Headless SDK Clippy passed with warnings denied. Formatting,
  documentation book/inventory, API surface, topology, tensor and the 800/2,000-line
  organization audit passed. The tensor retains 14 P0 qualification gaps and
  blocked release status; deployed/scientific grades were not promoted.
- The standalone Web suite was not rerun in this follow-up. The owned live target
  exercises the cross-process Orchestra, Agent and Rust SDK chain locally.

The legacy boundaries are unit evidence, not installed, deployed or numerical
qualification. Transport timeout still means unknown, not confirmed cancellation;
the held live fixture separately observes transport-induced termination before
rerun. Cancellation registration alone does not prove an execution has stopped.
No installed-service restart, App rebuild, version bump or Git operation was
performed. New evidence remains local/verified only.

## Exact Execution Cancellation Follow Up

Added the distinct native Agent `cancel_execution` RPC. It requires an exact
process instance, execution request id, positive generation and job id. Strict
decoding and bounded identity validation happen before cancellation. The live
control registry matches request/generation/job under its admission lock; the
process identity must also match. It never falls back to job-wide cancellation,
creates a pending cancellation, releases operator packages or authorizes replay.
The unchanged `cancel_job` contract still supports job-wide and one-shot
pre-admission cancellation, so late cleanup must not use that legacy operation.

Acknowledgements use `kyuubiki.agent-execution-cancellation/v1`, echo the exact
target and distinguish `requested` from `target_not_observed`. Registration is
not terminal confirmation, and absence is not proof of nonexecution. Four new
RPC unit tests were first observed failing because the new method was unsupported;
that was a missing capability, not four independently reproduced old bugs. An
additional isolated lifecycle-unavailable fixture then reproduced acceptance of
the diagnostic `unavailable` identity. The handler now rejects unavailable
process state before marking any live control or future admission. This fixture
injects a descriptor, not a real process-wide lock-poisoning fault.

The new owned-Agent live target has two functional tests and two existing harness
readiness tests. It holds two same-job computations on one Agent and a peer with
the same request/generation. Only the exact owner is cancelled; the sibling,
peer and higher-generation rerun complete with independent FL/EA and F/A checks.
A separate owned-process restart reuses the same request/job/generation under a
new process id; the stale target cannot cancel it or poison a future admission.
Three existing Rust SDK/Orchestra TaskIR cancellation chains now select the exact
native target for precomputation, 2D modal multiplication and 3D modal validation.
They still assert typed failed receipts, downstream isolation and explicit rerun.
The public Orchestra job-cancel routing test remains job-scoped and unchanged.

Verification at this follow-up:

- Native Agent binary: 180 unit tests passed, including five new cancellation
  boundaries and the unchanged watchdog, result-publication and delivery tests.
- Protocol: 114 unit tests and seven integration tests passed, including three
  new typed-request, method-advertisement and malformed/bounded-identity tests.
- New owned-Agent live target: four tests passed in 1.10 seconds. Complete owned
  Headless live target: 17 tests passed in 15.17 seconds after the final fix.
- Existing real solver cancellation target: 52 tests passed in 10.21 seconds,
  retaining thermal/iterative/dense/scaling/postprocessing and legacy job-cancel
  recovery coverage. Rust Headless SDK: 299 unit tests and 25 integration tests
  passed; its unit target took 31.31 seconds.
- Strict CLI/Headless SDK/Protocol Clippy passed with warnings denied. Formatting,
  documentation book/inventory, API surface, topology, tensor and organization
  checks passed, including the 800-source/2,000-document line limits. The tensor
  retains four maturity gaps, 19 evidence-grade gaps, 14 P0 qualification gaps and
  blocked release status. The new dedicated cancellation shard records local
  execution/contract/recovery evidence without promoting deployed grades.

The acknowledgement schema is valid JSON. Its identity pattern was checked
against 94 invalid and four valid samples, including Unicode whitespace and
preserved nonblank identities. Native tests also retain byte-bound and exact
round-trip checks. A full JSON Schema engine was unavailable and not installed;
this follow-up does not claim full acknowledgement-schema validation.

The fence is identity matching, not authentication. No public Orchestra HTTP or
Headless SDK exact-cancellation helper was added. There is still no atomic
cancel-versus-send guarantee after the final publication gate, durable cancellation
recovery, remote qualification or installed acceptance. The standalone Web suite
was not rerun; the owned live target exercised the cross-process chain locally.
No installed-service restart, App rebuild, version bump or Git operation was
performed. This is local/verified evidence, not a deployed-grade promotion.

## Public Observed Dispatch Cancellation Follow Up

The previous native-only scope is extended by the write-authorized
`POST /api/v1/operator-tasks/cancel-dispatch` route and Rust
`ServiceHeadlessExecutor::cancel_operator_task_dispatch`. Callers select a retained
attempt and supply one explicitly observed process/request/generation/job target.
Active inspection can return that target only when the Agent job matches the
retained task and native identity bounds hold. The SDK checks all target mirrors
before presenting the inspection. Missing or null target is not cancellation
authority. This is explicit observation, not initial-process attestation at dispatch.

The server matches task/digest/attempt and request/job identities before effects,
then contacts only the original currently configured endpoint/session fingerprint.
It does not inspect a newer generation, broadcast, use a static discovery fallback,
send `cancel_job`, update the dispatch journal, publish a result or authorize replay.
Missing attempts, undispatched or retained-terminal observations and absent or
replaced endpoints cause no Agent RPC. The legacy job-cancel routes are unchanged.

The acknowledgement uses `kyuubiki.operator-task-dispatch-cancellation/v1`.
Matching native acknowledgement means `requested` or `target_not_observed`, never
terminal proof. Invalid or unavailable delivery produces
`cancellation_outcome_unknown` and `cancel_registered: null`, not a claim that
registration failed. Foreign target identity, unknown fields, missing fields,
contradictory registration and any terminal/replay/pending-cleanup promise are
rejected. Raw Agent errors and oversized diagnostics are not echoed. The native
control round trip is bounded to five seconds and 64 KiB; the SDK shares one
ten-second HTTP deadline. SDK request errors also refuse automatic retry authority.

Verification at this follow-up:

- Ten new Web tests cover retained-owner-only routing, unchanged journal bytes,
  missing/terminal/replaced targets, stale-target acknowledgement, malformed query,
  invalid acknowledgement, unknown transport outcome, write authorization with
  unprotected reads, empty discovery registry and a real oversized RPC frame.
  The selected related Web suite passed 46 tests in 1.8 seconds, including existing
  dispatch inspection, journal/result retrieval and legacy job cancellation.
- Seven new SDK tests cover all status combinations, identity/authority mirrors,
  invalid Agent acknowledgements, validation before connection, one real HTTP
  control request with bearer token, malformed HTTP response without retry, and
  the inspection target's four identity mirrors. Complete Rust Headless SDK:
  306 unit tests passed in 31.33 seconds and 25 integration tests passed.
- Complete owned Headless live target: 17 tests passed in 15.28 seconds. Three
  existing real TaskIR cancellation/recovery chains now use inspection and the
  public Rust SDK method through Orchestra, not direct Agent cancellation. They
  retain precomputation/2D-modal/3D-modal failure, downstream isolation and explicit
  healthy rerun assertions. The public legacy job-cancel routing case still passes.
- Strict CLI/Headless SDK/Protocol Clippy and the Elixir test-environment build
  passed with warnings denied. Formatting,
  documentation book/inventory, API surface, topology, tensor and organization
  audits passed, with 800-source/2,000-document line limits and zero tracked debt.
  The tensor retains four maturity gaps, 19 evidence-grade gaps, 14 P0 qualification
  gaps and blocked release status. Public-chain evidence is local/verified only.

The full standalone Web, native Agent unit and protocol suites were not rerun in
this follow-up; native cancellation's previous tests remain separate evidence.
No full JSON Schema engine was installed or run. The schema and application-level
receipt validators are not formal proof or deployed/scientific qualification.
There is no durable cancellation-intent store, atomic cancel/send, Python/Elixir
parity or remote cancellation/restart qualification. No installed-service restart,
App rebuild, version bump, Git commit or push was performed. Product version
remains daji 3.4.7; installed binaries are not claimed to contain this source change.

## Scope

The coverage claim is local verified evidence for Orchestra execution, contract,
and recovery. JSON result consistency is not a cryptographic attestation of
calculation, and neither it nor passing API tests proves numerical correctness.
The suite's conditional skips are not counted as executed coverage. Broader
Agent engine integration beyond the local bar/modal/spring fixtures, external package execution,
authenticated deployed chains, installed acceptance, SDK parity, and scientific
qualification remain separate.

No version bump, commit, push, release rebuild, installed-service restart, or deployment
was performed. Product version remains daji 3.4.7; existing uncommitted Rust work
was preserved.
