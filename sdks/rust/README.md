# Kyuubiki Rust SDK

Protocol-driven Rust SDK for Kyuubiki headless integration.

## Role

The Rust SDK is the native embedding line for agents, solver-side utilities,
installer tooling, local automation, and high-confidence reference runners. It
shares the same headless contracts as the Python and Elixir SDKs; Rust CLIs are
packaged examples over those contracts rather than the only supported entry.

## Model collaboration

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
Headless session. Every gated step must match an exact caller-issued
`ModelPlanApproval`; missing or mismatched approval rejects the entire plan
before network access. A caller-owned `ModelApprovalVerifier` must authenticate
that approval independently of model output. `ModelResearchExecutionReceipt`
retains per-step authority, output, and bounded failures so partial attempts
cannot masquerade as completed research.

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
let retained_bundle: MaterialResearchBundle =
    serde_json::from_str(include_str!("../../../schemas/examples.material-research-bundle.json"))?;
retained_bundle.validate()?;

let rpc = SolverRpcClient::new("127.0.0.1", 5001);
let descriptor = rpc.describe_agent()?;

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
- Smoke test:
  `cargo test --manifest-path sdks/rust/Cargo.toml --test smoke`
  `cargo test --manifest-path sdks/rust/Cargo.toml --test workflow_contracts`
  `cargo test --manifest-path sdks/rust/Cargo.toml --test workflow_builders`
  `cargo test --manifest-path sdks/rust/Cargo.toml --test model_collaboration`
