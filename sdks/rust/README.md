# Kyuubiki Rust SDK

Protocol-driven Rust SDK for Kyuubiki headless integration.

```rust
use std::time::Duration;

use kyuubiki_headless_sdk::{
    ControlPlaneClient, KyuubikiAgentClient, KyuubikiAuth, KyuubikiSession, RetryPolicy, SolverRpcClient,
    WorkflowDatasetContract, WorkflowGraphDefinition, workflow_dataset_contract, workflow_dataset_value, workflow_edge, workflow_graph,
    workflow_node, workflow_port,
};

let cp = ControlPlaneClient::new("http://127.0.0.1:4000")?;
let health = cp.health()?;

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
.with_dataset_contract(built_dataset);
built_graph.validate()?;
```

Highlights:

- jobs/results/export control-plane surface
- workflow graph and dataset contract typed structs with validation
- workflow builder helpers for graph, node, edge, port, and dataset assembly
- direct solver-RPC client
- high-level `KyuubikiSession` for batch submit and wait loops
- `KyuubikiAgentClient` for run-study and chunk-browse flows
- retry, failure classification, and chunk iteration helpers
- reusable `KyuubikiAuth` plus more explicit error variants
- embedding-friendly API for headless agents and CLIs

Example:

- Run from [run_study.rs](examples/run_study.rs)
- Typical invocation:
  `cargo run --manifest-path sdks/rust/Cargo.toml --example run_study`
- Smoke test:
  `cargo test --manifest-path sdks/rust/Cargo.toml --test smoke`
  `cargo test --manifest-path sdks/rust/Cargo.toml --test workflow_contracts`
  `cargo test --manifest-path sdks/rust/Cargo.toml --test workflow_builders`
