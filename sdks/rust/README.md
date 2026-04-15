# Kyuubiki Rust SDK

Protocol-driven Rust SDK for Kyuubiki headless integration.

```rust
use std::time::Duration;

use kyuubiki_headless_sdk::{ControlPlaneClient, KyuubikiAgentClient, KyuubikiAuth, KyuubikiSession, RetryPolicy, SolverRpcClient};

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
```

Highlights:

- jobs/results/export control-plane surface
- direct solver-RPC client
- high-level `KyuubikiSession` for batch submit and wait loops
- `KyuubikiAgentClient` for run-study and chunk-browse flows
- retry, failure classification, and chunk iteration helpers
- reusable `KyuubikiAuth` plus more explicit error variants
- embedding-friendly API for headless agents and CLIs

Example:

- Run from [run_study.rs](/Users/Shared/chroot/dev/kyuubiki/sdks/rust/examples/run_study.rs)
- Typical invocation:
  `cargo run --manifest-path sdks/rust/Cargo.toml --example run_study`
- Smoke test:
  `cargo test --manifest-path sdks/rust/Cargo.toml --test smoke`
