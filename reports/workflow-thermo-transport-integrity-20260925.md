# Thermo-Mechanical and Transport Diagnostic Integrity, 2026-09-25

## Scope

This is bounded local thermo-mechanical and transport diagnostic integrity evidence, not complete extraction or industrial physics qualification.

This follows the [thermal/electromagnetic diagnostic audit](workflow-diagnostic-integrity-20260924.md).
The public paths are `extract.thermo_result_diagnostics` and
`extract.transport_result_diagnostics`, followed by registered thermal/transport
quality scorers and the composite objective. Solver equations, scheduling,
runtime deployment and external SDK interfaces are unchanged.

## Reproduction

On the old implementation, the initial 33-test suite produced 27 failures and
six passing controls: 15/18 thermo tests and 12/15 transport tests failed.
These are overlapping regression categories, not 27 independent solver defects.

- Non-object, missing, malformed and partial samples were silently filtered.
- Metadata-only payloads and explicit nonconvergence could become diagnostics.
- Stress aliases were selected globally, ignoring elements using another alias.
- Bad/partial stress and strain could fall back to healthy summaries/components.
- Missing custom field mappings and invalid configuration silently disappeared.
- Missing transport concentration/source groups produced invented zero values.
- Selected invalid transport aliases could fall through to healthy alternatives.
- Squared vector norms overflowed/underflowed even for representable results.
- Required sums/spans could become null, while concentration means unnecessarily
  required the unexported raw sum to remain finite.

Two additional positive-control tests cover mixed transport aliases over
1/2/3/64/257 samples and thermo temperature-mapping/null-identity precedence.
The final new boundary suite contains 35 tests, plus six new workflow tests.

## Contracts

Both operators reject explicit failed/malformed convergence markers before
reduction. Missing convergence markers retain the existing admission policy;
their absence is not proof of convergence. Source collections must be arrays
and records objects. An optional group can be wholly absent, but if it is
present, every source row must provide a finite selected sample. There must be
at least one actual measurement group, not just metadata/counts.

Thermo uses the shared checked-sample reducer. Its wrapper still owns field
selection, fallback ordering, domain identity and strain semantics:

- The explicit temperature-delta mapping takes precedence over the legacy
  temperature mapping. Invalid selected configuration is an error.
- Stress aliases `von_mises_stress`/`von_mises` are selected per record. Only
  a wholly absent default stress group can fall back to `max_stress`, then
  stress components. A selected corrupt or partial group never falls through.
- Explicit stress/strain mappings require that exact field. Default scalar
  strain precedes component fallback; every present component axis is checked
  over every element. Wholly absent axes remain optional.
- Component extrema compare absolute values but retain signed output, with
  existing `x`, `y`, `z`, `xy` ordering and last-record ties. They are not vector
  norms, equivalent strains or a reconstruction of arbitrary 3D strain tensors.
- Scalar peaks retain the missing-ID `unknown` alias; explicitly null IDs stay
  null. Vector IDs retain their previous semantics. Generic node/element counts
  are now also included alongside prefixed thermo counts.

Transport shares sample/alias validation but keeps its distinct reductions:

- Wholly absent concentration and source metrics are omitted. Source zero is
  valid only when measured; absent source keeps default quality and composite
  decisions blocked rather than becoming a favorable zero balance.
- Signed scalar fluxes keep precedence and absolute-magnitude peak ranking.
  Only absent scalar flux uses vector fallback, requiring both finite planar
  components. Selected invalid aliases cannot fall through to vector data.
- `hypot` preserves representable very large/small norms. Concentration uses a
  running mean without an unused raw sum. Published spans and source sums/means
  must remain finite; no successful nonfinite-to-JSON-null result is returned.
- The literal trimmed prefix, optional peak IDs and signed tie behavior remain
  unchanged. Transport does not adopt the thermo prefix-normalization rule.

Reductions scan borrowed records with bounded auxiliary state, without vectors
of all numeric samples. This is not a measured performance improvement claim.
The shared workflow test builder lives in `tests/support/diagnostic_chain.rs`;
domain fixtures and assertions remain separate rather than duplicating graphs.

## Workflow Evidence

Six new integration tests exercise 14 malformed/partial/failed/overflow cases,
fail-fast behavior, explicit branch recovery, corrected-data replay and missing
metric propagation to the final composite decision. Recovery retains raw input
and independent output while suppressing diagnostic, quality and objective
artifacts for the failed branch. These are in-process re-executions, not durable
checkpoint or process-restart recovery.

Three real solver chains cover thermal plane triangles, thermal plane quads and
advection-diffusion bars. Checks compare public raw results with diagnostic
counts, signed flux extrema and stress/strain outputs. The restrained uniform
thermal patch also checks the plane-stress magnitude `E * alpha * deltaT /
(1 - nu)` at relative tolerance `1e-12`. Corrupting each real result blocks its
quality branch; replaying the intact result restores readiness. This small
fixture is not a new general solver qualification.

The old transport vector test now uses its existing numerical tolerance helper:
stable `hypot(0.84, 1.12)` differs from the old square/sqrt result by one rounding
step. Exact identity/sign/count checks remain exact.

```text
./scripts/kyuubiki check-operator-validation --execute --profile workflow-thermo-transport-integrity --out tmp/workflow-thermo-transport-integrity.json
```

Completed local macOS ARM64 checks:

| Check | Result |
| --- | --- |
| Native profile, five commands / eight suites | 118 passed: 19 thermo + 23 transport + 51 diagnostic + 8 thermal guard + 17 workflow |
| New boundary suites | 35 passed: 19 thermo, 16 transport |
| New workflow suite | Six passed |
| Existing thermal/electromagnetic workflow suite after shared-builder refactor | Five passed |
| Engine library | 594 passed, one existing ignored |
| Headless SDK / protocol / solver libraries | 270 / 108 / 355 passed; six existing solver ignores |
| Four core libraries combined | 1,327 passed, seven existing ignored |
| Release-mode diagnostic, thermo/transport, metric and admission workflows | 17 passed |
| Engine all-target Clippy with warnings denied | Passed |
| Changed Rust formatting and whitespace checks | Passed |
| Operator-profile, tensor, organization and documentation gates | Passed; 52 profiles validated |

Counts overlap and are not coverage percentages. The scoped tensor claim is
verified local evidence, not qualified deployment or general physical evidence.
The tensor retains zero structural gaps, four maturity gaps, 16 evidence-grade
gaps and 11 P0 gaps; overall qualification remains blocked. The organization
audit reports zero source/document size-limit debt (800/2000 lines).

## Remaining Limits

- No remote Agent, installed App, process-kill, durable restart or large-mesh run.
- Generic field statistics/hotspots, other domain extraction and diagnostic
  bundles still require their own completeness and provenance audits.
- Valid higher-precedence scalars are not independently reconciled with unused
  components, aliases or summaries. No units/material-data certification claim.
- Existing signed stress/strain/source scoring conventions and quality weights,
  targets and cutoffs are unchanged and are not qualified by these checks.
- Ordinary required sums conservatively reject intermediate overflow, even if
  later cancellation could restore a finite result. No compensated-summation
  accuracy, interval bound or bitwise-equivalence claim for the running mean.
- Unconfigured displacement dimensions and unselected strain/shear aliases are
  not inferred; this round does not extend 3D tensor semantics.
- No version promotion, commit, push, App rebuild or installation in this round.

The later [field/bundle audit](workflow-field-bundle-integrity-20260925.md)
addresses the generic Rust extraction/bundle contracts separately. Its evidence
does not cover the still-separate Elixir implementations or report provenance.
