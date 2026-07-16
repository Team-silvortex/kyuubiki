# Physics Coverage Map

This document defines the active `moxi 2.x` physics-coverage baseline.

The goal is breadth with honest limits, not final numerical authority. The
coverage carried forward from `tamamono 1.x` now prepares the `moxi 2.x` work
on engine boundaries, executable task files, operator descriptors, and durable
workflow contracts.

## moxi 2.x Rule

Every major simulation family should have at least one runnable path through
the Rust solver and benchmark stack.

That path may still be screening-grade, simplified, or fixture-sized, but it
must be:

- visible in the benchmark catalog
- tied to a named solver or workflow family
- mapped to a headless workflow solve operator
- runnable without hidden frontend-only behavior
- honest about whether it is smoke coverage, baseline coverage, or validated
  engineering evidence

## Current Coverage Families

The `physics-coverage` benchmark matrix is the broad execution lane for this
work. The reliability evidence index for that matrix starts at
`config/operator-reliability-manifest.json`, loads per-domain shards from
`config/operator-reliability/`, and is guarded by
`make check-operator-reliability`.

For `moxi 2.x`, every covered solve operator now carries at least `review`
evidence. The manifest's `minimum_coverage_level` is `review`, so the guard
fails if a covered solver is accidentally downgraded to `baseline` or `smoke`.

It covers:

- structural mechanics: axial bar, spring 1D/2D/3D, nonlinear spring, contact
  gap, beam, truss, frame, solid tetra, plane triangle, and plane quad
- thermal and heat transfer: thermal bar, heat bar, heat plane triangle, heat
  plane quad, thermal beam, thermal truss, thermal plane, and thermal frame
- electrostatics: 1D bar plus 2D triangle and quad plane fields
- magnetostatics: 1D bar plus 2D triangle and quad plane fields
- transport: 1D advection-diffusion scalar concentration flow
- acoustics: 1D acoustic bar
- simplified CFD: Stokes flow plane quad and triangle
- modal dynamics: modal frame 2D and 3D
- coupled thermo-structural fixtures: thermal truss, thermal plane, thermal
  beam, and thermal frame families

Run it locally at fixture scale:

```bash
cargo run --release -q -p kyuubiki-benchmark -- --profile medium --matrix physics-coverage --repeat 1
```

The matrix is not a claim that every family is industrial-grade. It is a
contract that each family has a real execution path that future engine and task
format work must preserve.

## Why This Comes Before The 2.x Task Format Freeze

The executable task format should not be designed around only one or two
favorite solvers.

Before the engine-facing task shape is frozen, it needs examples from:

- scalar field solves
- vector or displacement solves
- coupled field-to-structure flows
- modal or eigen-style result families
- nonlinear or failure-classified solves
- fluid-like field solves
- transport and concentration-field solves
- material-study and optimization workflows

The transport line now also includes a headless-safe diagnostics and decision
path:

- `extract.transport_result_diagnostics` turns advection-diffusion results into
  summary metrics for concentration span, source totals, peak flux, and
  Peclet-number review; compact solver names such as `c`, `source_density`,
  `flux_x`/`flux_y`, and `peclet` are normalized into those diagnostics
- `transform.evaluate_transport_guard` evaluates those metrics against visible
  warn/block threshold rules
- `transform.benchmark_transport_pair` compares two transport candidates with
  weighted min/max criteria for optimization-oriented workflows
- `transform.score_transport_quality` scores one candidate across peak flux,
  Peclet number, concentration span, and source balance, giving headless SDK
  studies a stable optimization objective with enabled-term filtering,
  watch/missing counts, dominant-term explanation, and blocking-term summaries;
  solver aliases such as `peak_transport_flux`, `peclet_max`,
  `concentration_min`/`concentration_max`, and `source_balance` are normalized
  into the same contract

The electromagnetic line has the same decision posture for electrostatics and
magnetostatics: diagnostics can be checked by field/energy threshold guards and
paired candidates can be compared before a workflow commits to the next solve
or bridge step. Electrostatic quality can also normalize solver aliases such as
`max_electric_field`, `peak_energy_density`, and `potential_min`/`potential_max`
into the canonical field, energy-density, and potential-span terms.
Magnetostatic quality does the same for `max_magnetic_field_strength`,
`peak_flux_density`, `peak_energy_density`, and `total_current_density`.

The structural line now has a matching headless decision path too:
`transform.evaluate_structural_guard` checks displacement, stress, force,
contact, or stiffness metrics against visible rules, while
`transform.benchmark_structural_pair` compares structural candidates and
`transform.score_structural_quality` turns a single displacement/stress/mass
response into an optimization-ready quality score before a material-study
workflow promotes one design. The score result also exposes
`structural_quality_dominant_term`, `structural_quality_watch_count`, and
`structural_quality_blocking_terms`, so headless studies can explain why a
candidate needs another iteration without parsing every term manually. Solver
aliases such as `von_mises_peak`, `peak_displacement`, `total_mass`, and
`minimum_stiffness_margin` are normalized into the same quality contract.

The thermal line has guard, pairwise benchmark, and single-candidate quality
coverage through `transform.evaluate_thermal_guard`,
`transform.benchmark_coupled_heat_pair`, and
`transform.score_thermal_quality`, so heat and thermo-mechanical branches can
gate, compare, or rank designs by temperature, flux, temperature-delta, and
stress objectives. Thermal scores now include dominant, watch-count, and
blocking-term fields for automated recovery and next-step selection. Solver
aliases such as `temperature_min`/`temperature_max`, `peak_heat_flux`,
`thermal_stress_peak`, and `thermal_energy_total` are normalized too.

The electrostatic line now mirrors that optimization shape:
`transform.evaluate_electrostatic_guard`,
`transform.benchmark_electrostatic_pair`, and
`transform.score_electrostatic_quality` cover peak field, energy-density, and
potential-span objectives before downstream heat or material branches consume
the result. Electrostatic quality results expose the same dominant, watch, and
blocking explanation fields used by the other workflow-facing quality scorers.
Electrostatic diagnostics normalize compact names such as `phi`, `rho_e`,
`e_x`/`e_y`, and `electric_energy_density`.

The magnetostatic line follows the same contract through
`transform.evaluate_magnetostatic_guard`,
`transform.benchmark_magnetostatic_pair`, and
`transform.score_magnetostatic_quality`, covering peak magnetic field, flux,
energy-density, and current-density objectives. Magnetostatic quality now also
reports watch, dominant-term, and blocking-term explanations. Magnetostatic
diagnostics normalize compact field names such as `a`, `j`, `h_x`/`h_y`,
`b_x`/`b_y`, and `energy_density`.

The acoustic line can now do the same for frequency-domain duct studies:
`transform.evaluate_acoustic_guard` checks SPL, pressure, velocity, intensity,
or damping limits, `transform.benchmark_acoustic_pair` compares candidate
acoustic responses, and `transform.score_acoustic_quality` turns one response
into an optimization-ready acoustic quality score with watch/missing counts,
dominant-term explanation, and blocking-term summaries. Solver aliases such as
`peak_spl_db`, `peak_acoustic_intensity`, `pressure_amplitude_peak`, and
`damping_loss_total` are normalized into that same score contract.

The modal line is covered by `transform.evaluate_modal_guard`,
`transform.benchmark_modal_pair`, and `transform.score_modal_quality`, so
vibration studies can gate, compare, or rank candidate designs by
natural-frequency band, mass, period, and participation metrics before a design
is promoted. The modal quality score also reports watch/missing counts,
dominant terms, and blocking terms for automated review loops. Solver modes can
provide `frequency_hz` and `participation_norm` directly, while aliases such as
`modal_mass_total` are normalized into the same quality fields.

The dynamic line extends that optimization path to harmonic and transient
spring responses through `transform.score_dynamic_quality`. It reports peak
frequency, velocity, acceleration, watch/missing counts, dominant terms, and
blocking terms so automated sweeps can explain why a dynamic candidate is held.
Harmonic response entries can use aliases such as `freq_hz`,
`displacement_amplitude`, `acceleration_amplitude`, and `force_amplitude`.

Across domains, `transform.compose_quality_objective` combines these
single-domain quality scores into one weighted multiphysics objective. That
gives material exploration flows a stable loss-like scalar while preserving
per-domain ready state, missing metrics, watch counts, dominant risk terms, and
blocking explanations. `transform.rank_quality_candidates` and
`transform.prepare_quality_next_round_request` carry those explanation fields
forward, so an automated study can select the best candidate and still know
which physics metric should drive the next iteration. The next-round request
also emits an `optimization_hint` that normalizes the action into either
`fix_blocking_term` or `reduce_dominant_term`, with the focus domain and field
ready for headless SDK search loops. `transform.build_quality_parameter_sweep_plan`
consumes that hint when ordering sweep axes, and can keep the focused axis when
`max_axes` is used to cap the next round size. The plan also emits
`sweep_budget`, making axis truncation and `max_cases` pressure visible before
the expansion operator has to reject an oversized study. Its recommendation
field lets SDK callers decide whether to run the planned sweep, reduce axis
count, reduce samples per axis, or schedule a follow-up axis batch.
When the plan is materialized, that same hint and focused axis are copied into
each generated case's metadata, preserving a trace from result quality to the
next simulated candidate. Materialization also declares `expansion_budget_ready`,
so UI and SDK callers can stop before expanding an oversized plan while still
keeping the full recovery context. If a quality sweep still reaches expansion
while budget-blocked, `transform.expand_parameter_sweep` returns a structured
empty result with the blocking reason and budget recommendation instead of a
generic failure. Sweep result scoring maps that metadata back onto quality
candidates and emits dominant/blocking quality terms from the objective
breakdown. Candidate ranking and the next-round request preserve the
selected candidate metadata as `seed_metadata`, so the reason chain remains
visible after ranking, planning, and expansion. Sweep planning carries that
`seed_metadata` into the materialized case metadata, so generated candidates can
be traced back to both the selected seed and the optimization reason.
`transform.compose_quality_lineage_report` can then summarize ranking, request,
plan, and expanded case metadata into one machine-readable lineage report for
headless SDK logs or later recovery. It also emits `lineage_missing_fields`,
allowing watchdogs, retry loops, and SDK callers to distinguish a complete
research chain from a recoverable partial chain. Budget-blocked expansion
results are treated as recoverable complete states when the selected seed,
optimization hint, and budget recommendation are still present. The
parameter-sweep result scoring graph now emits that report as a first-class
output next to the next-round cases.
The Rust engine and Web/Elixir workflow runtime both expose these domain
quality score transforms, so GUI-driven workflows and headless SDK paths can
share the same optimization contract instead of depending on UI-only logic.

That variety is what prevents operator SDK work from hard-coding a single
physics family, and it prevents executable task files from becoming too narrow.

## Coverage Levels

Use these labels consistently:

- `smoke`
  the solver path runs and returns a structured result
- `baseline`
  the family has benchmark or accuracy expectations that can catch regression
- `review`
  the result carries enough metadata, assumptions, and diagnostics for human
  review
- `qualification`
  the family has external validation, convergence posture, and documented
  limits suitable for serious engineering claims

The current reliability pass requires all covered solve operators to reach at
least `review`. New experimental families may still start as `smoke` or
`baseline` outside this release gate, but they should not be added to the
release-gated `physics-coverage` manifest until their evidence satisfies the
declared minimum.

The current machine-readable manifest should be treated as the source of truth
for per-operator trust level. A solver family can only move from `smoke` to
`baseline`, `review`, or `qualification` when the manifest points to concrete
benchmark, test, validation, and limitation evidence.

## Exit Criteria

The physics-coverage baseline is ready to hand off to the 2.x contract work
when:

- the `physics-coverage` matrix runs all built-in benchmark templates
- each `physics-coverage` solver family has a matching headless workflow solve
  operator
- each `physics-coverage` benchmark payload can execute through that headless
  workflow solve operator
- missing template references fail loudly instead of silently shrinking coverage
- solver families are grouped by physics domain in docs and benchmark names
- material studies can point to the solver families they depend on
- TaskIR and executable task design have examples from all major coverage
  classes, not only mechanical fixtures
- `make check-operator-reliability` reports `38 operators, review=5, qualification=33`
