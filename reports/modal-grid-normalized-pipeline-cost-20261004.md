# Normalized Modal Pipeline Cost And Rejected Alternatives

Date: 2026-10-04
Source line: daji 3.4.5 working-tree overlay based on `431b2ac7`.
Platform: local macOS ARM64, Rust 1.88, optimized build.

## Scope And Decision

This is local test-only candidate pipeline benchmarking, not production modal qualification.

The [preceding normalized comparison](modal-grid-normalized-reliability-20261004.md)
retains five successful alternative candidate chains and one large-scale graded
internal rejection over six 128-element, 256-active-coordinate beam fixtures.
This follow-on measures the fresh candidate chain instead of only cached grid
correction, and compares bounded alternatives for the remaining rejection.
The five-success/one-rejection boundary is unchanged. Initial dense Jacobi
preparation is the dominant measured phase of this particular candidate harness,
not proof of a bottleneck on every public solver path.

No additional ordering, radius retry, acceptance gate or production path is enabled.
All new entries remain beneath the existing test-only portfolio registration.
The eigenvalue and `1e-8` residual gate stay frozen. Published shapes still
require strict unit norm and independently reassembled JSON residuals. The
installed application, public API, dependencies and version are unchanged.
Original public solving/publication retains the preceding failure boundaries.

## Order And Radius Controls

Four isolated orders are compared on each of the six original refined pairs:
canonical reverse, natural, ascending grid-column norm and descending norm.
Each starts from the same immutable seed with one bounded factor and at most
six original-operator certificates. Original coordinates are restored before
each real certificate; accepted shapes keep frozen anchor bits and complete
independent physical publication/readback.

Of 24 proposals, six succeed across the same five fixture families. Ascending
succeeds in graded unit/tiny and layered unit/large cases; reverse succeeds in
layered large/tiny cases. Layered large has two proposals, not two independent
fixtures. Natural and descending reject every fixture; graded large rejects
all four orders. There is no support for replacing natural with descending or
adding it as a fourth runtime retry.

The remaining case is compared at radii `1`, `16`, `256`, `4096`, `65536`,
`4194304` in reverse and ascending order. These 12 separate runs reuse one
factor per order but each restarts from the original seed, not as a combined
recovery loop. All reject; their best residual retains the seed's exact bits
at `5.144329240147668e-8`. This does not prove that every damping, anchor or
higher-precision proposal would fail.

## Fresh Pipeline Measurement

Six separate optimized processes are run sequentially, one fixture per process,
with one warmup and three measured fresh-input runs. Compilation and other
Solver test runs finish before retained measurements. This isolates fixtures
from each other, not from all desktop/OS activity. Timing spread is retained,
not selected from earlier faster replays.

Timing includes generated request construction, validation, stiffness/mass
assembly, dense matrix/Jacobi preparation, the actual failed four-step
refinement retaining its best pair, normalized direction construction and
internal grid recovery. Success then runs ordinary physical-shape preparation,
physical direction assembly and grid correction, constructed result JSON and
independent physical reassembly. Rejection skips publication and readback.
Every sample matches warmup eigenvalue, internal vector and published shape
bits plus factor/certificate counts and modeled reservations. Five successful
cases retain independent residuals from `7.078472e-9` to `8.981978e-9`.

Six warmups and eighteen samples are repeated executions, not 24 additional
physical fixtures. Timing excludes fixture destruction, independent inertia
diagnostics and original-production-failure comparisons. Factor observation
and receipt bookkeeping remain included. This measures the alternative test
harness, not successful public solver, Headless or Agent throughput.

### Timing Results

Milliseconds; independently computed phase medians are not additive. Physical
timing includes ordinary polishing and direction assembly, not only grid search.
Native RSS is process high water in KiB, including warmup and all samples; it
is not stage-local or incremental heap.

| Case | Outcome | Total median | Total min to max | Preparation | Internal | Physical | Readback | Peak RSS |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| graded unit | accepted | 542.026 | 458.710 to 657.290 | 465.622 | 61.379 | 14.871 | 0.202 | 12176 |
| graded large | rejected | 563.432 | 554.743 to 585.341 | 501.777 | 61.654 | not run | not run | 10592 |
| graded tiny | accepted | 634.130 | 622.095 to 703.978 | 558.278 | 61.397 | 14.450 | 0.200 | 13120 |
| layered unit | accepted | 602.901 | 531.816 to 720.327 | 527.698 | 60.601 | 14.495 | 0.183 | 12656 |
| layered large | accepted | 583.237 | 562.831 to 609.866 | 547.900 | 20.701 | 14.522 | 0.205 | 12080 |
| layered tiny | accepted | 685.658 | 547.473 to 686.595 | 650.057 | 20.784 | 14.506 | 0.200 | 12112 |

The table records the final six named-entry processes. An earlier fully
segmented replay of the same numerical/timing body has graded-unit internal
samples `91.169`, `205.869`, `227.878` ms and total samples `1008.591`,
`873.051`, `1337.565` ms. It is retained as an explicit variability warning;
the only later change was replacing environment selection with six named test
entries, not a numerical optimization. Three samples with this spread cannot
qualify a production latency SLO or demonstrate comparative speedup. No pre/post
optimization claim is made. Final process peaks are approximately 10.34 to
12.81 MiB, not proof of complete-pipeline compliance with a per-stage
8 MiB numeric payload limit.

### Preparation Breakdown

Milliseconds, independent phase medians. Dense Jacobi includes fallback matrix
creation; refinement includes initial residual measurement, the four-step run
and retained-pair checks.

| Case | Input validation and assembly | Dense matrix and Jacobi | Retained-pair refinement | Normalized directions |
| --- | --- | --- | --- | --- |
| graded unit | 0.322 | 460.896 | 4.060 | 0.335 |
| graded large | 0.333 | 497.192 | 3.924 | 0.319 |
| graded tiny | 0.320 | 553.581 | 3.964 | 0.320 |
| layered unit | 0.323 | 523.118 | 3.939 | 0.320 |
| layered large | 0.318 | 543.277 | 3.984 | 0.322 |
| layered tiny | 0.328 | 645.439 | 3.969 | 0.321 |

This motivates inspecting initial eigensystem preparation before serialization
micro-optimization or more retries. It does not justify truncating spectral
membership checks, changing roots, dropping weak entries or turning captured
failed refinement into the runtime pipeline.

## Budget And Qualification Boundary

Policies remain three factors/nineteen certificates per grid stage. At 256
coordinates the existing modeled stage reservation is 6,914,048 payload bytes
and at most 1,009,090,560 component visits. Graded unit/tiny and layered unit
use three internal factors/eleven checks and one physical factor/four checks.
Layered large/tiny use one factor/four checks per stage. Graded large rejects
with three factors/nine checks, no physical stage and no physical output.

Used modeled grid reservations are 1,347,092,480 visits for four-factor successful
chains, 676,003,840 for two-factor successful chains and 1,009,090,560 for the
rejection. They are conservative reservations, not measured instruction counts,
and exclude preparation, ordinary polishing and readback. The production
350-million budget is not increased. Native RSS cannot replace allocation
accounting or validate summing per-stage payloads.

Priorities remain graded-large numerical recovery, bounded higher-precision
factor/operator-discrepancy comparison, and complete production allocation/work
qualification. The two previously retained unequal-length downstream failures,
broader geometry and multimode evidence remain open. The tensor receives scoped
benchmark evidence, not a release-maturity promotion.

## Replay And Verification

From `workers/rust`:

```text
cargo test -p kyuubiki-solver --lib triangular_grid_normalized_ --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --lib --release triangular_grid_ --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --lib --release triangular_grid_normalized_pipeline_cost_graded_unit --locked --offline -- --ignored --nocapture --test-threads=1
```

Replace the test suffix with each of `graded_unit`, `graded_large`, `graded_tiny`,
`layered_unit`, `layered_large`, `layered_tiny`, separately and sequentially.
Debug benchmark builds reject. Output JSON retains all phase samples, not just
medians. Six explicit profile commands register the six exact named tests;
no environment-prefixed command or relaxed registry allowlist is needed.

| Final check | Result |
| --- | --- |
| Optimized triangular-grid regression | 33 passed, 0 failed, 7 explicit cost tests ignored |
| Debug normalized regression | 7 passed, 0 failed, 6 explicit cost tests ignored |
| Six isolated named pipeline benchmarks | One test passed per process, no failures |
| Solver and CLI all-target strict Clippy | Passed with `-D warnings` |
| Formatting and whitespace | Passed |
| Operator profile validator and self-test | Passed; 59 profiles; registry-only `executed=false` |
| Tensor validator and self-test | Passed structurally; 4 maturity, 16 evidence-grade and 11 P0 gaps retained |
| Documentation inventory and book | Passed; 26 HTML files; development/shipping 3.4.5 |
| Organization audit and self-test | Passed; source <=800, docs <=2000, tracked debt 0 |

Debug tests are a subset of optimized regression, not additive model coverage.
Benchmarks are explicitly selected when executed; ordinary suites intentionally
ignore cost runs. No new full-Solver, remote, installed application, Headless
or cross-platform qualification is claimed. The tensor remains `blocked` for
its existing wider qualification gaps; scoped local cost evidence does not
clear them.

The subsequent [wide-factor and bounded-beam diagnosis](modal-grid-wide-beam-reliability-20261004.md)
retains this report's timing and five-fixture scope as historical results. Its
different proposal algorithm has not inherited these benchmark numbers or
production-budget qualification.
