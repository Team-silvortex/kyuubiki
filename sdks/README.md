# Headless SDKs

`sdks/` contains protocol-driven, headless client libraries intended for:

- AI agents that need stable programmatic access to Kyuubiki
- automation pipelines
- CLI tools and notebooks
- backend integrations that should not depend on the browser workbench

Current language targets:

- `rust/`
- `python/`
- `elixir/`

Each SDK follows the same split:

- `ControlPlaneClient`
  Talks to `kyuubiki.control-plane/http-v1`
- `SolverRpcClient`
  Talks to `kyuubiki.solver-rpc/v1`

The first SDK cut focuses on the smallest useful headless surface:

- health and protocol descriptor discovery
- reachable agent discovery
- solver job submission through the control plane
- direct TCP RPC access to headless agents
- JSON-first payloads that AI models can generate or inspect easily

These SDKs intentionally target the public protocol boundaries described in
[`docs/protocols.md`](/Users/Shared/chroot/dev/kyuubiki/docs/protocols.md), not
frontend internals.
