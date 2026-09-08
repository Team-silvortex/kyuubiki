# Agent And Orchestrator Boundary

This document freezes the runtime-side boundary between:

- `solver agent`
- `orchestrator / control plane`
- `frontend surfaces`
- `headless SDK clients`

It exists to stop architecture drift, especially the common confusion between:

- "the agent is pure Rust"
- "the whole system is pure Rust"

Those are not the same claim.

## Hard Statement

In `daji 3.x`, the intended boundary is:

- `solver agent = Rust compute peer`
- `orchestrator = Elixir control plane`
- `frontend = UI shell and workflow surface`
- `SDKs = protocol clients, not hidden runtimes`

The Rust agent is the execution peer.
The Elixir orchestrator is the management and coordination peer.
Neither one should quietly absorb the other.

## Agent-Embedded Engine Rule

Every started agent owns one local engine instance.

That engine is part of the agent runtime, not a task object and not an
orchestrator-owned process. The distinction is:

- `agent`
  long-lived compute peer and RPC surface
- `engine`
  agent-embedded execution instance used to run assigned operator work
- `task`
  scheduled unit of work submitted manually, by SDK, by direct mesh, or by the
  bound orchestra
- `operator package`
  execution payload fetched from the bound orchestra library when required

The engine should be visible through runtime descriptors so operators can
inspect what is capable of executing a task. It should not make the agent a
second control plane.

## What The Agent Is

`agent` means the solver-side execution runtime.

It should own:

- FEM solve execution
- solver RPC serving
- job-local progress emission
- heartbeat and self-description
- peer-mesh participation
- one embedded engine instance per agent process
- operator execution on the compute side
- temporary operator package cache materialized for assigned work
- compute-local benchmarking support

It should be implementable and runnable without:

- React
- Next.js
- Phoenix
- Hub UI
- Installer UI

This is why the repository describes the agent/data plane as Rust.

## What The Agent Is Not

The agent is not:

- the desktop entrypoint
- the project browser
- the workflow editor
- the install/update surface
- the persistent control-plane authority
- the user-facing source of truth for workload history

If an execution peer needs one of those concerns, it must receive it through a
protocol contract rather than by inheriting product-layer logic.

## Result Delivery And Draining

An execution remains in the Agent lifecycle until its final response has been
written to the transport, or delivery has failed explicitly. Computing a result
is not enough to advertise `safe_to_replace`. The existing host-loopback drain
owner and generation contract remains unchanged: drain rejects new executions,
preserves in-flight work, and leaves control/inspection requests available.
Both solver RPCs and TaskIR responses retain the same execution lease through
delivery. Failed writes record `result_delivery_failed`; abandoned writers
record `result_delivery_aborted`, including the original request/job identity.
An existing solver failure is not overwritten by a later connection error.

`KYUUBIKI_AGENT_REPLY_TIMEOUT_MS` configures a total response-write deadline,
including serialization, progress frames and the final frame. The default is
10,000 ms; accepted values are integers from 1 through 300,000. Invalid values
stop Agent startup rather than disabling the bound. A failed frame write closes
the connection immediately, including a heartbeat failure, so neither a later
final response nor a following request can continue a truncated frame.
Configure this budget for expected result sizes/network throughput, and
prefer result artifacts for large data rather than an unbounded inline reply.

`describe_agent` and registration/heartbeat payloads expose the read-only
`reply_delivery` policy (`kyuubiki.agent-reply-delivery/v1`), including the
effective timeout and `completion_boundary`. Successful socket writes are not
durable receiver acknowledgements; `durable_receiver_acknowledgement` is false.
Orchestra result persistence and SDK readback still require separate gates.
The heartbeat thread is woken on completion/unwinding instead of adding a
one-second sleep to each short job. A blocked heartbeat write is also bounded.

## Native Execution Capacity And Orphaned Work

`KYUUBIKI_AGENT_MAX_ACTIVE_EXECUTIONS` is the Agent-local admission limit, default
1, accepted integers 1..=1024. Startup rejects invalid values; changing the
limit requires a process restart. Both solver RPC and TaskIR use the same
locked lifecycle count. The slot remains reserved until the last execution /
response lease is dropped, including failure and cancellation. Ping, inspection
and lifecycle controls do not consume execution slots. This is not a limit on
TCP connections, request bytes, or every background thread.

A full Agent rejects new work with `agent_at_capacity` before execution, rather
than relying exclusively on Orchestra's in-memory scheduling ledger. Descriptor
and registration/heartbeat payloads expose read-only `execution_admission`
(`kyuubiki.agent-execution-admission/v1`), including the effective limit, active
count and admission/release boundaries. Configure Orchestra's declared endpoint
capacity no higher than this native limit. The control plane does not yet adopt
the native limit automatically; a lower declared limit can underutilize an Agent.

Orchestra treats this specific rejection as non-admission, not a failed accepted
computation. It tries another eligible endpoint or waits in bounded 50 ms retry
intervals, without marking a full Agent unhealthy. One admission-wait deadline
starts at the first native capacity rejection and uses the existing queue timeout;
expiry returns `agent_capacity_timeout`. Existing transport deadlines still bound
individual RPCs. Every dispatch rechecks current authorization. A capacity retry
does not waive checkpoint-required policy for uncertain transport failure or
accepted execution, and does not reset the workflow execution deadline.

A failed heartbeat write marks cancellation on that specific execution guard,
not on a reusable request/job id. Cooperative boundaries check it before TaskIR
decoding or solver computation, and after solver parameter decoding. The
test-only hold also responds to transport loss and explicit cancellation without
requiring marker removal. An old guard cannot cancel a new execution that reuses
its request id, or a distinct live execution of the same job.

This does not preempt a solver already inside an uncooperative numerical call.
Such a call keeps its capacity slot until it returns; it must not advertise
spare capacity just because the control plane considers it cancelled. Heartbeat
write failure is not a proof of immediate network-blackhole detection. Native
admission protects one Agent across an Orchestra restart, but does not rebuild
the control plane's old execution ledger or guarantee exactly-once side effects.
This transport-loss observation covers job-bound executions that emit heartbeats;
it is not an independent disconnect monitor for every RPC.
The [installed orphan-execution study](../reports/orphan-execution-research-20260908.md)
separates these boundaries from the cases actually exercised.

## Cooperative Numerical Cancellation

`kyuubiki_solver::solver_control` provides a Rust-only, synchronous execution
scope: `SolverControl`, `with_solver_control` and `with_solver_observer`.
It has no GUI or Orchestra dependency and does not change physical input formats.
The Agent installs a fresh control token around each solver RPC and TaskIR call.
A token is sticky and must not be reset/reused for another execution. Scope exit,
including panic unwind, restores the calling thread's previous control context.
Nested scopes cannot mask a parent's cancellation. Child threads need their own
explicit scope; this is not an ambient async-task context or a thread-kill API.

Safe points currently cover common PCG iterations, dense LU and banded Cholesky
factorization/substitution, and prepared tridiagonal factorization/substitution.
Dense/banded paths check completed rows or pivots; the tridiagonal path batches
checks at 256-row boundaries and completion. Shared sparse regularization and
dense fallback paths check cancellation before retrying, so interruption is not
treated as a reason to regularize and recompute. Reusable prepared factors remain
unchanged by a cancelled substitution. An observed cancellation discards a
partial success and returns an error rather than exporting a partial field.

The planar triangle/quad heat and thermal-stress paths also poll during element
precomputation and assembly. Shared sparse compression, inverse-diagonal setup,
and IC(0) factor/transpose construction are fallible preparation stages. A
cancelled preparation never publishes a usable compressed matrix or prepared
solver. These stages check at entry, each 64 completed elements/rows, and the
final short batch; existing arithmetic and convergence criteria are unchanged.
The diagnostic stage names are `element_precompute`, `element_assembly`,
`sparse_compress`, `preconditioner_setup`, `ic0_factor` and `ic0_transpose`.
IC(0) transpose counts completed rows cumulatively through its count, prefix-sum
and fill passes. These counts are not percentages or a wall-clock latency bound.

Shared homogeneous/prescribed sparse constraint elimination is also fallible:
`constraint_index`, `constraint_map` and `constraint_reduce` poll at entry,
64-step boundaries and completion. They count constraint entries, original DOFs
and free rows respectively. All callers propagate errors rather than receiving a
partly reduced system. This does not cover every operator-specific constraint
algorithm or the memory allocation inside a step.

Built-in preconditioner application polls at `preconditioner_jacobi`,
`sgs_forward`, `sgs_backward`, `ic0_forward` and `ic0_backward`. Each sweep
counts completed rows, including reverse sweeps, and starts its count anew.
On error the output/workspace buffers can contain partial scratch values: callers
must discard them, not treat them as a valid vector. Prepared factors remain
immutable; a subsequent complete application overwrites/reuses the scratch
buffers. PCG propagates cancellation from both initial and later applications.
The shared symmetric tangent fallback checks cancellation before dense expansion,
and cohesive Newton/co-rotational failure paths distinguish cancellation from
ordinary nonconvergence instead of exporting degraded output or trying a fallback.

Compressed sparse matrix products and shared residual/refinement validation are
also fallible. `sparse_matvec` and `sparse_residual` count completed rows, polling
at entry, each 64 rows and the final short batch. `residual_validate` counts rows
cumulatively through equation-scale collection and worst-row selection. Cancellation
during final validation is an error, not an accepted solution with an incomplete
residual check. PCG, prepared, modal and buckling callers propagate these errors.

Rows with more than 1,024 entries additionally poll at entry, every 1,024 entries
and completion, using `sparse_matvec_row`, `sparse_residual_row` or
`residual_validate_row`. These counters reset for each wide row and identify
neither the row number nor overall progress. Short rows retain a direct loop.
Chunks retain the original accumulator and entry order; they are not parallel
or regrouped reductions. A cancelled product may have written earlier complete
rows to scratch; the vector is not a valid result and must be discarded/recomputed.

PCG's internal vector passes are fallible as well: `pcg_rhs_scale`,
`pcg_rhs_normalize`, `pcg_direction_copy`, `pcg_dot`, `pcg_norm`,
`pcg_vector_update`, `pcg_residual_update`, `pcg_direction_update` and
`pcg_solution_scale`. Each polls at entry, every 1,024 completed elements and
the final short block, retaining the original sequential arithmetic. Counters
reset per invocation; a norm counts visited elements even when their value is
zero. No partial dot/norm, normalized RHS or final scaled solution is returned
on cancellation. Mutable scratch may contain a prefix and must be discarded.
PCG and prepared-solver callers propagate cancellation instead of entering a
dense/regularized retry. Separate sparse-system scaling is covered below;
allocations and arbitrary vector operations in other algorithms remain distinct.

Shared sparse-system finite validation polls at `sparse_validate_rhs` every
1,024 RHS elements and `sparse_validate_matrix` every 64 rows, including empty
rows. Both poll at entry and completion. Rows longer than 1,024 entries also
poll at `sparse_validate_matrix_row` every 1,024 entries plus entry/completion.
RHS errors still take precedence over matrix errors; finite checks do not
identify an offending index. A boolean reduction may inspect a whole bounded
block before rejecting it, without regrouping any numerical sum.

`sparse_diagonal_scale` and `sparse_diagonal_magnitude` poll every 64 rows.
Diagonal scaling reuses SparseMatrix's existing binary lookup on its sorted,
unique columns; missing/zero diagonal behavior and arithmetic are unchanged.
`sparse_rhs_scale` and `sparse_solution_unscale` poll every 1,024 vector elements.
All include entry and the final short block. Original accumulation/multiplication
order is retained, and final unscaling must succeed before result acceptance.

Fallback matrix scaling and regularization are also fallible. Their capacity
hint scans poll at `sparse_capacity_scan` every 1,024 rows; `sparse_matrix_scale`,
`sparse_regularize_copy` and `sparse_regularize_diagonal` count 64-row blocks.
The copy/scale paths additionally poll within wide rows using
`sparse_matrix_scale_row` and `sparse_regularize_copy_row`, at 1,024-entry
boundaries. Row buffers are allocated along the checked traversal, not all in an
uninterrupted preallocation loop. Neither ordinary nor prepared solvers publish
a partial matrix or continue regularized retries after cancellation. Counters
reset per pass/row and remain diagnostic, not saved restart state.

An explicit job cancellation marks every matching execution currently registered
in the control registry, not a single globally consumed flag. Cancellation with
no matching registered execution retains the existing one-shot pre-admission
behavior. A transport failure marks only its guard; watchdog notifications match
both request id and execution generation. A late notification cannot target a
new execution that happens to reuse an id. This is not a persistent job tombstone.

Descriptor and registration payloads expose read-only `solver_control`
(`kyuubiki.agent-solver-control/v1`): active request/job identities, generation,
cancellation state and last safe point. Failure responses retain
`solver_checkpoint` and the original watchdog reason, when one already exists.
The checkpoint's `stage` and `completed_steps` are diagnostics for the current
kernel invocation, not whole-job progress, convergence, or a resumable state;
`resumable` is false. The observation resets when another kernel starts.
The scope label is `same_thread_builtin_numerical_safe_points`; the stages above
define its actual coverage, not arbitrary work invoked within that scope.

For isolated fault qualification only,
`KYUUBIKI_AGENT_FAULT_INJECTION_SOLVER_STAGE` selects `sparse_iteration`,
`dense_factor`, `tridiagonal_factor`, or one of the preparation, sweep, product,
residual, PCG vector or sparse scaling/validation stages above.
It requires the existing hold-file and
explicit execution-method settings. A matching job is held once, after at least
three completed steps at that stage, for no more than 120 seconds. Numerical
stage mode replaces the pre-computation hold. Descriptor metadata exposes phase,
stage and minimum step count, never the host marker path. Leave all three fault
controls unset in normal use. A slow observer delays its synchronous caller.

This scope does not interrupt arbitrary input parsing/validation, individual
allocations, custom constraint algorithms, physical-result postprocessing,
other preconditioner implementations,
or arbitrary matrix/vector operations. The row-interior guarantee above applies
only to the listed compressed products, sparse residual/finite checks and
scaling/copy paths, not dense factorization interiors or preconditioner rows.
Regularization's single Vec insert/remove may still shift an entire row without
an internal poll. Other capacity scans, structural-input checks and sparse-path
detection are not covered by the scaling capacity stage. PCG vector passes use their
separate element-block contract above, not the 64-row product contract.
Element-level coverage is limited to the four planar
heat/thermal paths above, not every operator's assembly or preconditioner type.
It does not add safe points to every analytical shortcut, nonlinear outer loop,
external worker, dynamic library or child thread. TaskIR's scope alone does not
make such implementations cooperative; the analytical `solve.bar_1d` path, for
example, has entry/exit control but no numerical kernel safe points here.
Uncooperative work still owns its capacity until it actually returns. There is
no universal cancellation-latency or numerical-checkpoint recovery guarantee.
See the [installed numerical cancellation study](../reports/cooperative-solver-research-20260908.md)
and [preparation-stage follow-up](../reports/preparation-cancellation-research-20260908.md)
plus [constraint/preconditioner sweeps](../reports/constraint-preconditioner-research-20260908.md)
and [matrix-product/residual validation](../reports/matvec-residual-research-20260908.md)
for real in-progress heat/structure faults and bounded acceptance evidence.
The [PCG vector follow-up](../reports/pcg-vector-research-20260908.md) separates
installed vector-stage faults from unchanged mixed-workflow numerical regression.
The [sparse scaling/validation study](../reports/sparse-scaling-research-20260908.md)
adds installed pre-solve and post-solve faults, with fallback/long-row coverage
kept explicitly at the native numerical-test layer.

## Termination Signals And Shutdown Budget

The native Agent now routes handled termination signals into the same admission
gate. Unix handles SIGINT, SIGTERM and SIGHUP through the
[ctrlc signal adapter](https://docs.rs/ctrlc/3.5.2/ctrlc/). Its callback runs on a
dedicated thread, not in an asynchronous signal context. The Windows adapter
uses platform Ctrl-C events; this is not Windows Service Control Manager or
Task Manager forced-termination support. Linux live tests exercise the signal
path; macOS compile checks alone do not qualify live delivery on every platform.

Shutdown irreversibly closes admission for that process and preserves an
existing Installer drain owner/generation. New execution returns
`agent_draining`; attempts to resume or reassign draining return
`agent_shutdown_in_progress`. Ping and inspection stay available while admitted
work and response delivery drain. Repeated signals neither reset the deadline
nor force an early successful exit. Normal RPC acceptance remains blocking and
event-driven, with no added polling delay on each request.

`KYUUBIKI_AGENT_SHUTDOWN_TIMEOUT_MS` is a total budget for execution draining and
background cleanup, default 30,000 ms, accepted integers 1..=300000. Invalid
values reject startup. Descriptor and registration/heartbeat payloads expose
the read-only `shutdown_policy` (`kyuubiki.agent-shutdown-policy/v1`). Graceful
completion exits 0. A deadline failure exits 1 and emits a structured
`kyuubiki.agent-shutdown/v1` event with `agent_shutdown_timeout`, the phase,
remaining execution identities and recent execution failures through existing
stderr logging. Blocking background cleanup cannot restart the budget. Events
are diagnostic evidence, not a signed or durably acknowledged result receipt.

The process supervisor must allow more time than this budget; for example, a
30-second Agent budget needs a longer Docker stop/systemd timeout, not Docker's
shorter default stop window. Continue using container init for process reaping.
Planned replacement should still use the Installer lifecycle sequence.

This protects admitted Agent executions, not every future node of a multi-node
research workflow. SIGKILL, power loss and forced supervisor kills bypass the
handler. A timed-out task is not silently declared successful or automatically
resubmitted by the Agent; Orchestra replay/checkpoint policy remains separate.
See the [signal and installed research evidence](../reports/agent-shutdown-research-20260908.md)
for the exact scope, including explicit process restart rather than implicit
replay of the timed-out computation.

## Workflow Activity And Replay

Workflow solve-node `retry_safety` and `replay_checkpoint` must reach Agent
transport recovery unchanged. `replay_safety` is an alias only when the canonical
key is absent. Only an absent policy may inherit the pure-solver default;
invalid explicit values or unverified `checkpointed` assertions require a
checkpoint. Connection failures before dispatch are different from uncertain
send/receive failures. Graph-level recovery policy controls whole-workflow
restart; it must not be confused with a node's transport-retry policy.

Agent activity now reaches the workflow coordinator. A valid current owner,
lease and generation can refresh job liveness at most once per second, without
rewriting artifacts, advancing completed graph nodes or resetting the original
execution deadline. Throttled activity still checks the claim. Cancellation,
stale-heartbeat detection and total execution timeout remain independent guards.

Idempotent restart can replay the whole graph, including completed upstream
nodes. Durable job progress and iteration retain their high-water marks;
progress events separately expose `generation`, `attempt` and raw
`execution_progress`. This avoids rejecting valid replay as progress regression
without disabling monotonic Job validation. It is not reuse of a solver checkpoint.

The explicit test-only `KYUUBIKI_AGENT_FAULT_INJECTION_HOLD_METHOD` narrows an
existing job-scoped hold to a supported execution RPC. It requires
`KYUUBIKI_AGENT_FAULT_INJECTION_HOLD_FILE`, whose path is supplied at deployment;
only exact job-id marker content activates the hold, for at most 120 seconds.
Invalid/non-execution methods reject startup. `describe_agent.fault_injection`
exposes the method scope but not the host file path. Leave both controls unset
outside reviewed isolated fault tests; this is not a scheduling or pause API.

See the [mixed research qualification](../reports/interrupted-thermal-research-20260908.md)
for actual SIGTERM, Agent SIGKILL, Orchestra restart and checkpoint-required
negative cases. Its earlier candidate allowed old/new computations to overlap
despite fenced result ownership. The subsequent
[orphan-execution qualification](../reports/orphan-execution-research-20260908.md)
adds native admission and cooperative cancellation: in the tested single-Agent
restart, the old held execution exits without marker release before a new one
is admitted. Capacity accounting is still distinct from result-commit fencing
and complete control-plane ledger reconciliation.

## What The Orchestrator Is

`orchestrator` means the control-plane runtime family.

It should own:

- job submission and cancellation
- workflow graph intake
- persistence
- result windows and chunking
- agent registry and routing
- cluster-aware coordination
- control-plane security policy
- workflow/operator catalog delivery
- authoritative operator package resolution for its bound agents

It may talk to many agents.
It may be local or remote.
It is not the same thing as the Hub.

## Task And Operator Fetch Boundary

Tasks and execution engines are deliberately separate.

- a task may be assigned manually, by a headless SDK, through direct mesh, or by
  an orchestra scheduler
- an agent executes the task with its embedded engine
- in `orch_managed` mode, the agent fetches required operator packages from the
  operator library owned by its bound orchestra
- in `offline_mesh` mode, the task source may be manual or mesh-driven, but the
  agent still must not pretend to own a full authoritative operator library
- fetched packages may be cached only as visible, cleanable execution cache

The concrete package boundary uses
`kyuubiki.operator-package-distribution/v1` at publish time and
`kyuubiki.operator-package-resolution/v1` for one bound-Agent target. Installer
can consume that resolution explicitly. An orchestrated Agent also consumes it
on demand after TaskIR digest, identity, authority, and admission checks. Fetch
uses only `orchestrator_url`, `cluster_api_token`, and the configured managed
`operator_packages_root`; there is no source fallback. The newly built dynamic
host replaces the previous host only after activation succeeds, so a failed
download or package activation does not clear a working host. Same-package version
changes are built as isolated, owner-marked generations. Host and binding switch
together; old generations remain available to in-flight tasks and are removed only
after their final host lease is released. This avoids deleting a still-loaded DLL
on Windows while preserving uninterrupted work on every platform.

Cache entries remain execution state rather than an Agent-owned library.
Each Agent process owns a file-locked cache session. The next startup reclaims an
unlocked session left by an abnormal exit, preserves a locked peer session, and
retains malformed ownership metadata fail-closed. Removed, active, and invalid
session counts are carried in the package execution receipt. Task-scope immediate
eviction is now explicit for `cache_scope: none`: the Agent switches to a
package-free generation after dispatch and records a portable eviction receipt.
For `cache_scope: job`, Orchestra supplies `job_id` on execution and calls
`release_operator_package_job` only after that job's task RPCs settle. The Agent
keeps shared package owners, evicts packages after the last job owner exits, and
uses the same cleanup path for cancellation. The full Windows/Linux/macOS
installed dynamic-host qualification matrix remains lifecycle work.
Transient fetch and cache-availability failures are retryable in the structured
failure receipt and remain isolated from unrelated tasks; identity and
activation failures stay fail-closed and require repair.

This means a workflow run can move between scheduling modes without changing the
core engine model: the scheduling authority changes, but the agent-local engine
remains the execution boundary.

Within one Orchestra authority, candidate filtering remains capability-,
placement-, package-runtime-, and authority-aware. Capacity admission then uses
the deterministic `least_utilized_capacity_v1` policy: it compares active slots
to declared capacity within one explicit routing tier, chooses the lowest
normalized utilization, and preserves candidate order when utilization ties.
An idle compatibility fallback therefore cannot overtake a capability or
authority match that still has capacity. Every grant exposes the selected Agent,
pre/post slot counts, capacity, utilization, queue wait, and policy; the watchdog
exposes the same per-Agent utilization and saturation view. The retained remote
Linux qualification installs one sealed Release Agent package into two isolated
Installer stores, starts capacities 3 and 1, and requires the exact normalized
lease sequence high, low, high, high. Both Agents execute the closed-form solve.
After the high-capacity process is stopped, Orchestra falls back to the low
Agent, keeps the failed endpoint in cooldown, observes a new process identity,
and schedules high capacity again after a successful health probe. The report is
retained at
`releases/usability-evidence/2.19.0/fleet-scheduling-operational-qualification.json`.
This proves installed fleet scheduling and rejoin on one remote Linux physical
host; multi-host package acquisition and installed macOS/Windows operation remain
separate qualifications.

## Operator Description Vs Execution Program

Operator descriptions may be authored, indexed, or served by the Elixir control
plane. That does not make Elixir part of the compute-side execution ABI.

The boundary is:

- `operator descriptor`
  catalog metadata used for search, UI grouping, validation, package fetch, and
  workflow graph assembly
- `operator task IR`
  the orchestration envelope that binds one operator, one input artifact, config,
  dataset context, routing hints, and integrity metadata
- `operator execution program`
  the language-neutral program contract inside the task IR that an agent engine
  can execute

The execution program is the part analogous to LSP in the VS Code ecosystem:
the editor extension can be written in TypeScript, but the language server
interaction is protocol-shaped. In Kyuubiki, the control plane can be Elixir,
but the agent-facing execution structure is:

- schema: `kyuubiki.operator-execution-program/v1`
- runtime protocol: `kyuubiki.operator-execution/v1` or `kyuubiki.solver-rpc/v1`
- package reference: `orchestra://operator-package/<operator-id>`
- ABI: JSON input/config/output bindings
- entrypoint: protocol-visible operator id or solver method

Agent engines should treat this as the execution contract. They should not
depend on Phoenix routes, Elixir modules, or control-plane private function
names to run operator work.

Agent-native builtins are allowed only when they still enter through the same
TaskIR and execution-program envelope. For example, the Rust agent may execute
library-managed material transforms such as
`transform.evaluate_material_margins`, `transform.rank_material_candidates`,
`transform.score_material_candidates`, and
`transform.evaluate_material_thermal_shock` directly after digest verification.
That is not a bypass around TaskIR; it is a compute-side dispatch implementation
for operators whose package reference is already represented as library-managed
or agent-native. External operator packages must still go through package
resolution, integrity verification, activation, dispatch, and result
serialization stages.

## Dual-Mode Task Description

Task descriptions are allowed to be authored through more than one runtime.

The preferred product path is:

- Elixir control-plane descriptor authoring
- fast catalog iteration
- hot-reload-friendly pure-function transforms
- workflow graph assembly and validation close to the orchestrator

But this is not exclusive. Rust-native operator SDKs and external SDK clients
may also author task descriptors directly, as long as they emit the same
language-neutral task IR and execution program.

Directly authored descriptors must still carry the minimum executable identity:
`id`, `family`, `kind`, and an `execution.package_ref` bound to the same
operator id. Bypassing catalog lookup must not mean bypassing package identity.
When TaskIR repeats identity in `execution_program` or `runtime_hints`, those
fields are mirrors, not independent override points: kind, package ref, and
package version must match the descriptor and execution program.

Task IR integrity includes both a descriptor digest and a task digest. The
descriptor digest covers the operator snapshot; the task digest covers the
actual execution envelope fields, including descriptor authoring, input,
config, dataset context, runtime hints, and execution program. Agents and
orchestrators can use this to audit whether a task changed after construction.

Task IR therefore carries `descriptor_authoring` metadata:

- `mode`
  examples: `elixir_control_plane`, `rust_native`, `external_sdk`
- `runtime`
  examples: `elixir`, `rust`, `python`, `elixir_sdk`
- `source`
  examples: `workflow_operator_catalog`, `rust_operator_sdk`
- `hot_reloadable`
  a description-layer property, not an agent execution requirement
- `execution_language = language_neutral`
  the important invariant for agents

This keeps Elixir as the rapid authoring and orchestration layer without making
Elixir the only way to describe valid work.

## What The Orchestrator Is Not

The orchestrator is not:

- the agent
- the Hub
- the browser workbench
- the installer
- the only deployment mode

It is one runtime target family that product surfaces can manage.

## Product-Side Boundary

Use this section only for the runtime-facing consequences of product shells.
The primary owner of Hub / Workbench / Installer role separation is
[app-runtime-boundaries.md](app-runtime-boundaries.md).

Frontend and desktop surfaces may:

- inspect agent health
- select runtime targets
- submit jobs
- inspect workflow results
- observe topology
- choose orchestrated or direct-mesh execution modes

Frontend and desktop surfaces must not:

- define agent-internal file layout
- depend on private runtime module names
- assume process-tree structure
- encode solver implementation details as UI behavior
- become the real authority for runtime architecture

If a UI needs runtime behavior, that behavior must be exposed through:

- HTTP APIs
- solver RPC
- manifests
- schemas
- dataset contracts
- operator descriptors

For the headless transport and gateway-vs-runtime rulebook, use
[headless-agent-contract.md](headless-agent-contract.md).
For one-orchestrator-versus-offline-mesh binding rules, use
[agent-control-authority.md](agent-control-authority.md).

## Allowed Deployment Shapes

This boundary supports several valid shapes:

- `local workstation`
  frontend + orchestrator + local Rust agents
- `cloud control plane`
  frontend + orchestrator + database + remote agents
- `distributed control plane`
  frontend/orchestrator separated from remote Rust agents
- `headless peer mesh`
  Rust agents without Phoenix on the hot path

The existence of several shapes is exactly why the agent must not inherit UI
or orchestrator internals.

Use [system-overview.md](system-overview.md) for the broader system map and
[operations.md](operations.md) for operator procedures inside those shapes.

## Transitional Reality In This Repository

Some parts of the current repository are still transitional.

These are acceptable only as implementation bridges, not as permanent
architectural truths:

- Elixir-side bridge helpers that invoke Rust worker or CLI processes
- browser-side direct-mesh helper routes that still mediate agent access
- runtime management flows that are surfaced through Hub or Installer shells
- mock or compatibility adapters used to keep local iteration moving

Those pieces do not redefine the target boundary.

The target boundary remains:

- Rust owns compute-peer execution
- Elixir owns control-plane orchestration
- UI surfaces consume contracts

## Current Transitional Inventory

The following areas are the main known transitional bridges in the repository
today.

### `apps/web/lib/kyuubiki_web/workers/mock_worker_adapter.ex`

Current role:

- Elixir-side bridge that invokes Rust CLI worker flows for local iteration

Why it is transitional:

- the control plane is still mediating a worker-launch shape rather than only
  speaking to long-lived agent/runtime peers through stable protocol paths

Desired end state:

- orchestrator submits work to protocol-visible Rust agents
- local developer flows may still exist, but as explicit dev-mode adapters
  rather than as architecture-defining runtime paths

### Native desktop direct-mesh gateway

Current role:

- `kyuubiki-runtime` exposes a loopback-only HTTP gateway that lets the browser
  workbench talk to Rust agents in direct-mesh mode

Why it remains a gateway:

- it translates browser HTTP into the same bounded solver-RPC frames used by
  headless callers, but does not own solver or scheduling truth

Desired end state:

- keep these routes as deliberate native product-owned gateway contracts
- require explicit deployment opt-in when they are exposed outside local
  workstation use
- do not let them become the hidden source of truth for runtime architecture
- preserve a parallel headless path where SDKs and non-UI callers can reach
  the same capability without inheriting frontend assumptions

### `apps/installer-gui/src-tauri/src/remote.rs`

Current role:

- desktop-side bootstrap and remote start helper for Rust agents

Why it is transitional:

- command construction and remote process launching are still tightly coupled to
  desktop-side management flows

Desired end state:

- keep desktop bootstrap as an operator convenience layer
- move long-lived runtime authority and execution semantics back behind stable
  manifests, installer contracts, and agent self-description

### Frontend runtime governance and dispatch helpers

Current role:

- frontend/runtime-mode selection, direct-mesh authority selection, and
  headless dispatch planning

Why it is transitional:

- parts of runtime policy are still described close to UI workflow logic

Desired end state:

- keep UI-side visibility and selection
- keep actual runtime authority, capability truth, and execution semantics in
  protocol-visible runtime descriptors and orchestrator contracts

## Convergence Priorities

Use this order when reducing transitional architecture over later `1.x` work.

1. keep agent capability truth in Rust
   The compute peer should remain the authoritative source for solver-side
   capability, health, and execution semantics.
2. keep orchestration truth in Elixir
   Scheduling, persistence, registry state, and control-plane security should
   not leak into frontend-owned logic.
3. convert bridges into explicit dev or operator layers
   If a bridge remains, name it as a bridge, scope it, and stop treating it as
   the permanent runtime model.
4. align SDK and UI surfaces to the same contracts
   If the browser can do something important, headless SDKs should be able to
   reach the same capability through public boundaries.
5. document every runtime-only authority clearly
   Package authority, agent authority, direct-mesh authority, and orchestrator
   authority should stay inspectable and explicit.

## Practical Review Checklist

Before accepting a runtime-facing change, confirm:

- the Rust agent remains runnable without Phoenix or React
- the orchestrator remains replaceable as one runtime target family
- the browser workbench is consuming a contract, not a private runtime detail
- Hub and Installer are managing runtime shape, not redefining it
- SDK callers can still reach the same capability through stable public
  boundaries
- any remaining bridge is explicitly marked as transitional, local-only, or
  operator-convenience behavior

## Red-Line Rules

Do not let the system drift into any of these states:

1. `agent` becomes UI-aware.
2. `orchestrator` becomes solver-hot-path compute.
3. `frontend` becomes the hidden runtime architecture authority.
4. `SDKs` become private escape hatches around public contracts.
5. `Hub` becomes the orchestrator itself.
6. `Workbench` becomes responsible for agent deployment internals.

Any of those should trigger architecture review, not opportunistic coding.

## Design Test

When adding a runtime feature, ask:

1. Could the Rust agent still run headlessly if this UI disappeared?
2. Could the orchestrator still coordinate remote agents if the desktop shell changed?
3. Is the behavior exposed through a named protocol or schema?
4. Are we adding capability, or leaking one layer into another?

If the answer to any of those is "no" or "not sure", the design is probably
crossing the boundary.

## Short Version

Use this sentence when people start mixing terms:

`The solver agent is Rust. The orchestrator is Elixir. The frontends are product shells. They meet through contracts, not inheritance.`
