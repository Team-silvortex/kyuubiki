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
