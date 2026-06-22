# Headless SDKs

Kyuubiki now ships a dedicated `sdks/` top-level directory for protocol-first,
headless integrations.

## Why these SDKs exist

The browser workbench is becoming a powerful editor and operator shell, but AI
models and automation systems should not need to drive a GUI to use Kyuubiki.

The headless SDK layer gives them a cleaner tool surface:

- discover the running deployment
- inspect protocol compatibility
- submit FEM jobs
- poll job state
- describe reachable solver agents
- talk directly to solver RPC agents when the control plane is optional
- build a machine-readable Rust execution plan before running a workflow
- start concrete material-research examples without opening the workbench

## Language targets

- Rust
- Elixir
- Python

Minimal end-to-end examples:

- [sdks/python/examples/run_study.py](../sdks/python/examples/run_study.py)
- [sdks/elixir/examples/run_study.exs](../sdks/elixir/examples/run_study.exs)
- [sdks/rust/examples/run_study.rs](../sdks/rust/examples/run_study.rs)

SDK-local smoke coverage:

- [sdks/python/tests/test_smoke.py](../sdks/python/tests/test_smoke.py)
- [sdks/elixir/test/smoke_test.exs](../sdks/elixir/test/smoke_test.exs)
- [sdks/rust/tests/smoke.rs](../sdks/rust/tests/smoke.rs)

All three SDKs expose the same conceptual split:

- `ControlPlaneClient`
- `SolverRpcClient`
- `Session`
- `AgentClient`

## Design goals

- protocol-driven rather than implementation-driven
- simple JSON payloads for AI-generated requests
- usable in cloud, distributed, and direct headless LAN deployments
- small enough to embed into agent runtimes without dragging UI dependencies
- explicit auth and error surfaces so higher-level agent loops can branch safely

## First-cut capabilities

### Control plane

- `GET /api/health`
- `GET /api/v1/protocol`
- `GET /api/v1/protocol/agents`
- `GET /api/v1/workflows/catalog`
- `GET /api/v1/operators`
- `GET /api/v1/jobs`
- `PATCH /api/v1/jobs/:job_id`
- `DELETE /api/v1/jobs/:job_id`
- `POST /api/v1/fem/*/jobs`
- `POST /api/v1/workflows/catalog/:workflow_id/jobs`
- `POST /api/v1/workflows/graph/jobs`
- `GET /api/v1/jobs/:job_id`
- `POST /api/v1/jobs/:job_id/cancel`
- `GET /api/v1/results`
- `GET /api/v1/results/:job_id`
- `GET /api/v1/results/:job_id/chunks/:kind`
- `PATCH /api/v1/results/:job_id`
- `DELETE /api/v1/results/:job_id`
- `GET /api/v1/export/database`
- `GET /api/v1/export/security-events`
- `GET /api/v1/export/security-events.csv`

### Solver RPC

- `ping`
- `describe_agent`
- `solve_bar_1d`
- `solve_truss_2d`
- `solve_truss_3d`
- `solve_plane_triangle_2d`
- `cancel_job`

## Intended AI use

For AI agents, the recommended flow is:

1. Query the control-plane protocol descriptor.
2. Inspect reachable agents or direct endpoints.
3. Generate a JSON payload for the desired FEM study.
4. Submit through the control plane or directly over solver RPC.
5. Poll and stream progress until completion.

The SDKs are deliberately thin wrappers over public contracts so higher-level AI
planning layers can stay language-agnostic.

They now also expose a small workflow layer:

- submit one job by solve kind
- submit many jobs in sequence
- plan headless workflow execution before submission, including runtime style,
  engine mix, step bindings, executor compatibility, and required
  sensitive/destructive confirmations
- generate Rust-driven material screening workflows, starting with a thermal
  heat-spreader candidate comparison for Aluminum 6061, Copper C110, and
  in-plane pyrolytic graphite
- generate structural panel material workflows over aluminum, steel, and carbon
  fiber candidates without opening the Workbench
- build material research reports from headless result payloads, with explicit
  metric specs, weighted ranking, and visible missing-metric warnings
- expose optimization profiles as first-class report contracts, including
  score formulas, constraints, normalized metric scores, and weighted
  candidate contributions
- validate workflow graphs and workflow dataset contracts before submission
- wait for terminal job states by polling the control plane
- optionally bypass the control plane and solve directly over solver RPC
- run one study and fetch its result bundle in one call
- browse large result windows in chunked pages
- retry transient failures without retrying auth or logic errors
- classify failures into machine-usable buckets for agent policy layers

Rust material reports can be generated headlessly:

```bash
kyuubiki-material-report heat-spreader --results results.json --out report.json --json
kyuubiki-material-report thermo-shield --results thermo-results.json --out thermo-report.json --json
kyuubiki-material-report thermo-shield --results thermo-results.json --profile profile.json --json
kyuubiki-material-report structural-panel --results structural-results.json --json
kyuubiki-material-report structural-panel --results headless-run-report.json --json
```
