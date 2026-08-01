# Automated Material Research Example

This document defines the first small but real automated material research
example for Kyuubiki.

Machine-readable expectations live in
[automated-material-research-example.manifest.json](automated-material-research-example.manifest.json).

It is deliberately modest: a heat-spreader screening study with three material
candidates. The value is not the sophistication of the fixture. The value is
that the whole loop is executable, machine-checkable, and explicit about its
limitations.

## Goal

Run a repeatable local material exploration that:

- uses real Rust solver kernels
- ranks multiple candidates with an optimization profile
- retains solver result payloads and report metrics
- emits an explicit next-round plan for repair/rerun or candidate expansion
- exposes reliability posture, assumptions, limitations, and quality gates
- fails if the output shape or expected winner drifts unexpectedly

## Command

Capture the example:

```sh
make capture-material-research-example
```

Verify the captured output:

```sh
make check-material-research-example
```

Run both steps as one regression target:

```sh
make verify-material-research-example
```

Build a retained research bundle with the initial exploration, next-round
execution plan, next exploration, chained rounds, checksums, and reproducible
commands:

```sh
make verify-material-research-bundle
```

By default both commands use:

- output: `tmp/material-research-example.json`
- bundle output: `tmp/material-research-bundle.json`
- study: `heat-spreader`; bundle generation also supports
  `STUDY=composite-thermo-electric-panel`
- runner: `workers/rust` `kyuubiki-material-explore` reference runner

The output is intentionally under `tmp/` and should not be committed directly.
The runner is only the first packaged executable for this example. The durable
interface is the material exploration contract, so Python, Elixir, remote
agent, mesh, or custom lab wrappers can reproduce the same loop without using
this exact CLI.

## Research Bundle

The bundle uses:

```text
kyuubiki.material-research-bundle/v1
```

The shared contract lives at
[schemas/material-research-bundle.schema.json](../schemas/material-research-bundle.schema.json),
with a compact fixture in
[schemas/examples.material-research-bundle.json](../schemas/examples.material-research-bundle.json).
Run `make check-material-research-bundle-contract` when changing the retained
bundle shape without regenerating solver output.

It is the first compact research-prototype artifact meant to be handed to an
agent, CI lane, or human reviewer. It contains:

- `initial_exploration`
- `next_round_execution_plan`
- `next_exploration`
- `chain`
- SHA-256 checksums for each retained artifact
- command templates for reproducing the same loop
- `execution_trace.authority`, which proves that the initial run, next run, and
  every retained chain round used real Rust solver kernels without mock or
  fallback execution
- a summary with winner, reliability decision, next-round decision, chain stop
  reason, convergence state, next iteration, and runnable next-step count
- `research_evidence`, a compact machine-checkable index of ranked candidates,
  optimization metrics, violated quality gates, focus candidates, plan step
  count, chain round count, and final chain winner
- `validation_evidence`, a screening-level validation envelope that records
  retained baseline references, material-card confidence counts, sensitivity
  proxy metrics, acceptance criteria, uncertainty limits, validation-readiness
  decision, and the required external validation plan

The retained bundle checker cross-validates that `summary.next_round_decision`,
`summary.next_iteration`, `summary.runnable_next_step_count`, and
`summary.chain_stop_reason` match the embedded execution plan, next exploration,
and chain artifacts. It also verifies that `research_evidence` keeps the winner,
ranked candidate set, focus candidates, quality-gate decision, plan decision,
step count, and chain trace count aligned with the retained artifacts. This
keeps the single-file story honest when an agent or reviewer reads only the
top-level summary first.

Execution authority and numerical qualification are intentionally separate.
`execution_trace.authority.assertions` must report `all_real_solver`,
`no_mock_execution`, and `no_fallback` as true or the bundle is rejected. The
bundle can still remain `screening_research_bundle` because real computation
does not by itself provide external calibration, high-confidence material
cards, coupled-field mesh convergence, or qualification-grade physics evidence.

The `validation_evidence` block is deliberately conservative. It does not claim
experimental qualification. It records that the current retained loop is a
screening validation artifact, points to the built-in deterministic baseline,
mirrors the violated quality gates as acceptance criteria, and keeps the
external validation plan visible next to the results. That gives agents a
machine-readable reason to continue with sensitivity and calibration work before
any qualification claim.

The composite thermo-electric profile also retains an independent
layered-dielectric closed-form cross-check. It derives the expected electric
field from displacement continuity across the three dielectric regions and
compares that result with the FEM maximum field for every candidate. The
`analytic_closed_form` baseline is emitted only when every candidate passes the
`1e-9` relative-error gate. This is an independent formulation check, not an
external-tool or experimental calibration, so it does not remove the
`external_validation_required` blocker.

The same profile executes real structured-quad electrostatic refinements at
`1`, `2`, `4`, and `8` elements per material layer. It retains each maximum
field, the error against the closed form, and the relative change between mesh
levels. A `mesh_convergence` baseline is promoted only when all candidates
contain all four solver runs and pass both the analytic-error and finest-pair
stability gates. This closes the electrostatic subproblem convergence loop; by
itself it does not qualify the remaining fields or their coupling.

The heat subproblem follows the same protocol. Its independent baseline uses
the downstream thermal resistance from the loaded conductor/dielectric
interface to the fixed-temperature edge and predicts `115.125 C` for the
retained fixture. Real `1/2/4/8` heat-quad refinements must pass both the
closed-form and mesh-stability gates before the bundle emits heat
`analytic_closed_form` and `mesh_convergence` baselines.

The thermal-structural subproblem now also runs real two-dimensional
`1/2/4/8` structured-quad refinement while preserving the original
piecewise-linear temperature field and each material layer. Its pass decision
uses maximum displacement and total strain energy; peak von Mises stress is
retained only as a diagnostic because fixed-edge and material-interface corners
can be singular. The retained candidates currently fail this gate: finest-pair
changes are about `2.4-2.6%` for displacement and `27.5-27.6%` for strain
energy, while peak stress changes by about `21.5%`. This is an intentional
blocker, not a relaxed baseline. Boundary regularization, interface mechanics,
local refinement, and coupled-field convergence remain required before
structural qualification.

A second diagnostic solve changes the full vertical clamp into a horizontal
roller edge with one vertical anchor. It reduces the finest-pair displacement
change to `1.21-1.25%` and the peak-stress change to about `13.5%`, but the
strain-energy change remains `30.6-30.8%`. The emitted
`kyuubiki.composite-thermal-constraint-sensitivity/v1` record therefore reports
`mixed_restraint_sensitivity_and_persistent_energy_nonconvergence`. Its
qualification effect is explicitly diagnostic-only and cannot override the
primary structural quality gates.

The same retained meshes also feed an area-weighted stress-recovery check.
It tracks von Mises RMS and P95 as pass metrics while keeping the raw maximum
and `max/P95` concentration ratio diagnostic-only. The current candidates still
fail: finest-pair RMS changes by about `1.07%`, P95 changes by about `24.4%`,
and the finest-mesh maximum is about `1.97` times P95. Nonconvergence therefore
extends beyond one isolated peak element. The result adds two stress-recovery
gates under `gate.thermal_stress_recovery.*` and points the next iteration
toward local refinement, higher-order recovery, or an independently correlated
structural formulation. RMS clears its gate, while P95 remains blocking.

An interface-graded companion run clusters the same element budgets at the
clamp, material interfaces, and free edges. In the current solver baseline it
cuts finest-pair RMS drift from about `1.07%` to `0.16%`, but P95 drift rises
from about `24.4%` to `32.5%`, strain-energy drift remains near `14.5%`, and
raw-maximum drift rises from about `27.4%` to `43.9%`. The machine-readable
diagnosis is therefore
`graded_mesh_did_not_resolve_nonconvergence`. The graded run remains useful
diagnostic evidence, but cannot override the uniform-mesh gates.

The four-level histories now also receive an observed-order and Grid
Convergence Index assessment. Displacement remains monotonic but
pre-asymptotic: its coarse-triplet order is about `0.54-0.60`, while the
fine-triplet order rises to `1.44-1.54`, so no Richardson value or GCI is
claimed. Strain energy is asymptotically consistent at orders near
`1.31-1.39`, but its fine-grid GCI is still `29.4-29.7%` and therefore violates
the `2%` gate. Peak stress has negative observed order and is classified as
monotonically divergent. This distinction prevents a decreasing response from
being mistaken for mesh independence.

Every thermal-structural solve also retains an original-system algebraic
residual profile. Across three candidates, three mesh families, and four
refinement levels, all `36` solves pass the `1e-10` relative-residual gate; the
largest observed value is about `5.13e-14`. This separates a converged linear
solve from a converged physical discretization: algebraic convergence passes,
while the displacement, energy, and stress mesh gates remain blocked.

Its `validation_readiness` sub-block is intentionally a scheduling signal, not a
material score: it records `screening_only`, a bounded readiness score, blocking
reasons such as external-validation and low-confidence material cards, and the
next validation actions required before stronger claims.

The initial winner and final chain winner are intentionally separate fields.
For simple heat-spreader screening they may match; for broader coupled-material
search they can diverge as the next-round loop finds a stronger candidate. That
drift is evidence, not noise.

The checker rejects local absolute repository paths and checksum drift. This is
still a screening artifact, not a qualification package, but it is now a single
file that captures the whole minimal research story.

To build the second retained bundle profile for the electric-thermal-structural
composite panel loop:

```sh
STUDY=composite-thermo-electric-panel OUT=tmp/material-research-bundle-composite.json make verify-material-research-bundle
```

To build both retained bundle profiles and a compact index for agents or CI:

```sh
make material-research-bundle-index
```

To validate an existing retained bundle index without rebuilding the bundles:

```sh
make check-material-research-bundle-index
```

The index is written under `tmp/material-research-bundles/index.json` with a
matching `README.md` summary. Each index row also carries the next iteration and
runnable next-step count so CI lanes or agents can choose cheap repair runs
before expensive exploration. It now also lifts the compact research evidence:
initial winner, final chain winner, metric count, violated-gate count, focus
candidates, chain round count, and chain trace count. It also lifts compact
validation evidence: screening posture, external-validation requirement,
baseline count, acceptance-criteria count, candidate confidence counts,
readiness decision, readiness score, blocking reasons, next validation action
count, deterministic validation priority, and priority reasons. Agents can
triage drift, blocked quality gates, and validation maturity from the index
before opening the full retained bundle. The index also carries
`validation_priority_counts` so dashboards can show p0/p1/p2 repair pressure
without scanning every row.
The index checker verifies those counts and evidence summaries before the file
is used as a lightweight planning artifact.
The index shape is pinned by
[schemas/material-research-bundle-index.schema.json](../schemas/material-research-bundle-index.schema.json)
and
[schemas/examples.material-research-bundle-index.json](../schemas/examples.material-research-bundle-index.json).
It is a local generated artifact and should stay out of Git unless a release
explicitly promotes it.

## Closed-Loop Step

The captured exploration includes:

```text
kyuubiki.material-exploration-next-round/v1
```

This `next_round` block is the first closed-loop research contract. If the
report has missing metrics or violated quality gates, it returns a
`repair_or_rerun` decision with actions such as `rerun_incomplete_candidates`.
If summary cross-validation blocks the report, it returns `repair_validation`
and reruns focused candidates before any new material candidate is generated.
If the current screening data is complete, it returns `expand_around_winner`
with actions such as `generate_neighbor_candidates` and `run_next_quality_batch`.

Each exploration artifact also carries its current `iteration`. The first
captured run is iteration `1`, its `next_round.iteration` points to `2`, and a
local `--run-next` result becomes iteration `2` with a new next-round pointer to
iteration `3`.

The same CLI can turn a captured exploration into a runnable next-round plan:

```sh
kyuubiki-material-explore --plan-next tmp/material-research-example.json --json
```

The output uses:

```text
kyuubiki.material-exploration-next-round-execution/v1
```

For `repair_or_rerun` and `repair_validation`, the plan emits only focused
candidate solve steps. For `expand_around_winner`, the current v1 implementation
emits the built-in study candidate generator as the next executable batch
shape; future iterations can replace that generator with DOE or Bayesian
neighbor generation.

The next-round plan also carries `optimization_objectives`, which records the
optimization mode, incumbent winner, primary metric IDs, and violated quality
gates. This lets a headless harness decide whether it is repairing data,
mitigating risk, or expanding around the winner without parsing prose.

The CLI can also execute that next-round plan locally and emit a fresh
exploration artifact:

```sh
kyuubiki-material-explore --run-next tmp/material-research-example.json --json
```

This keeps the current prototype honest: the closed-loop block is not only a
recommendation for an agent, it can already drive the next solver batch through
the same material exploration contract. The emitted next exploration carries
`lineage`, recording the source iteration, source winner, decision, focused
candidates, runnable step count, and optimization objectives behind the rerun.

For smoke-testing a continuous loop, the CLI can chain several next-round runs:

```sh
kyuubiki-material-explore --chain-next tmp/material-research-example.json --rounds 2 --json
```

The chain wrapper uses:

```text
kyuubiki.material-exploration-chain/v1
```

It contains one full exploration artifact per requested round plus a final
iteration, final winner, decision counts, a `stop_reason`, winner stability,
one compact summary per round, and a `repair_summary` that lifts violated
quality gates and focus candidates to the top level. Each summary carries its
next-round `optimization_objectives`, while `optimization_trace` lifts the
per-round mode, primary metrics, winner, and violated gates into a compact
lineage view. `convergence_assessment` compares winner stability, winner score
drift, and repair state so a stable but gate-blocked candidate is not mistaken
for validation. When repair is required, `repair_plan` lists concrete actions
such as inspecting failed gates, rerunning focused candidates, resolving
warnings, and rebuilding the report before expansion. This is intentionally
still small: it gives agents and CI a stable lineage envelope before a heavier
optimizer is added.

## Remote Lab Run

Run the same example on the lab machine, then add a larger release benchmark:

```sh
make remote-material-research-example
```

The remote runner:

- syncs the current working tree to `.kyuubiki-remote-runs/material-research-example`
- excludes local build output, dependency folders, `tmp/`, and `.git/`
- runs `make verify-material-research-example` on the remote host
- runs the material exploration CLI tests
- runs `kyuubiki-benchmark` with `PROFILE=100k`, `MATRIX=compound-core`, and `REPEAT=1`
- pulls JSON evidence back under `tmp/remote-material-research/`

Override the scale without changing the script:

```sh
PROFILE=400k MATRIX=thermal-core REPEAT=1 make remote-material-research-example
```

The runner requires an existing SSH key or host config. It does not store
credentials, and `rsync --delete` is scoped to the dedicated remote scratch
directory only.

## Study

The example runs `material_heat_spreader_screening`.

Candidates:

- `aluminum_6061`
- `copper_c110`
- `pyrolytic_graphite_in_plane`

The solve path uses `solve_heat_plane_quad_2d` for every candidate. The ranking
report then combines solver outputs with material-card metrics.

## Optimization Contract

The expected optimization id is:

```text
material.heat_spreader_screening.optimization.v1
```

The current score combines:

- peak temperature, minimize, weight `0.55`
- areal mass, minimize, weight `0.30`
- conductivity-density ratio, maximize, weight `0.15`

The expected winner for the current fixture is:

```text
pyrolytic_graphite_in_plane
```

This winner is not a production material recommendation. It is a regression
anchor for the automated research loop.

## Reliability Posture

The report must keep:

- `reliability.posture: screening_only`
- candidate-level optimization terms
- solver result payloads for all three candidates
- next-round decision, focus candidates, actions, and rationale
- reliability quality gates
- visible limitations and assumptions
- no local absolute repository paths

This makes the example useful for automation and review without overstating
industrial qualification.

## Why This Matters

This example is the first practical bridge between:

- solver execution
- headless material study generation
- optimization metrics
- report reliability envelopes
- machine-checkable research artifacts

Future material studies should follow this shape before becoming more complex:
small reproducible fixture first, stronger geometry and evidence second,
qualification claim last.
