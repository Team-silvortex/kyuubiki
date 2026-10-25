# Workflow Summary Comparison Range

Date: 2026-10-25. Source base: `e347cf8c`, retaining the local named-source and
summary-admission repairs. Package metadata remains 3.7.1. No commit, version
bump, deployment, desktop rebuild or historical-result rewrite in this run.

## Reproduction

Orchestra eagerly evaluated optional comparison expressions before deciding
whether to include their values. Zero baselines raised arithmetic errors even
when ratio output was disabled. Disabled deltas and relative metrics could
also overflow unnecessarily. The initial 11-case Elixir regression had nine
failures, including requested unrepresentable metrics escaping as exceptions.

Rust evaluated options lazily, but serialized infinite derived metrics as
successful JSON `null` values. It also lost representable percentage changes
when the intermediate absolute difference overflowed. Both initial native
regressions failed before the repair.

## Repair And Boundaries

Both runtimes now reject enabled non-finite derived metrics before publishing
any comparison. The error identifies the operator and output metric. Orchestra
computes optional metrics lazily and translates arithmetic-range failures into
ordinary operator errors. Zero baselines, including negative zero, omit ratio
and percentage fields rather than manufacture a value or divide by zero.

Percentage change retains `(right - left) / left * 100` for a finite difference,
preserving the nearby-value subtraction path. Only when that difference
overflows does it use `(right / left - 1) * 100`. Thus opposite values near the
range limit can still produce a finite percentage if delta output is disabled.
Enabling the unrepresentable delta correctly rejects the whole comparison.
No clipping, denominator floor, zero substitution or solver tolerance change.

If no enabled output can be emitted, the existing empty-comparison rejection
remains; the optional shared-field count is not itself an assessment. Missing
and nonnumeric shared fields retain their existing skip behavior. This work
does not certify arbitrary-integer precision, subnormal rounding, malformed
configuration parity, aggregation, normalization or ranking arithmetic.

## Verification

`make test-workflow-summary-comparison` passes 16 Elixir/API tests and five Rust
integration tests. Shared cases cover zero and negative-zero baselines,
disabled unsafe operations, recoverable percentage intermediates, adjacent
floating-point values, unshared fields, and delta/ratio/percentage overflow.
Both runtimes also exercise all 32 zero-baseline option combinations and 42
signed power-of-two rescalings from exponent -1000 through 1000, with explicit
expected deltas, ratios and percentages.

Graph regressions compare registered operators with graph execution and verify
fail-fast and explicit skip policies. No partial metric or failed-node lineage
is published, including when a safe field was calculated before a failing one.
The requested raw output survives isolated failure; Rust covers all three artifact projections.
Local API tests cover synchronous 422 errors, asynchronous failed jobs and
recoverable jobs, full/compact responses, disposable SQLite result readback and
fresh zero-baseline execution without overwriting retained failure evidence.

The full Web/Orchestra suite passes 3,519 tests with zero failures and 77
pre-existing skips. Rust Engine library and nine selected integration targets
pass 752 tests with zero failures and one pre-existing ignored library test.
All-target Engine Clippy with warnings denied, Rust/Elixir formatting, book and
inventory checks, the 800/2000-line organization audit and diff checks pass.
The tensor checker retains 13 modules and 11 paradigms, zero structural gaps,
two maturity gaps, 23 evidence-grade gaps and 18 P0 gaps; Daji remains blocked.

The `workflow-summary-comparison-range` component profile is stored in the
summary-numerics shard so the existing workflow profile file stays below the
800-line source limit. The profile was executed successfully, not merely
registered. Evidence extends the existing cross-runtime reporting
tensor claim without raising its grade.

This is bounded local floating-point summary-comparison evidence, not complete numeric-range, distributed recovery or industrial physics qualification.
No process-kill/power-loss, remote CI, installed desktop or multi-host execution
claim is made. Local environment: macOS AArch64, Rust 1.88.0,
Elixir 1.19.5/OTP 28. Test counts overlap broader suites and must not be added.
