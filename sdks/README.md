# Headless SDKs

`sdks/` contains protocol-driven, headless client libraries intended for:

- AI agents that need stable programmatic access to Kyuubiki
- automation pipelines
- CLI tools and notebooks
- backend integrations that should not depend on the browser workbench

## Quick Start

If you are integrating from outside the monorepo, start with:

1. [docs/protocols.md](../docs/protocols.md)
2. [docs/headless-sdks.md](../docs/headless-sdks.md)
3. one language example below

Current language targets:

- `rust/`
- `python/`
- `elixir/`

Minimal runnable examples now live at:

- [sdks/python/examples/run_study.py](python/examples/run_study.py)
- [sdks/elixir/examples/run_study.exs](elixir/examples/run_study.exs)
- [sdks/rust/examples/run_study.rs](rust/examples/run_study.rs)

Smoke tests now live at:

- [sdks/python/tests/test_smoke.py](python/tests/test_smoke.py)
- [sdks/elixir/test/smoke_test.exs](elixir/test/smoke_test.exs)
- [sdks/rust/tests/smoke.rs](rust/tests/smoke.rs)

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
  AI-oriented orchestration helper for run-study, workflow-run, job-bundle, and chunk-browse flows

Recent additions:

- retry policies for transient study failures
- explicit error-to-failure classification
- auto-paged chunk iterators or streams for large result windows
- broader solve-kind coverage across mechanical, thermal, thermo-mechanical,
  and electrostatic families through one normalized SDK dispatch surface
- workflow catalog and inline-graph runs lifted into the session and agent layers
- catalog workflow descriptors can be fetched directly, and catalog runs can auto-resolve
  their backing graph for output validation

The current SDK cut focuses on the smallest useful headless surface plus a
thin workflow layer:

- health and protocol descriptor discovery
- reachable agent discovery
- jobs/results/export CRUD through the control plane
- workflow catalog discovery, operator discovery, and workflow job submission
- workflow graph and workflow dataset contract validation helpers
- distributed workflow execution hints through dispatch policy, operator fetch
  plan, placement tags, and required capability fields
- workflow output manifest extraction and result-contract validation helpers
- solver job submission through the control plane
- batch submit and terminal-state polling helpers
- direct TCP RPC access to headless agents
- structured transport / HTTP / RPC / timeout errors
- JSON-first payloads that AI models can generate or inspect easily

The current solve-kind dispatcher now covers:

- `axial_bar_1d` / `bar_1d`
- `thermal_bar_1d`, `heat_bar_1d`, `electrostatic_bar_1d`
- `beam_1d`, `thermal_beam_1d`, `torsion_1d`
- `spring_1d`, `spring_2d`, `spring_3d`
- `truss_2d`, `thermal_truss_2d`, `truss_3d`, `thermal_truss_3d`
- `frame_2d`, `thermal_frame_2d`, `frame_3d`, `thermal_frame_3d`
- `plane_triangle_2d`, `heat_plane_triangle_2d`,
  `thermal_plane_triangle_2d`, `electrostatic_plane_triangle_2d`
- `plane_quad_2d`, `heat_plane_quad_2d`, `thermal_plane_quad_2d`,
  `electrostatic_plane_quad_2d`

These SDKs intentionally target the public protocol boundaries described in
[`docs/protocols.md`](../docs/protocols.md), not
frontend internals.
