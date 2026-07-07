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

By default both commands use:

- output: `tmp/material-research-example.json`
- study: `heat-spreader`
- runner: `workers/rust` `kyuubiki-material-explore`

The output is intentionally under `tmp/` and should not be committed directly.

## Closed-Loop Step

The captured exploration includes:

```text
kyuubiki.material-exploration-next-round/v1
```

This `next_round` block is the first closed-loop research contract. If the
report has missing metrics or violated quality gates, it returns a
`repair_or_rerun` decision with actions such as `rerun_incomplete_candidates`.
If the current screening data is complete, it returns `expand_around_winner`
with actions such as `generate_neighbor_candidates` and
`run_next_quality_batch`.

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

For `repair_or_rerun`, the plan emits only focused candidate solve steps. For
`expand_around_winner`, the current v1 implementation emits the built-in study
candidate generator as the next executable batch shape; future iterations can
replace that generator with DOE or Bayesian neighbor generation.

The CLI can also execute that next-round plan locally and emit a fresh
exploration artifact:

```sh
kyuubiki-material-explore --run-next tmp/material-research-example.json --json
```

This keeps the current prototype honest: the closed-loop block is not only a
recommendation for an agent, it can already drive the next solver batch through
the same material exploration contract.

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
quality gates and focus candidates to the top level. When repair is required,
`repair_plan` lists concrete actions such as inspecting failed gates, rerunning
focused candidates, resolving warnings, and rebuilding the report before
expansion. This is intentionally still small: it gives agents and CI a stable
lineage envelope before a heavier optimizer is added.

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
