# Heterogeneous Modal Grid Boundary Validation

Date: 2026-10-04
Source line: daji 3.4.5 working-tree overlay based on `431b2ac7`.
Platform: local macOS ARM64, Rust 1.88.

## Scope And Decision

This is heterogeneous single-mode validation and failure/replay evidence, not production canonical-grid admission or general modal qualification.

The [canonical grid comparison](modal-grid-canonical-cost-reliability-20261004.md)
covered uniform straight beams. This follow-on adds variable modulus, density,
area, second moment and unequal element lengths. It exposes two separate
remaining limitations: upstream normalized-spectrum recovery and downstream
physical-shape correction. Successful candidate cases do not erase either gap.

No production algorithm, tolerance, retry budget, dependency, public API,
format, version or installed application changes. New fixtures and tests remain
behind `cfg(test)`; one existing test-only reference module is shared rather
than loaded twice.

## Fixture Construction

Three deterministic profiles use 64, 96 and 128 elements and coordinate scales
`1`, `1e14`, `1e-10`: 27 distinct parameterized input fixtures. Each beam is
horizontal, clamped at its first node, with all axial translations restrained.
Only its first bending mode is requested; this is not curved geometry, free-free,
three-dimensional, multimode or experimental material qualification.

For element fraction `t=(i+0.5)/segments`, the graded profile uses modulus
multiplier `0.7+0.6*t`, area `1+0.3*t`, second moment `0.75+0.5*t` and
density `1.25-0.5*t`. The layered profile uses unit multipliers in its first
half and modulus 6, area 0.7, second moment 0.4, density 2.5 in its second half.
The unequal-length profile repeats length multipliers `[0.7,1.1,1.4,0.8]`
and alternates modulus `[0.85,1.2]`, area `[1,0.8]`, second moment
`[0.9,1.1]`, density `[1.15,0.85]`.

Coordinates are accumulated in dimensionless form and multiplied by the scale.
Modulus is multiplied by scale cubed and density by inverse scale. These are
synthetic numerical range fixtures, not three physical materials from a measured
database. Assembly always uses actual rounded coordinate differences.

Requests pass the production input validator. The fixture extracts the
horizontal bending stiffness using production element coefficients and mass
helpers, then uses production spectrum preparation and the existing ordinary
shape-proposal seed. A failed spectrum preparation is not bypassed with a raw
Jacobi vector, a relaxed gate or a test-only eigenvalue.

Accepted shapes are checked after constructed result JSON round-trip. A shared
independent checker reassembles the request's element stiffness and masses,
using neither production assembly nor product helpers. Every accepted shape
passes `relative <= 1e-8`, strict unit norm and exact numeric shape/eigenvalue
readback. The original eigenvalue and frozen anchor are retained.

## Retained Coverage And Rejections

For every prepared fixture, four numberings (identity, reversed, shuffle seeds
7 and 113) and both signs form eight transformations of the same assembled
matrix, seed and masses. Every callback restores original physical coordinates.
Sign-restored accepted bits match across all eight transformations.

| Fixture class | Fixtures | Canonical executions | Accepted | Rejected |
| --- | --- | --- | --- | --- |
| Already-passing seed retention | 13 | 104 | 104 | 0 |
| Seed requires correction and succeeds | 6 | 48 | 48 | 0 |
| Seed requires correction and fails | 2 | 16 | 0 | 16 |
| Upstream normalized-spectrum failure | 6 | 0 | 0 | 0 |
| Total | 27 | 168 | 152 | 16 |

The six upstream failures prevent their 48 planned transformations from running.
They are retained negative preparation/public-entry cases, not executed
canonical searches. A nominal 216-combination plan is therefore not a claim
of 216 completed searches, 100% numerical success or independent geometry
coverage. The 104 seed-retention cases are not recovery successes.

All 64-element seeds already pass. At 96 elements the graded unit/large scales
and all three layered scales need correction; graded tiny scale and all three
unequal-length scales retain passing seeds. At 128 elements graded/layered
preparation fails in all scales, while unequal-length unit/large scales reject
inside the canonical portfolio and tiny scale succeeds.

Per-fixture stage, seed classification, raw outcome, canonical outcome,
certificate count, selected order and factor count are asserted explicitly.
The raw comparison runs identity only: it passes 19 of 21 prepared fixtures,
with the same two downstream failures. It is not an arbitrary-numbering baseline.

| Profile | Elements | Scale | Seed residual | Canonical residual | Independent JSON residual | Certificates |
| --- | --- | --- | --- | --- | --- | --- |
| graded | 96 | 1e0 | 1.172169e-8 | 1.895383e-9 | 1.895383e-9 | 4 |
| graded | 96 | 1e14 | 1.129681e-8 | 3.180485e-9 | 3.180485e-9 | 4 |
| layered | 96 | 1e0 | 1.027879e-8 | 2.751801e-9 | 2.751801e-9 | 4 |
| layered | 96 | 1e14 | 1.198134e-8 | 2.613786e-9 | 2.613786e-9 | 4 |
| layered | 96 | 1e-10 | 1.282384e-8 | 1.661437e-9 | 1.661437e-9 | 4 |
| unequal-lengths | 128 | 1e0 | 2.795071e-8 | rejected | no candidate | 10 |
| unequal-lengths | 128 | 1e14 | 2.097123e-8 | rejected | no candidate | 10 |
| unequal-lengths | 128 | 1e-10 | 2.625921e-8 | 9.088672e-9 | 9.088672e-9 | 10 |

The five corrected 96-element fixtures use one reverse-order factor and four
actual certificates. The corrected 128-element tiny unequal-length fixture
uses the third grid-norm order, three factors and ten actual certificates;
the raw identity comparison uses eleven certificates. Both rejected fixtures
exhaust three factors after ten canonical certificates, with zero unit-norm
rejections. Passing-seed retention uses one factor and three certificates.
Existing three-factor/nineteen-certificate caps and returned work reservations
are unchanged.

## Upstream Public Failure Boundary

Each blocked fixture is also submitted to the actual public
`solve_modal_frame_2d` entry. Its error matches the extracted bending fixture's
normalized-spectrum error exactly. This separates an upstream recovery
limitation from an input validation error or downstream grid rejection.
Returning `Err` gives no partial result.

| Profile | Elements | Scale | Rejected normalized residual |
| --- | --- | --- | --- |
| graded | 128 | 1e0 | 1.402419e-8 |
| graded | 128 | 1e14 | 1.544046e-8 |
| graded | 128 | 1e-10 | 1.596671e-8 |
| layered | 128 | 1e0 | 1.750558e-8 |
| layered | 128 | 1e14 | 1.862264e-8 |
| layered | 128 | 1e-10 | 1.771579e-8 |

These are legitimate current rejections above the unchanged `1e-8` gate,
not a claim that the real physical model is unsupported or that no admissible
f64 representation exists. The canonical physical candidate cannot repair a
spectrum that was never admitted. No other prepared fixture is assumed to
succeed through the public entry merely because a private candidate passes.

## Failure And Cancellation Replay

An actual public 64-element graded solve passes independent JSON reassembly.
A subsequent 128-element graded request fails in normalized recovery; a fresh
64-element solve then reproduces the baseline result and shape bits exactly.

A separate 96-element layered model starts above the physical gate and reaches
a certified canonical correction. Cancellation is requested at the final
restored-shape publication checkpoint, after four actual certificates. The
entry returns no shape; a fresh invocation again uses four certificates and
passes independent physical/JSON reassembly. This exercises a real heterogeneous
operator, not a synthetic identity matrix.

This is local public-API and private-candidate replay. No live Agent slot,
Headless transport, installed application or remote lifecycle claim is made.

## Verification And Remaining Work

| Check | Result |
| --- | --- |
| Final full Solver regression | 1,471 passed, 0 failed, 10 ignored; 186 result groups |
| Final optimized triangular-grid replay | 26 passed, 0 failed, 1 cost test ignored |
| Solver/CLI strict all-target Clippy | Passed with `-D warnings`; no duplicate-module suppression |
| Formatting, whitespace and changed-file line limits | Passed; source <=800, docs <=2,000 |
| Tensor/profile validators and self-tests | Passed; 59 profiles, registry-only `executed=false` |
| Documentation book/inventory | Passed; 26 HTML files, development/shipping 3.4.5 |
| Project organization | Passed; tracked debt 0 |

The full regression and optimized triangular-grid replay use the final shared
reference source. The ten ignored full-suite tests comprise nine original
ignores and the explicit cost test; no new performance run is claimed here.
The tensor still has 13 modules, 11 paradigms, no structural gaps, 4 maturity
gaps, 16 evidence-grade gaps and 11 P0 gaps; qualification remains blocked.
Registry checks do not execute numerical qualification, and overlapping
regression suites are not additive physical coverage. Cross-platform execution
of these new fixture expectations remains unverified in this local run.

Replay from `workers/rust`:

```text
cargo test -p kyuubiki-solver --lib triangular_grid_canonical_heterogeneous_ --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --lib --release triangular_grid_ --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --locked --offline
cargo clippy -p kyuubiki-solver -p kyuubiki-cli --all-targets --locked --offline -- -D warnings
```

The most immediate follow-ups are:

1. Extend bounded normalized-spectrum recovery for the six blocked graded/layered
   inputs, retaining the original eigenvalue/physical validation contracts rather
   than bypassing preparation.
2. Diagnose the two unequal-length downstream rejections and qualify an improved
   proposal policy within explicit aggregate bounds, not only a larger radius.
3. Cross-check independently rebuilt numbering and clustered multimode spectral
   membership, mass orthogonality and final physical readback before production
   integration.

Current retained evidence remains scoped validation/recovery, not a maturity or
release-qualification promotion. No new benchmark measurement, packaging,
version bump or Git submission is claimed.

The [normalized recovery and physical publication follow-on](modal-grid-normalized-reliability-20261004.md)
now compares a separate internal-direction and physical-candidate chain for
the six blocked inputs. Its five private two-stage fixture successes do not
supersede this report's unchanged production/public-entry rejection boundary.
