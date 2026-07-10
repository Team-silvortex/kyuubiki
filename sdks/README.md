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
- [sdks/rust/examples/run_material_envelope.rs](rust/examples/run_material_envelope.rs)
- [sdks/python/examples/plan_material_study.py](python/examples/plan_material_study.py)
- [sdks/elixir/examples/plan_material_study.exs](elixir/examples/plan_material_study.exs)
- [sdks/rust/examples/plan_material_study.rs](rust/examples/plan_material_study.rs)
- [sdks/python/examples/run_material_report.py](python/examples/run_material_report.py)
- [sdks/elixir/examples/run_material_report.exs](elixir/examples/run_material_report.exs)
- [sdks/rust/examples/validate_material_research_bundle.rs](rust/examples/validate_material_research_bundle.rs)
- [sdks/python/examples/validate_material_research_bundle.py](python/examples/validate_material_research_bundle.py)
- [sdks/elixir/examples/validate_material_research_bundle.exs](elixir/examples/validate_material_research_bundle.exs)
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
- Rust, Python, and Elixir SDK callers can build the
  `workflow.material-study-envelope-ranking-json` material envelope catalog
  request without embedding the workflow graph inline
- headless workflow plans now expose runtime style, engine mix, step bindings,
  and required confirmation flags before live execution
- the Rust headless SDK includes a concrete material-research template,
  `material_heat_spreader_screening`, for comparing thermal heat-spreader
  candidates through solve/wait/result chains
- the official SDK material-report helpers include `material_dielectric_screening`, an
  electrostatic dielectric material study that ranks breakdown margin, field
  intensity, dielectric loss proxy, and mass
- `material_composite_thermo_electric_panel` is available through the Rust,
  Python, and Elixir material-report helpers as the first mixed-material
  sequential multiphysics prototype, running electrostatic, heat, and
  thermo-mechanical solves for conductor/dielectric/substrate panel stacks,
  with screening-level interface mismatch risk in the ranked report
- Rust, Python, and Elixir material research helpers can turn result payloads into a
  ranked report with explicit metric contracts and missing-metric warnings
- material reports include first-class optimization profiles so downstream
  agents can inspect score formulas, constraints, normalized scores, and
  weighted metric contributions
- material reports expose reliability envelopes with quality gates, summarized
  gate decisions, and blocking gate IDs so automated next-round planning can
  distinguish repair, mitigation, and expansion across Rust, Python, and Elixir
- Rust, Python, and Elixir SDK callers can now use one material-report dispatch surface to extract
  successful `result_fetch` payloads from a headless run and build the matching
  heat-spreader, dielectric, thermo-shield, or structural-panel report
- Rust, Python, and Elixir control-plane clients can prepare, execute,
  prepare batches, batch-execute, checkpoint, and verify checkpoint manifests
  for language-neutral Operator TaskIR payloads, then derive resume plans through
  `/api/v1/operator-tasks/*`, including `quality_execution_batch` files emitted
  by optimization workflows
- Operator TaskIR batch prepare/execute responses expose top-level
  `error_codes` and `error_code_counts`, so headless agents and benchmark
  runners can classify failures without scanning every case result
- Rust headless dry-run previews reuse the same TaskIR digest, mirror, and
  execution-program error-code family as agent runtime preflight

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
kyuubiki-material-explore composite-thermo-electric-panel --json
```

This CLI is a reference runner on top of the SDK contracts, not the universal
headless gateway. Rust, Python, and Elixir remain peer official SDK families;
teams can build their own wrappers and research harnesses around the same
schemas.

`kyuubiki-material-explore` enumerates the study candidates, runs the generated
models through local Rust solver kernels, and feeds the real result payloads
back into the material report ranking layer. The output uses the reusable
`kyuubiki.material-exploration-run/v1` SDK contract, so later local, remote
agent, and mesh runners can share the same result shape. It also emits a
`kyuubiki.material-exploration-next-round/v1` `next_round` plan that tells
automation whether to repair/rerun incomplete results or expand around the
current winner. When quality gates fail with complete data, the plan uses
`mitigate_design_risk` so agents generate lower-risk neighbors instead of
blindly rerunning the same candidate. The `--plan-next` execution plan also
includes `risk_mitigation_hints` with focused candidate IDs, violated gates,
dominant risk drivers, and recommended mitigation moves. It also includes
`optimization_objectives`, a machine-readable block with the next-round mode,
winner candidate, primary metric IDs, and violated quality gates. It can also
emit `candidate_drafts` such as compliant-interlayer or lower-CTE dielectric
variants; these drafts are explicitly marked as requiring solver reruns before
they can be ranked. Candidate drafts carry `draft_id`, `lineage`, and
`required_result_schema` fields so agents can deduplicate, preserve provenance,
and route the draft to the right solver contract. `candidate_draft_summary`
adds aggregate draft counts, source candidate IDs, strategy counts, and required
result schemas so orchestration layers can size and route the next batch without
scanning every draft. `draft_execution_batches` groups draft IDs by required
result schema and marks them as `pending_agent_materialization`, giving
orchestra or mesh agents a direct dispatch shape for rerunning the focused
solver contract. Draft batches include an execution policy that requires human
review, disallows automatic materialization, and blocks qualification claims
until the rerun quality gates pass. They also include a `review_checklist` for
material-card provenance, geometry deltas, SI units, solver result schema, and
rerun quality gates. `review_status` starts as `pending_review` and `blocking`
so agents can report an audit gate instead of misclassifying the batch as a
solver failure. `allowed_review_actions` exposes approve, request-changes, and
reject transitions, each requiring reviewer identity and a reason.
`review_decision_template` gives UI and headless clients the standard payload
shape for submitting that decision. `review_decision_contract` lists required
fields, allowed actions, approval checklist requirements, and timestamp format
for preflight validation. SDK callers can apply a review decision to the batch
state, turning approved batches into `approved_for_materialization` while
keeping request-changes and reject paths blocking with reviewer provenance.
Approved batches can then produce a
`kyuubiki.material-candidate-materialization-request/v1` payload that is ready
for agent materialization and solver rerun dispatch. Combining that request
with the candidate draft list produces a
`kyuubiki.material-candidate-materialization-plan/v1` containing materialized
candidate specs and `requires_solver_rerun` status. The shared schema is
[schemas/material-candidate-materialization-plan.schema.json](../schemas/material-candidate-materialization-plan.schema.json),
with a concrete fixture in
[schemas/examples.material-candidate-materialization-plan.json](../schemas/examples.material-candidate-materialization-plan.json).
Before executing solver kernels, SDK and CLI users can also request a
non-executing material study plan through `--plan-study` or the Rust SDK
`build_material_study_execution_plan` helper. That output is governed by
[schemas/material-study-execution-plan.schema.json](../schemas/material-study-execution-plan.schema.json),
with a concrete fixture in
[schemas/examples.material-study-execution-plan.json](../schemas/examples.material-study-execution-plan.json).
Repeated next-round runs use
[schemas/material-exploration-chain.schema.json](../schemas/material-exploration-chain.schema.json)
to pin convergence assessment, optimization trace, repair planning, compact
summaries, and retained run artifacts. A compact fixture lives at
[schemas/examples.material-exploration-chain.json](../schemas/examples.material-exploration-chain.json).
The retained research bundle wraps the initial exploration, next-round
execution plan, rerun, chain, checksums, and reproduction commands under
[schemas/material-research-bundle.schema.json](../schemas/material-research-bundle.schema.json),
with a compact fixture at
[schemas/examples.material-research-bundle.json](../schemas/examples.material-research-bundle.json).
Rust SDK callers can decode that artifact with `MaterialResearchBundle`, while
Python and Elixir expose `validate_material_research_bundle` helpers for the
same fixture and generated bundle shape before passing retained artifacts to a
custom review, CI, notebook, or agent harness.
Composite panel materialized
specs can be converted back into concrete
`solve_composite_thermo_electric_panel` workflow steps, applying the selected
strategy to model parameters before rerun instead of only renaming the
candidate. The CLI can also consume that materialization plan directly and emit
a `kyuubiki.materialized-candidate-rerun/v1` artifact with rerun results,
materialized candidate IDs, materialized ranking report, source
materialization schema/status, and the next-round decision. Each exploration
artifact carries an `iteration`, so repeated runs can preserve lineage instead
of looking like disconnected one-shot solver batches.

To materialize that next step without rerunning the first round:

```bash
kyuubiki-material-explore --plan-next exploration.json --out next-round.json --json
```

The generated `kyuubiki.material-exploration-next-round-execution/v1` payload is
intended for agents, CI, and future orchestra runners.

Export a review decision template from the pending draft batch:

```bash
kyuubiki-material-explore --review-template next-round.json --out decision-template.json --json
```

The template export is read-only; it does not approve or materialize candidates.
After review, either edit the template manually or generate an explicit approval
decision with reviewer identity, reason, and timestamp:

```bash
kyuubiki-material-explore --approve-review-template decision-template.json --reviewer-id reviewer-1 --reason "prototype rerun approved" --decided-at 2026-07-07T00:00:00Z --out decision.json --json
```

Apply the explicit decision to produce a materialization plan:

```bash
kyuubiki-material-explore --materialize-reviewed next-round.json --review-decision decision.json --out materialization-plan.json --json
```

To run approved materialized composite candidates locally:

```bash
kyuubiki-material-explore --run-materialized materialization-plan.json --out materialized-rerun.json --json
```

To run that next step locally and emit the next exploration artifact:

```bash
kyuubiki-material-explore --run-next exploration.json --out next-exploration.json --json
```

The emitted next exploration includes `lineage` with the source iteration,
source winner, next-round decision, focus candidates, runnable step count, and
the `optimization_objectives` that drove the local rerun.

For a minimal continuous-loop smoke test, chain repeated next-round execution:

```bash
kyuubiki-material-explore --chain-next exploration.json --rounds 2 --out chain.json --json
```

The chain wrapper uses `kyuubiki.material-exploration-chain/v1` and keeps the
full exploration artifact plus a compact per-round summary for each generated
round. It also exposes `stop_reason`, decision counts, and winner stability so
agents can decide whether to continue, repair, or escalate. Each summary carries
the next-round `optimization_objectives`, and `optimization_trace` lifts the
per-round mode, primary metrics, winner, and violated gates into a compact
lineage view. `convergence_assessment` compares winner stability, winner score
drift, and repair state so automation does not confuse a stable but gate-blocked
candidate with a validated result. When repair is required, `repair_summary`
lifts violated quality gates and focus candidates to the chain top level, while
`repair_plan` turns that state into concrete agent-facing actions.

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
