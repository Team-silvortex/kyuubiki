# Workflow Named-Source Integrity

Date: 2026-10-25. Source base: `e347cf8c`. Package metadata remains 3.7.1.
No version bump, deployment, desktop rebuild or historical-result rewrite.

## Reproduction

Orchestra's named-input assembly previously unwrapped a source-port field for
non-envelope operators even when its object contained sibling metrics or
failure markers. A source with `cost: 10` and `payload: {cost: 1}` was replaced
by the nested object. Against a second source with `cost: 4` and nested
`cost: 99`, direct selection chose `right`, but graph execution chose `left`.
This is a result-changing routing defect, not just inaccurate trace metadata.

The same assembly discarded convergence and diagnostic-count rejection
markers before the diagnostics-bundle operator could validate them. The
pre-fix source-integrity, binding and API regression run had 35 tests with
18 failures. Existing Rust behavior preserved sibling data and passed the
shared real-operator controls.

Full web regression also identified four existing API journeys that require
single-field source-port wrappers. Simply prohibiting every non-envelope
unwrap would break those journeys. A new Rust legacy-wrapper control failed
before the native rule was aligned; no existing acceptance test was removed.

## Bounded Repair

Both graph runners now unwrap exactly one level only when the entire source
artifact has one field whose name matches the source port. Sibling metrics,
convergence markers, diagnostic counts and nested objects remain intact.
The existing explicit single-input envelope mode is unchanged. Raw retained
artifacts, target-port assembly, edge ordering and execution receipts are
unchanged. No solver algorithm or diagnostic threshold is relaxed.

## Regression Entry

`make test-workflow-named-input-contract` runs 74 Elixir runner/recovery tests,
nine Rust shared-contract tests and four local API tests, all passing. The
existing `workflow-named-input-contract` component profile uses this entry.
These counts overlap broader suites and must not be added to them.

- Callback checks exercise all 20 declared named-input operator routes with
  conflicting same-named nested fields and reversed node declarations.
- Shared real-operator cases cover merge, comparison, aggregation, selection
  and diagnostic bundles; they compare graph execution with direct execution
  and independent expected metrics, not just two implementations agreeing.
- False/null convergence, nested non-convergence and negative diagnostic counts
  must fail before publication. Explicit skip policy records failure without a
  result or lineage, retains raw data and permits a corrected fresh run.
- Native tests cover full/output-only/no-artifact projections. Single-field
  legacy wrappers preserve nested data and match unwrapped results.
- Local API routing covers asynchronous execution and disposable SQLite result
  readback, plus synchronous rejection and recoverable failure persistence.

## Broader Verification

The final full Web/Orchestra suite passes 3,486 tests with zero failures and
77 pre-existing skips, including the four legacy-wrapper API journeys. Rust
Engine library and five selected integration targets pass 740 tests with
zero failures and one pre-existing ignored library test. All-target Engine
Clippy with warnings denied, Rust/Elixir formatting, documentation book and
inventory checks, the 800/2000-line organization audit and diff checks pass.
The tensor check retains 13 modules, 11 paradigms, zero structural gaps, two
maturity gaps, 23 evidence-grade gaps and 18 P0 gaps; Daji remains blocked.

The existing tensor claim is extended without promoting its evidence grade.
This is bounded local input-routing evidence, not numerical or distributed
qualification, authenticated provenance, process-restart/power-loss evidence,
installed desktop acceptance or a remote CI run. Validation is on macOS
AArch64 with Rust 1.88.0 and Elixir 1.19.5/OTP 28.
