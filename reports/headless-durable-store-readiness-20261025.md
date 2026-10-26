# Durable Store Readiness Before Research Submission

Date: 2026-10-25. Platform: local macOS ARM64. Package baseline: daji 3.7.1.
This extends the official Rust headless SDK and its durable modal research
example. Engine/Agent execution policy, Solver numerics and receipt protocols
are unchanged. It follows the [offline export recovery work](headless-modal-offline-export-recovery-20261025.md).

## Reproduced Defect

The research example previously extracted only `store_id` from `describe_agent`.
An unavailable store can still advertise its retained identity. The example
therefore created input/intent records and attempted dispatch despite an explicit
`available: false` descriptor. A temporarily busy descriptor, which has no ID,
was instead reported as a missing required store.

The regression ran a real native Agent and completed a task, then displaced only
that test Agent's temporary execution directory. The descriptor correctly
reported unavailable with the old identity. Before the fix, example submission
failed the assertion that no new submission directory should exist. The Agent
already rejected unsafe admission; this fixes caller preflight and diagnosis,
not an observed bypass of Agent ownership checks or lost existing results.

## SDK Contract

`SolverRpcClient::describe_durable_agent_store()` performs one descriptor read
and returns `DurableAgentStoreStatus`: `Ready`, `Disabled`, `Busy`, `Unavailable`
or `AdmissionBlocked`. `from_descriptor` also parses caller-retained descriptor
JSON. `ready_store_id()` is a new-work preflight helper, not a dispatch method.

- Schema, replay policy, enabled/restart state, identity and explicit availability
  must be consistent. Missing or wrongly typed required fields fail closed.
- Ready requires a valid store identity and a false write-failure admission block.
- A readable store with admission blocked cannot pass the new-submission gate.
- Busy and unavailable observations never imply a task is missing or failed.
- Unavailability reasons are bounded; unknown valid reasons remain unavailable,
  not inferred ready. Contradictory available-plus-unavailable-reason is rejected.
- Disabled remains distinct from contention and damaged store binding.

The example applies the gate before creating its submission directory. Existing
intent/handle-based recovery APIs are unchanged and do not apply a new-admission
gate. They preserve the native `durable_execution_busy` RPC error and details.
The SDK neither polls automatically nor replays work. Ready is a point-in-time
snapshot, not a reservation, capacity guarantee or authentication; concurrent
changes and final admission remain the Agent's decision.

## Test Evidence

Four ordinary SDK tests cover the five states, missing and malformed fields,
contradictory declarations, bounded reasons and invalid caller-constructed IDs.
Three opt-in process/protocol tests cover:

1. **Real store binding loss (Unix):** a completed original receipt remains tied
   to the same ID; unavailable descriptors reject new submission before creating
   records. Restoring the same test directory permits original receipt readback,
   with one stored task and no additional dispatch.
2. **Real lock contention:** the existing test-only native store-read hook holds
   the actual mutex. Descriptor read promptly returns Busy, new submission creates
   no files, and SDK/example recovery reads retain `durable_execution_busy`,
   unknown execution state and false retry/replay authority. Releasing the hold
   permits explicit inspection and verified export of the unchanged original task.
3. **Protocol-fixture declarations:** a bounded loopback peer starts from an actual
   Agent descriptor, then supplies admission-blocked, disabled, busy, unavailable,
   missing-availability and contradictory variants. Each SDK/example attempt makes
   one descriptor read, no mutation, and no new submission directory. These
   declaration cases are controlled protocol fixtures, not real disk-write-failure
   or disabled-service fault qualification.

The store-read fault hook already existed in the Agent; this turn only configures
its absent-by-default hold file in the isolated test harness. No new production
pause hook, global retry mechanism or Engine/SDK coupling was introduced.

## Validation

- `make test-sdk-modal-recovery`: passed, including 32 opt-in Agent/protocol
  integration tests. The new descriptor-declaration fixture is not counted as
  an actual disk-write-failure experiment.
- The formal `modal-sdk-durable-research` profile executed this target again
  successfully (`executed=true`), without warnings.
- SDK all-target tests: 185 passed, zero failures, 32 ignored in that invocation;
  the 32 were executed separately above. Two ordinary entries are inert child
  fixtures unless their test-only context is supplied.
- All-target Clippy with warnings denied, targeted Rust formatting and
  `git diff --check`: passed.
- Documentation book/inventory and project organization checks: passed, with
  the 800-line source / 2000-line documentation limits and zero tracked debt.
- Tensor structure: passed with zero structural gaps. Overall daji status remains
  `blocked`, with 2 maturity gaps, 23 evidence-grade gaps and 18 P0 gaps.

Runs overlap and are not additive. Evidence remains bounded `verified`, not
installed, authenticated remote, cross-platform or host-power-loss qualification.
No version bump, commit/push, installation or deployment is included.

## Native Write Failure Follow-Up

Date: 2026-10-26. Same local macOS source-built scope and package baseline.
Two new real-Agent regressions extend the descriptor fixture with actual
filesystem `create_new` failures. No production implementation defect was found
in these two paths, and no production code or fault hook was added.

- **Admission write failure:** complete an unrelated original study, then create
  a private, test-owned pending admission file for another ID. The real example
  passes its ready preflight, retains input/intent, and encounters the native
  write failure during dispatch. It preserves `durable_execution_unavailable`,
  creates no false local admission, and leaves the original pending bytes intact.
  Inspection is `not_found`, never an execution handle or permission to retry.
- **Receipt write failure:** prepare a study with an explicit job ID, retain its
  input/intent and admission using the example's record writer, and observe the
  real modal solver at its existing hold checkpoint. A colliding pending receipt
  then prevents publication when the hold is released. The settled execution is
  `outcome_unknown` with no result, resumability or replay authority, not a
  fabricated failed/successful receipt. The preexisting pending bytes survive.
- **Isolation:** both cases wait for the actual native `AdmissionBlocked` state
  and released execution slots. Fresh example submission creates no directory;
  a direct SDK submission bypassing that preflight still gets the native refusal,
  not a safe capacity-rejection classification. Previously completed work stays
  readable, independently exportable and offline-verifiable. Original native
  receipt bytes do not change.
- **Restart:** force-terminate and restart the Agent. Startup removes the pending
  names without promoting their contents. Readiness returns, but the original
  task observation remains exactly missing/unknown and cannot publish a verified
  result. Caller evidence is unchanged. A separately authorized new study under
  a different ID completes and exports successfully; it does not replace or
  resolve the uncertain task.

The tests explicitly count retained executions and zero active work/reservations.
They use isolated temporary stores, not installed services or permanent backups.
These are real file-collision failures, not ENOSPC, permission-denial, failed
directory synchronization, post-publication acknowledgement loss, hostile
filesystem concurrency or physical-power-loss qualification. General disk-fault
coverage is not claimed. The production restart policy and no-replay contract
remain unchanged.

Follow-up validation:

- Both focused native write-failure tests passed and are registered in
  `make test-sdk-modal-recovery` and the formal validation profile.
- `modal-sdk-durable-research`: passed with `executed=true`, including all
  40 opt-in Agent/protocol integration tests.
- Full SDK all-target tests: 200 passed, zero failures. The 40 opt-in tests are
  ignored by this invocation and were executed separately through the profile;
  overlapping ordinary profile tests are not an additional coverage count.
- All-target Clippy with warnings denied, targeted Rust formatting and
  `git diff --check`: passed.
- Documentation book/inventory and project organization: passed; source limit
  800, documentation limit 2000, zero tracked debt.
- Tensor structure and command checks: passed, with zero structural gaps.
  Maturity/evidence gaps are unchanged: 2 maturity, 23 evidence-grade and 18 P0
  gaps, overall daji status `blocked`. The bounded evidence grade was not raised.

No version bump, commit/push, packaging, installation or deployment was made.

## Live Inventory Follow-Up

Date: 2026-10-26. Same local macOS source-built scope and package baseline.
This follow-up changes native admission policy, not operator numerics or the
engine's execution interface.

The recovery index exposed a real accounting gap: after an Agent started with
records absent, live restoration made them readable while its cached entry/byte
counts stayed low. New task admission still reserved execution using those stale
counts. Native negative controls reproduced the same unsafe reservation after
record loss, receipt-size changes and foreign/nonregular directory entries.

The Agent now retains finalized names and encoded lengths from startup and its
own successful commits. Before new execution reservation, it compares bounded
directory metadata with that inventory. Mismatch or read failure latches an
admission fence; it neither reconciles counters nor modifies external records.
Already full cached capacity can still reject before scanning because it does
not permit any new execution. Existing-ID observation does not reserve a slot.

The descriptor exposes `admission_blocked_after_inventory_change`. The Rust SDK
maps a true flag to existing `AdmissionBlocked`, keeping the original store ID.
The optional extension may be absent for older Agents; present null/non-boolean
values fail closed. Descriptors remain snapshots, not a directory rescan or a
reservation. Direct dispatch cannot bypass the native fence by skipping preflight.
The RPC remains `durable_execution_unavailable`, not an attempt-certified capacity
refusal or automatic retry authorization.

The fence isolates fresh work rather than cascading into unaffected executions:
original readable receipts and healthy in-flight completion remain available.
After stopping the Agent and explicitly inspecting/restoring records, restart
rescans accounting. It does not replay old tasks. Restoring bytes while still
running does not clear an already latched fence.

Coverage added:

- Native restored-record accounting, receipt loss, size growth/shrink, foreign
  entries, nonregular files, Unix symlinks, directory-count bounds, sticky fencing
  and restart re-accounting. Tests assert the reservation callback never runs.
- An unchanged-length checksum-corruption control intentionally passes the
  metadata gate while receipt readback fails, pinning the limited validation scope.
- SDK descriptor decoding for the new flag, contradictory/malformed values and
  older descriptors without it.
- Real native modal execution through the SDK: move committed records while
  stopped, start with an empty inventory, restore them live, refuse fresh work
  without new files, retain old result/index reads, restart and verify re-accounting.
- Real in-flight modal execution: grow an unrelated receipt, trigger the fence,
  release the solver checkpoint and independently validate healthy completion.
  Restoring the original bytes permits original result reads but does not lift
  the fresh-work fence. Restart permits separately authorized new-ID research.
- Existing native Mesh regression updated for the stricter shared-store contract:
  spare solver capacity cannot admit new work on the changed local inventory,
  but an earlier committed sibling remains readable and another healthy mesh
  Agent still executes. Restoration/restart preserves the original unknown
  task without replay and permits a separately authorized new identity.

The scan uses keyed metadata lookups, with a 4,097-entry bound, and does not read
or hash large receipts. Pending names count toward that bound but are excluded from
committed-byte accounting; their existing publication-failure and restart-cleanup
policy is unchanged. This is not a total disk quota, immutable-content audit,
defense against concurrent filesystem mutation, physical-power-loss experiment
or cross-platform/installed deployment qualification. No new solver coupling,
automatic replay, deployment or permanent backup is introduced.

Inventory follow-up validation:

- Seven focused native inventory tests and the existing store regressions:
  55 passed. The updated Mesh target passed all four entries (two Mesh scenarios
  and two shared harness checks).
- Full CLI/Agent all-target tests: 921 passed, zero failures, eight ignored.
  Not run: explicit LAN-interface coverage, three disposable-PostgreSQL cases,
  one release microbenchmark and three prebuilt native-operator package cases.
- Full standalone SDK all-target tests: 206 passed, zero failures, 46 ignored
  by that invocation. The formal `modal-sdk-durable-research` profile then
  explicitly ran all 46 opt-in SDK/Agent cases and passed with
  `executed=true`, `ok=true`; it also executes native store and Mesh checks.
- The first new in-flight fixture used its full dispatch, including test-only
  `job_id`, as the fixed physics reference and was correctly rejected by that
  reference's exact input gate. The corrected test keeps the full dispatch bound
  by intent/handle but verifies the solver's typed physical input against the
  unchanged independent model; no numerical acceptance threshold was weakened.
- CLI and SDK all-target Clippy with warnings denied, targeted formatting and
  `git diff --check`: passed.
- Documentation book/inventory and organization checks: passed; source limit
  800 lines, documentation limit 2000 lines, zero tracked organization debt.
- Tensor structure/command checks: passed with zero structural gaps; 2 maturity,
  23 evidence-grade and 18 P0 gaps remain. Overall daji status stays `blocked`;
  this bounded evidence does not raise qualification grades.

These runs overlap; counts are not additive coverage. No version bump,
commit/push, packaging, installed-service restart or deployment was performed.
