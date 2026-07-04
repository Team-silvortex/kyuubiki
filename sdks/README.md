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
- [sdks/python/examples/run_material_envelope.py](python/examples/run_material_envelope.py)
- [sdks/elixir/examples/run_material_envelope.exs](elixir/examples/run_material_envelope.exs)
- [sdks/python/examples/execute_operator_task_batch.py](python/examples/execute_operator_task_batch.py)
- [sdks/elixir/examples/execute_operator_task_batch.exs](elixir/examples/execute_operator_task_batch.exs)
- [sdks/rust/examples/execute_operator_task_batch.rs](rust/examples/execute_operator_task_batch.rs)

The batch examples accept JSON shaped like
[schemas/examples.operator-task-batch.json](../schemas/examples.operator-task-batch.json).
In production, these files are usually emitted by
`transform.compose_quality_execution_batch` after a quality/parameter-sweep
workflow has materialized candidate cases.

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
- Python and Elixir SDK callers can build the
  `workflow.material-study-envelope-ranking-json` material envelope catalog
  request without embedding the workflow graph inline
- headless workflow plans now expose runtime style, engine mix, step bindings,
  and required confirmation flags before live execution
- the Rust headless SDK now includes a concrete material-research template,
  `material_heat_spreader_screening`, for comparing thermal heat-spreader
  candidates through solve/wait/result chains
- the Rust headless SDK also includes `material_dielectric_screening`, an
  electrostatic dielectric material study that ranks breakdown margin, field
  intensity, dielectric loss proxy, and mass
- Rust, Python, and Elixir material research helpers can turn result payloads into a
  ranked report with explicit metric contracts and missing-metric warnings
- material reports include first-class optimization profiles so downstream
  agents can inspect score formulas, constraints, normalized scores, and
  weighted metric contributions
- heat-spreader reports now include a reliability envelope with material-card
  provenance, unit-system metadata, model assumptions, quality gates, and
  explicit screening-only limitations
- Rust, Python, and Elixir SDK callers can now use one material-report dispatch surface to extract
  successful `result_fetch` payloads from a headless run and build the matching
  heat-spreader, dielectric, thermo-shield, or structural-panel report
- Rust, Python, and Elixir control-plane clients can prepare, execute,
  prepare batches, batch-execute, checkpoint, and verify checkpoint manifests
  for language-neutral Operator TaskIR payloads, then derive resume plans through
  `/api/v1/operator-tasks/*`, including `quality_execution_batch` files emitted
  by optimization workflows

The same report path is also exposed as a Rust CLI:

```bash
kyuubiki-material-report list --json
kyuubiki-material-report describe dielectric-screening --json
kyuubiki-material-report heat-spreader --results results.json --out report.json --json
kyuubiki-material-report dielectric-screening --results dielectric-results.json --json
kyuubiki-material-report thermo-shield --results thermo-results.json --out thermo-report.json --json
kyuubiki-material-report thermo-shield --results thermo-results.json --profile profile.json --json
```

For a first end-to-end local material exploration prototype, run:

```bash
kyuubiki-material-explore heat-spreader --json
kyuubiki-material-explore dielectric-screening
kyuubiki-material-explore thermo-shield --out thermo-exploration.json
kyuubiki-material-explore structural-panel
```

`kyuubiki-material-explore` enumerates the study candidates, runs the generated
models through local Rust solver kernels, and feeds the real result payloads
back into the material report ranking layer. The output uses the reusable
`kyuubiki.material-exploration-run/v1` SDK contract, so later local, remote
agent, and mesh runners can share the same result shape.

`list` and `describe` expose the machine-readable study contract: aliases,
template id, report schema, research domain, objective, and metric specs.

The current SDK cut focuses on the smallest useful headless surface plus a
thin workflow layer:

- health and protocol descriptor discovery
- reachable agent discovery
- jobs/results/export CRUD through the control plane
- workflow catalog discovery, operator discovery, and workflow job submission
- operator TaskIR prepare/execute/prepare-batch/batch-execute/checkpoint/verify-checkpoint/resume-plan through the control plane
- headless workflow plan/preflight reports for CI and agent policy checks
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
- `thermal_bar_1d`, `heat_bar_1d`, `electrostatic_bar_1d`, `magnetostatic_bar_1d`
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
