# Operator Reliability

Kyuubiki now keeps operator reliability as a machine-readable contract instead
of a loose project memory item.

The source of truth is:

- `config/operator-reliability-manifest.json`
- `config/operator-reliability/*.json`
- `config/operator-validation-profiles.json`
- `config/operator-qualification-roadmap.json`
- `config/operator-qualification-evidence-kits.json`
- `schemas/operator-reliability-manifest.schema.json`
- `schemas/operator-reliability-shard.schema.json`
- `schemas/operator-qualification-roadmap.schema.json`
- `schemas/operator-qualification-evidence-kits.schema.json`
- `make check-operator-reliability`
- `make check-operator-validation`
- `make verify-operator-validation`

The manifest index maps release-level metadata to per-domain shards. Each shard
maps its `physics-coverage` solve operators to:

- its benchmark template
- its physics domain
- its current trust level
- its headless workflow evidence
- its test or accuracy-baseline evidence
- its explicit limitations

`config/operator-validation-profiles.json` is the first Operator Validation
Harness contract. It groups operators into validation profiles and records:

- analytic or cross-check methods
- local formal invariants
- evidence paths that must remain repo-relative
- commands that can execute the validation profile

The input profile shape is retained as
`schemas/operator-validation-profiles.schema.json`.

`make check-operator-validation` validates the profile contract and writes
`tmp/operator-validation-report.json` without running heavy commands.
`make verify-operator-validation` executes the declared commands and writes the
same report with command status and output tails. This is not a whole-system
formal proof; it is a practical lane for accumulating executable local
invariants and cross-validation evidence per operator family.
The report shape is retained as
`schemas/operator-validation-report.schema.json` with
`schemas/examples.operator-validation-report.json` as the fixture.

Validation commands use a small controlled kind vocabulary:

- `analytic`: closed-form or derived reference checks
- `cross_check`: independent implementation, shape, or representation checks
- `boundary_regression`: boundary-condition and edge-case regression checks
- `invariant`: local conservation, finiteness, or sign invariants
- `contract`: repository-level contract checks that support the profile

The first validation profiles cover:

- `line-field-closed-form`: 1D analytic closed-form checks and tolerance policy
- `stokes-flow-screening`: CFD Stokes screening boundary and tolerance checks
- `electrostatic-plane-patch`: triangle/quad constant-gradient electric field
  and stored-energy patch checks
- `heat-plane-patch`: triangle/quad temperature-gradient and heat-flux patch
  checks

For `tamamono 1.15.x`, the manifest also declares
`minimum_coverage_level: review`. `make check-operator-reliability` treats this
as a release gate, so future edits cannot silently downgrade a covered operator
back to `baseline` or `smoke`. The Make target runs the checker self-test first
to keep the trust-level ordering and release-gate behavior from regressing.

The shard layout keeps each domain contract below the project source-size limit
while preserving one release-level verification command.

The Rust engine now applies a workflow security preflight before executing a
graph. The guard rejects unsupported workflow schema versions, excessive graph
sizes, duplicate node or edge ids, malformed identifiers, unsupported
operators, invalid edge references, and input artifacts that target non-input
nodes. It also applies JSON security budgets to workflow node configs, workflow
dataset contracts, input artifacts, and output artifacts. This keeps GUI,
headless SDK, and agent/orchestra execution paths behind the same first safety
gate.

Workflow execution is still fail-fast by default. A non-core node can opt into
node-level recovery with `config.on_error: "skip"` or
`config.recovery.on_error: "skip"`; `fail` is also accepted as an explicit
fail-fast policy. Other recovery policy values are rejected during workflow
security preflight. When a recoverable node returns an error or panics, the run
records the node as `failed`, preserves its error message in `node_runs`, rolls
back artifacts written by the failed node, skips downstream nodes that cannot
resolve artifacts, and continues independent branches. This prevents
recoverable analysis/reporting failures from cascading across the whole graph
without hiding the failure from SDK or GUI callers.

The qualification roadmap lives at
`config/operator-qualification-roadmap.json`. The same checker validates that
roadmap candidates reference existing manifest operators and already satisfy
the roadmap's minimum candidate level.

The qualification evidence kits live at
`config/operator-qualification-evidence-kits.json`. They are deliberately
planning-grade: they list the artifacts that must be collected before a
roadmap candidate can be promoted into real `evidence.qualification` manifest
entries. The checker keeps every kit tied to an existing roadmap candidate and
prevents operators from drifting into the wrong qualification group.
`make build-operator-qualification-readiness` writes a local JSON report that
summarizes which roadmap artifacts are present, command-backed, missing, or not
started. The generated report also includes a `next_actions` queue so release
owners can see the highest-priority evidence collection step without manually
diffing every candidate kit. The make target validates the generated report so
the queue stays machine-consumable for release gates and future UI surfaces.

## CFD Stokes Screening Scope

`solve.stokes_flow_quad_2d` is a Stokes-only screening operator. It is meant to
exercise low-Reynolds-number velocity, pressure, divergence, and viscous
dissipation plumbing through the same headless workflow path as the other
physics operators. It is not a general Navier-Stokes solver, turbulence model,
compressible-flow solver, or industrial CFD validation claim.

The current review evidence is limited to compact rectangular quad fixtures:
one body-force response and one lid-driven shear boundary response. That is
enough to catch shape, boundary, diagnostic, and non-finite input regressions,
but not enough to claim mesh-converged CFD accuracy.

## CFD Stokes Divergence Tolerance

The screening divergence gate is `1e-10` for the current compact Stokes
fixtures. This tolerance is a regression guard for the single-quad arithmetic
path and boundary assembly, not a reusable engineering qualification tolerance.
If the operator graduates beyond screening, the tolerance must be replaced or
backed by retained convergence evidence, solver-version provenance, and a
documented scope of validity.

The machine-readable screening policy lives at
`evidence/operator-qualification/stokes-flow-screening-tolerance-policy.json`.
That artifact pins the current regression scope and explicitly blocks using
the same tolerance for Navier-Stokes, turbulence, compressible-flow, or
mesh-convergence claims.

The CFD quality transform also exposes review-facing explanation fields:
`cfd_quality_dominant_term`, `cfd_quality_watch_count`, and
`cfd_quality_blocking_terms`. These fields make headless material or flow
screening runs explainable: a candidate does not only receive a score, it also
reports which diagnostic term dominated the penalty and which missing or
out-of-target terms caused a block. The same transform also accepts
`enabled_terms` and common alias fields such as `max_divergence_error`,
`max_reynolds_number`, `total_viscous_dissipation`, `velocity_span`, and
`pressure_span` so retained screening studies can narrow their objective
without rewriting upstream diagnostics. Diagnostics also normalize compact CFD
post-processing names such as `vx`/`vy`, `p`, `div_u`, `reynolds`, and
`dissipation`, while quality scoring accepts aliases such as `div_u_peak`,
`re_peak`, `dissipation_total`, `speed_span`, and `p_span`.

## Electromagnetic Plane Review Scope

The 2D electrostatic and magnetostatic plane operators are review-grade
single-patch field checks. They verify that triangle and quad elements can
report gradients, field strength, flux density, and stored energy through the
same headless workflow contract. They do not yet claim mesh convergence,
rotated-patch orientation invariance, coupled high-frequency electromagnetics,
or production qualification.

The near-term qualification blocker is orientation evidence: the same field,
flux, and stored-energy checks must pass across at least two patch
orientations before these operators can graduate beyond review.

## Electromagnetic Plane Material And Energy Notes

The current fixtures assume positive scalar linear material parameters.
Electrostatic plane elements use permittivity; magnetostatic plane elements use
permeability. The stored energy diagnostics are regression evidence for this
linear material path, not a broad material-card validation claim.

Before qualification, these operators need material-card provenance for the
permittivity or permeability values and an energy-density tolerance derivation
that explains where the stored-energy comparison is valid.

## Thermal Plane Review Scope

The 2D heat-plane and thermoelastic-plane operators are review-grade compact
patch checks. Heat-plane fixtures exercise steady temperature gradients and
heat-flux diagnostics. Thermoelastic-plane fixtures exercise restrained
thermal strain, mechanical strain, stress, and von Mises diagnostics. They are
not yet mesh convergence, mixed-boundary coverage, or qualification evidence.

The `thermal-plane-patch` roadmap candidate must stay below qualification
until mesh convergence and boundary coverage artifacts are linked from the
operator reliability entry.

## Thermal Plane Material And Boundary Notes

The current thermal fixtures assume linear material behavior. Heat-plane
elements use positive scalar conductivity. Thermoelastic-plane elements use
linear plane stress plus positive elastic constants and thermal expansion
coefficients. These material values are fixture parameters, not material-card
provenance.

Before qualification, the thermal plane group needs material-card linked
benchmark fixtures, a boundary-condition coverage report, and a mesh
refinement report that explains where the conductivity, thermal expansion, and
stress tolerances are valid.

## Current State

The first `tamamono 1.15.x` manifest covers all 37 solve operators in the
`physics-coverage` benchmark matrix, with a release gate requiring at least
`review` evidence for every covered operator.

Current level distribution:

- `baseline`: 0 operators
- `smoke`: 0 operators
- `review`: 37 operators
- `qualification`: 0 operators

This is intentionally conservative. The platform has broad executable
coverage, but most evidence is still regression-oriented rather than
engineering-qualification evidence.

The CFD-facing Stokes quad operator is still `screening_only`, but it now has
two review fixtures: the original body-force response and a lid-driven shear
boundary response. That moves the `screening-cfd-boundary` evidence kit into
`collecting`, not `qualification`. The test suite now encodes a screening
divergence tolerance and the retained screening policy documents its current
scope, but the operator still needs mesh-convergence or external-reference
evidence before any stronger claim.

The first qualification evidence collection track is now active for
`line-field-closed-form`. Its versioned baseline artifact lives at
`evidence/operator-qualification/line-field-closed-form-baseline.json` and is
paired with
`evidence/operator-qualification/line-field-closed-form-derivation.md` plus
`evidence/operator-qualification/line-field-tolerance-policy.json`. These are
checked by `make check-line-field-closed-form-baseline`. This pins the
closed-form expected values, tolerances, and tolerance scope for `solve.bar_1d`,
`solve.thermal_bar_1d`, `solve.heat_bar_1d`, and
`solve.electrostatic_bar_1d`, but it is not a trust-level promotion by itself.
`make capture-line-field-qualification-provenance` can emit the release-time
revision, toolchain, platform, and input-hash envelope without adding local
machine paths to Git. `make capture-line-field-qualification-release-evidence`
runs the evidence checker and solver baseline and writes the release-retained
regression bundle. The remaining blocker is attaching that generated bundle to
the actual release record before any manifest entry becomes `qualification`.

`solve.solid_tetra_3d` is now part of `physics-coverage` through a dedicated
solid-tetra benchmark template and a solver-level review fixture for a
restrained single-tetra load path. It is still a screening fixture, not a
mesh-convergence or qualification claim.

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
- turn the `line-field-closed-form` baseline artifact into a complete
  qualification packet with derivation and provenance notes
- add mesh, boundary, and material-assumption evidence where review coverage is
  still based on compact screening fixtures
- expand Stokes-flow screening evidence beyond the second boundary-response
  fixture and screening tolerance policy into convergence or reference-tool
  evidence
- keep `qualification` empty until external, convergence, or literature
  evidence exists
