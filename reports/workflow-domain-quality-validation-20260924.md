# Domain Quality Validation, 2026-09-24

## Scope

This is bounded local domain-quality validation, not complete metric extraction, optimization or industrial physics qualification.

The retained public transform registrations cover structural, thermal, acoustic,
modal, dynamic, electrostatic, magnetostatic, CFD and transport quality scores.
This follows [result admission](workflow-result-admission-20260924.md) and
[guard/pair validation](workflow-guard-validation-20260924.md).

## Reproduced Failures

Before the implementation change, the initial 20-test regression suite had
17 failures and three passing positive controls. In particular, finite input
`1e308`, target `1` and weight `2` produced a non-finite penalty. JSON converted
that penalty to `null`; a later `filter_map` dropped it from the total. The
structural score returned `excellent` and `ready=true` for that invalid evidence.

Other reproduced boundaries included malformed or repeated selected metrics,
silent fallback/clamping of invalid targets, weights and readiness thresholds,
unknown override names, canonical metric corruption hidden by a healthy alias,
non-finite derived spans, total overflow and a hidden positive-target floor.
These are regression categories, not a count of independent physical defects.

## Changes

- Nine domain wrappers now share typed term evaluation and aggregation in
  `workers/rust/crates/engine/src/workflow_domain_quality.rs`.
- Domain metric names, defaults, optional terms, result fields and grade cutoffs
  remain in their existing wrappers. Solver equations and workflow scheduling
  are not coupled to this shared validation code.
- Omitted term lists use defaults. Explicit lists must be nonempty arrays of
  known, unique metric names. Names are trimmed before duplicate detection.
- The config container must be an object or null. Explicit target/weight groups
  must be objects; all override names and values are validated, including known
  inactive overrides. Common workflow options such as `on_error` remain valid.
- Targets must be finite and strictly positive; they are no longer raised to
  `1e-12`. Weights and ready limits must be finite and nonnegative. Omitted
  options retain defaults; present invalid options are errors.
- Zero weight stays supported and does not evaluate an unused ratio, but the
  selected metric must still be well-formed and present for readiness. A missing
  zero-weight metric still blocks the assessment.
- An explicit invalid canonical metric is an error, not a missing metric or an
  invitation to read a healthy alias. When canonical fields are absent, valid
  aliases and derived values remain supported. A derived non-finite value fails.
- Genuinely absent metrics retain the public missing/block result contract.
- Ratios, weighted penalties and accumulated totals must be finite before
  conversion to JSON. Numeric decisions no longer read values back out of JSON.
- Errors include the transform id and failing configuration/metric path. Default
  execution stops; explicit branch recovery preserves raw input and independent
  outputs but publishes neither a quality summary nor a composite objective.

## Validation

The dedicated unit suite has 22 tests, with table-driven checks across all nine
registrations and the five domains with inverse-goal terms. Four new graph tests
cover seven invalid-evidence cases, branch recovery, corrected-input replay and
missing-metric propagation to a blocking composite objective.

Reproduce the retained component profile through the native runner:

```text
./scripts/kyuubiki check-operator-validation --execute --profile workflow-domain-quality-validation --out tmp/workflow-domain-quality-validation.json
```

The profile also executes existing quality compatibility tests and previous
guard/result-admission recovery graphs. Results on local macOS ARM64:

| Check | Result |
| --- | --- |
| Native component profile, two commands / four suites | 151 passed: 141 unit + 10 workflow |
| Engine library | 511 passed, one existing ignored |
| Headless SDK / protocol / solver libraries | 270 / 108 / 355 passed; six existing solver ignores |
| Four core libraries combined | 1,244 passed, seven existing ignored |
| Release-mode quality, guard and admission workflow suites | 10 passed |
| Engine all-target Clippy with warnings denied | Passed |
| Changed engine Rust formatting and whitespace checks | Passed |
| Operator-profile, tensor, project organization and doc inventory gates | Passed |

Suite totals overlap; they are not additive coverage percentages. The tensor has
zero structural gaps but still four maturity gaps, 16 evidence-grade gaps and
11 P0 gaps; overall qualification remains blocked. Only the bounded new claim is
recorded. The organization gate reports no source/document size-limit debt.

## Limits

- Local in-process evidence only: no remote Agent, Orchestra, installed app,
  process-kill, durable restart or large-scale performance qualification.
- This does not certify scientific appropriateness, units or calibration of
  the existing dimensionless score heuristics. Existing absolute-value scoring,
  inverse-goal denominator regularization and readiness cutoffs are unchanged.
- This does not make every target a hard safety constraint. Domain guards remain
  separate from weighted optimization scores, including deliberate zero weights.
- Canonical-value precedence is checked at the scoring boundary. The generic
  metric resolver's alias fallback and array reductions have not received a
  complete corrupt-sample audit in this round. Unselected display metrics remain
  descriptive metadata rather than additional readiness constraints.
- Arithmetic that overflows an intermediate ratio is rejected conservatively;
  rescaled evaluation of every mathematically representable weighted result,
  underflow error bounds and interval arithmetic remain outside this evidence.
- No claim of full tensor closure or broad industrial physics qualification.
  No new package release, commit, push or app installation was performed.

The subsequent [shared-metric/CFD integrity round](workflow-metric-integrity-20260924.md)
adds selected-alias and sample-array validation at the shared metric resolver,
plus checked streaming Stokes diagnostics. Other dedicated reducers remain
separate coverage rather than being implied by that follow-up.
