# Thermal and Electromagnetic Diagnostic Integrity, 2026-09-24

## Scope

This is bounded local thermal, electrostatic and magnetostatic diagnostic integrity evidence, not complete extraction or industrial physics qualification.

This follows [shared metric and CFD integrity](workflow-metric-integrity-20260924.md).
The public paths are `extract.thermal_result_diagnostics`,
`extract.electrostatic_result_diagnostics` and
`extract.magnetostatic_result_diagnostics`, followed by their registered quality
scorers and the composite objective. No solver equations, task format, scheduler,
runtime deployment or external SDK API are changed.

## Reproduction

The initial 23-test boundary suite had 17 failures and six passing controls on
the previous implementation. These failures describe overlapping categories,
not 17 independent physical solver defects:

- Non-object rows and malformed/missing samples were silently filtered out.
- Metadata-only output passed the old diagnostic-field-count check.
- Explicit nonconvergence was erased when converting a raw result to metrics.
- Corrupt canonical values could fall back to healthy aliases or vector components.
- Scalar energy peak selection could ignore records using different aliases.
- Explicit invalid options and missing custom fields became defaults or omissions.
- Required distribution sums/spans and vector norms could become JSON null.
- Squared vector components overflowed or underflowed for representable norms.

The final boundary suite also rejects a mixed magnetic density/total-energy
fallback and covers secondary metric groups and authoritative explicit mappings,
for 26 tests. Existing whole-group `stored_energy` compatibility is retained,
but it cannot fill partial density evidence.

## Contract and Implementation

`workflow_diagnostic_samples.rs` centralizes the shared sample mechanics; the
three small domain wrappers own physical field selection, aliases and metric
group names. This is an internal refactor, not a new engine/operator coupling.

Source collections must be arrays and every record an object. Default optional
groups may be wholly absent, including empty source collections when another
group supplies actual measurements. Once a group is present, every source record
must supply a valid sample. A partial group, invalid value, malformed convergence
marker, explicit nonconvergence or metadata-only payload fails before any summary
is returned. Wholly missing quality metrics remain visibly blocked downstream.

The first present default alias wins on each record. Invalid selected aliases
cannot fall through. An explicit field mapping is authoritative and mandatory;
it no longer silently uses default aliases. Explicit known configuration values
must be nonempty strings; prefixes must normalize to a nonempty ASCII identifier.
Whole-config null and omitted options retain defaults; unrelated workflow options
such as `on_error` remain available to the executor.

Vector magnitudes must be finite and nonnegative. A supplied magnitude retains
precedence; selected components, when present, must also be finite. Otherwise
both planar components are required, and an explicitly mapped third component
is mandatory. Chained `hypot` avoids unused component-square overflow/underflow.
Unconfigured dimensions and lower-precedence aliases are not independently read.

Each reduction scans borrowed samples and retains constant-sized statistics or
a borrowed peak identity, rather than collecting full numeric vectors. Required
sums, means, spans and norms are checked before insertion into JSON. Scalar peak
selection does not compute an unused sum. Existing output aliases, prefix
normalization, signed scalar maximum and last-record peak ties are retained.
Thermal and electrostatic summaries now also include the generic node/element
counts already emitted by magnetic diagnostics.

## Validation

The 26 boundary tests cover all three domains, mixed aliases, malformed records,
selected configuration, optional-group controls, third components, numeric range,
custom sources, peak ties, independent reductions over 1/2/3/64/257 samples and
a corrupt tail record at index 4095.

Five workflow tests cover 12 negative graph cases, fail-fast behavior, explicit
branch recovery, corrected-input replay and visible missing-metric blocking.
They also execute six real solver chains: heat, electrostatic and magnetostatic,
each with triangle and quad elements, through extraction, domain scoring and
composite scoring. Public raw results are checked against diagnostic counts,
scalar maxima and independently computed vector norms.

```text
./scripts/kyuubiki check-operator-validation --execute --profile workflow-diagnostic-integrity --out tmp/workflow-diagnostic-integrity.json
```

Completed local macOS ARM64 checks:

| Check | Result |
| --- | --- |
| Native profile, three commands / six suites | 92 passed: 51 diagnostic + 27 coupled + 14 workflow |
| Engine library | 559 passed, one existing ignored |
| Headless SDK / protocol / solver libraries | 270 / 108 / 355 passed; six existing solver ignores |
| Four core libraries combined | 1,292 passed, seven existing ignored |
| New diagnostic workflow suite | Five passed |
| Release-mode diagnostic, metric and admission workflow suites | 11 passed |
| Engine all-target Clippy with warnings denied | Passed |
| Changed Rust formatting and whitespace checks | Passed |
| Operator-profile, tensor, organization and documentation gates | Passed |

Counts overlap and must not be interpreted as a coverage percentage. The tensor
still reports four maturity gaps, 16 evidence-grade gaps and 11 P0 gaps, with no
structural gap. Overall qualification remains blocked; only this bounded claim
is added. The organization gate reports zero source/document size-limit debt.

## Limits

- Local in-process evidence only; no remote Agent, installed App, process kill,
  durable restart or large-mesh benchmark claim.
- Thermo-mechanical, transport, generic field and diagnostic-bundle reducers are
  outside this report's scope. The later
  [thermo/transport audit](workflow-thermo-transport-integrity-20260925.md)
  covers the first two; generic field and bundle reducers remain separate.
- Existing alias semantics and supplied scalar/magnitude precedence remain;
  this is not a unit/provenance check or an independent reconciliation of valid
  but contradictory summaries, components or aliases.
- Magnetic `stored_energy` still has its legacy whole-group fallback and output
  label. It is not converted into an energy density. That compatibility behavior
  must not be treated as dimensional qualification.
- Distribution sums use the existing ordinary summation order. Intermediate
  overflow is conservatively rejected even if later cancellation could produce
  a finite result. No compensated-sum or interval-arithmetic guarantee is made.
- Unconfigured third dimensions are not inferred. This does not add 3D solvers
  or change existing quality weights, targets, grade cutoffs or physical models.
- Constant auxiliary storage follows from the implementation; no measured speed
  or peak-memory improvement percentage is asserted.
- No release promotion, commit, push, package rebuild or installation is part of
  this round. Other tensor gaps and overall scientific qualification remain open.
