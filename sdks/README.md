# Headless SDKs

`sdks/` contains protocol-driven, headless client libraries intended for:

- AI agents that need stable programmatic access to Kyuubiki
- automation pipelines
- CLI tools and notebooks
- backend integrations that should not depend on the browser workbench

## Quick Start

If you are integrating from outside the monorepo, start with:

1. [docs/protocols.md](/Users/Shared/chroot/dev/kyuubiki/docs/protocols.md)
2. [docs/headless-sdks.md](/Users/Shared/chroot/dev/kyuubiki/docs/headless-sdks.md)
3. one language example below

Current language targets:

- `rust/`
- `python/`
- `elixir/`

Minimal runnable examples now live at:

- [sdks/python/examples/run_study.py](/Users/Shared/chroot/dev/kyuubiki/sdks/python/examples/run_study.py)
- [sdks/elixir/examples/run_study.exs](/Users/Shared/chroot/dev/kyuubiki/sdks/elixir/examples/run_study.exs)
- [sdks/rust/examples/run_study.rs](/Users/Shared/chroot/dev/kyuubiki/sdks/rust/examples/run_study.rs)

Smoke tests now live at:

- [sdks/python/tests/test_smoke.py](/Users/Shared/chroot/dev/kyuubiki/sdks/python/tests/test_smoke.py)
- [sdks/elixir/test/smoke_test.exs](/Users/Shared/chroot/dev/kyuubiki/sdks/elixir/test/smoke_test.exs)
- [sdks/rust/tests/smoke.rs](/Users/Shared/chroot/dev/kyuubiki/sdks/rust/tests/smoke.rs)

Each SDK follows the same split:

- `ControlPlaneClient`
  Talks to `kyuubiki.control-plane/http-v1`
- `SolverRpcClient`
  Talks to `kyuubiki.solver-rpc/v1`
- `Session`
  A higher-level AI/automation entry point for submit, batch, and wait flows
- `Auth`
  Reusable header-based auth descriptor for control-plane clients
- `AgentClient`
  AI-oriented orchestration helper for run-study, job-bundle, and chunk-browse flows

Recent additions:

- retry policies for transient study failures
- explicit error-to-failure classification
- auto-paged chunk iterators or streams for large result windows

The current SDK cut focuses on the smallest useful headless surface plus a
thin workflow layer:

- health and protocol descriptor discovery
- reachable agent discovery
- jobs/results/export CRUD through the control plane
- solver job submission through the control plane
- batch submit and terminal-state polling helpers
- direct TCP RPC access to headless agents
- structured transport / HTTP / RPC / timeout errors
- JSON-first payloads that AI models can generate or inspect easily

These SDKs intentionally target the public protocol boundaries described in
[`docs/protocols.md`](/Users/Shared/chroot/dev/kyuubiki/docs/protocols.md), not
frontend internals.
