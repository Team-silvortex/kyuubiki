# LAN Mesh Discovery and Recovery Boundaries

Date: 2026-10-13. Line: daji 3.6.0.

Local multi-process evidence, not authenticated multi-host qualification.

## Implemented

- Explicit-seed peer discovery remains native and independent of Orchestra, the GUI,
  engine implementation and operator algorithms. Mixed Orchestra/peer authority is
  rejected before durable-store initialization. Exact cluster and offline authority
  checks prevent incompatible descriptors from extending local membership.
- Candidate discovery is bounded to 64 peers, eight probes per round, 1,500 ms per
  total query and 1 MiB per reply. DNS workers are bounded to two; late resolvers
  cannot open a connection. RPC version, response identity and envelope are checked.
  Partial reads cannot renew the total deadline. Endpoint normalization handles DNS
  case, bracketed IPv6, duplicates, invalid destinations and known self addresses.
- A peer hint is neither health proof nor execution permission. Startup seeds are
  unprobed; directly observed peers become healthy, then degraded on failure. Empty
  candidate sets do not silently change the configured mesh authority to standalone.
  Shutdown interrupts interval sleep instead of waiting up to five minutes.
- A read-only `peer_discovery` descriptor publishes the limits and non-guarantees.
  Candidate count is not quorum. Topology is rebuilt after restart; configured seeds
  stay pinned while repeatedly failing learned candidates retire with bounded cooldown.
  No task replication was added.
- The legacy native gateway solve path now discovers/selects before dispatch, and
  dispatches at most once. An uncertain reply or solver error cannot trigger another
  endpoint. If discovery finds no Agent, it does not execute blindly. Durable gateway
  operations remain pinned to one endpoint, independently of discovery.

## Regressions and Experiments

The descriptor test first reproduced acceptance of a reply with the wrong request
identity. The legacy solve regression first reproduced execution on a second peer
after the first peer accepted a solve but closed without a reply. Both failed before
their respective fixes and passed afterward.

The live suite uses actual `kyuubiki-cli agent` child processes on isolated loopback
ports with separate temporary ledgers, no Orchestra and no external services:

1. Three nodes, each seeded with only the next node, converge to the complete ring.
   Each performs a real bar solve with expected stress 10. A fourth task is held on
   the first node, which is forcibly killed. The other nodes observe degradation,
   execute new tasks and never acquire the interrupted task's record. On restart,
   the owner rejoins, retains `outcome_unknown`, rejects automatic replay by duplicate
   identity and completes a distinct new task.
2. Two Agents deliberately receive identical execution/request/job IDs. Their
   different process identities demonstrate Agent-local ownership. Cancelling the
   second with the first's receipt identity fails, and both computations still finish.
   This is not a cluster-wide exactly-once guarantee.
3. A foreign-cluster seed advertises another peer. Repeated probes neither accept the
   seed as healthy nor import its peer hint.
4. An idle Agent configured for a five-minute discovery interval exits successfully
   within a five-second SIGTERM test budget; its peer continues to execute work.

The live binary also includes two shared fixture tests. Unit tests cover authority
mixing, cluster matching, peer limits, round-robin coverage, IPv6 self filtering,
oversized/empty reply headers, malformed responses and trickled replies. Native
gateway tests cover lost replies, solver errors, pre-dispatch selection and no blind
execution after discovery fails.

Broader installed-runtime testing initially failed because its macOS temp fixture
used `/var` through a system symlink. The fixture now canonicalizes the existing temp
parent. Production rejection of symlinked ledger ancestors remains unchanged. All
installed-runtime round-trip and lifecycle-recovery cases passed after this fix.

Test-owned Agents and temporary ledgers are removed by fixture teardown unless an
explicit evidence-retention directory is configured. No backup tree, package rebuild,
server deployment, version bump or Git commit is part of this change.

Verification totals: 611 selected Rust tests passed: 369 CLI unit/live tests, 149
protocol tests, and 93 desktop-runtime tests. One pre-existing whole-stack test remains
ignored. These totals include prior regression coverage, not 611 new Mesh cases or
complete repository coverage. Clippy with warnings denied, Rust formatting, documentation
book/inventory, whitespace and the 800/2000 source/document organization checks passed.
Tensor checks retain zero structural gaps, four maturity gaps, twenty evidence-grade
gaps and fifteen P0 gaps; Daji status remains blocked. No GUI smoke or physical-host
test was run in this round.

## Follow-up: Persistent Execution Owner Fence

An endpoint can stay unchanged while DHCP, a replaced service or a fresh install puts
a different execution ledger behind it. Pinning only endpoint and execution ID was
not sufficient: another ledger can legitimately contain the same caller-generated ID.

The Agent now keeps a random, non-secret `store_id` in the existing exclusive
`owner.lock`, with a version header and checksum. It syncs this identity before
publication; restart preserves it. The reader is bounded to 160 bytes, and malformed,
truncated or checksum-invalid nonempty metadata fails startup without rewriting it.
An empty legacy lock initializes once without modifying admissions or receipts.
Deleting/emptying the lock creates a new identity, not recovery of the old one.

All durable RPC operations accept `expected_store_id`. A foreign guard is rejected
before admission, duplicate lookup, enumeration or cancellation. Reports, index and
admission/cancellation decisions echo the actual identity. Native gateway and frontend
clients validate the echo; a foreign or absent echo is not evidence for clearing the
original caller journal. The task-input digest and immutable task record formats are
unchanged. No model copies, extra per-task files or backup generations were added.

Workbench reserves the original store identity before dispatch in its existing native
journal or IndexedDB. GUI and PWDT derive recovery ownership from saved handles; an
explicit identity cannot override an existing pending handle. Manually tracked tasks
require the original index's `storeId`. Subsequent index pages require the same guard.
Legacy handles without a store identity remain visible and block new submissions,
but cannot be inspected, cancelled or acknowledged by silently adopting today's Agent.

Evidence:

1. A real native Agent in mesh mode admits a held task and is forcibly killed. Another
   mesh Agent takes over its exact port with a different ledger and a deliberately
   identical execution ID. The old guard cannot submit synchronously/asynchronously,
   inspect, enumerate or cancel there, even with the replacement's valid process and
   digest. No new admission appears, and the replacement's held solve completes with
   expected stress 10. Restoring the original ledger restores its identity and unknown
   task without replay; a distinct new task still succeeds. The binary also includes
   two shared fixture tests.
2. Four store tests cover identity persistence, distinct stores, owner rejection before
   the capacity callback, corrupt metadata preservation and empty-lock initialization.
   Protocol tests verify that the owner guard does not alter numerical request digests.
   Gateway tests cover every durable route, malformed request guards, missing/foreign
   response echoes and transient admission decisions. Native journal tests reject
   removal by another identity or an unfenced legacy handle.
3. Frontend tests cover pre-dispatch persistence, service recreation, all five receipt
   states from a foreign store, missing echoes, foreign rejection, legacy handles,
   manual recovery and pagination. Nine isolated browser/PWDT scenarios passed,
   including reload after lost submission, interrupted journal deletion, cancellation,
   worker failure and foreign-owner result rejection. The browser uses a mocked HTTP
   gateway; the real native Agent replacement test is a separate layer.

Follow-up verification: 622 selected Rust tests passed (376 CLI unit/live, 149 protocol,
97 desktop-runtime); one pre-existing whole-stack case remains ignored. The
desktop-runtime total combines the full package run with the final updated library
suite. All 137 focused frontend tests and nine isolated browser tests passed. These
are selected regressions, not full-repository or installed-application qualification.
The browser harness first hit sandbox loopback restrictions; its approved isolated
rerun passed and cleaned its temporary runtime. Frontend typecheck, Clippy with warnings
denied, Rust formatting, book/inventory, whitespace and organization checks passed.

The tensor retains zero structural gaps, four maturity gaps, twenty evidence-grade
gaps and fifteen P0 gaps, with Daji status blocked. The new evidence stays local
`verified`; it does not promote physical Mesh operations to `operational`.

This fence is not authentication or global exactly-once execution. Old raw native
clients may still explicitly omit it and lose its protection; Workbench does not.
Copying a ledger also copies its identity. Deliberate identity spoofing, replicated
ledgers, ownership transfer and partition consensus require separate controls.
No installed package rebuild, remote deployment, version change or commit was made.

## Follow-up: Bounded Native Gateway Recovery Transport

The Agent discovery loop was already bounded, but the native GUI gateway had a
separate transport that still used per-read socket timeouts. Partial TCP fragments
could renew its wait indefinitely, and streamed progress was retained without a
total retention bound. The ordinary terminal response path also accepted wrong RPC
versions and contradictory result/error envelopes. Two new regressions first failed
against that implementation: wrong-version success and progress on a control RPC.

The gateway now uses the shared Rust protocol validators, without depending on the
engine or operator SDK. Describe, async durable submission, receipt lookup, list and
cancel use one 15-second exchange budget across DNS capacity wait, DNS, connect, write
and read. Control replies cannot contain progress. Header and payload reads share
the same deadline; oversized lengths are rejected before payload allocation.

DNS work is bounded to four outstanding resolvers, including work that outlives its
caller. Capacity wait consumes the original deadline; completion returns the slot.
Late resolver work never connects. At most eight resolved addresses are considered,
each with a 500-ms connect cap, and non-unicast destinations are skipped. Literal IP
connections do not require a DNS slot. Metadata replies have a 1-MiB encoded frame
cap, durable submission/receipt replies 9 MiB, and legacy solver replies 64 MiB.

Legacy solves deliberately retain a per-complete-frame 15-second budget, not a total
15-second job deadline. Valid long-running progress remains supported, but fragmented
frames cannot prolong one frame indefinitely. Returned telemetry contains only the
newest 128 valid frames, with at most 1 MiB of normalized encoded progress. The
numerical result is not truncated. The read-only `kyuubiki.direct-mesh-transport/v1`
policy is exposed in discovery and solve responses and typed in the frontend contract.
No persisted files or backup generations are introduced.

Eleven additional tests cover protocol rejection, every control method, fragmented
headers and payloads, shared control deadlines, a healthy solve lasting longer than
the control budget, header-only oversize replies, 512-frame count/byte retention,
invalid progress, stalled-peer isolation, late DNS slot ownership/queued reuse, and
visible policy even when an Agent is unavailable. These gateway tests use controlled
TCP peers and shortened test budgets, not a throughput benchmark. A macOS fixture
race was also exposed: the fake peer accepted sockets from a nonblocking listener
without explicitly clearing nonblocking mode. That test fixture now selects blocking
I/O before reading; production client timeouts were not relaxed to make it pass.

Verification completed on 2026-10-15: 266 Rust tests passed (108 desktop-runtime, 149 protocol and nine
tests in the real-Agent LAN mesh/owner-replacement binaries), with one pre-existing
whole-stack case ignored. The 137 focused frontend tests and typecheck passed.
Formatting, warning-denied Clippy, documentation and organization/tensor checks were
also run. These totals are this follow-up's selected suites, not a claim that every
repository test was rerun. The browser scenarios above were not rerun in this follow-up.
The three native dispatch regressions also passed five consecutive reruns after the
macOS fixture correction; repeats are not counted as additional unique tests. The
tensor still reports zero structural gaps, four maturity gaps, twenty evidence-grade
gaps and fifteen P0 gaps, with Daji status blocked. No physical-LAN readiness upgrade
is inferred from this local verification.

Remaining transport boundaries: system DNS cannot be cancelled; all four stuck DNS
workers can exhaust hostname resolution capacity until they finish. TCP peers are
still trusted-network endpoints. Continuous valid solver heartbeats can keep a legacy
request alive; bounded progress memory is not a whole-job deadline or proof of
numerical progress. Encoded caps are not exact heap caps. A timeout or malformed reply
does not imply that execution or cancellation did not happen, and never authorizes
replay, terminal acknowledgement or deletion of the original durable handle.

## Follow-up: Learned Candidate Retirement

The bounded discovery table could stay full of failed learned addresses forever.
A new three-Agent regression first failed on the old implementation: after killing a
learned task owner, the observer had recorded thirteen failed probes but still retained
the address. Gossip from another peer could keep advertising that stale candidate.

The discovery-only candidate pool now distinguishes configured seeds from learned
addresses. Seeds remain until reconfiguration. Three consecutive failed direct probes
retire a learned address and discard only its local discovery health/failure metadata.
The address enters a 60-second monotonic cooldown, which hints cannot reset or bypass.
A compatible direct success before retirement resets the failure sequence. Expiry
alone does not resurrect a peer: a later hint may readmit it as unprobed, and a new
direct compatible reply is required before healthy status is restored.

At most 64 candidates and 128 retirement cooldown entries are retained. If the latter
is full, further failed candidates remain visible until a slot expires; existing
cooldowns are never shortened to accept churn. Probe rotation and all earlier RPC
bounds remain in place. The policy is exposed in `describe_agent.peer_discovery`.
No execution ledger, task receipt, pending recovery handle or operator state is
deleted. No persistence files, backups, dependencies or task migration were introduced.

Six deterministic unit tests cover a full 64-entry pool, seed/healthy-peer retention,
consecutive failure reset, untrusted advertised health, canonical-address cooldown
boundaries, a saturated 128-entry cooldown table, probe rotation after removal, unknown
observations and saturating failure counters. Simulated monotonic instants avoid
waiting a minute or changing the machine clock. A real three-Agent process test
proves removal on the learning observer, seed retention on its neighbor, cooldown
despite renewed healthy gossip, a new solve on the observer, and fenced inspection of
the original killed task after its owner restarts. The task stays `outcome_unknown`;
duplicate submission cannot replay it and no peer acquires its ledger record.

Verification completed on 2026-10-15: 528 selected Rust tests passed (16 CLI library,
245 CLI binary unit tests, seven LAN Mesh live tests, three owner-replacement live
tests, 108 desktop-runtime and 149 protocol tests). One pre-existing whole-stack
test remains ignored. Seven tests were added in this follow-up; the totals include
existing regressions and shared fixture tests. The live suites passed again after
the final code changes. Warning-denied Clippy for the CLI/library/live targets,
formatting, whitespace, book/inventory and organization/tensor checks passed.
The tensor remains at zero structural gaps, four maturity gaps, twenty evidence-grade
gaps and fifteen P0 gaps, with Daji status blocked; this remains local `verified`
evidence, not an `operational` upgrade. Frontend/browser and installed GUI tests were
not rerun for this Agent discovery-only change. No package, deployment, version
change, commit or push was made.

These are local tests, not physical multi-host qualification. A fully configured
64-seed list, persistent topology churn, DNS trouble or a long probe interval can
still delay discovery. Failed probes are query failures, not proof that the remote
process has stopped. The health score describes retained candidates only; pruning
a failed hint can increase it without restoring a lost task. Cooldowns and topology
remain process-local; restart reconstructs them from the configured seeds.

## Follow-up: Probe Fairness During Candidate Churn

Candidate retirement exposed a weakness in the old sorted-list/index scheduler.
Removing elements before its saved cursor skipped surviving waiters. Two new tests
failed before the fix: a retained seed waited eleven rounds between direct probes
while failed candidates were removed, and a newly learned address sorting ahead of
the cursor caused an already-probed seed to overtake a still-unprobed seed.

The candidate pool now owns a bounded FIFO probe queue separate from its sorted
descriptor list. Each selection moves to the tail. Newly admitted/readmitted addresses
join the tail; retiring an address removes only that queue entry, preserving all
remaining relative order. Display order is still deterministic. The old array-index
rotation helper was removed rather than retained as a second scheduling mechanism.
The read-only discovery policy exposes the order and a maximum of eight complete
running rounds before selecting a continuously retained candidate. This is a scheduling
bound, not a wall-clock deadline, quorum, task-recovery or execution-placement guarantee.

Three new deterministic tests cover removal-induced delay, hints that sort before
the old cursor, and 1,000 rounds of bounded candidate churn. The latter checks exact
queue/candidate membership, unique probes within each round, continued seed fairness,
sorted descriptors, cooldown exclusion and the existing 64/128 capacity limits. It
uses simulated monotonic instants and is not a 1,000-node test or throughput benchmark.
Existing real-Agent tests also check the published probe policy. No task ledger,
engine, operator, durable identity or cross-peer dispatch behavior is changed.

Verification completed on 2026-10-15: 531 selected Rust tests passed (16 CLI library,
248 CLI binary unit tests, seven LAN Mesh live tests, three owner-replacement live
tests, 108 desktop-runtime and 149 protocol tests). One pre-existing whole-stack
test remains ignored. Three tests were added in this follow-up; the total includes
existing regressions and shared fixture cases. Warning-denied Clippy for the
CLI/library/live targets, formatting, whitespace, book/inventory and organization
checks passed. The tensor still reports zero structural gaps, four maturity gaps,
twenty evidence-grade gaps and fifteen P0 gaps, with Daji status blocked. Evidence
remains local `verified`, not a physical-LAN `operational` upgrade. No frontend/browser,
installed GUI or multi-host test was run in this follow-up; no version bump, package,
deployment, commit or push was made.

No new dependency or persistent file is required. Slow peers, the discovery interval,
DNS resolution, shutdown and restart remain separate timing/recovery boundaries.

## Follow-up: Slow-Peer Isolation and Incremental Publication

The discovery loop previously probed its eight selected peers serially and published
health only after the complete round. A healthy reply could therefore stay invisible
behind seven stalled requests. A new real-Agent regression first reproduced this:
the healthy seed sorted first, but was not reported healthy within three seconds.

Discovery now has at most two scoped probe workers, matching its existing DNS worker
limit. The coordinator owns all candidate and runtime mutations and publishes each
completion immediately. The result channel holds at most one eight-probe round.
Selection stays FIFO, display stays sorted, and completion order is allowed to differ.
No next round starts before every worker in the current round has joined. Requests
retain their existing total deadline, frame bound and authority/cluster validation.

Worker creation failure leaves the remaining worker to drain the selected queue;
if none starts, a bounded serial fallback avoids skipping selected candidates.
An unwinding request panic becomes a failed probe without dropping other results.
Once stop is observed, queued requests do not start and late results are not published.
Shutdown joins the in-flight probe workers; underlying system DNS workers remain
separately bounded and non-cancellable. These choices are visible in the read-only
discovery policy, not hidden execution-engine or operator behavior.

Seven unit tests cover early publication behind a blocked probe, two-probe/eight-peer
bounds, exactly one probe per selected peer, stop/queue/join semantics, request errors
and unwind isolation, zero/single-worker availability, stop-aware serial fallback and
empty/stopped rounds. Worker availability is injected; no system-wide thread resource
exhaustion is induced. The real-Agent case uses two native Agents and seven listeners
that do not reply. It now observes the healthy peer within its three-second test
budget, completes a real local bar solve, exits on SIGTERM within five seconds while
discovery is active, and completes another solve on the surviving Agent. These are
test budgets, not deployment latency guarantees or a network throughput benchmark.

Verification on 2026-10-15: 539 selected Rust tests passed (16 CLI library, 255 CLI
binary, eight LAN Mesh live, three owner-replacement live, 108 desktop-runtime and
149 protocol). One pre-existing whole-stack case remains ignored. Eight new tests
were added; totals include existing regressions and shared fixtures. Warning-denied
Clippy passed for CLI/library/live targets. No frontend/browser, installed GUI or
physical multi-host tests were run, and no package, deployment, version bump or Git
commit was made. Candidate retirement, durable owner fencing and no cross-peer replay
remain unchanged. Test fixtures remove their own Agents and temporary ledgers.

Formatting, whitespace, documentation book/inventory and the 800/2000-line
organization checks passed. The tensor check retains zero structural gaps, four
maturity gaps, twenty evidence-grade gaps and fifteen P0 gaps; Daji status remains
blocked. The claim stays local `verified`, not physical-LAN `operational` evidence.

Two slow probes can still occupy both slots and delay queued candidates. Normal
worker availability improves isolation, not strict wall-clock fairness. DNS capacity
and configured intervals remain relevant. This local evidence does not qualify
authenticated LAN operation, partition consensus, task replication or workflow migration.

## Follow-up: Bounded Native Request Ingress

Outbound probing was bounded, but the native listener still allocated from an
unchecked four-byte request length, waited without an inbound deadline, and spawned
one thread per accepted socket without an application cap. A handler-spawn error
also propagated to the listener shutdown trigger. The new real-Agent regression
first failed safely with a 4,097-byte header and no body: the old implementation
waited rather than rejecting the configured 4,096-byte maximum. No large allocation
or resource-exhaustion attack was used to reproduce this issue.

The shared native ingress layer now validates length and reserves aggregate raw
request capacity before allocating. Defaults are 256 MiB per encoded request,
512 MiB aggregate retained encoded requests, 64 connection handlers and 30 seconds
total for each request's idle/header/body reads. Partial reads do not renew the
deadline. The budget does not run through solver execution. Capacity and timeout
environment variables are strictly validated before ledger initialization; effective
limits and current usage are published in describe and registration/heartbeat
`request_ingress` (`kyuubiki.agent-request-ingress/v1`). Changes require restart.

Connection/byte permits release on normal completion, error or unwinding. Frame
reservations remain held through dispatch and reply, not only network reception.
Excess sockets close without an application queue or a new handler. Handler creation
failure drops its socket/permit and leaves the listener serving. Empty, oversized,
truncated, timed-out or over-budget frames never reach RPC dispatch. A complete
invalid JSON frame retains the existing error response and subsequent framing.
No execution engine, operator, ledger format or automatic replay policy changed.

Eight new unit tests cover configuration bounds, zero/oversized headers (including
u32::MAX without allocating its body), aggregate reservation/return, truncated reads,
unwind cleanup, nonrenewable idle/header/body deadlines, persistent connections,
checked counter arithmetic and concurrent admission. Failed-spawn cleanup is modeled
by dropping an unstarted closure, not by exhausting host thread resources.
Four new real-Agent cases cover header-only rejection followed by a real bar solve,
connection saturation with a surviving control session and reusable slots, idle/
partial-header expiry, and complete malformed JSON followed by valid requests.
The Mesh case fills a small byte budget with an incomplete frame containing otherwise
valid durable-submission JSON. A second large header is rejected, the first expires,
the task remains `not_found`, capacity returns, and both local and peer solves succeed.
The live binary includes two existing shared readiness-fixture tests.

Verification on 2026-10-15: 614 selected Rust tests passed: 357 CLI unit/live tests,
108 desktop-runtime and 149 protocol tests. One pre-existing whole-stack case remains
ignored. Twelve tests were added; the total includes existing and shared fixture
cases, not complete repository coverage. Warning-denied Clippy passed for the selected
CLI/library/live targets. Formatting, whitespace, documentation book/inventory and
800/2000-line organization checks passed. The tensor still reports zero structural
gaps, four maturity gaps, twenty evidence-grade gaps and fifteen P0 gaps, with Daji
status blocked. The new ingress claim remains local `verified`. No frontend,
installed GUI, physical LAN or million-node benchmark was rerun for this change.
All stress inputs here were deliberately small; fixture-owned Agents and ledgers are
removed. No version change, package, deployment, backup tree, commit or push was made.

These are encoded-byte/handler/receive bounds, not decoded heap, solver-memory or RSS
limits. All control and discovery RPCs also consume ingress capacity; saturation can
still deny new controls until slots return. The raw listener is not authenticated,
and these controls do not prove resilience against sustained hostile load. A generic
client disconnect still cannot prove non-admission or authorize cross-peer replay.

## Follow-up: Dispatch Eligibility Is Not Recovery Readiness

A gateway regression reproduced a control-layer bug: successful discovery was
treated as eligibility for new legacy solves. A quiescent peer with a higher health
score or earlier endpoint position could receive the solver request, despite another
peer being ready. The native Agent could reject that request, but the gateway had
already chosen the wrong target. Dispatch failures must not cause cross-peer replay.

The gateway now filters candidates before health/order ranking for each new legacy
task. It requires the current solver RPC transport/protocol and advertised method,
an explicit valid accepting lifecycle, configured execution capacity and a bounded
health score. Missing metadata is not filled with an implicitly accepting descriptor
default. Lifecycle and admission counts come from separate snapshots, so their
maximum is used conservatively. No eligible candidate yields explicit reasons and
no solver dispatch. `kyuubiki.direct-mesh-selection/v1` is exposed read-only through
discovery, solve success and solve error responses. This is not a slot reservation:
the Agent remains responsible for final atomic admission after discovery races.

Six pure contract tests and four controlled-TCP regressions were added. They cover
missing/malformed metadata, protocol/method compatibility, lifecycle consistency,
capacity bounds and independently observed counts, bounded health ranking, skipping
busy/draining/quiescent/unsupported peers in both selection modes, zero dispatch when none is
eligible, and fresh capacity reads across three distinct tasks. The recovery case
discovers a busy or quiescent owner, rejects new legacy work, then inspects the pinned
original execution with its store fence. It receives `outcome_unknown` without a
solver request, owner reselection, automatic replay or gateway result-cache entry.
These gateway cases use controlled framed-TCP peers, not physical LAN deployments.

Review also found that the earlier partial-frame live test used an invalid durable
RPC method name. The name is corrected to `submit_durable_agent_task`, the request
must deserialize as a typed RPC, and the exact same body is now deliberately sent as
a complete frame afterward. The incomplete frame leaves no record; the complete
one produces a durable completed real bar result (`max_stress = 10`). The test also
reconnects its monitor after waiting for frame expiry, rather than racing the
monitor's own legitimate idle timeout. The two-Agent live case now proves actual
non-admission followed by successful execution, not merely rejection of a bad method.

Verification on 2026-10-15: 624 selected Rust tests passed (357 CLI unit/live,
118 desktop-runtime and 149 protocol), with one pre-existing whole-stack case ignored.
Ten gateway tests were added; totals include existing regressions/shared fixtures,
not repository-wide coverage. Execution algorithms, operator contracts, durable
ledger identity and pinned recovery routing were not changed. No version bump,
package, deployment, backup tree, commit or push was made.

Warning-denied Clippy passed for desktop-runtime all targets and the selected CLI
library/binary/live targets. Formatting, whitespace, documentation book/inventory
and the 800/2000-line organization checks passed. The tensor remains at zero
structural gaps, four maturity gaps, twenty evidence-grade gaps and fifteen P0 gaps;
Daji status stays blocked and Mesh evidence stays local `verified`. No browser,
installed GUI, physical multi-host or million-node qualification was rerun.

## Follow-up: Recovery Capacity Under General Ingress Exhaustion

A real-Agent regression reproduced a remaining failure with only a 4-KiB declared,
unfinished request: once the configured general byte budget was occupied, a new
inspection/description connection reset before its method could be read. Execution
slot isolation was insufficient because ordinary and recovery traffic shared the
same unclassified connection/encoded-byte admission pool.

The native listener now provides a fixed read-only recovery reserve on the same port:
four additional connection handlers and four concurrent reserved requests, each at
most min(configured frame bytes, 16 KiB). Reserved encoded bytes cannot exceed four
times that frame bound. General configured limits remain unchanged; the effective
total limits and separate usage counters are visible in `request_ingress`, including
`recovery_reserve` (`kyuubiki.agent-recovery-ingress/v1`). Defaults are therefore 68
total handlers and 512 MiB + 64 KiB retained encoded input, not an undocumented
unlimited fallback. There is no waiting queue, dependency or extra listener.

A connection admitted from the reserve remains control-only until it closes, even
if general capacity returns. General byte overflow can borrow a reserved frame, but
that frame is control-only. Reserved receive time is min(configured timeout, 1,000 ms)
from the original frame-read start; classifying a fallback or receiving a fragment
does not restart the clock. Permits remain owned through reply and release on errors,
timeout, closure or unwind. Large frames and full reserves reject before dispatch.
Only the published ping/description, lifecycle, result lookup and cancellation
allowlist can dispatch. Solver, TaskIR and durable submission, including duplicate
submission, receive `agent_recovery_ingress_only` and close without task admission.
Existing RPC validation, loopback-only lifecycle mutations and recovery fences are
unchanged. No task retry, owner transfer or durable-format migration was added.

Four new unit cases cover independent request/byte caps, unwind release, immutable
connection classification, reserved frame bounds, shortened deadlines and an
allowlist checked against all advertised methods. Three new real-Agent cases cover:

- General byte exhaustion while a real durable task is running: inspect/list still
  work, a foreign store fence fails, new submission creates no ledger record, and
  exact-owner cancellation produces a failed/cancelled receipt. After freeing capacity,
  explicitly submitted new work completes; the cancelled receipt survives restart.
- A Mesh-mode owner is forcibly killed with admitted work and restarted using its
  original ledger. With general connection capacity occupied, its reserved connection
  still reads the fenced `outcome_unknown` receipt and performs drain/resume while
  rejecting a foreign controller. The socket cannot submit work after general
  capacity returns. Both native Mesh Agents solve afterward; uncertain work stays
  uncertain rather than being replayed on either peer.
- General plus all four reserved connections reach the published total cap. A further
  connection is closed. Reserved idle sockets expire without restarting the Agent,
  and a fresh lifecycle query can reuse the returned capacity.

The existing partial-frame test now uses a 32-KiB frame/48-KiB general budget so its
second oversized-for-reserve header still tests immediate rejection. It retains the
typed legitimate durable request and full-frame positive control from the previous
follow-up. All stress inputs remain deliberately small and fixture-owned ledgers and
processes are removed after testing.

Verification on 2026-10-15: 633 selected Rust tests passed (366 CLI unit/live,
118 desktop-runtime and 149 protocol), with one pre-existing whole-stack case ignored.
Seven new tests were added; the total also includes two shared readiness cases in the
new live binary and existing regressions. This is not complete repository coverage.
No frontend, installed GUI, physical multi-host or power-loss qualification was run.
The engine, operators and durable record format remain unchanged. No version change,
packaging, deployment, commit, push or extra backup tree was made.

Warning-denied Clippy passed for the selected CLI library, binary and live-test
targets. Formatting, whitespace, documentation book/inventory and source/document
line-limit checks passed. Tensor structure remains complete with four maturity gaps,
twenty evidence-grade gaps and fifteen P0 gaps; Daji status remains blocked. The
updated ingress evidence stays local `verified`, not operational LAN qualification.

This reserve improves recovery under general-pool saturation, but is not authenticated
priority admission or a guarantee under sustained hostile load. Unknown small frames
can briefly consume reserve capacity until validation or timeout. Recovery handlers
and reply delivery can themselves be slow; their execution/write boundaries remain
separate from receive deadlines. Complete reserve exhaustion can still deny controls.
The earlier no-reserved-capacity limitation is superseded only within these bounds.

## Follow-Up: Store Contention Must Not Hide The Agent

The ingress reserve exposed the next blocking boundary: descriptor generation and
durable recovery used the same unbounded execution-store mutex as admission, receipt
serialization and disk commit. One slow store holder could therefore stall Mesh
description/heartbeats and consume recovery handlers. A new real-process fixture
first reproduced this: a held fetch caused description to exceed its two-second
socket deadline before the fix.

The Agent now publishes `durable_execution.store_access`
(`kyuubiki.agent-store-access/v1`). Description uses immediate `try_lock`; contention
reports `enabled: true`, `available: false`, `unavailable_reason: store_busy`, without
inventing counters or identity. Poisoned locks remain unavailable, never bypassed.
Fetch/list/exact cancellation share a 250-ms monotonic mutex-acquisition budget.
Expiry returns `durable_execution_busy`, with `operation_started: false`,
`execution_state_known: false` and `identity_check_performed: false`. These describe
the recovery request only, not the original task. There is no delayed action queue,
automatic recovery retry, task replay or peer reselection. Admission/receipt writes
retain their serialized durability boundary and the stored record format is unchanged.

Five focused unit cases cover uncontended access, simultaneous expired waiters with
no late mutation, release within budget, fail-closed poisoning and published policy.
The local live fixture starts two converged offline-Mesh Agents. It holds one real
bar task and a fetch's store lock, then checks prompt description and explicit busy
errors from inspect/list/cancel. After release the task is still running: the expired
cancel did not execute later. A new explicit cancel produces a persisted failed
receipt, which remains identical after force-killing and restarting the original
Agent. The peer has neither that execution nor an active owner. The native gateway
regression checks that all three busy recovery routes fail without an empty index,
missing-task result, cancellation acknowledgement or cached success.

The disabled-by-default fault hook uses
`KYUUBIKI_AGENT_FAULT_INJECTION_STORE_READ_HOLD_FILE`: an explicit absolute path,
regular file at most 128 bytes, exact execution ID and a ten-second hold budget.
Unix opens reject final symlinks and avoid FIFO-open blocking. Removing the marker
releases the fixture; owned processes, threads and temporary stores are cleaned up.
This is injected lock contention, not a physically stalled disk or power-loss test;
fault-file I/O itself has no enforced deadline.

The first broad regression also caught an old test assumption: the status poller
asserted success on every inspection during large receipt serialization. Its bounded
read-only polling loop now accepts only the explicit busy error, validates its
no-action/no-known-state/no-replay details and retries the same Agent/execution query.
Other RPC failures still fail the test. Submission, cancellation and single-shot
fetch helpers have not gained automatic retries or relaxed result assertions.

Verification on 2026-10-15: 642 selected Rust tests passed (374 CLI unit/live,
119 desktop-runtime and 149 protocol), with one existing desktop-runtime test
ignored. This includes the large-response, synchronous/asynchronous durability,
TaskIR, owner-fence, Mesh discovery, shutdown and ingress suites. These are scoped
regression results, not repository-wide coverage or benchmark qualification.
Tensor structure remains complete with four maturity gaps, twenty evidence-grade
gaps and fifteen P0 gaps; Daji status remains blocked. The new evidence claim is
local `verified`, not operational multi-host qualification.

Warning-denied Clippy passed for the selected CLI targets and desktop-runtime
library/tests. Formatting, whitespace, documentation book/inventory and the
800-line source / 2,000-line document organization checks passed.

The wait budget covers only the store mutex acquisition. Filesystem work after
acquisition, other locks, scheduler delays and reply delivery remain separate.
A stuck holder can still stall itself and writers, and repeated contention can
starve recovery success. Do not infer a total-operation latency SLA, disk-fault
isolation, task migration or automatic recovery from this change. No version bump,
commit, push, package, deployment or extra backup tree was made.

## Follow-Up: Recovery Errors Across The Gateway And Workbench

Following store contention through the caller found three contract/UX gaps. The
native gateway discarded the RPC error code. Workbench mistook busy metadata with
no store identity for an old Agent needing an upgrade, and could accept unavailable
metadata if an identity was present. The generic request-error helper also labelled
direct submission/cancellation timeouts and HTTP 5xx as retryable operations. Current
recovery callers did not automatically replay them, but that advice was unsafe for
future consumers and misleading to operators. New regressions reproduced the missing
code, wrong preflight diagnostic and retry classification before their fixes.

The native transport now retains a bounded diagnostic code after checking the RPC
envelope, live request identity and deadline. Durable submit/inspect/list/cancel
routes keep HTTP 502 with the original `error` message and nullable `agent_error_code`.
Only 1..128 ASCII alphanumeric/underscore/hyphen/period characters are retained.
Transport failures, malformed envelopes, invalid receipt shapes and rejected codes
do not claim an Agent error code. The discovery transport policy exposes these rules.
Peer error details are deliberately not forwarded: even a peer suggesting retry or
migration cannot overwrite `automatic_retry: false`, `automatic_replay_authorized:
false` or original-Agent inspection guidance. Codes remain diagnostics, not task
state, ownership checks, terminal receipts or non-admission evidence.

Workbench carries bounded codes through `WorkbenchRequestError.agentErrorCode`.
Direct mutations (durable submit, exact cancel and legacy solve) are not generic
retryable requests after HTTP/network/timeout failure, including reverse-proxy-prefixed
URLs. Inspect/list retain explicit read-only retry semantics on the saved owner.
There is no new automatic retry. Preflight distinguishes `direct:store_busy`,
`direct:store_unavailable` and `direct:store_write_failed`, rejects before reservation
or dispatch, and never erases an earlier pending identity. Missing legacy identity
remains a separate fail-closed owner-fence error.

Verification on 2026-10-15:

- 644 selected Rust tests passed: 374 CLI unit/live, 121 desktop-runtime and 149
  protocol, with one existing desktop-runtime test ignored. Gateway controlled-TCP
  cases cover diagnostic propagation, contradictory/invalid replies, refused
  connections, bounded codes and untrusted retry advice. The original two-Agent
  contention and forced-restart fixture was rerun as part of the CLI selection.
- 133 selected frontend unit tests passed. They cover preflight classification,
  HTTP-to-error propagation, no extra fetch, intact pending handles after failed
  inspect/cancel, explicit later terminal cleanup and existing owner/authority fences.
- 10 Chromium GUI/PWDT tests passed using an isolated source Workbench and mocked
  services. The added case reloads IndexedDB-backed pending state, observes busy
  metadata and failed inspection without resubmission or result replacement, then
  explicitly recovers the same owner receipt and clears only that handle. The first
  sandboxed attempt failed in setup because localhost listen was denied; the approved
  local rerun passed all cases. The fixture closes its browser/services and removes
  its temporary workspace rather than retaining a build/archive copy.
- Frontend typecheck and source line guard passed. Tensor structure still has zero
  structural gaps, four maturity gaps, twenty evidence-grade gaps and fifteen P0 gaps;
  Daji remains blocked. Evidence remains local `verified`.

These are layered checks, not one installed-WebView-to-real-Agent end-to-end test.
The browser backend is mocked; no new physical LAN, sustained-contention, Windows,
power-loss or stalled-device qualification is claimed. No engine/operator algorithm,
durable record format, replay policy, version, commit, push or installation changed.

## Follow-Up: Lost Cancellation Replies And Late Feedback

Tracing cancellation across the caller found a same-execution feedback race. The
ordinary cancel controller checked only the job polling epoch. Terminal observation
does not change that epoch, so a delayed cancellation acknowledgement or network
failure could overwrite a newer completed/failed message. Four deterministic unit
cases first reproduced the overwrite and then passed with the fix.

The controller now uses the existing workspace feedback ownership guard alongside
the polling fence. Newer feedback wins even when job/execution identity is unchanged.
Cancellation still returns its own success/failure to its caller, keeps observation
alive, never claims a terminal outcome from acceptance, and never clears pending
identity metadata on a lost reply. No new state subsystem or engine/operator coupling
was introduced. Ordinary Orchestra cancellation retains its authoritative job/result
updates while observing the same feedback guard.

The new native `agent_mesh_cancel_recovery_live` target uses two real Rust Agent
processes with converged offline Mesh discovery and temporary persistent stores:

- A test-only loopback relay forwards cancellation, observes the original Agent's
  acknowledgement and drops every response byte before the caller can receive it.
  After the failed/cancelled receipt is synced, the Agent is force-killed and restarted.
  Exact-owner inspection returns the same receipt, without replay or peer migration.
  An explicit later cancel for the old execution cannot cancel a fresh execution
  reusing the job/transport names.
- A truncated cancellation frame is disconnected before complete reception. The
  still-running Agent is force-killed, then restarted from the original store.
  The admission is `outcome_unknown`, not cancelled; explicit duplicate lookup does
  not run it again, and the peer can complete independent work without inheriting it.

These are two concrete interruption boundaries, not proof that every cancellation
instruction or filesystem crash boundary has been exhaustively tested. The first
case waits for receipt persistence before killing; it does not claim that in-memory
cancellation intent alone survives termination. Fixtures own/clean their processes,
stores and sockets and introduce no additional production fault-injection hook.

Verification on 2026-10-15 includes 378 selected CLI unit/live tests (the new target
has two scenario tests plus two shared fixture checks), and 257 selected frontend
unit tests. The isolated Chromium GUI/PWDT suite has 15 cases, including four late
cancel success/failure versus terminal success/failure races and a lost-cancel reply
with IndexedDB reload, unknown-state retention and later exact terminal cleanup.
Its initial new cases failed because the test did not accept the required cancel
confirmation dialog; after fixing the fixture, all 15 passed. These browser checks
use mocked services and are separate from the real native process tests.

The final rerun passed all 15 cases with cancellation-specific terminal fixtures.
Another 12 selected centralized cancellation browser cases passed, covering late
responses, contradictory state, history refresh failure and overlapping request
deduplication. Desktop-runtime/protocol added 270 passing tests with one existing
ignored test, bringing the selected Rust total to 648. Frontend typecheck, the new
native target's warning-denied Clippy, formatting, whitespace, documentation
book/inventory and project organization checks passed. All touched source/test files
remain below 800 lines; documents remain below 2,000. The tensor still reports zero
structural gaps, four maturity gaps, twenty evidence-grade gaps and fifteen P0 gaps;
Daji readiness remains blocked rather than being promoted by local test counts.

Evidence remains local `verified`, not installed-WebView-to-real-Agent, physical
multi-host LAN or power-loss qualification. No durable format, replay rule, numerical
algorithm, version, commit, push, packaging or installation changed.

## Follow-Up: Caller Context Isolation During Recovery

GUI/PWDT recovery previously read execution/process/digest fields from the caller's
mutable payload after awaiting the pending journal. Reusing the object could switch
inspection/cancellation to another execution while retaining other original fields.
The handler now snapshots supported scalar parameters at entry, before any await.

The shared API authority guard previously compared only effective URLs and headers.
With no configured token, direct-to-Orchestra and reverse mode changes could leave
both unchanged. The guard now includes control mode independently of credentials.
An observed mismatch or unreadable authority also latches invalidation: restoring
configuration does not revive that request. This is not a global configuration epoch;
an A-to-B-to-A change entirely between checks remains undetected. No guarantee of
automatic cancellation, task migration or replay is added.

Six new assertions failed against the old implementation: two mutable-payload
targets, two token-free mode transitions, restored gateway after observed invalidation
and restored storage after a failed authority check. All pass after the two focused
production changes. Additional gated tests cover context loss during journal lookup,
late running/terminal responses and fresh explicit cleanup of the original handle.

Verification on 2026-10-15:

- 475 selected frontend unit tests passed across recovery/run/request/feedback and
  shared library/admin-controller boundaries. The two selections have 274 and 201
  tests respectively; the focused 17-test rerun is included, not counted again.
- All 17 isolated Chromium direct recovery cases passed. Two new cases hold a real
  browser HTTP response during ordinary execution or manual recovery, change mode
  through PWDT, then release a terminal receipt. The retired operation rejects;
  model/result remain unchanged and the original IndexedDB handle remains. Returning
  to direct mode cannot submit replacement work; a fresh explicit exact inspection
  alone clears the handle. Neither case sends cancellation or a second submission.
- Another 12 centralized cancellation browser cases passed. These 29 browser cases
  use mocked services and are not installed-WebView-to-real-Agent qualification.
- Frontend typecheck and the 800-line source guard passed. Evidence is attached to
  the existing caller-journal and Mesh isolation claims without grade promotion.
- Documentation book/inventory, project organization and whitespace checks passed.
  Tensor validation still reports zero structural gaps, four maturity gaps, twenty
  evidence-grade gaps and fifteen P0 gaps; Daji readiness remains blocked.

No Agent, engine, operator, durable format, version, packaging, installation or Git
history changed in this follow-up. Earlier native process results above remain
separate evidence; they are not counted as new runs for this caller-side change.

## Follow-Up: Managed Configuration Roundtrip Fences

The preceding snapshot limitation is now closed for this JavaScript realm's managed
configuration writers. A request can be suspended while settings change A-to-B-to-A;
the final URL/header/mode values alone no longer let it regain ownership.

Five fixed current in-memory revision identities track mode, effective backend and
the three governed API credentials. They retain neither secrets nor a change history,
have no observer registry or polling timer, and add no persistent data. Settings
persistence, legacy settings scrubbing and explicit settings cleanup track mode and
effective target changes. The backend setter and in-memory secret setter participate.
Synchronous mutations are checked in `finally`, including an injected partial write
that changes storage before throwing. A failed write that changes nothing does not
invalidate a still-readable unchanged authority.

Header construction and revision dependencies share the existing governance routing
rules. Relevant credential addition/removal invalidates even when the request started
with no token. A configured cluster token masks the control-plane fallback until it
is removed. Theme/language, normalized no-op writes, unused credentials, unchanged
absolute request URLs and persisted gateway edits masked by query precedence do not
spuriously retire the request. No engine/operator or Agent-side protocol change is
needed for this caller-side fence.

Verification on 2026-10-15:

- The initial 14-test regression selection produced 11 failing cases against the
  snapshot-only implementation. It now has 17 passing cases, including additional
  gateway write failures and a governed-header route/fallback matrix.
- 510 selected frontend unit tests passed in the final combined run. This includes
  the new revision tests plus recovery, ordinary execution, API errors, settings,
  secrets, storage cleanup and shared admin/library controller coverage.
- 19 isolated Chromium direct recovery tests passed. The two new roundtrip cases
  hold an inspection HTTP reply for an ordinary run or manual recovery, switch to
  Orchestra and back through PWDT, and only then release the completed receipt.
  The old request rejects; its original pending identity survives; replacement
  submission is blocked; a fresh explicit inspection can clear that exact handle.
- Another 12 centralized cancellation browser cases passed. These 31 browser tests
  use mocked services, not real multi-host Agents or the installed desktop WebView.
- Frontend typecheck, source line guard, documentation book/inventory, organization
  and whitespace checks passed. Tensor structure remains valid with zero structural
  gaps, four maturity gaps, twenty evidence-grade gaps and fifteen P0 gaps. Evidence
  stays local `verified`; Daji readiness is not promoted.

Unmanaged storage writes, another tab/realm and untracked URL/history mutations still
rely on response-time snapshot checks. A roundtrip entirely through those bypasses
is not covered by the new managed revisions. No cross-window fencing, automatic
cancel/replay, durable workflow resumption, native/physical-LAN rerun, version bump,
commit, push, packaging or installation is claimed in this follow-up.

## Follow-Up: Delivered Cross-Window Configuration Fences

The preceding cross-tab limitation is narrowed for delivered same-origin storage
notifications. A second Workbench window can switch mode or its persisted gateway
A-to-B-to-A while an inspection response is held. Previously the first window could
accept the old terminal reply and clear its pending handle because its final snapshot
matched A and the managed revision only observed its own writes.

The shared API guard now installs one storage listener per window before capturing
authority. It uses the event's old/new values rather than rereading the latest values,
then advances existing mode/backend revisions. It does not retain a configuration
history, install per-request listeners, synchronize credentials, mutate settings or
journals, or cancel/dispatch/replay execution. Relevant settings/gateway event values
are bounded to 65,536 string code units before parsing. Malformed settings, oversized
payloads, unavailable storage access and clear notifications fail conservatively.
Session-storage events, unrelated preferences, equivalent normalized targets and
query-masked target edits do not spuriously revoke an unchanged request. Absolute
URLs do not depend on persisted target revisions.

Verification on 2026-10-15:

- The initial 16-case regression selection failed 11 tests against the previous
  implementation. Its final 19 cases all passed, including malformed backend fields,
  oversized gateway input and unavailable storage restored after event delivery.
- All 529 selected frontend unit tests passed, including shared admin/library
  controllers, ordinary run, durable recovery, cancellation feedback, settings,
  credentials, request errors and storage cleanup.
- All 25 isolated Chromium durable recovery cases passed. Six new two-page cases
  use real browser storage events: mode/gateway roundtrips for ordinary execution and
  manual recovery, plus URL-pinned and unrelated-preference controls. Each waits for
  actual notification delivery before releasing the held terminal HTTP response.
  Invalidated calls preserve the exact IndexedDB handle and current model/result,
  block replacement submission, and allow fresh explicit inspection to clear only
  that original handle. No cancellation, second submission or peer-window task call
  occurs. The two negative controls still complete normally.
- The first browser run passed 22 of 24 cases. Two gateway fixtures incorrectly
  expected invalidation despite the fixture URL explicitly pinning its backend.
  The corrected tests move that target into local storage before starting the request,
  and retain a separate query-pinned control. Production query precedence was not
  weakened to make the tests pass.
- Another 12 selected centralized cancellation browser cases passed, for 37 passing
  browser cases in this follow-up. Frontend typecheck, the 800-line source guard,
  documentation book/inventory, project organization and whitespace checks passed.
  Tensor validation reports zero structural gaps, four maturity gaps, twenty
  evidence-grade gaps and fifteen P0 gaps; Daji readiness remains blocked.

These are local browser checks with mocked services, not native multi-host LAN,
installed WebView or power-loss qualification. Storage events are asynchronous: a
reply processed before its configuration notification arrives is outside this fence.
No cross-window atomic transaction or persistent global epoch is claimed. Unmanaged
same-window writes and untracked URL/history changes still rely on response-time
snapshots. Evidence remains local `verified`; no grade promotion, native runtime
rerun, Agent/engine/operator protocol change, new persistent data, version bump,
commit, push, packaging or installation is claimed in this follow-up.

## Follow-Up: Terminal Cleanup Ownership Across Storage Awaits

The terminal acknowledgement path checked authority before calling journal removal
and after its promise completed, but removal itself could await database opening or
transaction admission. A context change during that wait was reported only after the
reservation had already been deleted. The desktop bridge's initial origin-discovery
handshake had a similar pre-dispatch gap. A separately captured acknowledgement
context also did not preserve the ordinary observer/manual recovery caller's lifetime.

Ordinary observation and manual recovery now pass their original current-owner guard
into acknowledgement. It is checked before scope hashing, after hashing and journal
lookup, and passed together with the service authority check into deletion. The
browser journal checks before/after opening IndexedDB, on the queued transaction read,
and on delete-request success while the transaction remains abortable. Guard failure
aborts that transaction instead of merely reporting stale ownership after deletion.
The exact identity comparison is unchanged; no new journal, backup or receipt format
is introduced. A fresh explicit recovery operation can still clear the original handle.

The desktop client checks before host discovery and again before posting removal,
including an already verified host. Its callback is local only: message fields, origin
pinning, native commands and Rust journal format remain unchanged. Generated shared UI
assets were synchronized through the native script runner, not manually edited.

Verification on 2026-10-15:

- Seven new service/caller cleanup tests and two new desktop bridge tests failed
  against the previous implementation. They pass after the repair. Two additional
  browser-journal unit cases verify no open for a retired caller and connection closure
  without transaction creation when ownership is lost during open.
- 538 selected frontend unit tests passed across recovery, run, authority, requests,
  cancellation feedback, shared admin/library controllers and storage boundaries.
  All 17 desktop-shared bridge tests passed, including checkpoint/layout isolation.
- All 29 isolated Chromium durable recovery cases passed. Four new cases exercise
  ordinary execution and manual recovery with a real IndexedDB readwrite transaction
  queued behind another transaction, or held after successful deletion but before
  commit. A second browser page switches mode away/back through PWDT; after actual
  storage notification delivery, the held callback resumes and the transaction aborts.
  The original reservation remains, replacement submission is blocked, fresh explicit
  inspection clears it, and there is no cancellation, replay or peer-window task call.
  Ordinary execution may already have published its valid result before cleanup starts;
  the test does not falsely claim to roll that result back on later owner loss.
- Frontend typecheck, desktop shared asset synchronization, documentation book/inventory,
  project organization and whitespace checks passed. All source/test files remain within
  800 lines and documents within 2,000. The tensor retains zero structural gaps, four
  maturity gaps, twenty evidence-grade gaps and fifteen P0 gaps; Daji readiness is blocked.

This narrows specific asynchronous gaps, not all storage/authority races. Configuration
writes and journal commits are not one atomic transaction. A change delivered after
the final active browser guard check can still race commit; an already dispatched
native deletion cannot be retracted by this callback. No physical power-loss,
installed-WebView-to-native-commit or authenticated multi-host LAN qualification is
claimed. Existing local `verified` evidence is extended without grade promotion. No
Agent/engine/operator protocol, version, commit, push, package or installation changed.

## Follow-Up: Desktop Journal Queue Ownership Across Document Reload

Verification date: 2026-10-16. The desktop direct-execution and model-checkpoint
hosts validated the iframe at message receipt, but queued native calls checked
only disposal. A replacement or reload could therefore let an accepted old reserve
or remove reach native storage later. A real iframe keeps the same WindowProxy
across navigation, so source identity alone also allowed old results into the new
document. Checkpoint queue payloads and response IDs additionally retained mutable
event-object fields instead of request snapshots.

Both hosts now capture validated metadata, response ID, source and a fixed-size
document-generation identity before queueing. Frame load retires that identity;
host pagehide retires and suspends it, and pageshow permits only fresh requests.
The queue rechecks generation, connection, source and disposal before native
dispatch and replies. Disposing removes all installed listeners. Retired queued
operations do not invoke storage; late success and error replies are suppressed.
The original eight-slot limit and serial native execution remain: loading another
document does not reset occupied capacity behind outstanding work. Fresh explicit
reads and cleanup still work after the old queue drains. No persistent generation
history, backup, timer, new command, wire format or journal schema was introduced.

Verification:

- The initial 16 lifecycle unit cases reproduced 14 failures on the old hosts;
  only the two existing disposal cases already passed. After the repair and four
  additional snapshot/capacity controls, all 20 new lifecycle cases pass. All 37
  desktop-shared unit tests pass, including unchanged checkpoint/layout clients.
- Eight isolated Chromium cases use real cross-origin iframe messages and navigation,
  explicitly asserting stable WindowProxy identity. They cover both hosts, held native
  reads/reservations, and reload/no-reload controls. Native invocation and the ledger
  are mocked. Reload blocks queued reserve/remove and old replies, preserves any
  already-started reservation, and permits a new explicit read and removal. Unchanged
  documents still drain in order. Each test closes its browser context.
- The first existing desktop GUI suite passed 45 of 47 cases. Both failures were
  static call-chain false positives: the scanner treated unused copied Workbench
  journal code as Hub/Installer native calls. It now traverses actual HTML script
  entries and static/re-export/literal-dynamic imports with the existing TypeScript
  parser. Cycles are bounded; unresolved, escaping or nonliteral imports fail closed
  instead of silently disappearing from coverage. Three focused controls verify
  module reachability and that only Workbench loads the journal host. No Hub or
  Installer native permissions were widened. This remains a literal-command static
  check, not complete dataflow proof of every computed IPC command.
- The native test runner now includes both journal bridge suites plus the new unit
  and browser lifecycle tests. Its rebuilt full desktop GUI suite passed 82 of 82
  cases, including the 37 shared tests and eight new browser cases; these counts
  overlap and must not be added as independent coverage.
- Frontend typecheck, generated desktop-shared synchronization, Rust formatting,
  documentation book/inventory, project organization and whitespace checks passed.
  Tensor validation retains zero structural gaps, four maturity gaps, twenty
  evidence-grade gaps and fifteen P0 gaps; Daji readiness remains blocked. The
  separate lifecycle claim is local `verified`, not new installed-app evidence.

At this stage, before the v2 follow-up below, the fence covered requests already
accepted before an observed lifecycle change,
not navigation-start atomicity. An old document's message first delivered after
the replacement load is not fenced by a document-session nonce; no such protocol
was added. Client timeout alone does not withdraw queued work. Native calls already
dispatched can still commit, and a blocked native call still holds the serial queue;
reply suppression is not rollback or cancellation. Configuration-only changes are
handled by the separate caller-authority guard, not this page-generation check.
No physical disk/power-loss, installed WebView, actual Agent process or authenticated
multi-host LAN qualification was run here. No Agent/engine/operator protocol, version,
commit, push, package or installation changed.

## Follow-Up: Session-Fenced Journal Messages After Navigation

Verification date: 2026-10-16. Six new regressions held old list/reserve/remove
messages until after a replacement load. All six failed with the prior host:
receipt-time generation capture incorrectly gave the late old message the new
document's authority. Both bridges now use v2 UI envelopes with an explicit
metadata-free connect handshake and an in-memory random host session identity.

Connect never invokes native storage or returns journal data. All journal operations,
including reads, require the current session. Load/pagehide rotates that identity;
old-session messages are rejected before queue admission, with only the old token
echoed and a generic unavailable error. The existing generation/connection guards
still retire requests already queued and suppress late results. A delayed connect
is harmless to the journal: a new client ignores its unrelated random request ID.
There is no v1 or missing-session fallback. Native commands, journal data formats,
Agent protocols and execution identities are unchanged. Shell and embedded frontend
must be updated together; stored reservations need no migration.

The shared UI-only session client pins the parent origin, checks response sessions,
deduplicates handshake probes and bounds logical operations, including handshake
waiters, to eight. Pagehide rejects all pending calls and retires waiting continuations.
Pageshow permits fresh explicit operations without replay. Session expiry fails the
current operation; only a later explicit call reconnects. Checkpoint reserve now
also snapshots caller metadata before handshake awaits. Sessions are not persisted,
put into URLs, sent to native commands or treated as Agent authentication credentials.

Verification:

- All 57 desktop-shared unit tests passed, including the six initially failing
  late-delivery cases, v1/unfenced rejection, handshake bounds, response-session
  validation, suspended/restored clients, no automatic retry and metadata snapshots.
- All 16 Chromium lifecycle tests passed. Eight new cases hold a real cross-origin
  MessageEvent before host receipt, navigate the iframe with the same WindowProxy,
  then deliver the held event. Old connect/list/reserve/remove messages make zero
  native calls. A real newly loaded client then connects, reads the retained entry
  and explicitly removes it. This is controlled delayed-delivery fault injection,
  not a claim that Chromium naturally reproduced that timing. Native storage remains
  mocked, while origin, navigation, event source and client-host messaging are real.
- The rebuilt native runner's full desktop GUI suite passed 110 of 110 cases,
  including the 57 shared tests and 16 lifecycle browser cases. Counts overlap.
  All 17 selected Rust desktop asset-sync and bundle-staging tests passed. The new
  shared client is included in the native mirror manifest for all three shells.
- Cross-layer source tests exposed an integration error during implementation:
  generated browser JS resolved the shared client, but direct TS source imports
  looked for a nonexistent JS source file. Two selected frontend suites failed to
  import it. Source now imports TS explicitly, with the shared compiler rewriting
  relative import extensions to JS. Generated output was unchanged by that repair.
  All 210 selected frontend recovery/checkpoint/authority/error tests then passed,
  as did frontend typecheck and the 800-line source guard.

This closes post-load stale-session delivery, not navigation-start atomicity or
physical durability. An initial iframe load can also retire pre-load work; callers
must explicitly inspect again rather than silently replay. Already dispatched
native calls may still commit, and a blocked native call still holds the serial
queue. Client timeout alone does not cancel host work. This run did not qualify
installed WebView, actual native journal commit, power loss, authenticated multi-host
LAN or distributed ownership transfer. Evidence remains local `verified`; there is
no operational-grade promotion, version bump, commit, push, packaging or installation.

## Follow-Up: Reply Delivery And Document Authority

Verification date: 2026-10-16. A received journal response had already left the
pending-message map but had not yet returned through its promise continuation.
Pagehide/disposal could retire that client in this interval without preventing
successful delivery. Seven new shared-client regressions failed before the repair,
including caller-guard loss after a deletion acknowledgement.

The shared client now checks the captured generation and optional caller guard again
after the reply await. Stale continuations reject without issuing cleanup, rollback
or replay commands. A native write may already have committed; this check does not
claim to retract it. Session, file and Agent protocol versions are unchanged.

The higher-level API authority guard had a separate gap: identical gateway/mode/token
settings after page restoration could preserve old calls. Five lifecycle unit cases
and both new Chromium/PWDT call-chain cases failed before the repair. In the ordinary
run browser case, the old terminal response was published as a completed result.

Authority captures now include a window-scoped active lifetime and generation. An
observed pagehide retires the generation; pageshow enables fresh captures without
reviving old ones, even when no guard ran while suspended. Captures made while
suspended remain invalid. Background visibility changes, blur, beforeunload alone
and duplicate pageshow are negative controls, not cancellation triggers. Each window
installs one listener pair, not one per request; 1,000 retirement/capture cycles keep
that bound. Nothing is written to disk, URLs or credentials for this fence.

Verification:

- All 64 desktop-shared tests passed, including the seven initially failing reply
  continuation checks. Generated shared assets for the three shells are synchronized.
- The full desktop GUI regression passed 117 of 117 cases, including those 64 shared
  tests and 16 cross-origin iframe lifecycle cases; these counts overlap.
- All 219 selected frontend direct recovery, checkpoint, authority and error tests
  passed. Two direct-run service cases distinguish confirmed unsent reservation
  cleanup from already-submitted uncertainty: the latter retains its original ID
  until a fresh explicit inspection returns a validated terminal receipt.
- All 67 API-focused unit tests passed (overlapping the selected frontend suite).
- All 31 Workbench durable-direct Chromium tests passed. The two new ordinary-run
  and manual-recovery cases hold a response, inject pagehide/pageshow, then release
  it. Neither publishes the retired result or removes its saved owner. A second run
  is blocked; fresh explicit inspection reconciles the exact original identity.
  Assertions permit only one submission, no cancellation and two inspection calls.
- Frontend typecheck and the 800-line source guard passed.
- Documentation book/inventory, shared-asset synchronization, project organization
  and whitespace checks passed. Tensor validation retains zero structural gaps,
  four maturity gaps, twenty evidence-grade gaps and fifteen P0 gaps; Daji remains
  blocked. Evidence remains local `verified`, not installed or operational promotion.

The new Workbench cases use the real UI/PWDT/service/IndexedDB chain with mocked
Agent responses and controlled PageTransitionEvent injection. They do not exercise
actual browser BFCache restoration, installed WebView, native journal disk commits,
physical power loss or authenticated multi-host Mesh. The API fence protects callers
that capture and check authority, not every raw HTTP call. At this stage, before the
checkpoint follow-up below, its retry/recovery pipeline was not covered. Already
dispatched side effects remain possible. No version
bump, commit, push, package or installation was performed.

## Follow-Up: Checkpoint Caller Authority And Conditional Cleanup

Verification date: 2026-10-16. Checkpoint recovery compared only current scope text,
so a connection or document roundtrip could preserve an old call. Conditional
journal removal did not accept a caller guard. Save retry also removed its identity
after success without rechecking the authority captured before submission.

Before repair, all twelve new service/storage boundary tests failed. Two of three
new Chromium recovery cases also failed: check and acknowledgement accepted a late
response after injected pagehide/pageshow. The open-version UI already rejected
through its outer guard; a separate service test showed that the recovery service
itself still needed to carry its guard through version validation.

The generic checkpoint services now accept an injected authority capture shared by
save/retry and recovery. The Workbench adapter supplies the existing API guard for
project, model, version and checkpoint routes. Neither journal scope nor format
changes; generic non-GUI users can supply their own guard. Hashing, reservation,
dispatch, response handling and cleanup retain the original capture. A rejected
late save response does not imply rollback: the exact original request identity
remains available for a later explicit idempotent retry or read-only reconciliation.
Restored callers cannot turn an old coalesced in-flight save into a new submission.

Conditional removal carries a local callback through the browser/native adapter.
IndexedDB checks before/after open, after transaction read, and on delete success
while the transaction is still abortable. A failed guard aborts rather than merely
reporting an error after deleting the identity. Desktop checks reuse the shared
handshake/dispatch/reply fence without serializing callbacks or credentials. Failure
after an actual commit cannot undo it; the service still rejects a retired caller's
success delivery. Storage-only cleanup failure under unchanged authority preserves
the existing behavior: a validated server commit is not reported as a failed save.

Verification:

- All 254 selected frontend direct/checkpoint/API/project-service/error tests passed,
  including fourteen new service/storage cases and prior retry/coalescing behavior.
- All 67 shared desktop tests passed, including new checkpoint guard checks before
  discovery, during handshake and after acknowledgement.
- The full desktop GUI regression passed 120 of 120 cases, including those 67
  shared tests and the 16 iframe lifecycle cases; these counts overlap.
- Both checkpoint Chromium suites passed 22 of 22 cases. Seven new cases cover
  page-lifecycle retirement during check/acknowledge/open, model save and Save As,
  and actual IndexedDB transaction abort at read success and delete success. Fresh
  explicit recovery clears only the original identity with no checkpoint POST.
- Typecheck initially caught a missing optional request-ID field in a new test's
  generic input type. After correcting the test definition, frontend typecheck and
  the 800-line source guard passed. Shared generated assets are synchronized.
- Documentation book/inventory, organization and whitespace checks passed. The
  tensor's renamed shared-test reference was synchronized; validation retains zero
  structural gaps, four maturity gaps, twenty evidence-grade gaps and fifteen P0
  gaps, with Daji blocked. This adds local verified evidence, not operational grade.

The browser uses real PWDT, service and IndexedDB code with controlled lifecycle
events and a mocked checkpoint service. This is not actual BFCache, installed
WebView, native disk/power-loss, authenticated multi-host Mesh or distributed
ownership qualification. Checks are not atomic with external configuration writes
or native commits; already dispatched work can finish. No snapshots, backups,
automatic replay, version bump, commit, push, packaging or installation were added.

## Follow-Up: Checkpoint Reservation Identity Parity

Verification date: 2026-10-16. Rust rejected a same-key reservation with a foreign
scope, operation or parent, but the browser journal returned it as an existing
reservation. Browser validation also retained the caller's mutable object across
database open. Desktop bridge replies checked the parent identity but accepted a
new reservation with a changed request ID or timestamp; save retry trusted adapter
replies without independent validation.

Before repair, eleven of thirteen new frontend unit cases failed, as did four of
six new desktop host/client cases. Both sets included passing legitimate-existing
reservation controls. All three new real-Chromium cases also failed: injected
same-key scope, operation or parent conflicts still allowed a checkpoint POST.
The service already used a mocked backend; this demonstrates an incorrect local
dispatch decision, not a compromise or failure observed on a physical server.

One shared reservation validator now checks key/scope/operation/parent identity.
New reservations must echo request ID and creation time exactly; existing ones
keep their original ID/time. Browser reserve snapshots metadata before opening the
database, and conflicts abort the read/write transaction without replacement.
Both desktop bridge ends validate responses, and save retry independently checks
them against a separate request snapshot before POST. A failed validation releases
only the memory coalescing slot, not the durable record. The native journal format,
IPC envelope version and actual Rust storage algorithm are unchanged.

Verification:

- Selected frontend direct/checkpoint/API/project-service/error tests: 267/267,
  including thirteen new reservation cases. Shape errors, adapter mutation, caller
  mutation during database open and legitimate original-ID reuse are covered.
- Shared desktop tests: 73/73, including six new host/client cases for changed new
  request IDs/timestamps and valid existing reservations.
- Full desktop GUI regression: 126/126, including the 73 shared tests and sixteen
  real-iframe navigation/lifecycle cases. These counts overlap.
- Both checkpoint Chromium suites: 25/25. The three new conflict cases retain the
  injected record, send no POST, and create no model version. Restoring only the
  test-injected corruption allows a fresh explicit retry with the original request
  ID, one committed version and normal conditional cleanup.
- Rust desktop-runtime checkpoint journal tests: 8/8 on local macOS. An added
  native parity case verifies all three identity conflicts preserve the exact file
  bytes; the reopen test now also verifies the original timestamp is retained.
- Frontend typecheck, the 800-line source guard, shared-asset synchronization and
  documentation book/inventory checks passed. Rust formatting, project organization
  and whitespace checks passed. Tensor validation retains zero structural gaps,
  four maturity gaps, twenty evidence-grade gaps and fifteen P0 gaps; Daji remains
  blocked. The new scoped contract claim does not promote operational maturity.

Evidence remains local `verified`. Chromium exercises real PWDT/save/IndexedDB code
against a mocked checkpoint service; desktop bridge tests mock native invocation.
The separate Rust tests exercise local temporary files, not an installed App or
mid-commit kill/power-loss qualification. This is not authenticated multi-host Mesh,
distributed deduplication or atomic cross-window API authorization. No automatic
repair, replay, eviction, backup series, format migration, commit, push, version
bump, packaging or installation was performed.

## Remaining Boundaries

- No physical LAN/multi-host, physical power-loss, Windows or installed GUI
  qualification is claimed by this local macOS run.
- The raw Agent TCP listener is still a trusted-network interface, not an authenticated
  LAN service. Cluster names are public selectors, not credentials. Discovery and
  gateway outbound RPC plus native ingress framing/receive capacity are bounded;
  authentication, decoded/solver memory bounds and sustained-load qualification
  remain separate work.
- Discovery is not execution authorization, consensus, global deduplication, durable
  workflow DAG scheduling, ownership transfer, partition recovery or task migration.
  A client must retain the original endpoint and store identity. Submitting the same ID
  unfenced to a different Agent can execute it independently; discovery does not prevent
  that. Carrying the original store guard rejects a different ledger before operation.
- The legacy gateway's result cache remains process-local; it cannot recover a lost
  solve result after restart. Durable tasks use the pinned task/inspection contract.
- Candidate limits are safety bounds, not 64-node performance qualification. Seeds
  and cooldown-table saturation can still occupy slots; no admission guarantee is
  claimed under sustained churn. DNS cancellation and topology persistence are not
  implemented. Learned-candidate retirement does not recover or migrate tasks.

Coverage evidence is recorded as local `verified` evidence only. Existing operational
Orchestra evidence is separate and does not qualify decentralized mesh recovery.
