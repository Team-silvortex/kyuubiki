# Protocols

Kyuubiki now treats the GUI, control plane, and solver agents as three
independent programs joined by explicit public contracts.

## Program split

- `kyuubiki-frontend`
  Browser-first GUI and editor shell. It should only consume stable HTTP APIs
  and result windows in orchestrated mode, and should be able to speak to
  headless Rust agents directly in direct-mesh mode.
- `kyuubiki-orchestrator`
  Control plane for jobs, persistence, scheduling, health, and distributed
  agent coordination.
- `kyuubiki-rust-agent`
  Compute-plane program that exposes solver capabilities over framed TCP RPC.
  It is intentionally headless and can run:
  - standalone
  - orchestrated by Phoenix
  - as part of a peer mesh on a LAN

## Public protocols

### Frontend runtime modes

The frontend is expected to split into two independent operating modes:

- `orchestrated_gui`
  - talks to `kyuubiki.control-plane/http-v1`
  - best for centralized persistence, cloud control planes, and distributed
    solver fleets
- `direct_mesh_gui`
  - talks directly to headless `kyuubiki.solver-rpc/v1` agents on a LAN
  - best for local peer meshes where the control plane is optional rather than
    mandatory

Both modes should share the same editor shell and model/result contracts, while
remaining independent at runtime.

### Control plane

- Name: `kyuubiki.control-plane/http-v1`
- Transport: `HTTP + JSON`
- Primary descriptor endpoint: `/api/v1/protocol`

The control plane exposes:

- health: `/api/health`
- protocol descriptor: `/api/v1/protocol`
- control-plane protocol descriptor: `/api/v1/protocol/control-plane`
- solver RPC compatibility descriptor: `/api/v1/protocol/solver-rpc`
- reachable agent descriptors: `/api/v1/protocol/agents`
- job/result/project/model CRUD over `/api/v1/...`

### Solver RPC

- Name: `kyuubiki.solver-rpc/v1`
- RPC version: `1`
- Transport: `TCP`
- Framing: `length_prefixed_u32`
- Encoding: `JSON`

The current generic/runtime methods are:

- `ping`
- `describe_agent`
- `release_operator_package_job`
- `cancel_job`
- `run_durable_agent_task`, `submit_durable_agent_task` (asynchronous admission),
  `fetch_durable_agent_execution`, `list_durable_agent_executions`,
  and `cancel_durable_agent_execution` (explicit Agent
  execution-store configuration required; see the
  [standalone durable execution contract](book-ch04-runtime-modes.html#standalone-durable-execution))

Durable requests accept optional `expected_store_id`, a 32-character lowercase hex
identity from `describe_agent.durable_execution.store_id`. Submit (sync and async),
inspect, index and cancellation reject a foreign store before operating. Reports and
live admission decisions echo `store_id`; guarded gateways/clients reject missing or
foreign echoes. The guard does not alter the task's numerical request digest. Persist
it with the original endpoint and execution ID before submission, not after reconnect.
This prevents accidental address reuse from redirecting recovery to another ledger.
The identity survives process restart in the synced, checksummed `owner.lock`; corrupt
nonempty identity metadata fails startup. It is not a credential, peer authentication,
global exactly-once guarantee or protection against copied ledgers. Legacy native
requests without the guard remain accepted but do not have this ownership guarantee.

`durable_execution.store_access` describes `kyuubiki.agent-store-access/v1`.
Description takes the store mutex without waiting. On contention it reports
`enabled: true`, `available: false`, `unavailable_reason: store_busy`, with no dynamic
store counters or identity. Poisoning reports `store_unavailable` and is never bypassed.
Missing temporary metadata does not invalidate a saved owner fence or authorize an
unfenced request. Fetch/list/cancel use a 250-ms monotonic mutex-acquisition budget;
timeout returns RPC `durable_execution_busy`. Its details describe this recovery call:
`operation_started: false`, `execution_state_known: false`, `identity_check_performed: false`,
and `automatic_replay_authorized: false`. It is not evidence about the original task,
not a missing execution or empty index, and not cancellation acknowledgement. No late
operation is queued. Explicit recovery retries must retain the original Agent/store;
the native gateway returns failure without result caching or peer reselection.
The budget does not bound filesystem operations, other locks or reply delivery.
Admission/receipt writes retain their existing serialized commit boundary.
Native durable HTTP routes retain valid request-bound RPC failure codes in nullable
`agent_error_code` (1..128 ASCII alphanumeric, underscore, hyphen or period), alongside
the existing `error` message. Transport/validation failures have no Agent code.
The gateway does not forward Agent error details or their retry/ownership advice;
HTTP 502 retains `automatic_retry: false`, `automatic_replay_authorized: false` and
original-Agent inspection guidance. Codes are diagnostics only, not task-state,
store-identity or non-admission evidence. Discovery's transport policy publishes
`error_diagnostics`. Workbench preserves the bounded code as
`WorkbenchRequestError.agentErrorCode`, keeps pending handles, and distinguishes
busy/unavailable/write-failed stores at preflight. Direct submit/cancel/legacy-solve
failures are not generic retryable operations, including network errors and timeouts.
Only explicit read-only inspect/list recovery may be retried against the saved owner;
no automatic retry or handle cleanup is added.
The disabled-by-default `KYUUBIKI_AGENT_FAULT_INJECTION_STORE_READ_HOLD_FILE` fixture
and its limits are documented in the linked runtime chapter.

Workbench new-run handles require the store identity. Legacy unfenced handles remain
pending and cannot be silently adopted or acknowledged. GUI/PWDT recovery uses the saved
owner, or an explicit `storeId` from the original index for manually tracked executions;
an explicit owner cannot override a matching local pending handle. The initial index
read is discovery, not permission to adopt unknown past work; use its identity on later
pages. Address, gateway and process/digest checks remain in force alongside this guard.
Recovery actions snapshot caller parameters before awaiting the journal. In-flight
authority includes control mode even without tokens, plus effective URLs and relevant
credentials. A detected change or unreadable authority permanently invalidates that
request: restoring settings requires a fresh explicit inspection, not publication or
cleanup from the old reply. Managed mode, effective backend and credential writes
also advance per-dimension in-memory revisions, so a roundtrip between checks cannot
revive that request. Repeated equivalent writes and irrelevant preferences/credentials
do not invalidate it. Cleanup and legacy settings scrubbing participate; partial
writes are checked even when they throw. One window-lifetime storage listener also
advances revisions for delivered same-origin cross-window local-storage mode and
gateway changes. Comparing event old/new values catches A-to-B-to-A even when current
storage already holds A. It never imports credentials or writes configuration/journals.
Session storage and unrelated preferences are ignored; malformed/oversized relevant
settings, inaccessible storage or clear notifications conservatively retire requests.
Relevant event parsing is bounded to 65,536 string code units per value. Query-pinned
targets ignore stored-target edits. Notifications are asynchronous, not a cross-window
transaction lock: a reply processed before delivery is outside this fence. Unmanaged
same-window writes and untracked URL/history changes retain snapshot-only roundtrip
detection. This does not cancel or replay Agent work.

Terminal acknowledgement carries the original caller guard through scope hashing,
journal lookup and deletion, alongside the service's own authority check. Browser
storage checks it before/after database open, when the queued transaction read
completes, and at delete-request success before commit. Guard failure aborts the
active transaction instead of only reporting the error after deleting the reservation.
An already published valid result is not rolled back; a fresh explicit inspection can
complete cleanup. The desktop bridge checks before/after host discovery and immediately
before posting deletion; the callback stays local and changes no wire or file format.
It cannot retract an already dispatched native deletion or undo a completed browser
commit. Configuration writes are not atomically coupled to the transaction; a change
delivered after the final active browser check is outside this fence even if commit
has not yet completed. This is not cross-window/process atomic authorization.

Desktop journal hosts also snapshot request identities and invalidate previously
accepted queue entries on iframe load, host pagehide or disposal. Source/detachment
checks run again immediately before native dispatch and replies. Frame load needs
its own generation because a WindowProxy survives document navigation. Pageshow
allows fresh requests, not revival of retired ones; the eight-request queue bound
is retained until outstanding native work drains. The UI message envelopes are now
`kyuubiki:direct-execution-journal/v2` and `kyuubiki:checkpoint-recovery/v2`:
`connect` accepts only type, request ID and action, performs no native call, and
returns a random in-memory session ID with a null result. All journal operations,
including list, require that session. Load/pagehide rotates it before later delivery;
an old or missing session never reaches native storage. Expiry returns only the
old token with a generic unavailable error, not the new session. A delayed handshake
has no journal side effects; clients accept its response only for a pending random
request ID from the pinned parent origin. No v1 or unfenced fallback is supported.
Client pagehide rejects pending operations and handshake waiters; pageshow permits
fresh explicit calls. One shared handshake and an eight-logical-operation cap bound
the client. Session expiry never automatically retries a journal operation; a later
explicit call may reconnect. Shell and embedded frontend must update together.
Session IDs are not credentials and never enter native payloads, files or URLs;
native journal formats, commands and Agent protocols remain unchanged.
After awaiting a successful reply, the client rechecks its captured document generation
and optional caller guard before returning the result. Even an already acknowledged
write cannot authorize a retired continuation; rejection does not undo that write.
Late replies are suppressed but already dispatched native calls can still commit.
Explicit reinspection, not replay, reconciles that case. This is not navigation-start
atomicity or Agent authentication; client timeout alone does not cancel queued work.
Real iframe navigation with mocked native invocation verifies the local host boundary,
not installed-WebView or disk durability, power loss, or atomic cross-process authority.

Workbench API authority captures additionally bind to the active document lifetime.
Pagehide retires old guards; pageshow enables new captures, never old or suspended ones.
One window-scoped listener pair tracks this boundary without persisted state or
per-request listeners. Visibility changes, blur and beforeunload alone do not revoke it.
Direct durable run/recovery callers recheck the guard at their existing await boundaries:
confirmed unsent reservations may be released, but a submitted execution with a rejected
late response retains its original identity for explicit inspection. This is not transport
cancellation, universal raw-HTTP fencing, or a guarantee of browser BFCache behavior.

Model-checkpoint saves and recovery use the same injected API-authority capture,
without embedding frontend credentials or lifecycle state in journal records. The
guard follows hashing, reservation, server responses, recovered-version validation
and conditional cleanup. The browser removal guard runs before/after database open,
after the transaction read and at delete success before commit; failure aborts an
active transaction. Desktop removal carries only the existing key/request ID on
the wire; its guard remains local. Late or retired calls retain uncertain identities
and never automatically replay a save. A fresh explicit save retry still uses the
original idempotency key; read-only recovery uses GET and performs no checkpoint POST.
Storage-only cleanup failure does not reclassify a validated commit as failed. A
retired continuation rejects delivery but cannot roll back server or native commits.

Checkpoint reservation replies are validated at both desktop bridge ends and again
before save transport. The reserved key, scope, operation and parent must match the
requested snapshot. `isNew: true` additionally requires the same request ID and
creation time; `isNew: false` preserves the earlier reservation's ID/time. Browser
transactions apply the same collision contract as Rust storage and snapshot metadata
before asynchronous database open. Conflicts or malformed replies fail closed with
`checkpoint:journal_corrupt`, without checkpoint POST, cleanup or record overwrite.
No journal format change, automatic repair or cross-Agent deduplication is implied.

Cancellation acceptance is not a terminal receipt. After a lost reply, inspect the
saved owner without automatically resending cancellation or migrating the task.
A synced failed/cancelled receipt survives Agent restart; an admission without a
terminal receipt remains `outcome_unknown`, not cancelled. The Workbench ordinary
cancel action also retains shared feedback ownership so late acceptance/failure
cannot overwrite a newer terminal message within the same job polling epoch.
The action's own outcome is still returned; pending identities are cleared only by
validated terminal inspection, not cancellation acceptance or a transport error.

Async durable submission rejected by execution-slot reservation (busy/draining)
persists a normal `kyuubiki.agent-durable-execution/v1` failed receipt, with nested
error code `durable_admission_rejected` and `details.execution_started: false`.
No engine dispatch or queue is created. The receipt can be inspected after a lost
reply or restart; duplicate IDs return the original failure, not a fresh execution.
Failure of either identity or receipt commit remains an uncertain storage error.

After admission, confirmed owner/worker startup failure before ownership transfer
also persists a normal failed receipt: `durable_execution_start_failed`, with
`details.phase: before_engine_dispatch`, `details.execution_started: false`, and
`details.automatic_replay_authorized: false`. The Agent's submit handler retains its execution slot
until this receipt commit finishes. Failure to commit does not produce a terminal
claim; an interrupted admission stays unknown after restart. Original IDs never
authorize another invocation, even after transient startup conditions have cleared.
The descriptor exposes this policy and the opt-in exact-ID test hook
`KYUUBIKI_AGENT_FAULT_INJECTION_START_FAILURE_EXECUTION_ID` (32 lowercase hex
characters), which simulates failure before spawning the worker, not a solver fault.

The async worker catches Rust unwinds around request execution, outside ledger
locks/commits. After unwinding it commits `durable_execution_panicked` as a failed
receipt, with `phase: async_worker_unwind`, `execution_may_have_started: true`,
`partial_result_discarded: true`, `external_effects_rolled_back: false` and
`automatic_replay_authorized: false`. It does not claim non-execution, numerical
validity, or rollback. The fixed message excludes the arbitrary panic payload;
the existing local panic hook/logging policy is unchanged. Poisoned shared locks
remain unavailable. Aborts, memory exhaustion, synchronous requests, process kills
and failed receipt commits are not converted into a caught-panic receipt.
`KYUUBIKI_AGENT_FAULT_INJECTION_DURABLE_PANIC_EXECUTION_ID` is an opt-in exact-ID hook
that panics after dispatch returns, before committing the durable response. It does
not inject into every solver algorithm, and the descriptor exposes its activation.

For synchronous and asynchronous durable TaskIR, `fetch_operator_task_result` publishes
only the committed durable receipt's nested RPC response. The descriptor exposes this
under `durable_execution.task_result_cache`, with `publication_boundary:
durable_receipt_commit` and `response_source: committed_receipt`. A failed commit leaves
no successful cached result: a still-pending reservation becomes `result_not_retained`.
Caught async panic failures and cancellations return the same response through both
recovery paths. The existing identity/generation fences, TTL and byte/count limits still
apply; restart loses this cache, and duplicate durable submissions do not rebuild it.
Ordinary non-durable TaskIR retains its completion-before-delivery cache boundary.

Synchronous `run_durable_agent_task` exposes its lifecycle policy under
`durable_execution.synchronous_execution`. Its execution owner stays live through
receipt commit, then through transport delivery. A failed commit records live watchdog
code `durable_receipt_commit_failed`; sending the resulting RPC error cannot count as
successful execution. An earlier generation-bound solver/cancellation failure is
preserved instead of overwritten or counted twice. Synchronous unwind records only a
live `durable_execution_aborted` diagnostic, not a persisted panic receipt. Neither
diagnostic converts a missing receipt from `outcome_unknown` to durable failure proof.

Response preparation is separated from receipt-file commit. A response-contract,
bounded-encoding or restart-decoding rejection produces a compact failed response with code
`durable_response_unrecordable`, phase `receipt_validation`, and reason `byte_limit`,
`serialization`, `restart_decode` or `invalid_response`. Its details explicitly include
`execution_may_have_started: true`, `original_response_discarded: true`,
`external_effects_rolled_back: false` and `automatic_replay_authorized: false`.
Only successful commit of that replacement permits a terminal `failed` report and
continued independent admissions. Failed replacement commit remains unknown and blocks
new admissions; it is never retried with another payload. Both durable modes, the TaskIR
cache and live completion accounting use the committed response. The descriptor exposes
the limits and this policy under `durable_execution.response_rejection`. Original solver
or cancellation diagnostics already recorded in the watchdog keep their first cause.

Both write preparation and retained-receipt inspection validate the RPC version and
mutually exclusive result/error state. A durable success requires a non-null top-level
result (JSON null decodes as an absent `Option<Value>`); nested nulls and non-null
scalar/array results are not rejected by this envelope check. Failure code and message
must contain non-whitespace text. An original request-ID mismatch is still an identity
error, never converted into a compact failure for another request. An already retained
invalid response causes inspection/duplicate/cancellation to fail without rewriting its
file or authorizing replay. Its admission remains indexable and unrelated work may
continue. The descriptor exposes this as `response_rejection.invalid_retained_receipt`.
The native gateway and frontend reject blank failure evidence too; invalid receipts
cannot authorize removal of the caller's recovery journal entry.

Ledger-capacity rejection can return `kyuubiki.agent-durable-admission-rejection/v1`
as its RPC result (`ok: true` means the admission exchange completed, not that a
computation succeeded). `status: not_admitted` binds the execution, method,
digest, producer and current RPC request ID. Only validated capacity/reservation
rejection before ledger admission permits retiring the caller's new reservation; generic
errors, write failures and `not_found` never grant that permission. No automatic
replay is authorized, and this transient decision is not an execution lookup record.

Examples of solver methods are (query `describe_agent` for the complete list):

- `solve_bar_1d`
- `solve_truss_2d`
- `solve_truss_3d`
- `solve_plane_triangle_2d`

Progress and runtime liveness are streamed back as RPC frames:

- `progress`
- `heartbeat`

The native direct gateway validates terminal and progress envelopes through the shared
Rust protocol validators. Its discovery response publishes `transport_policy` with
schema `kyuubiki.direct-mesh-transport/v1`; legacy solve responses expose the same policy
under `direct_mesh`. Control RPCs (describe, async durable submit, inspect, list, cancel)
have one 15-second exchange deadline and reject progress streams. Partial TCP reads do
not extend the deadline. DNS is limited to four outstanding workers, including late
resolvers after caller timeout; queued callers wait only within the original budget.
At most eight resolved unicast addresses are tried, with 500 ms per connect attempt.

Encoded response-frame caps are 1 MiB for describe/index/cancel, 9 MiB for async durable
submit/inspect (including retained results), and 64 MiB for legacy solver replies.
The legacy solver path deliberately uses a renewable per-complete-frame 15-second
deadline, not a new whole-job time limit. Its returned progress history is the most
recent window, capped at 128 validated frames and 1 MiB of normalized encoded data;
numerical results are not truncated. These limits do not bound decoded heap usage to
exactly the same byte count. A peer sending endless valid solver heartbeats can still
keep that legacy operation open. No automatic retry, inferred cancellation or terminal
receipt is created from transport failure; retain the original durable handle.

Native Agent inbound framing is bounded independently of gateway/discovery outbound
transport. All three runtime modes expose `request_ingress`
(`kyuubiki.agent-request-ingress/v1`) in describe and registration/heartbeat payloads.
General-pool defaults are a 256-MiB encoded request, 512-MiB aggregate reserved request
bytes, 64 simultaneous connection handlers and a 30-second idle/header/body receive budget.
The receive deadline is per request, not a solver timeout; fragments cannot renew it.
The encoded-byte reservation spans dispatch/reply and releases on completion, error
or unwinding. After the general pool and bounded recovery reserve fill, excess
connections close without an application queue. Malformed framing, exhausted total
byte/request capacity and timeouts close before dispatch. Partial input
cannot admit a task. Complete malformed JSON retains the existing `invalid_json`
response and persistent-connection framing.

The policy lists the four configuration environment variables and effective limits;
startup rejects invalid values and changes require restart. See the
[native Agent ingress configuration](book-ch04-runtime-modes.html#native-agent-ingress)
for accepted ranges. These limits are not authentication or exact heap/RSS caps.
A lost connection after a client submitted
work still has an uncertain outcome and never authorizes replay or journal deletion.

`request_ingress.recovery_reserve` (`kyuubiki.agent-recovery-ingress/v1`) is a fixed,
read-only same-port reserve: four extra connection handlers and four concurrent
reserved frames, each at most min(configured frame limit, 16 KiB). Total reserved
bytes cannot exceed four times that bound; defaults add at most 64 KiB. The top-level
`max_total_connections` and `max_total_in_flight_request_bytes` include both pools,
as do active/byte usage counters; general and reserve usage are also exposed separately.
Connection overflow stays control-only until socket close, even if general slots
return. Byte-budget fallback on a general connection applies to that frame only.
Reserved receive time is min(configured timeout, 1,000 ms), measured from the original
frame-read start without renewal on fallback or fragments. Reservations last through
reply and release on errors/unwind. Solver execution and response writing have their
own boundaries, not this receive timeout.

The reserve allowlist contains ping, Agent/lifecycle description, drain/resume,
durable inspection/list/cancel, operator-result lookup and execution/job cancellation.
It never admits solver, TaskIR or durable submission, including duplicate submissions.
A valid disallowed request returns `agent_recovery_ingress_only` and closes its socket
without dispatch. Existing envelope validation, host-loopback lifecycle restrictions
and store/process/digest fences are unchanged. No alternate client port or replay is
required. These bounded resources improve recovery under general-pool exhaustion,
not availability under sustained hostile traffic or concurrent reserve exhaustion.

Solver agents also self-describe their runtime topology:

- `runtime_mode`
  - `standalone`
  - `orchestrated`
  - `peer_mesh`
- `cluster_id`
- `headless`
- `peers[]`

In peer-mesh mode the current behavior is intentionally lightweight:

- Each Agent starts from explicit seeds, with at most 64 known candidates and eight
  rotating probes per round. Mixed Orchestra binding and peer configuration is rejected.
- `max_concurrent_probes` is 2 and `overlapping_probe_rounds` is false. The coordinator
  publishes results as probes finish (`state_publication=after_each_probe_completion`),
  rather than waiting for slower peers to complete the round. Completion order may
  differ from FIFO selection order. Two slow in-flight requests can still delay
  queued candidates. Worker creation failure uses the remaining worker or bounded
  serial fallback; this does not grant an extra query budget or retry a task.
- `shutdown_probes=stop_queue_and_join_in_flight`: after observing stop, no queued
  probe is started or completed result published. Scoped probe workers are joined
  before shutdown or another round. In-flight requests retain their original deadlines;
  underlying system DNS lookups are separately bounded, not cancellable.
- Probe scheduling is independent of sorted descriptor presentation. The read-only
  `probe_order` is `retained_fifo_new_candidates_at_tail`; removal preserves waiting
  order and newly learned/readmitted peers join the tail. Each continuously retained
  peer is selected within `max_probe_rounds_for_retained_peer` (8) complete running
  rounds. `probe_round_bound_scope` is `complete_running_rounds_not_wall_time`:
  this does not override query timeouts, the configured interval, shutdown or restart.
- `describe_agent.peer_discovery` publishes the read-only
  `kyuubiki.agent-peer-discovery/v1` policy. A query has a 1,500-ms total deadline,
  a 1-MiB response cap, bounded DNS workers, exact response identity and RPC validation.
- Only an exact cluster match and unbound `offline_mesh` authority can contribute
  peer hints. Hints never grant execution permission or establish health; each peer
  needs a direct compatible response. `cluster_size` counts candidates, not quorum.
- Configured seeds remain until reconfiguration. Learned candidates retire after
  three consecutive failed direct probes and enter a 60-second process-monotonic
  cooldown. Hints cannot reset failure counts or shorten/extend the cooldown. Rejoining
  candidates start unprobed, without inherited health. `peer_discovery` exposes
  `learned_peer_failure_threshold`, `learned_peer_cooldown_ms`,
  `configured_seed_retention`, `max_retired_peer_cooldowns` (128),
  `retirement_when_cooldown_full` (`retain_failed_candidate`), `cooldown_clock` and
  `candidate_retirement_scope` (`discovery_only`). Cooldown saturation retains further
  failed candidates until slots expire, rather than forgetting recent retirements.
  These are discovery bounds, not guaranteed new-peer admission under sustained churn.
  `health_score_scope` is `retained_candidates_not_task_recovery`: scores cover retained
  candidates, not lost tasks or complete cluster availability.
- Discovery topology is process-local. Each configured execution ledger belongs to
  one Agent; execution IDs are not globally unique reservations across the mesh.
  Candidate retirement never removes ledger records or redirects pinned recovery.
  An unknown interrupted task must stay pinned to its original owner, not be replayed
  on another peer. Cross-node recovery and authenticated peer identity are not provided.
- The native gateway's legacy multi-endpoint solve refreshes descriptors and filters
  eligibility before applying `first_reachable` or `healthiest`: current solver RPC
  protocol, advertised requested method, explicit valid accepting lifecycle and
  configured available execution capacity are required. Missing metadata fails closed;
  health ranks only eligible candidates. Separately sampled lifecycle/admission active
  counts are combined using their maximum, not assumed to match. No eligible candidate
  means no solver request. The read-only `kyuubiki.direct-mesh-selection/v1` is exposed
  as `selection_policy` in discovery/errors and `direct_mesh.selection_policy` on
  solve success. Discovery does not reserve a slot or establish authentication.
- Legacy dispatch never retries another endpoint after failure. Its result cache is
  not durable; the durable task route requires one explicitly pinned endpoint.
  New-work eligibility does not gate pinned inspection/list/cancel. Busy/draining
  owners remain queryable while transport capacity permits, and an uncertain receipt
  stays uncertain rather than being replayed or upgraded by a good health score.

This gives Kyuubiki a gossip-lite LAN discovery layer without introducing a
hard dependency on Phoenix for solver-to-solver awareness.
The raw listener remains trusted-network-only; cluster IDs are not credentials.
See the [LAN Mesh runtime chapter](book-ch04-runtime-modes.html#lan-peer-mesh) for
deployment boundaries and the distinction between discovery and workflow recovery.

## Why this matters

This split keeps deployment modes independent:

- local workstation
- cloud control plane
- distributed control plane with remote solver nodes
- headless peer-mesh solver clusters on a LAN
- future direct-mesh frontends that bypass Phoenix on the solver hot path

As long as the programs speak these public contracts, the frontend, Phoenix
control plane, and Rust agents can evolve on different release cadences without
re-coupling at the implementation level.

For the stricter runtime-side interpretation of this split, including
authority rules and which frontend layers are packaging-only, see
[headless-agent-contract.md](headless-agent-contract.md).
