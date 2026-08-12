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

SDK-local smoke coverage:

- [sdks/python/tests/test_smoke.py](../sdks/python/tests/test_smoke.py)
- [sdks/elixir/test/smoke_test.exs](../sdks/elixir/test/smoke_test.exs)
- [sdks/rust/tests/smoke.rs](../sdks/rust/tests/smoke.rs)

All three SDKs expose the same conceptual split:

- `ControlPlaneClient`
- `SolverRpcClient`
- `Session`
- `AgentClient`

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
- `solve_bar_1d`
- `solve_truss_2d`
- `solve_truss_3d`
- `solve_solid_tetra_3d`
- `solve_plane_triangle_2d`
- `cancel_job`

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
