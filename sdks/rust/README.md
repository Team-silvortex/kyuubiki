# Kyuubiki Rust SDK

Protocol-driven Rust SDK for Kyuubiki headless integration.

## Role

The Rust SDK is the native embedding line for agents, solver-side utilities,
installer tooling, local automation, and high-confidence reference runners. It
shares the same headless contracts as the Python and Elixir SDKs; Rust CLIs are
packaged examples over those contracts rather than the only supported entry.

## Research example

`examples/layered_thermal_research.rs` runs 14 bounded heat-to-structure workflow
cases through `KyuubikiAgentClient`, without a GUI or a separate research CLI.
It retains requests, results, checksums and real job ids, checks analytical
temperature/flux/expansion references, and exits nonzero on a failed gate.
Use a new output directory on the compute host:

```sh
cargo run --locked --manifest-path sdks/rust/Cargo.toml \
  --example layered_thermal_research -- research-round-001
```

See the [HTML research tutorial](../../docs/research-layered-thermal.html) for
parameters, authentication boundaries, limitations and first findings.
Append `thermal-patch` after the output directory to select 288 free/clamped
thermal-expansion cases, or `all` for all 302 reference cases. These suites share
the same reviewed submission, retention and numerical-gate path; no mock is
substituted for service execution. Current-source engine verification and
installed-service acceptance are reported separately.

After retaining a successful service round, use the read-only verifier to fetch
the same job identities and compare every result against the original evidence:

```sh
cargo run --locked --manifest-path sdks/rust/Cargo.toml \
  --example verify_layered_thermal_research -- research-round-003 research-readback-001
```

It submits no new jobs and requires the retained request/result digests, expected
case inventory, output contracts, and independent numerical gates to agree.
Restarting services is an external deployment action; retain process lifecycle
evidence separately before claiming restart durability. JSON decoding enables
`float_roundtrip` so scientific values retain their IEEE-754 representation
through save/reload, without introducing a comparison tolerance.

SDK library/example fixtures are bundled under `fixtures/`; repository tests
additionally check their parity with shared schemas. Bootstrap-document tests
still require repository documentation and the other official SDK surfaces.

## Durable Direct-Agent Modal Research

`examples/durable_modal_research.rs` submits a synthetic two-material,
unequal-length cantilever through the official `SolverRpcClient`. It requests
four bending modes and checks them against independently assembled stiffness,
original lumped masses, bracketed spectral roots, physical residuals and mass
orthogonality. This is a small numerical reference, not material certification.

Use an explicitly managed Agent with a durable execution store. Set
`KYUUBIKI_RESEARCH_AGENT_ENDPOINT` to its loopback `host:port`; this raw RPC
example adds no network authentication and must not expose an Agent publicly.
Choose a new caller-owned 32-character lowercase hexadecimal execution ID:

```sh
cargo run --locked --manifest-path sdks/rust/Cargo.toml \
  --example durable_modal_research -- submit tmp/modal-study-001 0123456789abcdef0123456789abcdef
cargo run --locked --manifest-path sdks/rust/Cargo.toml \
  --example durable_modal_research -- inspect tmp/modal-study-001 tmp/modal-export-001
```

The parent `tmp` directory must exist. `submit` saves input and a typed-digest
`DurableExecutionIntent` before sending, then retains admission separately.
`inspect` only reads the original store/ID and exports to a new directory.
If acknowledgement was lost, a completed receipt can be reconciled against
that saved intent and exact input, then independently checked before publication.
Changed input, different endpoint/store/digest, non-completed status, or a
present but corrupt/contradictory admission blocks result export. An old intent
without the typed digest cannot be upgraded by trusting a returned receipt.
After a valid observation, a later inspection failure keeps the raw receipt;
use a fresh output directory for another explicit read.

The example's formal export now lives under `verified/result.json` and
`verified/verification.json`, not directly in the output root. `receipt.json`
is raw diagnostic evidence, not a qualified result. The example checks the
physical result before staging, compares staged bytes with the original
serialization, and independently checks the disk readback. A same-parent
directory rename then publishes result and verification together. Consumers
must require both files, `passed: true`, matching execution identity and the
recorded result SHA-256; a directory name alone is not evidence.

Failed exports clean only their own `.verification-pending` directory. A
forced process kill can leave that pending directory, which is never a formal
export and is not reused automatically. Use a fresh output directory to read
the original execution again, without resubmission. Existing exports and
pre-existing staging data are not overwritten. This is a single-writer local
example, not a generic SDK filesystem policy, hostile-filesystem guarantee,
multi-writer transaction or host-power-loss qualification. See the
[publication-boundary regression](../../reports/headless-modal-export-publication-20261025.md).

The example's input, intent, admission and raw receipt records are also staged:
write a new same-directory `.NAME.pending` file, synchronize it, require exact
readback, then publish the final name with a no-replace hard link. The pending
alias is removed after publication; unsupported filesystems fail closed rather
than falling back to an overwriting rename. The writer uses the same 8 MiB
example-local record limit as the reader and only the four fixed filenames.

Ordinary failures clean only newly owned staging files. A forced process kill
can leave a partial or complete pending record, which is neither promoted nor
reused automatically. An interrupted admission write therefore leaves no corrupt
final admission; recovery can inspect the original task using its already saved
input and intent. Missing/partial intent remains a blocked local record, and
`not_found` is never permission to resubmit. Existing corrupt final admission
still blocks recovery and is not downgraded to missing. This is local process-kill
recovery, not directory/host-power-loss durability, a filesystem migration policy
or a generic SDK persistence API. See the
[record-publication interruption regression](../../reports/headless-modal-record-publication-20261025.md).

If export stopped after a complete `receipt.json` was retained, recover the
derived files without bringing the Agent back online:

```sh
cargo run --locked --manifest-path sdks/rust/Cargo.toml \
  --example durable_modal_research -- recover-export tmp/modal-study-001 tmp/modal-export-001 tmp/modal-export-recovered
```

`recover-export` is an explicit example-local offline operation, not a solver
resume/retry or generic SDK storage API. It binds the saved input/intent,
optional admission and completed receipt, rechecks the physics, and publishes
to a new directory using the same staged verification. It never reads/promotes
pending result files, modifies source evidence or contacts an Agent. The new
directory must be outside both source trees, including resolved parent aliases;
its parent must already exist. The receipt copy is reserialized from the checked
JSON; source bytes remain unchanged, and typed result identity is preserved.

Missing/partial/failed/unknown/foreign receipts and corrupt or contradictory
admission block recovery before output creation. Any existing `verified` entry
in the source, even an empty directory, file or dangling link, also blocks it:
use `verify` to diagnose published evidence instead of bypassing it. If the final
receipt is missing, only a fresh online `inspect` can read the original task;
pending receipts cannot authorize offline export or resubmission. This does not
authenticate caller-owned records or prove the current remote state. See the
[offline export recovery regression](../../reports/headless-modal-offline-export-recovery-20261025.md).

Recheck an existing export without an Agent or endpoint configuration:

```sh
cargo run --locked --manifest-path sdks/rust/Cargo.toml \
  --example durable_modal_research -- verify tmp/modal-study-001 tmp/modal-export-001
```

`verify` is an example-local, read-only consumer, not a generic headless CLI or
SDK storage policy. It checks the saved input hash and typed intent, optional
admission, original completed receipt, formal result hash/content and report
identity/claims, then repeats the independent physical checks. It neither
contacts the Agent nor creates, repairs, promotes or overwrites files. Missing
admission can still reconcile through the saved intent; present corrupt
admission cannot be ignored. Pending-only or incomplete exports fail.

The fresh JSON verdict goes to standard output with
`verification_source: "offline_recomputed"`. Historical root/residual arrays
must have valid finite shapes but are recomputed, not used as numerical truth.
`historical_diagnostics_match: false` explicitly flags differing diagnostics;
it does not by itself invalidate a result that passes fresh physical checks.
This avoids demanding bit-identical diagnostic arithmetic across toolchains.
The check does not authenticate a set of mutually forged records, verify the
current remote store, or qualify arbitrary modal models. See the
[offline readback regression](../../reports/headless-modal-offline-readback-20261025.md).

The independent reference also checks the full submitted input echo, each mode's
angular frequency against its eigenvalue and Hz value, its declared
`participation_norm` against the Euclidean norm of its shape, and the returned
minimum/maximum frequency summaries. Here `participation_norm` is a shape norm,
not an effective modal mass or participation factor. Missing, malformed or
contradictory fields reject both publication and offline revalidation even if
all saved copies and hashes agree. Elementary derived-value checks allow normal
floating-point rounding; the separate original-result binding remains exact
over typed values. This model-specific reference stays outside the generic SDK
and Agent engine. See the [modal metadata regression](../../reports/headless-modal-metadata-validation-20261025.md).

The modal example uses one bounded evidence reader for input, intent, admission,
receipt, published result/report and staged readback. Each file is limited to
8 MiB of encoded data and must be a regular non-symlink file in a non-symlink
immediate container. Only a genuinely absent admission leaf is optional;
broken links, directories and unreadable files cannot downgrade to missing.
Duplicate JSON object keys (including nested or escape-equivalent keys),
trailing documents, invalid UTF-8 and excessive parser depth are rejected.
Read/parse failures identify the affected filename. The same read supplies
both exact bytes for hashing and the parsed value.

This is the four-mode example's file policy, not a model-size or general SDK
RPC limit. It is not an exact JSON-heap budget, a complete ancestor-path check,
or a defense against hostile concurrent filesystem changes. See the
[bounded evidence regression](../../reports/headless-modal-evidence-boundaries-20261025.md).

Embedding applications should persist `DurableExecutionIntent::new(...)` with
input and endpoint before calling `submit_prepared_durable_agent_task` once.
Use `inspect_durable_agent_task` and `intent.handle_from_report` after transport
loss; `DurableExecutionHandle::completed_result` still requires completion.
The convenience `submit_durable_agent_task` also checks the input digest, but
does not persist anything for its caller. Raw inspection and retained-handle
fetch remain available. These checks bind identity/content, not authenticity
or scientific validity. Persistence, protected endpoint configuration and
numerical validation remain the embedding application's responsibility.
`outcome_unknown` and `not_found` never authorize automatic resubmission. An
explicit new attempt must use a new ID and remains separate from the old one.

Retain full observations when later reads must also preserve a known outcome.
`fetch_durable_agent_execution(&handle)` validates identity but the handle does
not contain previous terminal contents. Use
`fetch_durable_agent_execution_after(&retained_receipt)` for one read-only fetch
that also checks consistency with that observation, or call
`handle.validate_followup(&retained_receipt, &observed_receipt)` on records you
already have. A retained `completed`/`failed` outcome cannot change status,
disappear into a nonterminal state, replace its result, or rewrite its failure
code/message/details. Conflict is an error, not permission to resubmit.

The comparison covers known v1 terminal fields with lossless typed values,
including signed zero and low binary64 bits, without cloning a large result
payload. Optional absent/null wire fields are equivalent; unrelated observation
metadata is not a new outcome. This adds no wire digest or exchange format.
History-aware fetch validates and bounds retained terminal content before
connecting; a remote outage cannot hide that local content is unusable. The
retained commitment is computed once per call and reused after the read, not
stored in a global cache or persisted as a new wire field.
Running and unknown observations have no committed terminal payload and may
reconcile with a valid original terminal receipt. They still grant no replay
authority. A consistent failed receipt remains inspectable, not a successful
physical result; `completed_result` still returns the original solver error.

This is explicit caller-owned history, not a global SDK cache, authentication
or a complete chronological state machine. Save input/intent and the full
receipt before relying on later history checks. The example applies the same
SDK rule between a present saved admission and the observed/exported receipt
for both `inspect` and offline `verify`. See the
[terminal consistency regression](../../reports/headless-terminal-observation-consistency-20261025.md).

The example preflights local admission shape/intent binding before online
endpoint configuration or fetching, and before offline receipt consumption.
Known corrupt evidence therefore produces its local filename error without
contacting an Agent or creating an export. `submit` and `inspect` also reject
occupied destinations (including dangling links) and absent/non-directory
parents before network access. Parents must already exist. This read-only path
check is not a reservation or permission guarantee; the final exclusive
directory creation and publication checks still protect existing output.
Valid local records can still conflict with a later remote observation, which
is checked after fetching and may retain a raw diagnostic receipt, never a
verified result. See the [local preflight regression](../../reports/headless-modal-local-preflight-20261025.md).

For a separately decoded result, call
`handle.validate_completed_result(&receipt, &result)` before using it. This
requires the original identity-bound successful receipt and compares typed
content without copying the full payload. Signed zero, binary64 bits, integer
precision and integer-versus-float distinctions are preserved; object-key order
and equivalent JSON spelling are not differences. Missing and null fields
inside a result remain different, unlike optional wire-envelope fields.

This is not a physics verdict or file-integrity check. Check retained file bytes
against their recorded hash separately, then verify physical admissibility;
none of these three gates replaces another. The modal offline reader now uses
this SDK gate instead of ordinary JSON equality, which loses the sign of zero.
A consistent failed receipt still returns the original solver error, not a
matching result. No format, network request, automatic repair or replay is added.
See the [result-content regression](../../reports/headless-result-content-binding-20261025.md).

Terminal receipts validate their inner RPC version, original request ID,
exclusive result/error state and non-empty failure code/message, not only the
outer observation response. A valid failed receipt makes `completed_result`
return the original `SdkError::Rpc` with code, message, `request_id` and optional
`details`; it never becomes a result. Invalid or contradictory receipts remain
rejected before their diagnostics can be treated as a trusted failure context.

`make test-sdk-modal-recovery` builds disposable development binaries and runs
real local Agent process-kill, lost-acknowledgement/request, ledger replacement,
export/readback and numerical negative controls. Shared golden digest vectors
are checked by both the standalone SDK and Agent protocol. Tests clean their
own temporary directories. This is not a
host power cut, installed runtime qualification or the unresolved long-bending
multi-mode production campaign. See the [HTML tutorial](../../docs/tutorial-headless-rust.html#durable-modal-research).

## Direct RPC Resource Policy

`SolverRpcClient` validates protocol version, exact request identity, progress
payloads and mutually exclusive success/failure states. Known duplicate
envelope fields, foreign replies and invalid frames fail instead of becoming
results. Each call uses a process-wide sequence in its ID; this is correlation,
not authentication. It does not automatically retry or cancel a timed-out task.

Remote `SdkError::Rpc` preserves the validated response identity and optional
structured diagnostics. Locally generated configuration errors have neither.
Match `SdkError::Rpc { code, message, .. }` when the additional context is not
needed; exhaustive patterns and manually constructed variants must account for
the new fields. Diagnostic content is untrusted data, never retry permission.
`Display` emits only code/message, while `Debug` includes details. Neither is a
general redaction policy: apply caller-owned access and logging rules before
retaining or exposing remote messages, raw receipts or structured details.
See the [failure-context recovery report](../../reports/headless-failure-context-recovery-20261025.md).

Inspect `client.limits()` or configure `SolverRpcLimits` with `with_limits`:

| Setting | Default | Meaning |
| --- | --- | --- |
| `max_request_bytes` | 256 MiB | Encoded request payload; reject before connecting |
| `max_response_bytes` | 256 MiB | Each incoming payload, checked before allocating its buffer |
| `max_progress_bytes` | 4 MiB | Cumulative encoded progress/heartbeat payloads |
| `max_progress_frames` | 4096 | Maximum retained progress/heartbeat messages |
| `io_timeout` | 15 seconds | Idle I/O and per-address connection wait |
| `call_timeout` | 30 minutes | Shared network deadline, not extended by heartbeats or partial reads |

These are encoded-byte budgets, not exact parsed-JSON memory limits. Set larger
budgets explicitly for a known large solve and align them with Agent limits;
the SDK never silently expands them or resends an oversized request. Zero
progress budgets reject progress rather than silently dropping diagnostics.
OS hostname resolution is synchronous and cannot be forcibly interrupted by
this client, though its elapsed time consumes the deadline. Request encoding
precedes the deadline; numerical execution has its own server-side policy.
A timeout or protocol error after dispatch does not prove that execution
failed. For durable work, retain the intent and inspect the original execution.
Closing the connection can still trigger the Agent's own disconnect policy;
absence of an SDK cancellation RPC does not guarantee that work keeps running.
The example does not add TLS or authentication to raw Agent RPC.

## Model collaboration

Before projecting tools, `inspect_model_research_bootstrap(...)` checks the
machine bootstrap through a caller-owned resource resolver. The report exposes
the selected SDK entrypoints, missing project-relative resources, hard rules,
stop conditions, and completion contract. `ready_for_planning` never grants
execution authority; every report is fixed to `none_preflight_only`.
`build_bootstrapped_model_headless_plan(...)` then accepts only a Rust-bound,
blocker-free report whose workflow matches the collaboration session. It
returns the ordinary confirmation-aware plan and performs no dispatch.

The official Rust Headless SDK can expose a policy-filtered action catalog to
OpenAI Responses, OpenAI-compatible Chat, Anthropic, Gemini, or a canonical
JSON model gateway. Provider calls normalize into
`ModelWorkflowProposal`, then `build_model_headless_plan(...)` validates every
action and payload before any caller dispatches it.

```rust
use kyuubiki_headless_sdk::{
    ModelCollaborationSession, ModelProvider, build_model_collaboration_request,
    build_model_headless_plan, normalize_model_response,
};

let session: ModelCollaborationSession = serde_json::from_str(include_str!(
    "../../../schemas/examples.model-collaboration-session.json"
))?;
let request = build_model_collaboration_request(
    ModelProvider::OpenAi,
    session.clone(),
    serde_json::json!({"study": "thermal-screening"}),
)?;

// Send request instructions and tools with the caller-owned model client.
let proposal = normalize_model_response(ModelProvider::OpenAi, &session.session_id, &response)?;
let plan = build_model_headless_plan(&session, &proposal)?;
assert!(plan.ok);
```

The default catalog is service-only and read-only. Sensitive or destructive
actions must be enabled explicitly and still produce confirmation-gated plan
steps. The SDK owns no model credentials, billing, HTTP client, or direct
execution authority. See [Model Collaboration SDK](../../docs/model-collaboration-sdk.md).

After caller review, `execute_model_headless_plan(...)` and
`SessionModelActionDispatcher` provide the bounded bridge into the existing
Headless session. `build_model_plan_approval_request(...)` creates a read-only
review surface and `compute_model_headless_plan_digest(...)` binds the complete
plan. Every gated step must match an exact caller-issued `ModelPlanApproval` v2
with that digest; missing, stale, or mismatched approval rejects the entire plan
before network access. A caller-owned `ModelApprovalVerifier` must authenticate
that approval independently of model output. `ModelResearchExecutionReceipt`
retains the same digest, per-step authority, output, and bounded failures so
partial attempts cannot masquerade as completed research.

Synchronous `operator_task_execute` and `operator_task_batch_execute` actions
must publish identity-bound, complete, nonblocked results before the plan can
advance. Partial batches and nested pending Agent receipts stop the plan with
a v2 `failed` receipt; the problem step keeps its authority and raw output.
`operator_task_recovery_summary(...)` exposes blocked readiness repair actions.
After repairing the gate, explicitly execute a reviewed plan again rather than
inferring success or automatically resubmitting. Preparing tasks and submitting
asynchronous jobs remain separate from synchronous computation. Low-level
`ControlPlaneClient` methods intentionally return raw HTTP receipts: their
`Ok` does not certify task completion. See the
[completion gate reference](../../docs/headless-sdks.md#official-rust-model-task-completion-gate).

The native reference entry is
`examples/execute_model_research_plan.rs`. It reads a collaboration session,
canonical proposal, and caller-issued approval from project-relative JSON
files, uses `KYUUBIKI_BASE_URL` plus an optional `KYUUBIKI_ACCESS_TOKEN`, and
requires caller-owned `KYUUBIKI_APPROVAL_ID` and
`KYUUBIKI_APPROVAL_AUTHORITY` values before printing the retained execution
receipt. Its environment verifier is illustrative; production integrations
should implement `ModelApprovalVerifier` against their trusted policy or
credential boundary.

```rust
use std::time::Duration;

use kyuubiki_headless_sdk::{
    build_workflow_output_manifest, validate_workflow_result_against_graph, ControlPlaneClient, KyuubikiAgentClient, KyuubikiAuth,
    KyuubikiSession, RetryPolicy, SolverRpcClient, material_study_envelope_catalog_request,
    material_study_execution_plan_example, MaterialResearchBundle,
    WorkflowDatasetContract, WorkflowGraphDefinition, workflow_dataset_contract, workflow_dataset_value, workflow_defaults,
    workflow_edge, workflow_graph, workflow_node, workflow_operator_fetch_entry, workflow_port,
};

let cp = ControlPlaneClient::new("http://127.0.0.1:4000")?;
let health = cp.health()?;
let operators = cp.list_workflow_operators()?;
let structural_operators = cp.list_workflow_operators_with_query(Some(&[
    ("domain", "structural".to_string()),
    ("family", "solver".to_string()),
]))?;
let operator = cp.fetch_workflow_operator("solver.truss_2d")?;
let workflow_descriptor = cp.fetch_workflow_catalog_workflow("workflow.heat-to-thermo-quad-2d")?;
let material_envelope = material_study_envelope_catalog_request(None);
let material_envelope_job = cp.submit_workflow_catalog_job(
    material_envelope["workflow_id"].as_str().unwrap(),
    &material_envelope["input_artifacts"],
)?;
let material_plan = material_study_execution_plan_example();
assert_eq!(
    material_plan["schema_version"],
    "kyuubiki.material-study-execution-plan/v1"
);
let retained_json = std::fs::read_to_string("tmp/material-research-bundle.json")?;
let retained_bundle = MaterialResearchBundle::from_json_verified(&retained_json)?;

let rpc = SolverRpcClient::new("127.0.0.1", 5001);
let descriptor = rpc.describe_agent()?;
let release = rpc.release_operator_package_job("operator-job-42")?;

let auth = KyuubikiAuth::access_token("dev-token");
let session = KyuubikiSession::from_control_plane_with_auth("http://127.0.0.1:4000", Some(auth))?
    .with_solver_rpc("127.0.0.1", 5001);

let outcome = session.submit_and_wait(
    "truss_2d",
    &serde_json::json!({"model": {}, "case": {}}),
    Duration::from_secs(1),
    Duration::from_secs(60),
)?;

let agent = KyuubikiAgentClient::new(session);
let run = agent.run_study(
    "truss_2d",
    &serde_json::json!({"model": {}, "case": {}}),
    Duration::from_secs(1),
    Duration::from_secs(60),
    true,
)?;
let nodes_page = agent.browse_result_chunks(
    run.terminal["job"]["job_id"].as_str().unwrap(),
    "nodes",
    0,
    250,
)?;
let retried = agent.run_study_with_retry(
    "truss_2d",
    &serde_json::json!({"model": {}, "case": {}}),
    Duration::from_secs(1),
    Duration::from_secs(60),
    true,
    &RetryPolicy::default(),
)?;
let workflow_run = agent.run_workflow_catalog(
    "workflow.heat-to-thermo-quad-2d",
    &serde_json::json!({"thermal_case": {"loadcase": "baseline"}}),
    Duration::from_secs(1),
    Duration::from_secs(60),
    true,
)?;
let workflow_runtime = workflow_run.workflow_runtime.as_ref().unwrap();
for page in agent.iter_result_chunks(
    run.terminal["job"]["job_id"].as_str().unwrap(),
    "nodes",
    250,
    0,
    Some(2),
) {
    let page = page?;
    println!("{}", page["returned"]);
}

let dataset: WorkflowDatasetContract =
    serde_json::from_str(include_str!("../../../schemas/examples.workflow-dataset.json"))?;
dataset.validate()?;
let graph: WorkflowGraphDefinition =
    serde_json::from_str(include_str!("../../../schemas/examples.workflow-graph.json"))?;
graph.validate()?;

let built_dataset = workflow_dataset_contract(
    "dataset.demo/v1",
    "1.0.0",
    vec![workflow_dataset_value("thermal_case", "study_model", "json_object")],
);
let built_graph = workflow_graph(
    "workflow.demo",
    "Demo workflow",
    "1.0.0",
    vec!["input".into()],
    vec![
        workflow_node("input", "input").with_outputs(vec![workflow_port("case", "study_model/demo").with_dataset_value("thermal_case")]),
        workflow_node("output", "output").with_inputs(vec![workflow_port("case", "study_model/demo").with_dataset_value("thermal_case")]),
    ],
    vec![workflow_edge("edge-1", "input", "case", "output", "case", "study_model/demo").with_dataset_value("thermal_case")],
)
.with_dataset_contract(built_dataset)
.with_defaults(
    workflow_defaults()
        .with_cache_policy("cached")
        .with_orchestrated(false)
        .with_dispatch_policy("central_fetch")
        .with_placement_tags(vec!["cpu".into()])
        .with_required_capabilities(vec!["solver.thermal".into()]),
)
.with_dispatch_policy("central_fetch")
.with_operator_fetch_plan(vec![
    workflow_operator_fetch_entry("input", "input.demo")
        .with_package_ref("kyuubiki://operators/input.demo")
        .with_version("1.0.0")
        .with_integrity("sha256:demo")
        .with_cache_scope("agent"),
])
.with_placement_tags(vec!["mesh-enabled".into()])
.with_required_capabilities(vec!["artifact-cache".into()]);
built_graph.validate()?;
let output_manifest = build_workflow_output_manifest(&graph)?;
let validated_outputs =
    validate_workflow_result_against_graph(&graph, workflow_run.result.as_ref().unwrap())?;
```

Highlights:

- jobs/results/export control-plane surface
- operator catalog listing, filtering, and descriptor fetch
- workflow catalog descriptor fetch plus auto graph resolution for catalog runs
- material envelope catalog workflow helper for Rust automation clients
- material study execution plan contract fixture for schedulers that need to
  inspect `--plan-study`-style output before solver dispatch
- retained material research bundle validation for native CI, agents, and CLIs,
  including real-solver authority, ranked evidence, quality-gate, confidence,
  and screening-readiness consistency checks
- expanded solve-kind coverage across structural, thermal,
  thermo-mechanical, and electrostatic study families
- workflow graph and dataset contract typed structs with validation
- distributed workflow execution-hint fields for dispatch policy, operator fetch
  plan, placement tags, and required capabilities
- workflow builder helpers for graph, node, edge, port, and dataset assembly
- workflow output manifest and result validation helpers
- direct solver-RPC client
- high-level `KyuubikiSession` for batch submit and wait loops
- `KyuubikiAgentClient` for run-study, workflow-run, and chunk-browse flows
- retry, failure classification, and chunk iteration helpers
- reusable `KyuubikiAuth` plus more explicit error variants
- embedding-friendly API for headless agents and CLIs
- provider-neutral model collaboration with context redaction, constrained
  tool projection, response normalization, and confirmation-gated planning
- caller-verified research result handoff that binds a `ready_to_validate`
  frontier to its result receipt and workflow graph, with optional screening
  bundle validation and no qualification overclaim

Example:

- Run from [run_study.rs](examples/run_study.rs)
- Material envelope workflow example:
  [run_material_envelope.rs](examples/run_material_envelope.rs)
- Material study execution-plan example:
  [plan_material_study.rs](examples/plan_material_study.rs)
- Material research bundle validation example:
  [validate_material_research_bundle.rs](examples/validate_material_research_bundle.rs)
- Typical invocation:
  `cargo run --manifest-path sdks/rust/Cargo.toml --example run_study`
- Material envelope invocation:
  `cargo run --manifest-path sdks/rust/Cargo.toml --example run_material_envelope`
- Material study plan invocation:
  `cargo run --manifest-path sdks/rust/Cargo.toml --example plan_material_study`
- Material research bundle invocation:
  `cargo run --manifest-path sdks/rust/Cargo.toml --example validate_material_research_bundle`
- Validate a generated bundle:
  `cargo run --manifest-path sdks/rust/Cargo.toml --example validate_material_research_bundle -- tmp/material-research-bundle-composite.json`

The bundle example calls `MaterialResearchBundle::from_json_verified(&text)`
against the original file. It checks structure and all four embedded artifact
digests without re-encoding JSON. v1 ignores formatting whitespace outside
strings but retains key order, numeric tokens and escape spelling. The regular
`validate()` method checks in-memory metadata only. Digest agreement detects
content drift, not producer authenticity, whole-bundle metadata integrity or
physical qualification. Verification describes only the imported document and
does not survive edits to the returned mutable object. See the
[import regression](../../reports/material-research-bundle-import-20261008.md).

Additional checks:

- Research bundle integrity regression:
  `cargo test --manifest-path sdks/rust/Cargo.toml --test material_research_bundle_integrity`
- Smoke test:
  `cargo test --manifest-path sdks/rust/Cargo.toml --test smoke`
  `cargo test --manifest-path sdks/rust/Cargo.toml --test workflow_contracts`
  `cargo test --manifest-path sdks/rust/Cargo.toml --test workflow_builders`
  `cargo test --manifest-path sdks/rust/Cargo.toml --test model_collaboration`
  `cargo test --manifest-path sdks/rust/Cargo.toml --test model_research_bootstrap`
