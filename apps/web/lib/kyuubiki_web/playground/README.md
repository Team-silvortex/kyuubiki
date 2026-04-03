# Agent Runtime Domain

This directory contains the orchestrator-side runtime boundary for solver
execution.

- `agent_client.ex`
  Framed TCP RPC client, heartbeat handling, cancellation, and timeout logic.
- `agent_pool.ex`
  Agent discovery and routing across static, manifest, and registry-backed
  endpoint sets.
- `agent_registry.ex`
  Runtime registry for remotely deployed agents that self-register and
  heartbeat.
- `solver.ex`
  Local Elixir-side helper solver used by smaller browser/playground flows.

Despite the historical `playground/` name, this directory now functions as the
control-plane runtime/agent integration boundary.
