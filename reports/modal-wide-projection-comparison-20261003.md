# Modal Retained Wide Projection Comparison

Date: 2026-10-03
Source line: daji 3.4.4, local macOS ARM64, Rust 1.88.

## Scope And Decision

This is test-only retained-wide-projection diagnosis, not new runtime admission or general modal qualification.

Retaining double-double low parts through the proposal factor, projection,
scoring and coordinate correction does not resolve the tiny-coordinate
physical-publication failure. A bounded fine-coordinate rounding sweep
slightly improves two already-passing scale samples, but the one-anchor
proposal family rejects even those scales. All new algorithms remain behind
test-only module gates. Production tolerances, formats, dependencies and
version are unchanged; no failed candidate is published.

## Candidate Arithmetic And Comparisons

Actual rounded physical stiffness and mass form `D * (K - lambda M)`, with
`D = diag(1 / sqrt(mass))`. Subtraction and multiplication retain high/low
parts, rather than rounding each coefficient before projection. Binary column
scales normalize the fine columns; Gram products and an LDL factor retain the
same representation. The factor is prepared once and reused for right-hand
sides, projected coarse directions and all fine corrections.

Coarse scores, products, deltas and projections remain double-double. The
final fine-coordinate expression `old - correction` is rounded only after
the subtraction. The real operator certificate remains f64 and is not
replaced by a residual computed in the candidate arithmetic. Rounded columns
still select the automatic partition and pair topology; no high-precision
rank selection or new topology is claimed.

Four families use the same actual four-sweep physical-polisher seed at each
scale and the same checked eigenvalue:

| Family | Fine coordinates | Coordinate rounding |
| --- | --- | --- |
| Local wide | Existing automatic half partition | Nearest f64 after wide correction |
| Local wide plus fine neighbors | Same half partition | Forward/reverse single-coordinate up/down neighbors after each fine correction |
| Anchored wide | All coordinates except the largest mass-weighted amplitude coordinate | Nearest f64 after wide correction |
| Anchored wide plus fine neighbors | Same one-anchor partition | Same bounded fine-neighbor policy |

Fine neighbors use the original fine columns, not their orthogonal projections.
Their predicted residual starts from the actual certificate and includes all
rounded fine changes. They only rank candidates; the real operator decides
whether the complete candidate improves. This extra approximate work does
not add true-residual callbacks or extra outer iterations.

## Measured Residuals

The 128-element fixtures have 256 active bending coordinates and actual
rounded segment lengths. The unchanged physical residual gate is `1e-8`.

| Family | Segment length 1 | Segment length 1e14 | Segment length 1e-10 |
| --- | --- | --- | --- |
| Local wide | Pass, 9.205252983779463e-9 | Pass, 7.362945994602734e-9 | Reject, 1.7404270550916884e-8 |
| Local wide plus fine neighbors | Pass, 9.203443285302384e-9 | Pass, 7.359017553229027e-9 | Reject, 1.7404270550916884e-8 |
| Anchored wide, either rounding policy | Reject, 2.175846854093336e-8 | Reject, 2.0126781835612583e-8 | Reject, 1.7404270550916884e-8 |

Every accepted sample also passes the existing independent closed-entry
physical reassembly, unit-shape norm and constructed complete modal result
JSON readback, retaining exact eigenvalue and shape bits. These are candidate
records in a local experiment, not new public solver, Headless, Agent or
installed admission. Expected failures assert the error category and residual
band; failure is never silently skipped.

The anchored failures are not production regressions: that family is not used
by the runtime. More fine coordinates alone do not guarantee a better rounded
candidate. These tests do not prove that no f64 shape can satisfy the gate or
that other high-precision algorithms cannot help. General joint selection of
representable vectors remains a hypothesis requiring separate qualification.

## Independent Analytic Checks

A three-row least-squares sample has columns `[1,1,1]` and
`[1,1+delta,1-delta]`, with `delta=2^-30` and exact known coefficients one and
two. Its f64 Gram factor is rejected as unresolved. The wide factor recovers
the known coefficients and independently recomputed products across three
row orders, three opposite binary scale pairs and three sign pairs, including
column powers `2^-200` and `2^200`. Projection of this known column-space
right-hand side is also checked. Duplicate columns and a much smaller
`delta=2^-50` remain explicit unresolved-pivot failures.

A one-column system `[3,0]` with right-hand side `[1,0]` yields a retained
one-third correction. Subtracting it from one half before rounding recovers
the independently known nearest value of one sixth; rounding the correction
first gives a different f64 result. A separate orthogonal-residual sample
checks known projected entries `[2,-2,0]`. These tests validate concrete
arithmetic effects, not a universal accuracy or rank certificate.

The scalar representation reuses the existing test-only TwoSum/FMA arithmetic,
motivated by [Ogita, Rump and Oishi's sum/dot work](https://www.tuhh.de/ti3/paper/rump/OgRuOi05.pdf).
It is neither arbitrary precision nor interval arithmetic. LDL's bounded
positive-pivot policy is a proposal guard, not proof of numerical rank or
conditioning. For comparison, [LAPACK's least-squares guide](https://www.netlib.org/lapack/lug/node27.html)
distinguishes full-rank QR/LQ drivers from rank-deficient orthogonal/SVD
drivers. This experiment does not claim the coverage of those routines.

## Work And Recovery Guards

Dimensions remain 2 through 256. Nonfinite/ragged data, invalid partitions,
range loss, zero columns, unresolved pivots and malformed certificates fail
explicitly. Caller seed vectors and physical matrices are borrowed; private
candidate changes cannot escape a failed or cancelled correction. Cached
factors remain usable for fresh-control replay.

All four families share the existing four outer and four fine-fit iteration
limits. Each correction has one 80-check counter, with at most 74 callbacks
including initial and fresh final certificates. A decreasing synthetic
certificate sequence forces all 74 checks in both rounding policies and
confirms monotonic counters with zero refactor events during cached search.
This is control evidence, not a numerical acceptance oracle.

A separate plan models wide numeric/container payload and component visits.
At 256 coordinates the payload model is 6,553,600 bytes, under 8 MiB, and the
work model is 1,476,395,008 visits, under a separate 1,500,000,000 ceiling.
The work model covers preparation, repeated wide substitutions/projection,
single/pair sweeps and fine-neighbor selection; it is not elapsed time, f64
instruction count, measured allocations or peak RSS. It excludes caller-owned
models and comparison fits. The original runtime plan does not cover these
algorithms, and no performance improvement is claimed.

Preparation cancellation covers partition validation, factor/substitution,
norm scans, dots and vector updates. Cached-search cancellation covers
iteration, substitution, dot/update, residual search and final validation,
with replay after each. An initially passing certificate followed by an
above-gate or malformed final certificate fails. Actual-operator errors
propagate, and cancellation never returns a partial result.

## Verification

The retained `wide_projection_` filter passes seven tests. Full Solver
regression passes 1,431 tests with zero failures and nine pre-existing ignored
tests across 186 result groups; the seven new tests are included in that total.
The Headless modal integration suite passes all 37 tests. Strict all-target
Solver/CLI Clippy and workspace formatting checks pass with no warnings.

Native tensor, operator-validation, documentation and organization checks pass.
Operator-validation checks verify the registry without executing its commands.
Tensor self-tests and operator-validation self-tests also pass. The tensor's
overall readiness gaps remain unchanged: four maturity gaps, 16 evidence-grade
gaps and 11 P0 gaps, with daji qualification still blocked. This evidence adds
scoped numerical diagnosis and recovery tests, not qualification of the failed
tiny-coordinate mode or promotion of the proposal algorithms.

Commands from `workers/rust`:

```text
cargo test -p kyuubiki-solver --lib wide_projection_ --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --locked --offline
cargo test -p kyuubiki-cli --test modal_spectrum_operator --locked --offline
cargo clippy -p kyuubiki-solver -p kyuubiki-cli --all-targets --locked --offline -- -D warnings
cargo fmt --all -- --check
```

No remote, live Agent or installed qualification, packaging, version change
or Git submission is claimed. Source/document limits remain 800/2,000 lines.
