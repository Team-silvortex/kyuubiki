# Shared Metric and CFD Diagnostic Integrity, 2026-09-24

## Scope

This is bounded local shared-metric and CFD diagnostic integrity evidence, not complete extraction or industrial physics qualification.

This follows [domain quality validation](workflow-domain-quality-validation-20260924.md).
The public paths are the nine domain guard/quality/pair registrations and
`extract.stokes_flow_result_diagnostics`. No solver equations, physical scoring
heuristics, scheduling policy or external SDK API are changed.

## Reproduction

The initial 18-test suite had 15 failures and three passing controls before
implementation. The failures are categories, not 15 independent physics defects:

- A corrupt first alias could be replaced with a healthy later alias.
- Invalid or incomplete frequency, node and mode records were filtered out
  before computing an extremum, leaving incomplete evidence presented as complete.
- A malformed frequency collection could silently fall back to node data.
- CFD ignored non-object rows and invalid/missing sample fields, and synthesized
  zero measurements for empty collections. It also dropped nonconvergence markers.
- Squaring large or tiny velocity components overflowed or underflowed even when
  the Euclidean norm was representable.
- A finite CFD pressure mean could be lost because the unused raw sum overflowed.
- Non-finite norms, spans and dissipation totals could be serialized as null.

## Contract and Implementation

`workflow_metric_resolver.rs` now exposes a checked result for decisions.
The declarative aliases are in `workflow_metric_resolver/aliases.rs` and retain
their precedence. A selected invalid value is an error, not absence. Explicit
valid canonical values still override aliases and derived sources. Unselected
metadata is not implicitly validated.

Frequency and mode bounds and transient node extrema use checked constant-space
reductions. Every consumed row must be an object and every required field or
selected alias must be a finite number. A present frequency array is the selected
source, including when empty; it does not switch to the transient-node fallback.
Empty/absent selected collections return unavailable metrics, allowing quality
scoring to return its existing visible missing/block result. Corrupt collections
and partially missing samples are errors with array/index/field paths.

Guards, paired comparisons and quality scorers propagate these failures. Optional
display summaries explicitly use `display_metric_value`, a best-effort API that
returns no value on invalid evidence; it is not used for acceptance decisions.

The CFD extractor requires nonempty nodes and elements and checks all of its
required per-sample fields. It honors explicit convergence admission. It performs
one node pass and one element pass, retaining only statistics and borrowed peak
identities rather than five numeric sample arrays. `hypot` evaluates velocity
magnitude without unweighted squares. A weighted running mean avoids the need
for an unused representable total. Required dissipation sums, output means and
spans are checked for finiteness. Signed pressure statistics, signed peak values,
last-sample tie identity, aliases and custom output prefixes are retained.

Default workflow execution stops before publishing partial evidence. Explicit
`on_error: "skip"` preserves raw inputs and independent outputs, but downstream
quality/composite stages are skipped. Corrected-input replay runs those stages
successfully. Eight graph failure cases cover both CFD extraction-to-quality and
dynamic sample-to-quality-to-composite chains.

## Validation

The completed dedicated suite has 22 tests plus three new workflow tests.
It includes a corrupt tail record at index 4095, nine-domain alias rejection,
independent CFD reductions over 1/2/3/64/257 samples, retained peak tie semantics,
numeric range controls and absent-versus-invalid data controls.

```text
./scripts/kyuubiki check-operator-validation --execute --profile workflow-metric-integrity --out tmp/workflow-metric-integrity.json
```

The retained native profile additionally runs existing resolver, quality, guard,
result-admission and recovery compatibility tests. Local macOS ARM64 results:

| Check | Result |
| --- | --- |
| Native profile, four commands / seven suites | 200 passed: 26 resolver/integrity + 141 quality + 20 guard + 13 workflow |
| Engine library | 533 passed, one existing ignored |
| Headless SDK / protocol / solver libraries | 270 / 108 / 355 passed; six existing solver ignores |
| Four core libraries combined | 1,266 passed, seven existing ignored |
| Release-mode metric, quality, guard and admission workflow suites | 13 passed |
| Engine all-target Clippy with warnings denied | Passed |
| Changed engine Rust formatting and whitespace checks | Passed |
| Operator-profile, tensor, organization and documentation gates | Passed |

Counts overlap and must not be added into a coverage percentage. The tensor still
has four maturity gaps, 16 evidence-grade gaps and 11 P0 gaps, with no structural
gap. Overall qualification remains blocked; this round records a scoped claim
only. The organization gate reports no source/document size-limit debt.

## Limits

- Local macOS ARM64, in-process evidence only. No remote Agent, installed app,
  large-scale benchmark, process-kill or durable restart claim.
- No audit of every dedicated thermal, thermo-mechanical, electrostatic,
  magnetostatic, transport, generic field or bundle diagnostic reducer.
  A subsequent [three-domain diagnostic audit](workflow-diagnostic-integrity-20260924.md)
  supplies separate thermal, electrostatic and magnetostatic evidence.
- Raw arrays are not cross-validated against an authoritative finite summary;
  first-mode participation only consumes the first mode. Conflicting but finite
  aliases retain precedence rather than being independently reconciled.
- Existing one-axis transient-node, frequency-amplitude and two-dimensional CFD
  vector semantics are retained. This is not a new 3D norm or physical-unit model.
- Constant auxiliary memory follows from the implementation; no throughput or
  peak-memory benchmark improvement percentage is asserted.
- Conservative rejection of overflowing sums/spans is not interval arithmetic,
  compensated-summation qualification or a proof of all finite-range accuracy.
- No new release, commit, push or installation is part of this change. Overall
  scientific qualification and the other tensor gaps remain separate work.
