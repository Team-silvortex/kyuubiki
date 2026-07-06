# Operator Reliability

Kyuubiki now keeps operator reliability as a machine-readable contract instead
of a loose project memory item.

The source of truth is:

- `config/operator-reliability-manifest.json`
- `config/operator-reliability/*.json`
- `config/operator-qualification-roadmap.json`
- `config/operator-qualification-evidence-kits.json`
- `schemas/operator-reliability-manifest.schema.json`
- `schemas/operator-reliability-shard.schema.json`
- `schemas/operator-qualification-roadmap.schema.json`
- `schemas/operator-qualification-evidence-kits.schema.json`
- `make check-operator-reliability`

The manifest index maps release-level metadata to per-domain shards. Each shard
maps its `physics-coverage` solve operators to:

- its benchmark template
- its physics domain
- its current trust level
- its headless workflow evidence
- its test or accuracy-baseline evidence
- its explicit limitations

For `tamamono 1.15.x`, the manifest also declares
`minimum_coverage_level: review`. `make check-operator-reliability` treats this
as a release gate, so future edits cannot silently downgrade a covered operator
back to `baseline` or `smoke`. The Make target runs the checker self-test first
to keep the trust-level ordering and release-gate behavior from regressing.

The shard layout keeps each domain contract below the project source-size limit
while preserving one release-level verification command.

The optional qualification roadmap lives at
`config/operator-qualification-roadmap.json`. The same checker validates that
roadmap candidates reference existing manifest operators and already satisfy
the roadmap's minimum candidate level.

The optional qualification evidence kits live at
`config/operator-qualification-evidence-kits.json`. They are deliberately
planning-grade: they list the artifacts that must be collected before a
roadmap candidate can be promoted into real `evidence.qualification` manifest
entries. The checker keeps every kit tied to an existing roadmap candidate and
prevents operators from drifting into the wrong qualification group.

## Current State

The first `tamamono 1.15.x` manifest covers all 36 solve operators in the
`physics-coverage` benchmark matrix, with a release gate requiring at least
`review` evidence for every covered operator.

Current level distribution:

- `baseline`: 0 operators
- `smoke`: 0 operators
- `review`: 36 operators
- `qualification`: 0 operators

This is intentionally conservative. The platform has broad executable
coverage, but most evidence is still regression-oriented rather than
engineering-qualification evidence.

## Smoke-Level Gaps

There are currently no smoke-only operators in `physics-coverage`.

## Upgrade Rules

Do not raise a solver trust level by editing the label alone.

To move from `smoke` to `baseline`, the manifest should point to at least one
of:

- an accuracy-baseline test with an explicit baseline function name
- a focused reliability test suite
- a benchmark profile that can catch performance or result-shape regressions

To move from `baseline` to `review`, add:

- documented assumptions and limitations
- mesh, geometry, boundary, or material preflight evidence where relevant
- tolerances or comparison criteria that are meaningful to the domain
- reportable diagnostics that a human reviewer can inspect
- a `review` evidence block in the manifest with assumptions, boundary checks,
  diagnostics, and focused tests

To move from `review` to `qualification`, add:

- external-tool, literature, analytic, or convergence evidence
- versioned baseline provenance
- release-blocking regression checks
- a documented scope of validity
- an `evidence.qualification` block with validation sources, convergence
  checks, provenance, release gates, and focused tests

`production_qualified` is deliberately outside the `1.x` target. That level
requires process controls and domain-specific validation that should not be
implied by the current screening and baseline stack.

## Near-Term Push

The most useful next upgrades are:

- harden selected review operators toward `qualification` with external,
  convergence, or literature-backed evidence
- add mesh, boundary, and material-assumption evidence where review coverage is
  still based on compact screening fixtures
- add a second Stokes-flow fixture that checks a different boundary response
  shape before any CFD-facing claim moves beyond screening
- keep `qualification` empty until external, convergence, or literature
  evidence exists
