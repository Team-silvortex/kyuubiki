# Kyuubiki Rust SDK

Minimal protocol-driven headless SDK for Kyuubiki.

```rust
use kyuubiki_headless_sdk::{ControlPlaneClient, SolverRpcClient};

let cp = ControlPlaneClient::new("http://127.0.0.1:4000")?;
let health = cp.health()?;

let rpc = SolverRpcClient::new("127.0.0.1", 5001);
let descriptor = rpc.describe_agent()?;
```
