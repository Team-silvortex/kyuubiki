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
nonempty identity metadata fails startup. Missing or empty identity metadata also
blocks startup when any other directory entry remains, including pending files;
only an otherwise empty store may initialize a new owner. Restore the original
owner file, not a newly generated ID. `durable_execution.identity_initialization`
exposes this policy; it neither detects whole-store loss nor migrates old ledgers.
The identity is not a credential, peer authentication,
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

Orchestra checks terminal RPC envelopes before exposing results or typed errors,
including nested original-attempt receipts. Success requires a non-null result
and no non-null error; failure requires no non-null result and a typed error with
nonblank code/message. Omitted inactive fields and explicit null remain compatible;
non-null scalar/array results, nested nulls and extension payloads remain supported.
Terminal frames cannot carry `event` or `progress`; progress/heartbeat frames
cannot carry `ok`, `result` or `error`, even as null. Mixed progress frames fail
before callbacks. These are protocol failures, not valid capacity rejections or
local callback faults. Managed TaskIR dispatch remains unknown without peer replay
under idempotent/checkpointed routing, while valid pre-admission refusals still
permit ordinary failover. These framing checks are not scientific result qualification.

Before those checks, Orchestra decodes each Agent frame once while preserving
object pairs, then rejects repeated decoded keys in every object. This includes
arrays, failure details, nested cached receipts and opaque extensions; equal
duplicate values and escaped spellings of the same key are still invalid.
Keys in different objects remain independent. No Unicode normalization or numeric
re-encoding occurs: native integer precision, float bits and array order are
preserved. JSON syntax and duplicate-key errors expose only `invalid_json` or
`duplicate_json_key`, never the raw frame or rejected key. Original-result reads
classify these protocol failures as `agent_receipt_invalid`, not an unreachable
endpoint, without updating the journal or authorizing replay/publication.
The existing encoded frame cap still applies before decoding; temporary object
pairs and normalized trees use additional memory. This adds no decoded-heap cap.

Orchestra's `request_timeout_ms` is one network exchange budget, starting after
local request encoding and before connecting. Connection and send waits use only
the remaining budget; receiving a new frame does not reset it. Complete
heartbeats and partial frame bytes cannot renew it. Expiry is checked before and
after JSON decoding and before terminal output. The transport reuses its encoded
request rather than encoding again after connection. One-shot RPC sockets close
without waiting for abandoned output to drain, including receive timeouts when
the socket driver has buffered unsent request bytes. Send timeout errors never
expose a backend's unsent-data suffix. A timed-out send is not evidence of
non-admission: managed TaskIR keeps an unknown attempt and never retries a peer.
The execution lease is released so unrelated work can proceed. Encoding,
decoding and local callbacks are synchronous, not forcibly preempted; this is
not a CPU/memory limit, proof of remote termination or cancellation authority.

Observed execution generations and echoed cancellation targets must be positive
integers within the native `u64` range. Orchestra validates the closed target
shape and compares all four identity fields without numeric coercion: `3.0` or
`3e0` cannot acknowledge integer generation `3`. Invalid observation payloads and
raw protocol failures report `agent_observation_invalid`; invalid cancellation
acknowledgements report `agent_acknowledgement_invalid` with registration still
unknown, not false. Actual connection/receive failures remain unreachable. Rejected
evidence cannot rewrite dispatch history, retarget a newer execution, increment a
successful registration count, or authorize replay. Even a valid acknowledgement
does not establish remote termination. See the
[cancellation identity follow-up](../reports/orchestra-task-completion-20261005.md#control-plane-cancellation-identity-follow-up).

Dispatch inspection rereads its selected task/digest records after all bounded
Agent probes, before exposing observations or cancellation targets. One extra
batch lookup verifies inventory and the full selected set (up to 512 records),
including entries beyond the 128-row display limit. Changed membership or a
legitimate state transition returns `operator_task_dispatch_observation_changed`;
damaged or unavailable storage keeps its existing storage error. The former
rejects only the stale read, without poisoning healthy history. Unrelated task
or digest updates do not invalidate it. No automatic probe retry, execution,
cancellation or journal rewrite occurs. This is a point-in-time read check, not
an atomic snapshot across Agents or a lock extending through later cancellation.
See the [inspection snapshot follow-up](../reports/orchestra-task-completion-20261005.md#dispatch-inspection-snapshot-follow-up).

Explicit retained-dispatch cancellation also revalidates the selected journal
observation after endpoint discovery and before sending. Discovery can block;
changed, missing or corrupt evidence must not authorize a stale cancellation.
A legitimate transition rejects that request without poisoning healthy history;
unrelated attempts do not replace the selected attempt or trigger retargeting.
This is a pre-send check, not an atomic cancellation/commit operation. Storage
loss after sending does not turn delivery into `not_sent`: an exact valid
acknowledgement still reports its registration decision, while a lost reply
stays unknown. Neither grants termination, replay or journal-repair authority.
The Rust SDK keeps HTTP rejection unconfirmed and does not automatically retry.
See the [cancellation handoff follow-up](../reports/orchestra-task-completion-20261005.md#cancellation-pre-send-handoff-follow-up).

Immediate captured-request cleanup independently rechecks the original endpoint
fingerprint (ID, address, port and session) before observation and each send.
Descriptions may be shared within the bounded batch, but do not retain endpoint
membership authority. Sends use current matching endpoint metadata, not the old
capture. Removal or replacement reports `original_endpoint_not_configured`;
directory failure reports `endpoint_directory_unavailable`, without falling back
to captured addresses or static peers in registry mode. Earlier send receipts
remain intact, including unknown registration after a lost acknowledgement.
The 15-second delivery-start budget begins before discovery. Each directory wait
is capped by the remaining budget and five seconds; expiry after lookup prevents
another network action. Queued-only cleanup needs no directory read. These are
point-in-time membership gates, not an atomic registry/network transaction,
credential-rotation qualification or remote termination proof. Explicit public
cancellation remains separate. See the [live-directory follow-up](../reports/workflow-scoped-cancellation-20261013.md#live-directory-follow-up).

Terminal workflow reconciliation treats history as diagnostic metadata, not a
prerequisite for writing the already-retained terminal state. Non-list history or
an unusable final entry selects a state-specific fallback. A usable nonblank UTF-8
message takes precedence over a reason in that same entry; the final prefixed
reason must also fit the existing 4096-byte progress-message limit. Raw history
is retained unchanged, and completed/cancelled handling is unchanged. Local
SQLite/JSON regressions cover continued scanning and coordinator restart; this
does not authenticate terminal records, authorize replay or establish atomicity
against concurrent record edits. See the
[terminal-diagnostic report](../reports/workflow-terminal-diagnostics-20261028.md).

Durable workflow expiry sweeps now reuse this lookup after reserving each attempt
and after binding/authorizing the exact target, instead of retaining one endpoint
list per job. Because the final directory lookup can wait, the same delivery
reservation is authorized again afterwards. Changed reservations or damaged
intents cannot use the earlier authorization; failed authorization never rewrites
that evidence. A target observed before membership loss stays bound, so later
explicit recovery cannot select a newer process/request/generation.
`endpoint_directory_unavailable` and `delivery_budget_exhausted` are additional
private intent reason values. They are sealed, consume the existing three-attempt
budget, and end unresolved rather than fabricating cancellation acknowledgement.
No address, credential, raw error, field or new storage bucket is added. Older
reason values remain valid; older binaries are not promised to read new reasons.
Already-sent positive/negative acknowledgements and lost replies retain their
distinct outcomes despite later directory failure. These are bounded checkpoint
checks, not an atomic registry/storage/network transaction: state can change after
its final check, and storage waits are not forcibly preempted. See the
[durable handoff follow-up](../reports/workflow-durable-cancellation-20261013.md#durable-directory-handoff-follow-up).

The private intent also validates lifecycle consistency, not just individual field
types and the digest. The permitted entry phases are:

| Entry state | Attempts | Reason | Delivery and target |
| --- | --- | --- | --- |
| Fresh `pending` | 0 | None | No delivery nonce or bound target |
| Retriable `pending` | 1..2 | A recognized delivery failure | Valid nonce; target absent or retained |
| `dispatching` | 1..3 | None | Valid nonce; target absent or retained |
| `requested` / `target_not_observed` | 1..3 | None | Valid nonce and exact bound target |
| `exhausted` | 3 | A recognized delivery failure or `retry_budget_exhausted` | Valid nonce; target absent or retained |

`retry_budget_exhausted` is produced only when another reservation encounters an
already spent budget; it is not a valid delivery outcome for `finish`. A crashed
third reservation remains valid `dispatching` state until that later reservation
marks it exhausted. Contradictory records are rejected even if their digest has
been recomputed: startup and explicit recovery neither send them nor repair them
in place, and separately valid jobs can still progress. The read-only summary
marks invalid entry shapes/phases as `invalid` rather than reporting acknowledged
counts. It is not parent/digest authentication or proof of remote termination;
the SHA-256 seal is not a signature. No stored field or schema version changes.
See the [state-consistency follow-up](../reports/workflow-durable-cancellation-20261013.md#cancellation-state-consistency-follow-up).

Local concurrent-record regressions also exercise reservation, binding,
authorization and receipt completion after a separate writer changes storage.
Unrelated job/result annotations survive; a replaced intent or deleted job/result
cannot inherit the old delivery, resurrect records or block a separate healthy
job. Replacement intents require a fresh trigger. If the network request already
went out, rejecting its late receipt does not retract that request or prove remote
termination. Separate stale-snapshot tests cover the paired job/result commit for
reservation, binding and completion, preserving newer data and allowing bounded
explicit recovery. The handler-boundary tests and paired-commit tests are distinct;
they do not enumerate every interleaving or make the final check/send atomic.
See the [record-mutation follow-up](../reports/workflow-durable-cancellation-20261013.md#cancellation-record-mutation-follow-up).

Independent local child-VM tests now SIGKILL Orchestra after the Agent fixture's
acknowledgement, both before and after receipt persistence, then reopen the same
SQLite or JSON store in fresh VMs. Before persistence, the reserved attempt and
exact target survive as `dispatching`; startup recovery may retry that same target,
not infer success from a lost in-memory reply. After persistence, the acknowledged
entry is not sent again. Untouched entries still progress, and a changed peer
process identity cannot retarget the old reservation. Three consecutive crashes
consume the three stored attempts; the next recovery records exhaustion without
a fourth send and reports `unresolved`. Another fresh VM leaves the settled intent
unchanged and emits no RPC. This is local POSIX process-crash evidence with a
protocol fixture, not hardware power-loss, live PostgreSQL, native-Agent or Windows
qualification. See the [process-crash follow-up](../reports/workflow-durable-cancellation-20261013.md#cancellation-process-crash-follow-up).

Durable cancellation authority also includes a private, in-memory lease-acquisition
identity. Every successful coordinator acquisition replaces it, including return
after storage loss with the same database fence; normal renewal preserves it.
Bind, authorize and finish require both the coordinator session and that acquisition
identity in addition to the current database lease, intent and delivery nonce.
Old delivery tokens cannot adopt a reacquired lease or settle a late reply under it.
Their retained records remain unresolved; fresh explicit recovery consumes the
existing attempt budget and preserves any bound exact execution target. The marker
is neither a persisted checkpoint nor a wire credential, and does not change the
intent schema. It cannot retract a request already sent or make the last local
authorization atomic with the network send. See the
[lease-acquisition follow-up](../reports/workflow-durable-cancellation-20261013.md#lease-acquisition-authority-follow-up).

The entire durable sweep now pins that authority at launch, not just individual
reservations. Watchdog, deadline and recovery-scan launches pass their existing
coordinator authority directly; other internal callers capture it through a
guarded coordinator call before starting the worker. Every scoped load, reservation,
binding, authorization and finish rejects a changed acquisition before accessing
the intent. The first endpoint-directory lookup is followed by a scoped authority
check before observation. Stale sweep/delivery authority or ownership/storage
unavailability halts both entry and job traversal; the old worker cannot obtain
new delivery tokens or carry its cached description into the new acquisition.
Remaining entries keep their retry budget. Fresh explicit recovery re-reads the
records, preserves bound exact targets and observes unbound ones anew. Ordinary
renewal still permits a healthy multi-entry/multi-job batch. This adds no persisted
field or external RPC and does not make local authorization atomic with network
sending. See the
[whole-sweep follow-up](../reports/workflow-durable-cancellation-20261013.md#whole-sweep-authority-follow-up).

Concurrent valid kicks that find the sweep slot occupied now coalesce in the
existing coordinator: at most 128 distinct job IDs, 16 IDs per follow-up batch,
and a 30-second waiting deadline that duplicate triggers and later chunks cannot
extend. The coordinator monitors the incumbent, rechecks authority and slot
availability on exit, then starts a fresh exact-target sweep. Its snapshot exposes
`cancellation_handoff` with read-only limits, pending count, waiting state, remaining
wait and `last_error`; it does not expose queued IDs, monitor references or authority.
Overflow, expiry and unavailable Registry/worker supervisor remain visible without
consuming omitted jobs' attempts or inventing a receipt. `last_error` retains the
most recent queue error for that coordinator instance, even after a later drain.
The internal `kick` result only acknowledges trigger-worker startup, not queue
acceptance, delivery or remote termination.

Busy workers select at most 129 distinct, shape-valid input candidates before
sending that handoff message (128 queue slots plus one valid overflow sentinel).
Repeated or malformed IDs cannot consume that candidate limit or hide a later
valid job; first occurrence order is retained. Malformed members encountered
during selection collapse to one trailing `nil` diagnostic marker, never retained
raw values. Thus a message contains at most 129 valid IDs and one marker; only
valid IDs count toward capacity. Selection runs in the trigger worker and is
reapplied by the coordinator before merging, including for direct internal hints.
This bounds retained metadata, not traversal time for arbitrarily long duplicate
or malformed input. No deadline, authority or retry allowance is renewed. See the
[duplicate-prefix follow-up](../reports/workflow-durable-cancellation-20261013.md#busy-duplicate-prefix-follow-up)
and [valid-ID selection follow-up](../reports/workflow-durable-cancellation-20261013.md#cancellation-valid-id-selection-follow-up).

This queue is an ephemeral scheduling hint, not a persisted checkpoint or a new
service. Ownership loss discards it; coordinator restart recovers from durable
intents under fresh authority rather than restoring old queue tokens. Discarded
or unresolved work needs a later recovery trigger. An unknown reply alone does
not schedule another attempt, and retained triggers still obey the three-attempt
limit and original exact target. The waiting deadline does not preempt an already
started sweep or replace its existing 15-second delivery-start budget. There is
no solver resubmission, broad cancellation or global ingress-backpressure guarantee.
See the [bounded-handoff follow-up](../reports/workflow-durable-cancellation-20261013.md#bounded-cancellation-handoff-follow-up).

An initially idle sweep uses that same handoff for unvisited later jobs when its
16-job limit or 15-second delivery-start budget is reached. The continuation keeps
the initiating authority and original 30-second waiting deadline, including time
already spent in the first batch; overflow and expiry use the same visible queue
errors. Pending jobs already visited by one trigger are skipped if their IDs occur
again, so an unknown outcome cannot consume another attempt from a duplicate ID.
Deduplication retains only the current batch's visited pending-job IDs; continuation
selection retains at most 128 distinct valid IDs plus one valid overflow sentinel
and one invalid diagnostic marker, not a copy of the complete history. Selection
stops at that valid sentinel; invalid-only hints record an input error, not false
capacity overflow, and do not rearm existing waiting work. This bounds retained
metadata, not input-traversal latency.
Authority or storage failure still halts traversal rather than scheduling a retry.
A partially visited job is not automatically requeued: its unresolved entries need
a later recovery trigger. An abrupt worker kill before handing off also leaves its
private unvisited tail for later durable recovery; this is not a persisted cursor
or an exactly-once delivery guarantee. See the
[batch-continuation follow-up](../reports/workflow-durable-cancellation-20261013.md#idle-sweep-continuation-follow-up).

Idle traversal and deferred queues share the same internal job-ID shape check:
a binary of 1..1,024 bytes. An invalid list member is skipped before any result
lookup, so a malformed hint cannot crash the worker and abandon later valid jobs.
This is scheduling-input validation, not the stricter Agent execution-identity
contract or cancellation authority. Missing/legacy results and invalid sealed
intents remain non-actionable; storage unavailability still halts the sweep.
The change neither catches arbitrary exceptions nor grants another attempt for a
duplicate ID after an unknown outcome. All batch, wait and retry limits remain
unchanged. See the
[input-isolation follow-up](../reports/workflow-durable-cancellation-20261013.md#cancellation-input-isolation-follow-up).

Empty or entirely invalid handoff hints do not rearm an existing queue's holder,
monitor, timer or deadline; invalid input still records its diagnostic. When a
current-authority trigger with valid IDs and an unexpired incoming deadline is
admitted, any elapsed waiting queue is retired first, even if its expiry message
is still in the coordinator mailbox. Only that new trigger's IDs enter the new
queue, with its own remaining budget capped at 30 seconds. Old unmentioned IDs
remain in durable intent for later recovery; they are not implicitly retried.
Late expiry/ready/DOWN messages for the retired queue cannot clear the new one.
Unexpired queues still merge accepted triggers at the earliest deadline, including
duplicate triggers; stale authority and expired continuations cannot renew work.
Admission does not reset attempt counts, change targets or preempt an active sweep.
See the
[admission-expiry follow-up](../reports/workflow-durable-cancellation-20261013.md#cancellation-admission-expiry-follow-up).

Capacity admission is based on the retained 128-ID prefix, not merely on whether
the incoming IDs have valid shapes. If every incoming ID falls outside that
prefix, the coordinator records `cancellation_handoff_full` without replacing the
existing holder, monitor, timers or deadline. Rejected overflow cannot expire
already accepted work. Partial admission preserves existing order and accepts
only available slots; an incoming ID already in the retained queue still counts
as an accepted trigger. Both cases retain the earliest deadline. Overflow consumes
no attempt for omitted intents; they need a later trigger, and an unknown reply
alone does not authorize retry. Elapsed waiting queues are still retired before
capacity checks.
See the [capacity-admission follow-up](../reports/workflow-durable-cancellation-20261013.md#cancellation-capacity-admission-follow-up).

On each verified intent load, the sweep selects only pending/dispatching entries
and orders them by persisted attempt count, then by their original capture index.
An untouched entry therefore precedes repeated slow/unknown attempts on the next
recovery trigger, and equal counts retain capture order rather than lexical order.
This is a read-only view of at most 128 entries: the sealed stored list, exact bound
targets, receipts and three-attempt cap are unchanged. Corrupt intents are rejected
before sorting, and every reservation/send still uses the existing authority gates.
Restart or lease reacquisition derives the same priority from durable counts without
a new cursor or scheduling record. This is per-intent ordering at load time, not
cross-job/global fairness, continuous retry, guaranteed delivery within one budget
or automatic resumption of a partially visited job. See the
[partial-intent fairness follow-up](../reports/workflow-durable-cancellation-20261013.md#partial-intent-fairness-follow-up).

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
