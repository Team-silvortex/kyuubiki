# Nonseparable Four Mode Candidate Feasibility

## Findings And Scope

On 2026-10-03, the daji 3.4.4 source overlay based on `155d6bc6` was checked
on macOS ARM64. The test-only candidate experiment now covers the first four
modes of nonseparable three-beam fixtures with stiffness and generalized
inertia changes inside each branch. Baseline seeds come from the existing
production Jacobi routine, not the reference spectrum. Sampled perturbations
require actual correction and pass without changing the `1e-8` residual gate.

This extends the [connected two-beam experiment](modal-connected-cluster-feasibility-20261003.md).
The previous ideal two-component reduction no longer applies to these offset
connections. Separate wide spectral arithmetic, true operator residuals,
root inertia brackets, subspace projectors and physical JSON readback are
checked. Duplicate directions and wrong root assignments fail explicitly.

New code is reachable only through test modules. Production dispatch,
formats, dependencies and the four-inverse-step budget are unchanged. The
public 128-element long-bending request still fails its original gate without
partial modes. No live Agent, remote or installed application run was made.

## Nonseparable Fixture

Three fixed-root branches have 8 or 16 beam elements each, giving 48 or 96
reduced translation/rotation coordinates. Their element stiffness coefficients
are `4^branch * (1 + 0.125 * ((2 * element + branch) mod 5))`. Generalized
inertias are `4^branch`, multiplied by four at every third offset node.
Both translation and rotation inertias follow this synthetic pattern.

Positive translation and rotation springs connect the branch ring `(0,1)`,
`(1,2)`, `(2,0)` at even nodes. Target nodes are offset and wrapped differently
for each pair, not matched coordinate by coordinate. Link strengths use the
unit-inertia component first root times relative spring `1e-4`, `0.1` or `1`,
a node-dependent factor and a rotational factor `0.25`. Every connector
remains nonzero after normalization and fresh assembly.

This is a synthetic generalized-inertia fixture, not a calibrated material
or continuum mass model. No coordinate labels reach the automatic fit.
The first three low roots and the fourth higher root are checked together;
these are not four nearly repeated roots or a general spatial frame model.

## Independent Spectral And Assembly Checks

The reference constructs retained f64 stiffness coefficients from closed
node and neighboring-entry formulas. The exercised system instead uses
element-by-element sparse assembly. Both share the fixture specification,
but the reference never reads the production matrix or its eigenpairs.
Spring diagonal additions retain their actual f64 rounding. All generalized
inertias have exact binary inverse square roots, so normalized entries need
no additional mass-factor rounding in these samples.

A separate cyclic Jacobi implementation operates on double-double matrix
entries and rotations. It accepts 2 to 96 coordinates and at most 40 sweeps;
remaining relative off-diagonal entries above `1e-26` cause explicit failure.
Nonsymmetric inputs, oversized work requests, unfinished solves and nonpositive
roots fail. The small tridiagonal spectrum `2-sqrt(2), 2, 2+sqrt(2)` calibrates
the implementation. It shares the Jacobi method family with production,
so this is arithmetic/assembly independence, not full algorithmic diversity.

Six fixture spectra contain 432 reference eigenpairs in total. Each pair
is rechecked against the original independent normalized matrix with a
`1e-22` residual gate. The largest observed wide residual is `8.3499420069e-26`.
These are bounded floating-point references, not interval certificates or
formal proofs.

Independent unpivoted wide LDLT counts negative pivots of shifted matrices.
For each of the first four roots, brackets at relative offsets `-1e-9` and
`+1e-9` have counts `index` and `index+1`. That gives 48 inertia evaluations
across six models, cross-checking low-root positions rather than only matching
a nearby value. A zero or nonfinite pivot is rejected, not assigned a sign.
The finite nonzero pivot assumptions were checked on these brackets only.

Twenty-four fresh assemblies cross original, reversed, grouped and mixed
coordinate orders. Every normalized entry exactly matches the independent
closed-entry coefficient. Compensated matrix products agree with independent
wide products to better than `1e-13` absolute. Nonzero links are not dropped
to create independent components.

## Four Mode Baseline And Correction

Twenty-four baseline sets take the first four existing production Jacobi
seeds through test-only joint residual and orthogonality admission. Independent
wide roots agree within `1e-8` relative. All 96 direction checks pass the
true and independently recomputed `1e-8` residual gates, pairwise overlap
below `1e-10` and a four-direction projector gate of `1e-10`.

| Elements Per Branch | Largest Actual Residual | Largest Independent Residual | Largest Four Mode Projector Distance |
| --- | --- | --- | --- |
| 8 | 2.3050675356e-11 | 2.3050649819e-11 | 3.9190160177e-12 |
| 16 | 4.3974906550e-10 | 4.3974907344e-10 | 6.6445893861e-12 |

Eight additional sets at 16 elements per branch use relative springs `1e-4`
and `0.1` across four assembly orders. These deliberately use rounded wide
directions, add `0.125` times each accepted prior direction, then perturb
original coordinate 31 by `1e-10`. Unlike the production-seed baseline,
they isolate correction feasibility from initial eigenvector construction.

Every projected seed remains above the gate. One prepared inverse is reused
within each four-mode set, with one residual-inverse proposal per mode.
The proposal unconditionally deflates against prior modes and checks a fresh
true residual before mandatory joint admission. This is the unchanged
test-only proposal from the preceding experiment, not a new production path.

| Relative Spring | Post Projection Residual Range | Largest Corrected Actual Residual |
| --- | --- | --- |
| 1e-4 | 2.38875e-7 to 8.58379e-6 | 1.0036066893e-10 |
| 0.1 | 2.38813e-7 to 8.51000e-6 | 1.0095230454e-10 |

All 32 corrected directions pass independent residuals and pairwise overlap.
The largest corrected four-mode projector distance is `1.2888007004e-11`.
Each fit uses two residual callbacks after the proposal; the proposal has
two additional true residual evaluations and the inverse has its existing
linear validation/refinement work. No hidden retry loop or speedup is claimed.
Borrowed seeds and accepted bases remain unchanged.

## Physical Shape Readback

Eight sets at 16 elements per branch use springs `1e-4` and `1` across four
orders. Production-seed candidates expand to complete physical coordinates,
undergo existing unit-shape normalization and become constructed existing
`ModalFrame2dModeResult` records. All 32 records and every shape bit survive
JSON readback. Indices, eigenvalue bits, independent roots, frequency/period
consistency, unit participation/shape norms and zero root/axial dofs are checked.

Actual and independent physical residuals remain below `1e-8`; their sampled
maximum is `3.8487908448e-10`. Pairwise overlap is independently checked using
original physical masses at `1e-10`. Whitened four-mode projector distance has
a sampled maximum of `6.6445892823e-12`, below `1e-10`.

These initial shapes already pass; no extra physical polish is demonstrated.
Constructed records are not full public result envelopes, SDK execution or
Engine/Agent tasks. The perturbed-correction lane is separately checked
internally, not asserted to execute this physical publication path.

## Rejection And Cancellation Boundaries

All four individually converged directions are rejected when offered again
against the four-mode basis. Independence is lost before any fit callback.
Assigning the first root to the fourth direction leaves relative residual
`0.9719315194`; the bounded fit rejects after 10 callbacks without modifying
the seed or basis. It does not substitute a frequency or silently return
the wrong mode. The preceding unresolved-gap counterexample remains valid:
these sampled resolved roots do not prove unique directions for every cluster.

The fourth candidate carries three accepted prior directions. Cancellation
is injected after 64 components of projection against the third prior
direction, at dense substitution step 64 and at final proposal validation.
The projection test counts the fifth vector-update pass: two private scales
precede the three basis updates. Each specific checkpoint is asserted.

No partial candidate is returned; all borrowed seeds and three prior vectors
remain unchanged. Fresh control scopes replay bitwise-identical proposals
with the prepared inverse, and final residual/overlap gates pass independently.
These are helper recovery checks, not new distributed recovery evidence.

## Remaining Production Boundaries

This narrows the test-only gap beyond separable two-mode fixtures. Arbitrary
spectra, spatial geometry, general coordinate transformations, large seeds,
nonbinary/extreme mass scales and larger repeated clusters remain unqualified.
Root brackets cross-check sampled ordering; they are not general completeness
or uniqueness certificates.

Production integration still needs work/memory budgets and full
SDK/Engine/Agent admission. The public 128-element failure remains unchanged.
Only scoped numerical validation is added to the tensor; runtime execution,
recovery, benchmark and industrial readiness are not promoted.

This is bounded nonseparable four-mode candidate feasibility, not a production modal solver or general spatial spectrum qualification.

## Reproduction

From `workers/rust`:

```sh
cargo test -p kyuubiki-solver --lib subspace::connected::irregular --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --locked --offline
cargo test -p kyuubiki-cli --test modal_spectrum_operator --locked --offline
cargo clippy -p kyuubiki-solver -p kyuubiki-cli --all-targets --locked --offline -- -D warnings
```

The registered refinement target includes these child tests. Registration
does not execute live Agent commands. Counts across overlapping targets
are not coverage percentages.

## Validation Summary

| Check | Result |
| --- | --- |
| New nonseparable four-mode tests | 7 passed, 0 failed |
| Full Solver suite on the final source | 1393 passed, 0 failed, 9 existing ignored tests across 186 result groups |
| Headless modal operator suite | 35 passed, 0 failed |
| Solver and CLI all-target Clippy with warnings denied | Passed |
| Workspace formatting and patch whitespace | Passed |
| Operator profile self-test and registration | Passed; 59 profiles, `executed=false` |
| Tensor self-test and structural validation | Passed; 13 modules, 11 paradigms, 0 structural gaps |
| Documentation book and inventory | Passed; 26 HTML files, development/shipping 3.4.4 |
| Project organization audit | Passed; source limit 800, documentation limit 2000, tracked debt 0 |

No new ignored test was added. Headless tests check the unchanged production
path, not execution of the new helpers. Tensor readiness remains blocked with
4 maturity gaps, 16 evidence-grade gaps and 11 P0 gaps. No version bump,
commit, push or packaging was performed.

## Source Fingerprints

SHA-256 paths pin this source overlay, not a committed revision. Production
refinement is unchanged. Earlier reports retain their historical fingerprints;
the connected helper now also registers the new child module.

| Source | SHA-256 |
| --- | --- |
| `workers/rust/crates/solver/src/modal_frame_irregular_reference.rs` | `78b07af9992b5e09da978fc0f8824f82df26d42a6cf1f5e6d9a123490996e292` |
| `workers/rust/crates/solver/src/modal_frame_irregular_tests.rs` | `c3ca8a1f72e07f4cb034b1cc4421eccac4dd8937b017ffbe579b0d0bf524daaa` |
| `workers/rust/crates/solver/src/modal_frame_wide_spectrum_reference.rs` | `d978123663264fec9f461a1cee3bfa08974c14a34a1a39233fbc83311c639a3d` |
| `workers/rust/crates/solver/src/modal_frame_connected_reference.rs` | `5596e612c1ce0726b01c85b491dbb488c46c11f8ac388bacba54ee264113a059` |
| `workers/rust/crates/solver/src/modal_frame_refinement.rs` | `7cb3bbb92287010bd0eab5b1c351fb2ca7ac906621465d39cbbbacb72d774724` |
