# Material Research Simulation Roadmap

This roadmap tracks what Kyuubiki still needs before material research
workflows can be called reliable rather than merely runnable.

The current `tamamono 1.15.x` baseline already has useful pieces:

- Rust solver coverage across structural, thermal, electrostatic,
  magnetostatic, acoustic, simplified CFD, modal, contact, and nonlinear
  spring families
- workflow graph and workflow dataset contracts for multi-operator composition
- Rust-first headless SDK material studies for heat spreaders, dielectric
  screening, thermo-mechanical shields, and structural panels
- optimization profiles and material-report ranking contracts
- a first reliability envelope for heat-spreader reports, including material
  card provenance, unit metadata, model assumptions, quality gates, and
  explicit screening-only limitations

That is enough for prototypes and internal exploration. It is not yet enough
for serious material qualification, published research claims, or commercial
engineering decisions.

## Reliability Goal

Every serious material study should eventually answer six questions without
requiring tribal knowledge:

- what material data was used
- what model and assumptions were used
- what solver and mesh path produced the result
- what validation baseline supports the result
- what optimization objective ranked the candidates
- what limits make the result screening-only, review-ready, or qualification-ready

The target state is a reproducible material-study run bundle:

`material card + model + workflow + solve results + reliability envelope + report`

## Phase 1. Screening Studies Become Honest

Status:
active for `1.15.x`.

Primary objective:
make existing material studies transparent about their limits.

Required work:

- add reliability envelopes to dielectric, thermo-shield, and structural-panel
  reports, matching the heat-spreader shape
- expose material-card ids, unit systems, confidence levels, and parameter
  scopes in every material report
- keep all current material examples labeled as `screening_only` unless they
  have documented convergence and external validation
- add report-level quality gates for completeness, constraint violation,
  solver warnings, and candidate comparability
- document the distinction between ranking, review, and qualification

Exit criteria:

- every material report has a reliability envelope
- every candidate row has material-card provenance
- missing metrics and violated quality gates are machine-readable
- Rust headless SDK tests cover all material-study reliability envelopes

## Phase 2. Material Cards And Units Become First-Class

Primary objective:
stop letting material parameters behave like loose demo constants.

Required work:

- define a versioned material-card schema under `schemas/`
- include scalar, anisotropic, temperature-dependent, and uncertainty-aware
  parameter forms
- add unit-system normalization and unit mismatch diagnostics
- track source, confidence, applicable temperature range, and processing notes
- support project-local material libraries and built-in reference cards
- add import/export for material cards independent of UI state

Exit criteria:

- material studies consume material-card references rather than anonymous
  inline constants
- unit mismatches fail preflight before solver execution
- material-card changes are reflected in workflow lineage and report output

## Phase 3. Mesh, Geometry, And Boundary Conditions Become Reviewable

Primary objective:
make the model setup trustworthy enough for review, not just execution.

Required work:

- add mesh quality metrics for aspect ratio, collapsed elements, duplicate
  nodes, isolated regions, and boundary coverage
- add material-region assignment checks
- make boundary condition coverage visible in workflow and headless reports
- support imported mesh references in material-study bundles
- add mesh convergence hooks for selected thermal and structural studies
- record geometry, mesh, and boundary-condition fingerprints in results

Exit criteria:

- material-study reports can explain the mesh and boundary assumptions
- bad mesh or missing boundary coverage is a hard preflight failure where
  appropriate
- at least one heat-spreader or structural-panel case has a convergence record

## Phase 4. Solver Physics Deepens Beyond Linear Fixtures

Primary objective:
move from simple comparative fixtures toward research-relevant physics.

Required work:

- add anisotropic heat conduction for graphite-like heat-spreader studies
- add temperature-dependent conductivity and expansion coefficients
- deepen thermo-mechanical coupling with material curves
- add yield, safety factor, fatigue, and damage-oriented structural metrics
- improve dielectric studies with breakdown margin, field concentration, and
  loss models that can be traced to material cards
- define when simplified CFD is acceptable as a screening model

Exit criteria:

- reports can distinguish scalar, anisotropic, and temperature-dependent
  material assumptions
- at least one material family has a validated nonlinear or temperature-aware
  path
- solver limitations are visible in the same report shape as optimization
  results

## Phase 5. Validation Baselines Become Scientific Evidence

Primary objective:
make reliability claims benchmark-backed.

Required work:

- add analytic baseline cases for heat conduction, thermo-mechanical expansion,
  electrostatic fields, and structural deflection
- add literature or external-tool comparison baselines where possible
- add mesh convergence and tolerance convergence reports
- define acceptable error bands per study family
- store baseline provenance, solver version, and environment in test artifacts
- wire selected baselines into CI and release checks

Exit criteria:

- material reports can reference validation baseline ids
- accuracy claims include tolerances and source provenance
- regressions block release when they break trusted baseline families

## Phase 6. Optimization Becomes Research-Grade

Primary objective:
move beyond fixed weighted ranking into reproducible exploration.

Required work:

- add design-of-experiments plans for material and geometry parameters
- support multi-objective Pareto ranking
- add uncertainty and sensitivity analysis
- support repeated runs, failure classification, and partial-result recovery
- add optimizer provenance, random seeds, sampling strategy, and stop criteria
- expose optimization runs through the shared headless contracts, with the Rust
  reference runner used to harden schemas before Python and Elixir package
  surfaces consume the same artifacts

Exit criteria:

- optimization reports can be rerun deterministically
- Pareto fronts and sensitivity summaries are machine-readable
- failed candidates remain explainable rather than disappearing from rankings

## Phase 7. Distributed Execution And Persistence Become Boring

Primary objective:
make large material sweeps safe to run on real agents.

Required work:

- run material exploration through remote agents without frontend involvement
- store study inputs, results, reports, logs, and reliability envelopes as
  versioned artifacts
- add artifact deduplication and cache keys for repeated candidate solves
- support resume, retry, and cancellation at candidate and study level
- record agent, solver, operator, material-card, and workflow versions
- make installer-managed runtime dependencies reproducible across machines

Exit criteria:

- a material sweep can run remotely, fail halfway, resume, and produce a
  complete audit trail
- reports can be traced back to exact inputs and runtime versions
- agent execution does not require hidden SSH-only operational knowledge

## Phase 8. Workbench Review Catches Up To Headless Research

Primary objective:
make the UI a review surface for serious study runs.

Required work:

- show reliability envelopes in the Workbench material-study UI
- visualize quality gates, warnings, assumptions, and material-card provenance
- add comparison views for candidate ranking, Pareto sets, and sensitivity
  summaries
- add result-field inspection for thermal, structural, electric, and coupled
  study families
- keep UI automation stable by exposing fixed review surfaces, not
  user-extended arbitrary widgets

Exit criteria:

- the same material-study bundle is useful from CLI, SDK, and Workbench
- reviewers can understand why a candidate won without opening raw JSON
- wasm Python automation can navigate review surfaces through stable selectors

## Trust Levels

Kyuubiki should use explicit trust labels:

- `screening_only`
  suitable for early ranking and workflow development
- `review_ready`
  suitable for internal engineering review with documented assumptions and
  baselines
- `qualification_candidate`
  suitable for deeper validation, external comparison, and limited partner use
- `production_qualified`
  not expected in `1.x`; requires mature solver evidence, process controls,
  and documented domain limits

## Near-Term Priority

The next practical sequence should be:

1. add reliability envelopes to the remaining material report families
2. define `schemas/material-card.schema.json`
3. make heat-spreader use material-card references instead of only inline
   candidate constants
4. add one mesh/convergence record for heat-spreader thermal screening
5. add one external or analytic baseline id into the heat-spreader report
6. expose reliability envelope summaries in the Workbench material study view

This keeps progress close to a real end-to-end material workflow instead of
spreading effort across too many speculative solver families at once.

Current implementation note:

- `schemas/material-card.schema.json` now defines the first versioned
  material-card contract.
- `transform.validate_material_card` now provides workflow-level preflight for
  material-card provenance, unit expectations, confidence, parameter presence,
  temperature scope, and reliability-envelope output.
