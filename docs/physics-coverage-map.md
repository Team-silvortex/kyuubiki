# Physics Coverage Map

This document defines the `tamamono 1.14.x` physics-coverage push.

The goal is breadth with honest limits, not final numerical authority. The
coverage created in this window prepares the `1.15.x` and `1.16.x` work on
engine boundaries, executable task files, operator descriptors, and durable
workflow contracts.

## 1.14.x Rule

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

The `physics-coverage` benchmark matrix is the broad smoke lane for this work.

It covers:

- structural mechanics: axial bar, spring 1D/2D/3D, nonlinear spring, contact
  gap, beam, truss, frame, plane triangle, and plane quad
- thermal and heat transfer: thermal bar, heat bar, heat plane triangle, heat
  plane quad, thermal beam, thermal truss, thermal plane, and thermal frame
- electrostatics: 1D bar plus 2D triangle and quad plane fields
- magnetostatics: 1D bar plus 2D triangle and quad plane fields
- transport: 1D advection-diffusion scalar concentration flow
- acoustics: 1D acoustic bar
- simplified CFD: Stokes flow plane quad
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

## Why This Comes Before 1.15.x And 1.16.x

The executable task format should not be designed around only one or two
favorite solvers.

By the end of the `1.14.x` window, the engine-facing task shape needs examples
from:

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
  Peclet-number review
- `transform.evaluate_transport_guard` evaluates those metrics against visible
  warn/block threshold rules
- `transform.benchmark_transport_pair` compares two transport candidates with
  weighted min/max criteria for optimization-oriented workflows
- `transform.score_transport_quality` scores one candidate across peak flux,
  Peclet number, concentration span, and source balance, giving headless SDK
  studies a stable optimization objective

The electromagnetic line has the same decision posture for electrostatics and
magnetostatics: diagnostics can be checked by field/energy threshold guards and
paired candidates can be compared before a workflow commits to the next solve
or bridge step.

The structural line now has a matching headless decision path too:
`transform.evaluate_structural_guard` checks displacement, stress, force,
contact, or stiffness metrics against visible rules, while
`transform.benchmark_structural_pair` compares structural candidates and
`transform.score_structural_quality` turns a single displacement/stress/mass
response into an optimization-ready quality score before a material-study
workflow promotes one design.

The thermal line has guard, pairwise benchmark, and single-candidate quality
coverage through `transform.evaluate_thermal_guard`,
`transform.benchmark_coupled_heat_pair`, and
`transform.score_thermal_quality`, so heat and thermo-mechanical branches can
gate, compare, or rank designs by temperature, flux, temperature-delta, and
stress objectives.

The electrostatic line now mirrors that optimization shape:
`transform.evaluate_electrostatic_guard`,
`transform.benchmark_electrostatic_pair`, and
`transform.score_electrostatic_quality` cover peak field, energy-density, and
potential-span objectives before downstream heat or material branches consume
the result.

The magnetostatic line follows the same contract through
`transform.evaluate_magnetostatic_guard`,
`transform.benchmark_magnetostatic_pair`, and
`transform.score_magnetostatic_quality`, covering peak magnetic field, flux,
energy-density, and current-density objectives.

The acoustic line can now do the same for frequency-domain duct studies:
`transform.evaluate_acoustic_guard` checks SPL, pressure, velocity, intensity,
or damping limits, `transform.benchmark_acoustic_pair` compares candidate
acoustic responses, and `transform.score_acoustic_quality` turns one response
into an optimization-ready acoustic quality score.

The modal line is covered by `transform.evaluate_modal_guard`,
`transform.benchmark_modal_pair`, and `transform.score_modal_quality`, so
vibration studies can gate, compare, or rank candidate designs by
natural-frequency band, mass, period, and participation metrics before a design
is promoted.

Across domains, `transform.compose_quality_objective` combines these
single-domain quality scores into one weighted multiphysics objective. That
gives material exploration flows a stable loss-like scalar while preserving
per-domain ready state, missing metrics, and contribution breakdowns.
The Rust engine and Web/Elixir workflow runtime both expose these domain
quality score transforms, so GUI-driven workflows and headless SDK paths can
share the same optimization contract instead of depending on UI-only logic.

That variety is what prevents `1.15.x` operator SDK work from hard-coding a
single physics family, and it prevents `1.16.x` executable task files from
becoming too narrow.

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

Most `1.14.x` additions should stop at `smoke` or `baseline`.

## Exit Criteria

`1.14.x` is ready to hand off to the `1.15.x` and `1.16.x` contract work when:

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
