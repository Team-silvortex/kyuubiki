# Workflow Summary Reduction Range

Date: 2026-10-25. Source base: `e347cf8c`, with the local named-source,
summary-admission and comparison-range repairs retained. Package metadata
remains 3.7.1. No commit, version bump, deployment, installed-app update or
historical-result rewrite in this run.

## Reproduction

The initial shared fixture produced 13 failures in 20 Elixir tests and failures
in all three native operator groups. Aggregation overflowed a sum even when its
mean was representable and lost the small remainder in cancellation cases.
Normalization could emit successful JSON null or hide an overflowing product
behind a clamp. Selection could rank an infinite score and publish a winner
with a null score. Orchestra raised arithmetic exceptions on these paths.
Malformed numeric options could also fall back to defaults in Rust, or throw
unstructured arithmetic errors in Orchestra; inverted clamps were accepted.

## Bounded Changes

- Aggregate means use compensated summation in original units, with a scaled
  fallback only on sum overflow. This preserves the tested cancellation tails
  and subnormal equal-value controls without routinely dividing tiny terms
  before accumulation. Mean and span must be finite before publication.
- Normalization validates rule objects, required field names, numeric options
  and clamp order before checking source presence. Numeric multiplication and
  addition must remain finite before clamping; a clamp cannot disguise overflow.
  Valid nonnumeric field copying remains available.
- Selection validates every weighted term and running total before sorting.
  Hiding breakdowns or candidate scores does not bypass checks. Explicit null,
  strings, booleans, arrays and objects are not default numeric options. Zero
  and negative finite weights retain their previous semantics.
- Orchestra enumerates named sources deterministically, matching native source
  ordering. Its selection breakdown now uses linear-time list construction,
  retaining criterion order instead of repeatedly appending to a growing list.

These changes do not add extended precision or promise correct rounding for
all inputs. Normalization and scoring conservatively reject unrepresentable
intermediates, even if a later offset or term could mathematically cancel them.
The mean fallback does not certify all mixed-scale underflow cases. Existing
missing/nonnumeric aggregate field behavior remains unchanged; broader option
schema parity, exact integer arithmetic and field-coverage policy are separate
work. No solver algorithm or numerical tolerance was changed.

## Regression Entry

`make test-workflow-summary-reduction` passes 25 Elixir/API tests and six Rust
integration tests. The fixture has 20 cases across three registered operators,
with positive controls for ordinary arithmetic, large finite means, subnormal
values, valid clamping, text copying and zero/negative weights. Additional
controls cover 18 cancellation permutations/scales, a three-source maximum
finite mean, and 25 malformed numeric-option combinations per runtime.

Graph checks cover direct/operator parity, fail-fast, explicit skip, no partial
result or failed-node lineage, retained requested raw output and clean fresh
execution. Earlier safe fields cannot leak out when a later field fails.
Native tests cover all three artifact projections. Local API tests cover
synchronous rejection, failed asynchronous jobs and isolated failures,
full/compact responses and disposable SQLite result readback. A fresh valid
request does not overwrite retained failure evidence. These are fresh runs,
not process-restart or checkpoint-continuation tests.

Full Web/Orchestra regression passes 3,544 tests with zero failures and 77
pre-existing skips. Rust Engine library and ten selected integration targets
pass 758 tests with zero failures and one pre-existing ignored library test.
All-target Engine Clippy with warnings denied, Rust/Elixir formatting, book and
inventory checks, the 800/2000-line organization audit and diff checks pass.
The tensor check retains 13 modules, 11 paradigms, zero structural gaps, two
maturity gaps, 23 evidence-grade gaps and 18 P0 gaps; Daji remains blocked.

The `workflow-summary-reduction-range` component profile uses the summary
numerics shard and was executed successfully, not merely registered. The
existing cross-runtime reporting tensor claim is extended
without raising its evidence grade. Counts overlap broader suites and must
not be added.

This is bounded local summary-reduction evidence, not correctly rounded arithmetic, distributed recovery or industrial physics qualification.
No remote CI, authenticated multi-host test, power-loss or installed desktop
acceptance is claimed. Local environment: macOS AArch64, Rust 1.88.0,
Elixir 1.19.5/OTP 28.
