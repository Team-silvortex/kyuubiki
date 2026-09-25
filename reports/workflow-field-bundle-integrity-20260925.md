# Field and Diagnostic-Bundle Integrity, 2026-09-25

## Scope

This is bounded local Rust field and diagnostic-bundle integrity evidence, not cross-runtime parity or industrial physics qualification.

This follows the [thermo/transport audit](workflow-thermo-transport-integrity-20260925.md).
The changed operators are `extract.field_statistics`, `extract.field_hotspots`,
`transform.compose_diagnostics_bundle` and
`transform.evaluate_diagnostics_bundle_guard`. Solver equations, scheduling,
deployment and SDK interfaces are unchanged. Field extraction is separated
from the generic reporting module; source admission/count validation is kept
separate from bundle report presentation.

## Reproduction

Against the old Rust implementation, the initial 32-test boundary suite had
25 failures: 11 of 16 field tests and 14 of 16 bundle tests. The seven passing
controls included existing nonconvergence handling in field extraction. These
are overlapping regression categories, not 25 independent solver defects.

- Corrupt, missing and non-object samples could disappear from reductions.
- Invalid percentile, sort and sampling options were ignored or defaulted.
- Required sums and squared deviations could overflow into successful nulls;
  tiny squared deviations could underflow. Hotspot means required an unused sum.
- Missing guard metrics/sources and malformed rules could yield a false pass.
- Invalid guard thresholds, comparisons and severity could activate defaults.
- Bundle composition could discard explicit nonconvergence or malformed sources.
- Missing counts became zero; invalid counts vanished; adding valid huge counts
  caused a debug panic (`attempt to add with overflow`).
- Metadata-only diagnostic objects counted as measurement evidence.

Two further positive/negative controls exercise legacy prefixed count precedence
and malformed selected/retained source shapes. The final boundary suite has
34 tests: 16 field and 18 bundle. Five new integration tests cover the full
field-to-report chain.

## Checked Contracts

Both field extractors use the shared selected-sample scanner. Every selected
row must be an object and its field finite numeric; the collection must be
nonempty. Errors retain source/index/field paths. Explicit convergence failure
is rejected, but an absent marker is not proof of convergence. Configuration
names must be nonblank strings; present null/invalid options are not defaults.

Statistics check the published sum, mean and population standard deviation.
Scaled `hypot` deviations avoid squaring overflow/underflow and, when necessary,
scale before subtracting an otherwise unrepresentable deviation. Percentiles
must be in [0,100] with unique normalized output keys. Samples are only collected
when percentiles are requested and sorted once. Interpolation handles constant
values, endpoints and opposite-sign large values without overflowing their
difference. Tests also compare 1/2/3/64/257 linear samples to analytic reductions.

Hotspots validate all selected records, including the tail below an absolute
threshold. A valid explicit threshold retains precedence over percentile
selection; an invalid explicitly supplied percentile still errors. The default
sample limit is eight, zero remains valid, and the cap remains 32. Ties preserve
input order. Sorting borrows source records and only the bounded published
samples clone full records; all hotspot IDs are still published, so this is not
a constant-memory claim. The running mean avoids an unexported overflowing sum.
No matching records remains an error, not an invented zero result.

Bundle composition still ignores ordinary unmarked inputs by default. With
`include_non_diagnostics=true`, plain measured objects are admitted, not scalar
or malformed sources. Claimed diagnostic contracts must be v1; declared
diagnostic metadata without its contract is an error. Selected sources require
actual finite numeric measurements, not just counts and metadata. Convergence
admission covers the envelope and selected sources even with payload retention
disabled. Optional source labels and metric groups are validated when present.

Canonical counts retain precedence over legacy prefixed counts. Present invalid
canonical values never fall through. Missing counts remain `null` in source
items and totals; real zero counts remain zero. Known totals use checked u64
addition, including a known subset when another source has an unknown count.
This intentionally changes the old absent-count output from zero to unknown.

Bundle guards share the typed guard contract for comparisons, severity and
threshold aliases, but address exact published fields rather than domain
resolver aliases. Every configured rule must be parsed and evaluated before a
successful result counts it. A missing source/metric or malformed rule errors;
all retained sources are checked for convergence, not just the rule target.
Actual threshold violations remain successful evaluations with a blocked or
warning decision. They are distinct from invalid evidence or failed execution.

## Workflow Evidence

Five integration tests exercise both extractors through bundle composition,
guard evaluation, report composition and JSON export. Eight fault mutations
per extractor cover bad samples, failed admission, bad field selection, failed
extra sources, invalid source counts, absent metrics, invalid severity and
disabled payload retention needed by a source rule.

Default execution fails at the responsible node. Opt-in recovery retains both
raw inputs and an independent branch while skipping report/export/decision
artifacts; the failed node trace contains the source or rule path. Re-executing
corrected inputs produces the complete report. These are fresh in-process
executions, not checkpoint resumes or process-restart tests.

The positive chain invokes real heat-plane-quad and electrostatic-plane-quad
solvers before graph execution, then feeds their outputs into both field/report
graphs. Four fully prescribed values (10/20/30/40) give independently checked
mean 25, median 25, p90 37, and two hotspots above threshold 25. Corrupting one
raw value blocks the report; replaying the intact output succeeds. These are
small fully constrained data-contract fixtures, not new solver qualification
or mesh-convergence evidence.

```text
./scripts/kyuubiki check-operator-validation --execute --profile workflow-field-bundle-integrity --out tmp/workflow-field-bundle-integrity.json
```

Completed local macOS ARM64 checks:

| Check | Result |
| --- | --- |
| Native profile, five commands / nine suites | 115 passed: 20 field + 18 bundle + 51 diagnostic + 4 hotspot + 22 workflow |
| New boundary suites | 34 passed: 16 field, 18 bundle |
| New field-to-bundle-to-report workflow suite | Five passed |
| Engine library | 628 passed, one existing ignored |
| Headless SDK / protocol / solver libraries | 270 / 108 / 355 passed; six existing solver ignores |
| Four core libraries combined | 1,361 passed, seven existing ignored |
| Release-mode workflow unit tests (`workflow_` filter) | 564 passed, including checked-count overflow |
| Release-mode field/bundle, diagnostic, thermo/transport, guard and admission workflows | 22 passed |
| Engine all-target Clippy with warnings denied | Passed |
| Changed Rust formatting and whitespace checks | Passed |
| Operator-profile, tensor, organization and documentation gates | Passed; 53 profiles validated |

Counts overlap and are not coverage percentages. The scoped tensor claim only
concerns the Rust engine's numerical-validation/recovery coordinates; it does
not promote overall maturity or deployment qualification. The tensor retains
zero structural gaps, four maturity gaps, 16 evidence-grade gaps and 11 P0 gaps;
overall qualification remains blocked. Organization checks retain zero size
debt at the 800-line source and 2,000-line document limits.

## Remaining Limits

- The reachable Elixir Web dispatcher still calls separate implementations in
  `apps/web/lib/kyuubiki_web/workflow_reporting_runtime.ex` and
  `apps/web/lib/kyuubiki_web/workflow_summary_runtime.ex`. They retain permissive
  sample/rule filtering and missing-count defaults. Aligning their contracts
  and running paired Rust/Elixir cases is the next gap, not covered evidence.
- Arbitrary hand-authored bundles are not fully schema/provenance reconciled.
  Unused aliases/components, report focus, generic result summaries, CSV exports
  and arbitrary metadata are not comprehensively audited in this round.
- Required sums conservatively reject intermediate overflow even when later
  cancellation could restore a finite sum. No compensated-sum accuracy,
  interval bounds or bitwise-equivalence claim is made for the stable reductions.
- No remote Agent, installed App, large-mesh benchmark, process-kill or durable
  restart was exercised. Structural allocation improvements are not timings.
- No version promotion, commit, push, App rebuild or installation in this round.

The later [cross-runtime reporting audit](workflow-reporting-cross-runtime-20260925.md)
aligns these four Elixir operations against shared Rust contract fixtures. It
does not retroactively broaden the Rust-only evidence recorded above or claim
equivalent branch-recovery behavior for both graph runners.
