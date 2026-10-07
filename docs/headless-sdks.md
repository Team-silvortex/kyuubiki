# Headless SDKs

Kyuubiki now ships a dedicated `sdks/` top-level directory for protocol-first,
headless integrations.

## Why these SDKs exist

The browser workbench is becoming a powerful editor and operator shell, but AI
models and automation systems should not need to drive a GUI to use Kyuubiki.

The headless SDK layer gives them a cleaner tool surface:

- discover the running deployment
- inspect protocol compatibility
- submit FEM jobs
- poll job state
- describe reachable solver agents
- talk directly to solver RPC agents when the control plane is optional
- build a machine-readable Rust execution plan before running a workflow
- start concrete material-research examples without opening the workbench

Headless SDK is a product philosophy and contract layer, not one executable,
one CLI, or one language binding. The SDKs are peer clients of the backend
service contract. They do not depend on Workbench state, WebView lifecycle, or
GUI automation hooks. The GUI uses the same backend HTTP contract through a
configurable transport target, so a feature is considered headless-ready only
when it is reachable without clicking a UI.

The Workbench TypeScript client follows the same rule internally. Its API core
can run outside a full browser `window`, resolves backend targets explicitly,
and reads only a lightweight in-memory secret store for operator tokens. That
keeps GUI convenience code separate from the service contract that headless
SDKs depend on.

Authentication descriptors in the Rust, Python, and Elixir SDKs retain their
simple constructor APIs, but their debug, `repr`, and `Inspect` forms never
render the credential value. Header names and token values are validated before
network I/O; empty values, control characters, embedded whitespace, oversized
values, and CRLF injection fail without echoing the rejected secret. The native
Rust service executor follows the same rule and exposes only whether a token is
configured in debug output. These are logging and transport boundaries, not a
credential persistence mechanism; callers still own secure token acquisition
and lifetime.

Rust Headless operator-task preparation also verifies that the TaskIR summary,
execution preview, and `kyuubiki.headless-operator-task-provenance/v1` profile
describe one digest-bound task. Profile tamper or summary/preview drift fails
before dispatch. Engine solver outputs follow the adjacent
`kyuubiki.engine-solver-provenance/v1` contract: canonical result content is
bound to SHA-256, the expected operator identity, and result type, and can be
checked with `verify_solver_result_provenance` before durable retention.

## Language targets

The official SDK families are expected to stay peer implementations over the
same protocol and data contracts:

- Rust: distributed as native crates and `cargo install` tools for engine-side
  embedding, solver agents, installers, local automation, and high-confidence
  reference runners.
- Elixir: distributed through Mix for orchestration, workflow composition,
  control-plane integrations, and fast functional iteration around operator
  descriptions.
- Python: distributed through pip for research scripts, notebooks, data
  analysis, optimization loops, and lab automation.

Domain CLIs such as `kyuubiki-material-explore` are reference runners built on
top of the SDK contracts. They are intentionally not the universal headless
gateway. External users should be free to write their own wrappers, pipelines,
build systems, and research harnesses on top of the same schemas.

The stable headless surface is the contract set: task and workflow envelopes,
operator descriptors, result bundles, report schemas, review records,
materialization plans, lineage metadata, and execution status semantics.

Repository development and research automation should enter that surface from
one command family:

```bash
cargo kyuubiki headless templates
cargo kyuubiki headless init --template direct_heat_bar --out workflow.json
cargo kyuubiki headless run workflow.json --json
```

Run these commands from the repository root. The repository-local Cargo alias
and `scripts/kyuubiki` compatibility shim both enter the same native Rust
runner. Do not assemble a `cargo run -p ... --bin ...` command in automation:
that leaks workspace layout into research projects and can select the wrong
binary or working directory.

The Rust CLI keeps preview and research execution distinct. `--execute` always
requires an explicit `--executor`; it never silently selects `mock`.
`--execution-posture research` accepts only the `service` executor because
`mock` and the browser-capable `hybrid` path cannot provide a no-mock
guarantee. Local material exploration is a separate native reference runner:
its artifacts carry `kyuubiki.execution-authority/v1` and identify the linked
Rust solver kernels as the result source.

`headless run --json` also keeps machine output complete on failure. Once run
options are parsed, document decode, material-report validation, executor
selection, executor compatibility, and endpoint configuration failures emit a
`kyuubiki.headless-execution-run/v1` report to stdout and to `--report-out`
when requested. The process still exits nonzero and writes the compact
`kyuubiki.headless-cli-error/v1` diagnostic to stderr. The run report uses
`status: invalid`, retains validation issues, and exposes the stable reason in
`execution_summary.failure`; automation no longer has to infer failure from an
empty JSON file. Runtime failures use the same run-report envelope with failed
step evidence. A decoded batch that fails contract validation is rejected before
dry-run preview or any mock, service, or hybrid executor call, with
`executed_step_count: 0` and an empty `steps` array.

Retry safety remains narrower than error reporting. The service `job_wait`
path can resume polling the same accepted `job_id` under a bounded server
deadline. The CLI does not replay an entire workflow automatically, because a
blind replay could duplicate non-idempotent submissions or side effects.
Legacy or third-party workflow documents with a fixed polling budget can use
an explicit per-run override without duplicating a large input artifact:

```bash
cargo kyuubiki headless run workflow.json --execute --executor service \
  --api-base-url http://127.0.0.1:4000 --job-wait-timeout-ms 1200000
```

The override rewrites every `job_wait` in the in-memory execution batch, never
shrinks an existing `max_total_timeout_ms`, and records the change in run
warnings. It does not mutate the source document or either server-side timeout.

Iterative research must also prove that each round changed the intended
physical input. Use a guarded `kyuubiki.headless-parameter-patch/v1` document
instead of ad-hoc text replacement or rebuilding a large workflow in a shell
script:

```bash
cargo kyuubiki headless validate workflow.json \
  --parameter-patch schemas/examples.headless-parameter-patch.json --json
cargo kyuubiki headless run workflow.json \
  --parameter-patch schemas/examples.headless-parameter-patch.json \
  --execute --executor service --execution-posture research --json \
  --report-out round-2-run.json
```

Every change targets an existing zero-based JSON Pointer below
`/steps/<index>/payload/...` and includes both `expected` and `value`. The SDK
rejects missing paths, duplicate paths, workflow or template mismatches,
baseline drift, no-op replacements, and attempts to alter actions, risk, or
document identity. Patch documents are bounded to 8 MiB, and mismatch errors
fingerprint rather than echo compound or string values. Application is atomic.
A successful call returns a
`kyuubiki.headless-parameter-patch-receipt/v1` record with canonical before and
after SHA-256 fingerprints over execution content; diagnostic warnings are
excluded so provenance text cannot change physical-input identity. CLI runs
retain the same receipt fields in a batch warning and can write the structured
receipt with `--parameter-patch-receipt-out`. `inspect`, `validate`, `render`,
`plan`, and `run` all apply the same patch path, so preflight and execution
cannot silently observe different rounds.

For a retained research loop, add a
`kyuubiki.headless-research-round-spec/v1` document. It names the round and
iteration, binds the intended workflow, and selects numeric domain results
through canonical JSON Pointers. Research support artifacts are bounded to
16 MiB before decoding:

```bash
cargo kyuubiki headless run round-1.batch.json \
  --execute --executor service --execution-posture research \
  --research-round-spec round-1.spec.json \
  --research-round-out round-1.evidence.json \
  --report-out round-1.run.json

cargo kyuubiki headless run round-1.batch.json \
  --parameter-patch round-2.patch.json \
  --parameter-patch-receipt-out round-2.receipt.json \
  --execute --executor service --execution-posture research \
  --research-round-spec round-2.spec.json \
  --previous-round-evidence round-1.evidence.json \
  --research-round-out round-2.evidence.json \
  --report-out round-2.run.json
```

The resulting `kyuubiki.headless-research-round-evidence/v1` artifact is
emitted only when every batch step completed through the service executor and
every declared metric resolved to a finite number below
`/steps/<index>/result_preview/result/...` or `/metrics/...`; job progress,
status, and echoed inputs cannot qualify as domain measurements. Iteration 2
and later additionally require
contiguous previous evidence and a patch whose before/after fingerprints match
the previous/current batch. A repeated batch, skipped round, mock result, stale
patch, missing metric, or literal `n/a` therefore fails qualification instead
of becoming a misleading success table.

Qualified rounds can be exchanged as a self-contained `.kcore` research
series. Bind the latest `evidence.research-round` entrypoint with the
`headless-research-round` KCore contract, and include each round's
`workflow.execution-batch` and `evidence.execution-run` plus every later
round's `workflow.parameter-patch`. The native KCore exporter and verifier
recompute report and batch identities, walk the complete ancestry, and replay
each patch against the previous batch. A structurally valid archive with a
missing or stale research artifact is therefore rejected. See
[kcore-exchange-format.md](./kcore-exchange-format.md#headless-research-round-profile).

Use the native packaging helper instead of assigning those fixed fields by
hand:

```bash
cargo kyuubiki kcore research-export research-series.json \
  --out thermal-research.kcore
```

`research-series.json` follows
`kyuubiki.kcore-headless-research-series/v1`: round one lists its effective
batch, run report, and evidence; each later round lists the same three files
plus its guarded parameter patch. The builder fixes all KCore roles, schema
references, contract bindings, and the latest evidence entrypoint.

A repeated workflow with identical before/after payloads is not a research
iteration even if every process exits zero. Research-loop qualification should
require a distinct patch receipt and should read domain-specific result fields
such as `max_temperature`, `max_stress`, or ranked workflow artifacts instead
of filling unrelated material or electrostatic columns with `n/a`.

Material-report workflows also fail closed on duplicated material-property
drift. For dielectric screening, `research.relative_permittivity` is the
dimensionless source value while each solver element stores absolute SI
`permittivity` in F/m; edit both together or regenerate the candidate model.

Service execution retries only transient TCP connection failures that happen
before any request bytes are written. Interrupted writes and response failures
are never replayed automatically, so POST-based job submission remains
at-most-once from the Headless client's perspective.

Coupled multiphysics routes are discoverable through the Rust SDK's
`coupled_workflow_catalog()`, `find_coupled_workflow()`, and
`search_coupled_workflows()` APIs. The catalog is projected from protocol-owned
descriptors, so SDK callers receive the same source artifact, result artifact,
physical-domain, and bridge-operator contracts that the engine dispatches.

The Rust headless SDK now exposes a machine-readable surface index through
`headless_sdk_surface_manifest()` under `workers/rust/crates/headless-sdk`.
Treat that manifest as the compact source-of-truth for headless capability
families: contracts, execution planning, direct FEM routes, templates, Operator
TaskIR, material research, retained research artifacts, and workflow dataset
preflight. The same manifest now includes a model-collaboration area that
projects the authoritative action catalog into OpenAI, Anthropic, Gemini, and
canonical tools, then compiles untrusted proposals back into the existing
Headless execution plan. See [model-collaboration-sdk.md](./model-collaboration-sdk.md).
This is separate from the Rust-only operator SDK used to author and package new
operators.

Minimal end-to-end examples:

- [sdks/python/examples/run_study.py](../sdks/python/examples/run_study.py)
- [sdks/elixir/examples/run_study.exs](../sdks/elixir/examples/run_study.exs)
- [sdks/rust/examples/run_study.rs](../sdks/rust/examples/run_study.rs)
- [sdks/python/examples/execute_operator_task_batch.py](../sdks/python/examples/execute_operator_task_batch.py)
- [sdks/elixir/examples/execute_operator_task_batch.exs](../sdks/elixir/examples/execute_operator_task_batch.exs)
- [sdks/rust/examples/execute_operator_task_batch.rs](../sdks/rust/examples/execute_operator_task_batch.rs)

The operator task batch examples read files shaped like
[schemas/examples.operator-task-batch.json](../schemas/examples.operator-task-batch.json).
The intended producer is the workflow transform
`transform.compose_quality_execution_batch`, which turns expanded optimization
cases into language-neutral TaskIR envelopes.

Rust SDK dry-run and mock execution previews verify TaskIR before dispatch.
Failures expose both a human-readable `error` and a stable `error_code`, such
as `operator_task_digest_mismatch`, `operator_task_mirror_mismatch`, or
`operator_task_execution_abi_mismatch`, so automation can branch before
contacting an orchestra or solver agent. A digest-valid TaskIR can still be
rejected as `operator_task_admission_rejected`: digest integrity does not make
an authority downgrade, mismatched central package identity, offline
Orchestra fetch, unsupported cache scope, or malformed routing list safe.
Successful previews include a
`kyuubiki.operator-task-admission/v1` report. The same report shape is produced
by the Elixir control plane and enforced again by the Rust Agent before package
resolution or engine execution, so Headless preflight cannot silently weaken
the runtime boundary.

### Native batch completion gate

The Rust native batch executor under `workers/rust/crates/headless-sdk` separates
HTTP delivery from task execution. `operator_task_execute` validates TaskIR
before any executor dispatch or service connection. Service replies must match
the submitted `task_id`, `task_digest`, `operator_id`, and `program_id`.
An outer Orchestra `executed` envelope cannot override a nested Agent's blocked
or pending TaskIR receipt. Missing results, unknown states, contradictory
readiness, or mismatched identity fail closed without echoing rejected values.
Executed readiness cannot simultaneously declare a blocking stage, blocking
reason, or required repair action.

Only `HeadlessExecutorOutcome.status == "executed"` publishes bindings and
increments `executed_step_count`. `blocked`, `failed`, `cancelled`, and unknown
executor statuses halt the batch before later side effects. A blocked receipt
retains its readiness and recovery action; it is not a confirmation receipt
unless `blocked_by_confirmation` explicitly identifies a confirmation gate.
The mock TaskIR execute path remains a blocked preview, not a solver result.
Explicit CLI execution returns a nonzero exit code for blocked runs, after
writing the requested run report; JSON stderr identifies
`headless_execution_blocked` with `retryable: false`. Dry-run remains a separate
planning mode. These are native batch/CLI guarantees, not a claim that all
three standalone SDK clients or installed deployments have been qualified.

Local verification is retained in
[native-headless-task-completion-20261005.md](../reports/native-headless-task-completion-20261005.md):
malformed/stale receipt rejection, side-effect isolation, full-result binding
preservation, and a real development-Agent TCP sequence with an independent
axial-bar displacement check. The test-only HTTP envelope adapter does not
replace an actual Orchestra deployment or qualify unresolved modal materials.

The same preview payload now includes a
`kyuubiki.headless-operator-task-failure/v1` `failure_receipt` with the failed
stage, task identity when available, and a recovery action. This mirrors the
agent-side `operator_task_failure_receipt` shape while staying local to SDK
dry-runs.
Python and Elixir SDK helpers can recursively extract both receipt shapes from
control-plane, batch, or agent RPC payloads, so automation can route recovery
without knowing where the service embedded the failure envelope.
Control-plane batch preparation/execution failures use
`kyuubiki.control-plane-operator-task-failure/v1` and are surfaced both on the
failed case entry and in the batch-level `failure_receipts` list.
Batch checkpoints retain those receipts in their preparation/execution summary,
and resume plans expose `recovery_actions` so automation can decide whether to
retry failed cases or repair invalid TaskIR/batch entries first.

### Official Rust model task completion gate

The standalone SDK under `sdks/rust` checks synchronous task completion inside
`execute_model_headless_plan`, including custom `ModelActionDispatcher`
implementations. `operator_task_execute` binds the receipt to the submitted
task id, digest, operator, and program. Nested Agent state and present
validation/provenance mirrors are checked separately; an outer dispatch-success
wrapper is not sufficient.

`operator_task_batch_execute` additionally verifies the batch digest, execution
contract, complete counts, unique case coverage, and each case's task identity,
publication, and readiness. Partial, duplicate, stale, skipped, failed, or
contradictory receipts stop before later plan actions. Result order may differ
from request order; cases are matched by id rather than position.

The existing v2 model receipt records the attempt as `failed`, retains the
problem step's authority and unmodified output, and counts only preceding
successful actions. This means the plan did not complete, not that every
preceding calculation failed. A caller-verified receipt can enter a blocked
research frontier. `operator_task_recovery_summary` also extracts deduplicated
`required_action` values from blocked or package-resolution readiness. Repair
the named gate and explicitly invoke another approved execution; no automatic
resubmission or permission bypass occurs.

Low-level `ControlPlaneClient` methods still return raw HTTP receipts, including
blocked or partially executed batches. Their `Ok` proves transport success,
not calculation success. Preparing a task and submitting an asynchronous job
remain distinct actions and do not require a synchronous solver result. TaskIR
admission remains the runtime's responsibility; this standalone gate checks
receipts rather than duplicating the Agent engine or full TaskIR validator.
See [Rust SDK task completion verification](../reports/rust-sdk-task-completion-20261005.md)
for the bounded local Rust tests. Python/Elixir parity, installed acceptance,
authenticated deployment, and scientific qualification are separate remaining
gates.

### Orchestra task completion and batch recovery

Orchestra now validates Agent completion before publishing an executed TaskIR
envelope. Receipt task id, digest, operator, and program must match the submitted
task; present validation/provenance mirrors must agree. Pending execution or
package resolution remains `blocked`, rather than becoming completed merely
because the RPC returned successfully. Executed readiness cannot still declare
a blocking stage, reason, or required action. Only absent legacy readiness is
normalized; explicitly malformed readiness is rejected, without echoing rejected
receipt values in the error.

Tasks declaring `execution_mode: agent_native` now cross the Agent's
`run_operator_task_ir` boundary rather than being converted back to legacy solve
RPCs. An unbundled native task with `agent_fetchable: false` does not require an
attached external-package runtime. Package references and central fetch tasks
still require package readiness. `agent_fetchable` must be a JSON boolean;
strings such as `"false"` are rejected during admission, before selecting an
Agent. This routing change does not expand the Agent's advertised operator set.

An Agent RPC execution error carrying
`kyuubiki.agent-operator-task-failure/v1` is validated before Orchestra publishes
a `status: failed` execution envelope. HTTP 200 means the structured execution
receipt was delivered, not that the calculation succeeded. The failure owner,
schema, task id/digest/operator, optional program id, RPC code/message, recovery
booleans, and readiness must agree. Malformed or stale receipts return
`operator_task_execution_receipt_invalid` without reflecting rejected values.
Consumers bound identity strings to 1,024 UTF-8 bytes, stage/code/action to 128
bytes, and the failure message to 4,096 bytes. Unknown failure/recovery fields
are not forwarded by Orchestra. The message bound also keeps the native receipt
intact under run-report compaction.

Failure details remain in `failure_receipt`: branch on `reason_code`,
`failure_stage`, and `recovery.required_action`, not message text. The native Rust
run summary derives typed retryability and recommended action from that bound
receipt; its general Headless category remains `runtime_failure`, with the exact
Agent code/stage retained in the step preview. A task failure is not a transport
outage, does not penalize Agent connection health, and never automatically
fails over or replays. Existing generic RPC errors and capacity queuing retain
their prior behavior; retry advice is not replay authorization.

TaskIR cancellation before computation (`before_execution`), at cooperative
numerical safe points (`execute_solver`), or when observed at the result
publication boundary (`publish_result`) retains the submitted task id, digest,
and operator in the same failure contract. Cancellation is a failed execution,
not a successful calculation. Its action is
`inspect_cancellation_before_explicit_rerun`, with `retryable: false`; no later
native batch action is authorized. If a watchdog has already recorded a failure
for this execution generation, the RPC code/message and bound receipt retain
that terminal reason, rather than rewriting it as a later cancellation.
`watchdog_timeout` requires `inspect_watchdog_timeout_before_explicit_rerun`.

The Agent's recent-failure list is a bounded diagnostic view of 16 entries, not
the owner of an execution's terminal cause. One immutable failure record is shared
by that generation's live execution guards. Diagnostic eviction cannot replace
its request/job/method identity, original reason, generation or timing, and late
callbacks do not count or reinsert the same failed execution again. The separate
record is released with the last execution guard; it is not a disk archive or
restart-recovery ledger. Already-known failures remain readable if watchdog state
becomes unavailable, without reopening admission to new execution.

Legacy solver RPCs use that same original reason for both the outer error code
and failure details, including parameter decoding and result encoding failures.
A later decode/transport error cannot replace an already-recorded watchdog
timeout. Without a prior terminal failure, these paths still report
`invalid_params` or `result_transport_failed`, respectively. Failure replies
contain no success result or success progress; an explicit rerun has a new generation.

The final publication gate runs after the heartbeat thread has stopped, so an
in-flight heartbeat write failure cannot set cancellation after that check.
It also checks the current watchdog generation: an already-terminal execution
cannot publish success while its solver cancellation signal is still propagating.
The retained original-attempt response uses this gated reply, including a typed
`publish_result` failure instead of the computed success value. Legacy solver RPCs
likewise recheck after serialization and heartbeat shutdown, discarding success
responses and progress frames when cancellation or termination is observed.

The Agent's `solver_checkpoint` is diagnostic only (`resumable: false`), not a
numerical continuation snapshot. Orchestra forwards known failure fields, not
this separate RPC diagnostic. Local HTTP regressions cancel an owned Agent
directly, check that no downstream project or partial result is published,
and explicitly rerun the identical task on the same Agent. They cover a
precomputation axial bar, 2D modal matrix multiplication, and 3D modal final shape
validation. The publication boundary and already-recorded watchdog reason are
tested separately at the Rust unit boundary. Deterministic thread-shutdown tests
inject cancellation and watchdog termination only during heartbeat stop; a separate
legacy RPC regression injects cancellation during result serialization. They are
boundary fixtures, not a real-heartbeat transport fault or numerical qualification.
This does not qualify Orchestra
multi-Agent cancellation routing by itself; the separate known-dispatch
regression below covers the public asynchronous cancel path locally. Neither
qualifies authenticated deployment or cancellation after publication has begun.
There is no atomic cancel-versus-send guarantee: cancellation arriving after
the last publication check may race with response delivery.

#### Known-dispatch cancellation

`POST /api/v1/jobs/{job_id}/cancel` commits the control-plane cancellation intent
and returns a separate `cancellation` object using
`kyuubiki.orchestra-job-cancellation/v1`. It snapshots active execution leases
for that job, not the current Agent pool or a historical worker id. Queued and
reserved requests are cancelled locally; dispatched requests contact only their
captured endpoints. Active capacity is retained until the execution caller
finishes or exits. The total server-budget cleanup path captures those targets
before killing its local waiting task, so owner cleanup cannot erase the target.
Locally cancelled transport failures cannot automatically fail over; an existing
native TaskIR failure receipt is preserved instead of flattened to a generic error.

Inspect `status`, `target_count`, `registered_count`, local cancellation counts,
and per-Agent `targets`. `requested` means cancellation registration was
acknowledged, not that computation has stopped. `partially_requested` and
`delivery_failed` retain unsuccessful targets without broadcasting to idle peers.
`no_active_dispatch` means no matching live lease was found, not that no remote
execution exists. The acknowledgement sets `execution_terminal_confirmed: false`;
the persisted job's cancelled intent is distinct from a terminal native receipt.

Native `kyuubiki.agent-job-cancellation/v1` acknowledgements likewise distinguish
`cancel_registered` from `operator_package_job_release`. A bound-package cleanup
failure stays in that release field without rejecting an already registered
cancel. Unbound jobs normally return `already_released`, not a cleanup error.
The legacy `cancelled` field means registration, not proof of stopped execution.

#### Exact native execution cancellation

The separate Agent `cancel_execution` RPC selects one observed execution, not a
whole job or a future admission. Read `lifecycle.process_instance_id` and the
matching `solver_control.active` entry from `describe_agent`, then send all four
identities unchanged. The cancellation RPC's own `id` is not the target request id:

```json
{
  "rpc_version": 1,
  "id": "cancel-control-request",
  "method": "cancel_execution",
  "params": {
    "process_instance_id": "<observed Agent process instance>",
    "request_id": "<observed execution request>",
    "generation": 1,
    "job_id": "<observed job>"
  }
}
```

Use the observed generation, not a default of `1`. Missing, null, unknown or
malformed fields fail with `invalid_params`; generation must be a positive u64
integer. Identity strings must be nonblank, control-character-free and at most
256 UTF-8 bytes. The native validator does not normalize or truncate identities.
Unavailable process identity fails with `agent_lifecycle_unavailable` before any
cancellation registration. This identity fence is not authentication or authority
to contact an Agent; use the existing trusted control channel.

The acknowledgement uses `kyuubiki.agent-execution-cancellation/v1` and echoes
`execution_target`. `requested` with `cancel_registered: true` means the exact
live control was marked, not that the solver stopped or a terminal receipt was
delivered. `target_not_observed` with `cancel_registered: false` means the exact
control was not observed; it does not prove the original task never executed.
Both outcomes keep `execution_terminal_confirmed`, `pending_cancellation_created`,
`operator_package_cleanup_performed` and `automatic_replay_authorized` false.
There is no job-wide fallback, pending cancellation or package-cache cleanup.
Read the original result separately, and authorize rerun explicitly.

Local owned-Agent regressions cover a same-job sibling, an unrelated Agent with
the same request/generation, late cancellation during request-id reuse, and
process restart with generation reuse. Three real Rust SDK/Orchestra TaskIR chains
now use the public retained-dispatch path below for precomputation, 2D modal and
3D modal cancellation, then verify typed failure, downstream isolation and healthy
rerun. This does not change `POST /api/v1/jobs/{job_id}/cancel`, which remains
job-scoped. Evidence remains local, not remote or installed qualification.

#### Exact retained-dispatch cancellation

`POST /api/v1/operator-tasks/cancel-dispatch` requires write authorization and
exactly `task_id`, `task_digest`, `attempt_id` and `execution_target`. The target
contains the four native identities above; its job must equal the task id and
its request must equal the selected retained dispatch. Callers cannot supply an
endpoint, select the latest generation implicitly or request job-wide fallback.
The server sends one `cancel_execution` RPC only to that record's original
currently configured endpoint/session fingerprint. Missing attempts, retained
terminal observations, undispatched attempts and replaced or absent endpoints
cause no Agent RPC. The journal, job status and result publication are unchanged.

Use `inspect_operator_task_dispatch` first and explicitly choose an attempt.
A compatible active observation includes `execution_target`; null or missing
means no usable cancellation target was observed. Do not fabricate one from a
worker id or reuse it as permission to rerun. In Rust:

```rust
use kyuubiki_headless_sdk::CancelExecutionRequest;

let inspection = executor.inspect_operator_task_dispatch(task_id, task_digest)
    .map_err(|error| error.message)?;
let attempt = inspection["attempts"].as_array()
    .ok_or("missing attempts")?.iter()
    .find(|attempt| attempt["attempt_id"] == selected_attempt_id)
    .ok_or("selected attempt not observed")?;
let target: CancelExecutionRequest =
    serde_json::from_value(attempt["observation"]["execution_target"].clone())?;
let cancellation = executor.cancel_operator_task_dispatch(
    task_id, task_digest, selected_attempt_id, &target,
).map_err(|error| error.message)?;
```

Inspect the `kyuubiki.operator-task-dispatch-cancellation/v1` acknowledgement,
not HTTP success alone. `requested` means the native acknowledgement registered
that exact target; `target_not_observed` means it did not. Neither proves terminal
execution. Transport failure or an invalid Agent acknowledgement produces
`cancellation_outcome_unknown` with `cancel_registered: null`, not false. The
SDK verifies identity mirrors, all status/registration combinations and both
layers' no-terminal/no-replay authority flags. A client-side request error also
does not authorize automatic retry, fallback or replay. Fetch the original
computation receipt separately before deciding any explicit recovery.

The Agent control round trip is bounded to five seconds and a 64 KiB RPC frame;
the Rust SDK shares a ten-second HTTP deadline. No target probe, job-wide cancel,
package cleanup, durable cancellation intent or journal mutation is added.
The observed process/generation is explicit caller input, not an attestation of
the original process at dispatch. Expired or stale identity is never replaced
automatically. These guarantees do not make cancellation atomic with final reply
delivery or qualify Python/Elixir SDK parity or authenticated remote deployment.

#### Legacy job-wide cancellation

The direct Agent `cancel_job` RPC preserves one-shot pre-admission cancellation:
when no matching execution control is live, it marks the next admission under
that job id for cancellation. Do not send a late job-wide cancel merely to clean
up an already-terminal original attempt; it can affect an explicit rerun using
the same job id. Inspect the original attempt's receipt separately.

A local two-real-Agent regression saturates both slots, queues an asynchronous
spring job, then cancels it only on its owner. The peer's held TaskIR remains
uncancelled; reusing the cancelled job id on that peer does not consume a stray
cancel. A following public job completes with independent displacement, force,
and strain-energy checks. Controlled tests additionally cover queue/reservation
cancellation, multiple known owners, failed acknowledgements, retained capacity,
typed failure preservation, and capture before local task shutdown. These do not
qualify durable target recovery after Orchestra restart/transport loss, remote or
authenticated deployment, every asynchronous solver, or atomic cancel-versus-send.

#### Synchronous TaskIR waiting budgets

The native Rust `ServiceHeadlessExecutor` uses an explicit bounded waiting policy
for `operator_task_execute`, separate from `job_wait` and metadata requests.
The default is 120,000 ms of Agent capacity waiting and 120,000 ms of Agent RPC
waiting, with 10,000 ms additional connection/framing allowance: a 250-second
monotonic HTTP budget. DNS, connection retries, request writes and response reads
share that deadline; incoming bytes do not renew it. HTTP has no Agent heartbeat
stream, so a silent computation is not cut off by the metadata client's 30-second
idle limit. Task response framing is capped at 64 MiB; job status polling retains
its separate 8,000,000-byte limit and existing deadlines.

Configure the native executor with a validated budget:

```rust
use kyuubiki_headless_sdk::{OperatorTaskRequestBudget, ServiceHeadlessExecutor};

let budget = OperatorTaskRequestBudget::new(120_000, 240_000)
    .map_err(|error| error.message)?;
let executor = ServiceHeadlessExecutor::try_new("http://127.0.0.1:4000")
    .map_err(|error| error.message)?
    .with_operator_task_budget(budget);
```

A step can override that default with `execution_budget` beside `task`, using
`kyuubiki.operator-task-request-budget/v1`. The policy has exactly
`schema_version`, `queue_timeout_ms` and `request_timeout_ms`; both phases require
integer encodings in `1..=600000`. Explicit nulls, unknown fields, strings,
booleans, decimal encodings and out-of-range values fail before dispatch instead
of falling back. The schema and sample are
`schemas/operator-task-request-budget.schema.json` and
`schemas/examples.operator-task-request-budget.json`.

`POST /api/v1/operator-tasks/execute` validates this envelope, forwards the queue
and RPC budgets to the Agent client, and echoes the accepted policy separately
from task identity. TaskIR digest and numerical configuration stay unchanged.
Budgeted requests require a checkpoint before replay after send/receive failure,
including pure solver tasks that otherwise have legacy idempotent routing.
Connection failure before dispatch may still select another candidate. Requests
without this envelope retain the service's existing defaults and routing policy.
These phase limits are not a global multi-Agent scheduler deadline or CPU kill
policy. Orchestra-local operators are not forcibly interrupted by them.

If native HTTP transport fails, the report uses
`kyuubiki.headless.operator_task_outcome_unknown` with `retryable: false` and
`retry_strategy: none`. The chain stops before downstream actions; inspect the
submitted task on Orchestra and its owner before an explicit rerun. Timeout is
not evidence of cancellation, nonexecution, or absence of a remote result.
Normal task-bound receipt validation still applies after a successful response.
Native batches apply this policy per task step, not as one overall batch budget;
it is not currently the `execute-batch` endpoint's envelope.

Local regressions cover a silent response beyond the old 30-second cutoff,
bounded stalled requests, slow-drip deadline exhaustion, invalid policy rejection,
unchanged signed tasks, full results larger than the status limit, queue cleanup,
and no post-dispatch replay to an idle peer. Modal cancellation stage observation
now uses the same 120-second execution budget instead of an unrelated 15-second
window. Model sizes and numerical tolerances remain unchanged. This resolves the
specific waiting mismatch exposed by the earlier loaded-host run, but does not
qualify remote performance, installed deployment or cancellation after disconnect.

#### Retained TaskIR dispatch inspection

`POST /api/v1/operator-tasks/inspect-dispatch` is a read-authorized, read-only
observation endpoint. Supply exactly `task_id` and the submitted `task_digest`;
caller-supplied hosts, ports and other fields are rejected. The native Rust SDK
exposes it without executing a new task or reusing the failed HTTP connection:

```rust
let observation = executor
    .inspect_operator_task_dispatch(task_id, task_digest)
    .map_err(|error| error.message)?;
assert_eq!(observation["automatic_replay_authorized"], false);
```

Before crossing the Agent RPC boundary, execution-mode TaskIR records an attempt
ID, RPC request ID, task/operator/program identity and a fingerprint of the chosen
Agent ID, address and registration session. It stores no input model, result,
token, raw address or raw session. A verified Agent response may become
`observed_executed`, `observed_failed` or `observed_blocked`; pre-connect failure
or explicit capacity rejection becomes `not_dispatched`. Malformed or mismatched
completion, transport loss and caller death remain unknown or
`dispatch_boundary_unconfirmed`. Responses are checked with the existing
task-bound completion gate before retaining a successful observation.

The journal is a separate bounded runtime sidecar, independent of SQL table
migrations: `operator-task-dispatches/*.json` under `KYUUBIKI_DATA_DIR` (or the
existing default runtime data root). One Orchestra process owns each journal
directory. SQLite, Postgres and memory backends share this file contract.
It has at most 512 records of at most 4,096 bytes each, plus at most one in-flight
replacement, and retains at most 128 confirmed-response history records.
Unresolved records are never automatically evicted. Saturation stops new TaskIR
dispatch with `operator_task_dispatch_journal_full`, rather than deleting
unconfirmed ownership. Unknown generations, incomplete replacements, corruption,
symlinks or unavailable storage fail closed; there is no previous-generation
rollback. Local checksum checks detect accidental corruption, not hostile
filesystem tampering or cryptographic execution provenance. Atomic file
replacement and synced contents have local process-restart tests, not power-loss
or shared-directory multi-writer qualification. Current policy is visible in the
inspection response, not a new user-editable scheduling or storage setting.

Inspection resolves only the original fingerprint against currently configured
or registered targets. A changed address/session is not probed; registry mode
does not fall back to an unrelated default endpoint. At most four distinct
targets are probed, with 1.5-second connection limits, 2-second RPC waits and
1-MiB frames, and at most 128
attempts are returned with explicit truncation metadata. SDK HTTP observation
uses a separate 20-second deadline. Only `describe_agent` is sent; no broadcast,
cancel, package operation, replay, pool health mutation or result publication
occurs. The descriptor is reduced to a matching active request's process ID,
generation and cancellation flag; raw Agent diagnostics are not echoed.

`original_endpoint_reports_active_request` is a current observation, not proof
of eventual completion. `request_not_observed_active`, unreachable/replaced
targets and `no_retained_dispatch` do not prove cancellation or nonexecution.
Process identity is observed at inspection time, not pinned before dispatch.
All responses have `automatic_replay_authorized: false` and
`terminal_result_available: false`; this is neither a result cache nor an
exactly-once protocol. Existing explicitly chosen legacy idempotent routing
is unchanged, and the journal never grants extra retry permission. Native
budgeted requests keep their post-dispatch no-replay boundary. Confirmed old
observations can age out, and this sidecar does not cover Orchestra-local
operators, direct asynchronous solver submissions or external-package cleanup.

Local tests cover bounded retention, corruption and partial writes, owner death,
read authentication, changed targets, probe/response limits, and a real two-Agent
chain. That chain times out a held TaskIR, restarts its owned temporary Orchestra
with the same journal directory, observes the same attempt, and verifies that
an idle request remains unknown. A later explicit rerun has its own attempt and
independent axial-bar correctness checks; the first attempt is not rewritten as
successful. Remote cancellation, authenticated deployed recovery and installed
acceptance remain separate gaps. Original-result retrieval is a separate opt-in
read below, not part of the inspection response. See
`schemas/operator-task-dispatch-record.schema.json`,
`schemas/operator-task-dispatch-inspection.schema.json` and its sample.

#### Original TaskIR attempt result retrieval

After inspecting retained dispatches, explicitly choose an `attempt_id` and call
`executor.fetch_operator_task_result(&task, attempt_id)`. The Rust SDK verifies the
supplied TaskIR locally and sends only its ID/digest and the selected attempt to
`POST /api/v1/operator-tasks/fetch-dispatch-result`. This read-authorized route
resolves only the attempt's original configured Agent fingerprint, sends one
`fetch_operator_task_result` RPC, and never broadcasts, retries execution,
updates the dispatch journal or publishes a project/job result. Caller-supplied
destinations are rejected. RPC limits are 10-second waits, 1.5-second connections
and 9-MiB frames; the SDK has a 15-second total deadline and 10-MiB response limit.

Orchestra adds the random dispatch attempt ID to the RPC envelope, outside TaskIR
and its digest. Execution requests carrying that correlation reserve a
process-local Agent entry and retain the computation response before transport
delivery, including final typed cancellation/failure receipts. Task, digest,
operator, program, RPC ID, attempt ID and execution generation fence late
responses. Reusing a retained attempt ID marks it ambiguous and clears its result.
Preflight and requests without this correlation do not populate the cache.
Retrieval does not require the caller to upload the model again.

The read-only `task_result_retention` descriptor exposes the policy: at most
64 entries, 8 MiB per encoded response, 32 MiB of retained encoded response bytes,
and 10 minutes from reservation creation. Metadata and transient JSON allocations
are additional, not part of that encoded-byte budget. Count pressure evicts the
oldest reservation, including pending entries; byte pressure evicts old retained
responses. Late completion cannot recreate an evicted reservation. Oversized
responses become `result_not_retained` without blocking or truncating the original
execution. Reads do not renew retention. Expiry is reclaimed lazily on the next
cache operation. No result files, backups or separate input copies are created; Agent
restart clears this entire cache.
The retained response itself can include operator-defined input echoes or other
sensitive output; it is not a redacted journal. Protect the existing Agent
transport boundary and configure read authorization for the control-plane API.

`receipt_recovered` contains `completion` only after the existing task-bound
completion/failure gates pass, checked again by the Rust SDK. Its `outcome` may
be `executed`, `failed` or `blocked`; HTTP success alone is not execution success.
This proves the original computation response, not successful transport delivery,
durable receiver acknowledgement, published research results or scientific
qualification. The original journal observation remains unchanged. `pending`,
`not_retained`, `result_not_retained`, `attempt_identity_ambiguous`, missing/replaced
targets and invalid receipts have `outcome: unknown` and no completion. Responses
always keep `automatic_replay_authorized: false` and `publication_performed: false`.
There is no pre-dispatch boot pin, cryptographic provenance, exactly-once guarantee
or Agent-restart durability. Older Agents do not gain recovery by inference. See
`schemas/operator-task-dispatch-result.schema.json` and
`schemas/agent-task-result-retention.schema.json`.

Execution batches separate `attempted_count`, `executed_count`/`ok_count`,
`blocked_count`, `error_count`, and `skipped_count`. The invariant is
`task_count = executed_count + blocked_count + error_count + skipped_count`.
Strict execution stops at the first noncomplete case and retains skipped case
ids. Independent execution may continue healthy cases without promoting the
blocked or failed ones. A validated failure declaring
`recovery.safe_to_continue_other_tasks: false` stops even independent execution
and retains skipped ids. Actual Agent results, failure recovery, and readiness
are retained.

Checkpoint archival requires unique, task-bound published results and consistent
completion metadata, not just `error_count == 0`. Package-resolution cases remain
recovery targets; readiness repair actions survive JSON checkpoint reload.
`resolve_incomplete_cases` covers mixed failed/blocked/skipped batches and keeps
valid completed cases out of the recovery target set. Inconsistent completion
metadata cannot archive and may require revalidating the whole batch. A recovery
plan does not itself execute or automatically retry any task.

See [Orchestra completion verification](../reports/orchestra-task-completion-20261005.md).
The API tests use controlled Agent receipts. The native `headless-live-test`
target additionally runs a real local Rust Agent through actual Orchestra HTTP:
a bar solve is compared with the independent FL/EA solution, invalid input is
repaired and recomputed, blockers/failures cannot create downstream projects,
and mixed-batch recovery targets and engine repair actions survive checkpoint
file reload. Modal 2D fixtures with 96/100 beam segments and a modal 3D fixture
with 100 segments retain full mode arrays before report compaction, compare
against independent roots, and check matrix residuals. Invalid 3D density fails
without consuming the Agent permanently; repaired input and a fresh digest
execute under the same task id. These are bounded unit-beam fixtures, not a
large-model performance benchmark or arbitrary-geometry qualification. The CLI's
nonzero blocked exit also preserves its requested report. This bounded local
chain does not replace external-package execution, authenticated or installed
deployment, Python/Elixir SDK parity, or scientific qualification.

SDK-local smoke coverage:

- [sdks/python/tests/test_smoke.py](../sdks/python/tests/test_smoke.py)
- [sdks/elixir/test/smoke_test.exs](../sdks/elixir/test/smoke_test.exs)
- [sdks/rust/tests/smoke.rs](../sdks/rust/tests/smoke.rs)

All three SDKs expose the same conceptual split:

- `ControlPlaneClient`
- `SolverRpcClient`
- `Session`
- `AgentClient`

## Rust Electrothermal Projection

The study-level Rust helpers have an explicit load-order contract:

1. `project_composite_dielectric_loss_to_heat` builds dielectric-only nodal
   loads, replacing seed loads. Its spatial RMS field comes from integrated
   electric energy, so opposing subcell fields do not erase dielectric heating.
2. `project_composite_solved_current_to_heat` or
   `project_composite_joule_heating_to_heat` adds selected conductor power to
   those loads. Finite cooling loads remain valid; evidence reports actual
   nodal increments rather than intended power alone.
3. Submit the resulting heat model to the solver, then use the existing
   thermal expansion projection or temperature-feedback helpers as needed.

The helpers return errors for nonfinite/negative powers, unrepresentable
increments, and missing contact/terminal heat mappings. They do not silently
drop interface losses or mutate the input seed on failure. Equal four-node
power lumping remains an approximation, not subcell source quadrature. See the
[bounded regression report](../reports/composite-heat-projection-20260920.md).

For temperature feedback, `composite_feedback_relative_change` preserves
`abs(current - previous) / previous` for finite non-negative values with a
positive previous value, even below machine epsilon. Zero-to-zero is `0`;
zero-to-positive is explicitly `1`. Invalid inputs or an unrepresentable ratio
return infinity and cannot satisfy convergence. This is not a dimensional
absolute-power tolerance.

`assess_composite_electrothermal_feedback` recomputes temperature residuals,
combined dielectric/Joule loss changes, and per-region conductivity changes
from the trace before accepting its convergence flag. An inconsistent trace is
an error; an empty trace is `missing`; a valid unconverged trace is `fail`.
The native study runner reports the failed iteration/stage and does not mutate
the input models. A failed iteration budget is not proof of convergence. See
the [feedback regression report](../reports/composite-feedback-convergence-20260923.md)
for an analytic fixed-point cross-check and the limits of this validation.

`project_composite_heat_to_thermal` and
`project_composite_temperature_dependent_expansion` share a same-mesh identity
contract: complete nonempty node sets, unique nonblank IDs, result indices that
match the retained heat input, finite temperatures and matching coordinates
within `1e-12 m`. Reordering arrays does not change node identity. Expansion
also checks unique element IDs and matching cyclic quad connectivity rather
than comparing raw array indices. This is not remeshing/interpolation.

Temperature differences and adjusted expansion coefficients must remain
representable. Zero coefficients remain valid; silent underflow of a nonzero
coefficient is rejected. The current thermal-plane solver requires non-negative
expansion coefficients, so unsupported negative reference/adjusted coefficients
now fail in the SDK rather than after dispatch. See the
[temperature-transfer regression report](../reports/composite-thermal-projection-20260923.md)
for real heat-to-structural solves and a bounded local mapping microbenchmark.

The `composite_heat_cross_validation*` and `composite_heat_mesh_convergence*`
assessments normalize temperature errors by the analytic rise above the fixed
35 C boundary, not the total Celsius temperature. True zero heat requires an
exact reference-temperature match; positive heat whose predicted rise cannot
be represented above that reference cannot pass as zero heat. Invalid material
values, negative generation and malformed/nonfinite mesh samples are `fail`;
an otherwise valid, incomplete refinement prefix is `missing`.

The fallible distributed/regional heat-refinement builders reject invalid
conductivities and unrepresentable source assembly. Regional power is checked
at each nodal increment, per layer and over the whole model; a small source
cannot silently disappear at a shared interface behind a larger source. The
legacy infallible interface-load fixture builder still returns model requests;
the solver validates those requests before execution. These are fixed-geometry,
ideal shared-node interfaces, not contact-resistance or cooling models. See the
[layered heat validation report](../reports/composite-heat-interface-validation-20260923.md).

For actual finite interface resistance, the existing planar heat operators now
accept `contact_interfaces` describing matching edges with independent nodes.
The headless payload/plan path is unchanged, and temperature handoff preserves
the jump between coincident nodes with distinct IDs. See the
[thermal contact operator contract](thermal-contact-operator.md) for SI units,
the complete input model, output fields, deployment requirements and limits.
This is separate from the ideal-interface three-layer analytic reference above.

## Design goals

- protocol-driven rather than implementation-driven
- GUI-independent: Workbench is one client, not the runtime owner
- simple JSON payloads for AI-generated requests
- usable in cloud, distributed, and direct headless LAN deployments
- small enough to embed into agent runtimes without dragging UI dependencies
- explicit auth and error surfaces so higher-level agent loops can branch safely
- no hidden dependency on Workbench component state, browser-local settings, or
  GUI-only lifecycle hooks for core backend calls

## First-cut capabilities

### Control plane

- `GET /api/health`
- `GET /api/v1/protocol`
- `GET /api/v1/protocol/agents`
- `GET /api/v1/workflows/catalog`
- `GET /api/v1/operators`
- `POST /api/v1/operator-tasks/prepare`
- `POST /api/v1/operator-tasks/execute`
- `POST /api/v1/operator-tasks/execute-batch`
- `GET /api/v1/jobs`
- `PATCH /api/v1/jobs/:job_id`
- `DELETE /api/v1/jobs/:job_id`
- `POST /api/v1/fem/*/jobs`
- `POST /api/v1/workflows/catalog/:workflow_id/jobs`
- `POST /api/v1/workflows/graph/jobs`
- `GET /api/v1/jobs/:job_id`
- `POST /api/v1/jobs/:job_id/cancel`
- `GET /api/v1/results`
- `GET /api/v1/results/:job_id`
- `GET /api/v1/results/:job_id/chunks/:kind`
- `PATCH /api/v1/results/:job_id`
- `DELETE /api/v1/results/:job_id`
- `GET /api/v1/export/database`
- `GET /api/v1/export/security-events`
- `GET /api/v1/export/security-events.csv`

### Solver RPC

- `ping`
- `describe_agent`
- `release_operator_package_job`
- `solve_bar_1d`
- `solve_truss_2d`
- `solve_truss_3d`
- `solve_solid_tetra_3d`
- `solve_plane_triangle_2d`
- `cancel_job`

### Summary cross-check gates

The Rust functions `material_validation_quality_gate` and
`material_validation_repair_hint` consume
`kyuubiki.summary_tolerance_validation/v1` reports. A pass requires an explicit
`validation_passed: true`, a positive `validation_checked_field_count`, a zero
`validation_failed_field_count`, and a valid `validation_missing_field_count`
consistent with `validation_fail_on_missing` (default: true). If failure details
are supplied, they must be an empty array for a pass. Missing or malformed
success metadata is not inferred as success.

Recognized reports that fail these checks produce a violating gate and a repair
hint, even if their raw pass flag is true. Oversized blocking counts saturate
rather than overflowing. The resulting reliability summary drives
`repair_validation` in the next-round plan instead of advancing the research.
A different report contract returns `None`, which is not evidence of a pass.

The producer requires at least one actual numeric comparison even when missing
fields are optional. See the
[book's summary cross-check contract](./book-ch05-workflow-and-operators.html#summary-validation)
for field selection, tolerance defaults, non-finite failures, and explicit
workflow recovery. Summary agreement alone does not certify physical accuracy.

### Large model and result artifacts

Large service submissions do not expand a complete FEM model into the Elixir
process or one solver RPC frame. The control plane streams the model into its
SHA-256 store and sends a `kyuubiki.model-artifact-ref/v1` reference to the
selected Rust Agent. The Agent verifies the declared byte length and digest
before decoding it.

When a solve was sourced from a model artifact, its result follows the same
bounded transport rule. The Agent serializes directly to a temporary file,
uploads `application/vnd.kyuubiki.result+json`, and returns a compact
`kyuubiki.solver-result-reference/v1`. Job storage and `result_fetch` retain
that reference instead of copying a potentially multi-gigabyte result into
RPC, SQL JSON, or the Headless run report. Consumers can inspect metadata or
download immutable content through:

- `POST /api/v1/model-artifacts`
- `GET /api/v1/model-artifacts/:artifact_id`
- `GET /api/v1/model-artifacts/:artifact_id/content` for an authenticated Agent
- `POST /api/v1/result-artifacts` for an authenticated Agent
- `GET /api/v1/result-artifacts/:artifact_id`
- `GET /api/v1/result-artifacts/:artifact_id/content`

The active limits and storage namespaces are published by `GET /api/health`.
`KYUUBIKI_MODEL_ARTIFACT_MAX_BYTES`, `KYUUBIKI_RESULT_ARTIFACT_MAX_BYTES`, and
`KYUUBIKI_ARTIFACT_TEMP_RETENTION_SECONDS` keep the disk policy explicit.
`KYUUBIKI_MODEL_ARTIFACT_MAX_BYTES` is one cross-process transport contract and
must resolve to the same value in Orchestra, the Headless caller, and the Rust
Agent. The fail-closed default is 512 MiB. Raising it therefore requires an
explicit capacity decision for all three processes rather than a server-only
override. Headless callers count JSON bytes without allocating a second
in-memory serialized copy and stream large models through a temporary file;
Agents likewise stream downloads to disk before digest verification and
decoding.

Installer exposes this contract as **Model artifact limit (bytes)** on every
remote Agent launch profile and persists it in the remote node registry as
`model_artifact_max_bytes`. Both orchestrated and offline Mesh launches export
the value to the Agent process. A large-artifact deployment is aligned only
when the same byte value is also configured for Orchestra and the Headless
caller; leaving the Installer field empty resolves to the explicit 512 MiB
default, while zero is rejected.

If the serialized model exceeds the active client limit, Headless fails before
upload with `kyuubiki.headless.model_artifact_limit_exceeded` at the
`artifact_upload` stage. The failure is non-retryable until all three process
limits are aligned or the model is reduced; automation never needs to parse
the byte-count prose to distinguish this contract failure.

Large heat and electrostatic plane models may omit node and element `id`
fields. The Agent assigns stable index-derived IDs (`n0`, `n1`, `e0`, `e1`,
...) after decoding either inline JSON or an immutable model artifact, while
preserving every non-empty caller-supplied ID. Other missing solver fields fail
closed as `invalid_solver_params` at the `agent_decode` stage, so automation
can repair the model instead of retrying an invalid request.

Headless SDKs must use the runtime control-plane endpoint for artifact-backed
models, not the local GUI frontend. Known local frontend URLs fail fast before
upload with `frontend_proxy_artifact_limit` at the `artifact_upload` stage;
small inline requests remain available to GUI development routes. This keeps
large transport out of Next.js request cloning and preserves frontend/runtime
separation.

Large solves also use two separate server-side timing contracts. Agent capacity
waits are governed by `KYUUBIKI_AGENT_QUEUE_TIMEOUT_MS`; artifact-backed execution
is governed by `KYUUBIKI_ARTIFACT_EXECUTION_TIMEOUT_MS`. Static endpoints pass
through the same capacity gate as registered endpoints, so concurrent 1M jobs
queue instead of opening unbounded solver connections. `job_wait.timeout_ms` is
only the SDK polling budget and never silently overrides either server budget.
With the explicit `resume_policy: "server_deadline"`, it becomes one observation
window: the SDK keeps polling the same `job_id` while the server timing contract
is active, without resubmitting work. `max_total_timeout_ms` remains a mandatory
client-side ceiling for that policy; `direct_mesh_pipeline` uses 60-second
windows and a one-hour total ceiling. Successful waits expose `wait.policy`,
`poll_attempts`, `resume_count`, and `elapsed_ms` for automation and benchmarks.
Polling uses `/api/v1/jobs/:job_id/status`, which never embeds solver results;
`result_fetch` retrieves the result once and does not retain a duplicate `raw`
mirror. Full values remain available for downstream step bindings, while run
reports summarize oversized arrays. This prevents report size from scaling with
repeated copies of a solver result.
In the native Rust service executor, the wait deadline also bounds DNS,
connection retry backoff, writes, and reads; receiving a few more bytes does
not restart the budget. Fixed waits never issue another poll after their
window expires or accept a completion that arrives after that deadline.
For `server_deadline`, a previously observed, still-active server timing grant
can authorize an in-flight read across a soft window, but never beyond
`max_total_timeout_ms`. That grant ages locally while the client waits; stale
timing cannot indefinitely renew observation. No timing means no extension.
System DNS lookups are not cancellable, so at most four may remain outstanding
per process; late DNS answers are discarded and cannot send requests. Job
status responses are limited to 8,000,000 bytes including HTTP headers.

A wait timeout ends observation, not server-side execution. Retain the
accepted `job_id`, inspect `job_fetch`, and resume `job_wait` on that ID if
appropriate. Do not replay the submit step or the whole
`solve_and_wait_from_model_version` action to recover a polling timeout.
Malformed timing values, zero budgets, and conflicting timing aliases fail
validation rather than silently falling back to defaults. The composite
solve-and-wait action validates these options before loading or submitting a
model; both snake_case and camelCase wait keys retain the same behavior.
These transport guarantees currently describe the native Rust service
executor used by the reference runner, not a new cross-language parity claim
for the independent clients in `sdks/`.

Inspect `job.status_detail.timing` for `effective_timeout_ms`,
`job_submission_deadline`, `execution_started_at`, and `effective_deadline`.
The timing object also exposes `queue_wait_ms`, `execution_elapsed_ms`, and
`total_elapsed_ms`; SDK callers must not recover these values from log text.

Every Headless run report exposes `execution_summary`. It folds repeated
submit, wait, and fetch observations into one latest timeline per `job_id`.
Failed execution steps emit a `kyuubiki.headless-failure-receipt/v1` record
with a stable error code, failure stage, retryability, retry strategy, and
recommended recovery action. SDKs should branch on those fields instead of
matching human-readable error messages.

Development source launches use debug Agents by default. Qualification runs
must set `KYUUBIKI_AGENT_BUILD_PROFILE=release`; installed runtime payloads are
already release binaries. This distinction is material at million-node scale
and must be recorded with benchmark evidence.

## Intended AI use

For AI agents, the recommended flow is:

1. Query the control-plane protocol descriptor.
2. Inspect reachable agents or direct endpoints.
3. Generate a JSON payload for the desired FEM study.
4. Submit through the control plane or directly over solver RPC.
5. Poll and stream progress until completion.

The SDKs are deliberately thin wrappers over public contracts so higher-level AI
planning layers can stay language-agnostic.

They now also expose a small workflow layer:

- submit one job by solve kind
- submit many jobs in sequence
- plan headless workflow execution before submission, including runtime style,
  engine mix, step bindings, executor compatibility, and required
  sensitive/destructive confirmations
- generate Rust-driven material screening workflows, starting with a thermal
  heat-spreader candidate comparison for Aluminum 6061, Copper C110, and
  in-plane pyrolytic graphite
- generate structural panel material workflows over aluminum, steel, and carbon
  fiber candidates without opening the Workbench
- submit the built-in material envelope workflow through the Orchestra catalog
  with the `material_study_envelope_catalog` template
- keep an offline material envelope graph path available through
  `material_study_envelope_ranking` when a client cannot rely on the catalog
- build material research reports from headless result payloads, with explicit
  metric specs, weighted ranking, and visible missing-metric warnings
- expose optimization profiles as first-class report contracts, including
  score formulas, constraints, normalized metric scores, and weighted
  candidate contributions
- validate workflow graphs and workflow dataset contracts before submission,
  including duplicate dataset values, unresolved graph references, port/edge
  mismatch, unsupported data classes, empty schema refs, and semantic/artifact
  drift
- wait for terminal job states by polling the control plane
- optionally bypass the control plane and solve directly over solver RPC
- run one study and fetch its result bundle in one call
- browse large result windows in chunked pages
- retry transient failures without retrying auth or logic errors
- classify failures into machine-usable buckets for agent policy layers
- execute language-neutral Operator TaskIR envelopes and
  `quality_execution_batch` files without using the Workbench
- validate Operator TaskIR against an agent execution capability before
  dispatch, including digest, runtime protocol, ABI, operator ID, and
  package-fetch constraints

Agent-native solver TaskIR currently admits `solve.bar_1d`,
`solve.modal_frame_2d`, and `solve.modal_frame_3d`. Read the Agent descriptor's
`headless_bridge.headless_entrypoints[0].solver_execution_capability` rather
than assuming every direct solver RPC is also TaskIR-enabled. Built-in
entrypoint names must match the operator ID, and central-fetch tasks cannot
bypass the attached capability's authority/package restrictions. The modal
routes retain the same Engine numerical gates and typed failure receipts;
[local TCP evidence](../reports/modal-agent-taskir-reliability-20261002.md)
does not qualify installed/remote execution or all Python/Elixir SDK paths.

Rust material reports can be generated headlessly:

```bash
kyuubiki-material-report heat-spreader --results results.json --out report.json --json
kyuubiki-material-report thermo-shield --results thermo-results.json --out thermo-report.json --json
kyuubiki-material-report thermo-shield --results thermo-results.json --profile profile.json --json
kyuubiki-material-report structural-panel --results structural-results.json --json
kyuubiki-material-report structural-panel --results headless-run-report.json --json
```

Material envelope automation now has two explicit SDK paths:

- `material_study_envelope_catalog`
  submits `workflow.material-study-envelope-ranking-json` through
  `workflow_submit_catalog`, then waits and fetches the result. This is the
  preferred path for normal Orchestra-connected deployments because the graph
  remains owned by the central workflow catalog. Rust, Python, and Elixir SDKs
  expose request helpers for this catalog-first path.
- `material_study_envelope_ranking`
  submits an inline workflow graph through `workflow_submit_graph`. This is the
  fallback path for offline or decentralized runs where the catalog is not
  reachable.
