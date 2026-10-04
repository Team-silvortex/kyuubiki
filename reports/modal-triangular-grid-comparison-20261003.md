# Modal Quantized Triangular Grid Comparison

Date: 2026-10-03
Source line: daji 3.4.4, local macOS ARM64, Rust 1.88.

## Scope And Decision

This is test-only quantized grid candidate evidence, not new runtime admission or general modal qualification.

A fixed-grid QR proposal now finds a concrete f64 unit shape for the actual
tiny-coordinate 128-element first bending fixture. Reverse coordinate order,
quantization during backsolve and a radius cap of 65,536 yield original-operator
relative residual `6.358817804510388e-9`, below the unchanged `1e-8` gate.
Independent physical reassembly, unit-shape norm and constructed result JSON
readback also pass. The eigenvalue and frozen anchor are not changed.

This establishes a representable candidate for this fixture; it does not make
the production solve succeed. Order/anchor policies, broader geometry,
multimode behavior and runtime costs still require qualification. New search
and quantized QR code remain test-only. Runtime algorithms, tolerances, public
APIs, dependencies, formats and version remain unchanged.

Follow-on anchor, sign and coordinate-numbering tests now cover 72 additional
beam configurations. They expose a publication boundary: four candidates pass
the physical gate but drift beyond the existing unit-norm contract. A separate
test-only unit-shape entry rejects these without renormalizing an already
certified shape. Thirty-eight configurations pass both gates and thirty fail
the physical gate. The search still depends on coordinate ordering; these
tests diagnose that dependence rather than claiming to remove it.

## Residual And Grid Diagnosis

All three 128-element fixtures have 256 active bending coordinates, actual
rounded physical stiffness/mass and the same production first eigenvalue and
four-sweep physical-polisher seed used in the prior neighborhood comparisons.
Translation/rotation labels below are fixture knowledge, not inferred generic
coordinate semantics. The shares measure squared residual, not displacement
error, energy error or the fraction of failing modes.

| Segment length | Initial relative residual | Translation share of residual square | Largest continuous grid correction | Corrections beyond 64 steps |
| --- | --- | --- | --- | --- |
| 1 | 2.175846854093336e-8 | 91.2383% | About 1,812,341 | 255 |
| 1e14 | 2.0126781835612583e-8 | 89.1283% | About 144.506 | 16 |
| 1e-10 | 1.7404270550916884e-8 | 89.3999% | About 187.100 | 21 |

The tiny sample's eight largest squared residual rows are zero-based active
indices `206, 172, 170, 234, 190, 168, 192, 250`, all translations. Rotation
residuals are still retained and checked, not omitted.

Continuous coefficients and sequential integer decisions are not equivalent.
Rounding during backsolve changes upstream right-hand sides; the unclipped
reverse-order tiny candidate uses integer decisions up to 23,147 grid steps,
despite its continuous correction maximum being about 187. A 65,536 cap leaves
those decisions unchanged and gives the same candidate result as the larger
4,194,304 cap. Increasing the radius does not enumerate additional choices.

## Candidate Construction And Acceptance

Cached wide directions are `D * (K - lambda M)`, with
`D = diag(1/sqrt(mass))`. Each grid spacing is the seed coordinate's
`next_up - seed`. The largest mass-weighted amplitude coordinate is frozen as
an anchor; all other columns are multiplied by their grid spacing, rounded
to f64 and factored by the existing scaled Householder QR helper. Nonzero
entries cannot be silently lost in scaling. The QR factor is prepared once
per order and reused across correction passes and sampled policies/radii.

The independent policy solves for continuous grid coefficients, rounds them
individually and clamps them to the radius. The in-backsolve policy walks the
upper triangle in reverse, rounds/clamps each coefficient in original column
units, converts that integer decision back to QR's internal scaled units and
uses it in every upstream row. Internal column normalization must not define
the integer lattice. Integer ties use Rust's away-from-zero rounding.

The candidate coordinate expression retains wide addition/multiplication
until its final f64 rounding. Grid spacings stay fixed from preparation; this
is not an exhaustive lattice search or a guarantee that every nominal step is
one adjacent ULP after a bin boundary crossing. Actual operator certificates
decide acceptance even if the fixed-grid prediction is imperfect.

Four passes are allowed. A candidate is retained only for a strictly smaller
actual relative residual, and its actual residual replaces the old right-hand
side. A nonimproving proposal stops that policy. One counter covers initial,
up to four candidate and fresh final certificates, at most six. An initially
passing or newly passing candidate still needs fresh final validation; a
malformed/worse final certificate cannot publish it.

## Actual Beam Results

Three orders are compared: natural active index order, its reverse, and
ascending initial grid-column norm with index tie breaking. The latter is a
column-based ordering, but generic ordering/anchor robustness is not claimed.
Every order compares independent/in-backsolve policies and radii
`1, 4, 16, 64, 256, 1024, 4194304`. Reverse order additionally compares radius
65,536 for the large/tiny-coordinate fixtures. All 130 configurations have
explicit success/failure expectations: seven pass and 123 reject.

| Rounding and order | Radius | Segment length 1 | Segment length 1e14 | Segment length 1e-10 |
| --- | --- | --- | --- | --- |
| In backsolve, reverse | 4,194,304 | Pass 7.794966084968067e-9 | Pass 8.109485283404323e-9 | Pass 6.358817804510388e-9 |
| In backsolve, reverse | 65,536 | Not sampled | Pass 8.109485283404323e-9 | Pass 6.358817804510388e-9 |
| In backsolve, ascending grid norm | 4,194,304 | Pass 1.5806687639389976e-9 | Pass 1.7362081283361288e-9 | Reject 1.0764733339343638e-8 |
| In backsolve, natural | All sampled | Reject, original residual | Reject, original residual | Reject, original residual |
| Independent rounding, any order | All sampled | Reject, original residual | Reject, original residual | Reject, original residual |
| In backsolve, any order | 1 through 1,024 sampled | Reject, original residual | Reject, original residual | Reject, original residual |

Successful policies need three actual certificates: initial, candidate and
fresh final. All seven accepted shapes preserve the original root and anchor
bits, pass unit norm within the existing `1e-10` contract, and pass separate
closed-entry reassembly after complete constructed modal-result JSON readback.
Readback retains every physical shape/eigenvalue bit. These are locally
constructed candidate records, not public solver, Headless or Agent outputs.

All failures assert the unchanged-gate error category and verify that its
reported relative residual equals the best actual value checked by that
search. The tiny ascending-norm/full-radius policy improves once, rejects the
next proposal and finishes with four checks; every other rejected policy
retains the initial shape and uses three. No failure is skipped and no
unsupported configuration is promoted as a passing result.

## Anchor Sign And Coordinate Numbering

The follow-on comparison uses the same first eigenvalue and original polisher
seed, in-backsolve quantization and radius 4,194,304. Anchors are selected from
the seed's mass-weighted amplitude: the largest, the runner-up and the largest
member of the opposite translation/rotation family. They are active indices
`252, 250, 253` in all three fixtures. Family membership is test knowledge, not
a generic engine policy. Every check restores the candidate to original
physical coordinates and evaluates the original operator; only the residual
components fed back into QR are remapped.

The identity-layout comparison covers three scales, three anchors, three
orders and both signs, 54 cases. Positive/negative signs have identical
admission classifications in each sampled pair. Every callback checks frozen
anchor bits. Accepted shapes still pass independent reassembly and complete
constructed result JSON readback, with no root replacement or normalization.

| Segment length | Anchor | Natural order | Reverse order | Ascending grid norm |
| --- | --- | --- | --- | --- |
| 1 | 252 | Residual reject | Pass 7.794966e-9 | Pass 1.580669e-9 |
| 1 | 250 | Residual reject | Pass 5.320222e-9 | Pass 1.587218e-9 |
| 1 | 253 | Residual reject | Norm reject, residual 7.187854e-9 | Norm reject, residual 1.762444e-9 |
| 1e14 | 252 | Residual reject | Pass 8.109485e-9 | Pass 1.736208e-9 |
| 1e14 | 250 | Residual reject | Pass 8.897275e-9 | Pass 1.736208e-9 |
| 1e14 | 253 | Residual reject | Pass 7.101344e-9 | Pass 2.243175e-9 |
| 1e-10 | 252 | Residual reject | Pass 6.358818e-9 | Residual reject 1.076473e-8 |
| 1e-10 | 250 | Residual reject | Pass 8.671853e-9 | Residual reject 1.076473e-8 |
| 1e-10 | 253 | Residual reject | Pass 6.360522e-9 | Pass 8.394559e-9 |

For the unit-length/rotation-anchor cases the norms are
`0.9999999998859599` and `0.9999999998871124`, outside the unchanged strict
`abs(norm - 1) < 1e-10` contract under both signs. Raw residual-only correction
is not a publication certificate. The new `correct_unit_shape` wrapper checks
the initial and final shape norms and rejects drift. It does not mutate the
seed, return a rejected shape, or renormalize it. Existing generic `correct`
remains available to closed systems that do not have a unit-shape contract.

The numbering comparison has 18 cases: three scales, reversed or
translation-then-rotation numbering, and three orders with the primary anchor.
Reversed numbering's natural order succeeds on every sampled scale, while its
reverse order rejects every scale. Ascending grid norms still reject the tiny
case, at `1.132095e-8` for reversed numbering and `1.076473e-8` for family
grouping. Family grouping's reverse order qualifies the tiny shape at
`3.27235705951954e-9`. Thus simple reverse-index selection is not invariant to
renumbering, even though every physical certificate uses the original model.

All 72 configurations have explicit acceptance, residual-rejection or
norm-rejection expectations: 38, 30 and 4 respectively. Residual rejections
report the best actually checked residual, not a QR prediction. Each search
uses three true certificates except six tiny ascending-norm rejections, which
use four; the six-certificate cap remains unchanged. The comparison runs fits
independently; it is not a bounded multi-fit runtime fallback or its aggregate
work/memory model.

A separate closed identity system checks exact representable targets under
both signs, three permutations and two column orders. Its residual is divided
by the original grid spacing so a one-ULP error cannot pass just because its
absolute value is small. Exact shape bits are required. A unit-seed identity
control distinguishes a passing one-step target from a physical zero-residual
target with norm drift, and rejects a nonunit seed before calling the operator.
Cancellation during final norm evaluation or the final publication checkpoint
returns no shape and permits a fresh replay of the same cached factor.

## Independent Controls And Bounds

A three-row, two-column closed system has columns `[1,0,0]` and `[4,1,0]`
with target `[2.21,0.49,0]`. Its continuous solution is `[0.25,0.49]` and
independent integer rounding gives `[0,0]`; quantized backsolve gives `[2,0]`.
Enumeration over both integer coordinates in `[-4,4]` verifies the selected
closed residual square is minimal within that box. This particular optimum
is not a global-optimum claim for arbitrary inputs.

The same integer result survives three row orders and common binary powers
`-200,0,200`. A separate diagonal system checks original column units under
opposite binary powers `-400,400`, and radius-one clipping. A coupled physical
coordinate example checks the actual two-grid-step move under both seed signs
against its closed residual; its illustrative gate is `0.25`, not a claim to
pass the physical modal `1e-8` gate.

Controls verify all six monotonically counted certificates with no factor
preparation during cached search, and directly verify residual refresh after
an accepted move. Malformed initial/final certificates, a passing searched
candidate with failing final validation, operator errors and an improving
prediction with worse actual residual all reject. Seeds stay private/intact.
Preparation, factorization, reflector/update/dot, triangular decision and
final-validation cancellation return no partial shape and allow fresh replay.

Dimensions are 2 through 256. Nonzero input heads for directions, seed and
actual residual are bounded to `1e-50..=1e50`; malformed wide components,
invalid orders/anchors, zero nonanchor seeds, rank loss and scaling loss fail
explicitly. The positive integer radius is at most 4,194,304. These are prototype
guards, not supported-domain promises for every valid modal model.

At 256 coordinates the separate structural model allows 3,670,016 bytes of
numeric/container payload and 335,544,320 component visits, below 8 MiB and
350,000,000. Radius changes no loop bound or storage size. This per-fit model
excludes caller physical models, shared direction construction, diagnostics,
comparison fits and actual certificate work. It is not a wall-time benchmark,
flop count, allocator audit or measured peak RSS. The diagnostic QR factor in
the comparison is separate from the cached search factor and callback cap.

## Verification

Twelve retained `triangular_grid_` tests pass, including strict assertions for
the original 130 beam configurations (7 accepted and 123 residual rejections),
72 anchor/sign/numbering configurations (38 accepted, 30 residual rejections,
4 norm rejections), and independent closed-target/publication controls.
The full Solver regression passes 1,457 tests with
zero failures and 9 ignored tests across 186 result groups. All 37 Headless
modal integration tests pass. All-target strict Clippy, formatting and diff
whitespace checks pass.

Tensor and operator registry self-tests/checks, documentation book/inventory
checks and project organization audit pass. Registry checks do not execute
physical qualification. The tensor still reports 4 maturity gaps, 16
evidence-grade gaps and 11 P0 gaps; qualification remains blocked. This
test-only candidate does not erase those runtime qualification boundaries.

Commands from `workers/rust`:

```text
cargo test -p kyuubiki-solver --lib triangular_grid_ --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --locked --offline
cargo test -p kyuubiki-cli --test modal_spectrum_operator --locked --offline
cargo clippy -p kyuubiki-solver -p kyuubiki-cli --all-targets --locked --offline -- -D warnings
cargo fmt --all -- --check
```

No live Agent, installed/remote qualification, packaging, version change or
Git submission is claimed. Source/document limits remain 800/2,000 lines.
