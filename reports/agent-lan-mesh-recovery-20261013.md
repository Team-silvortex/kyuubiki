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
