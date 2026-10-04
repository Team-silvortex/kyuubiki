# Joint Modal Subspace Candidate Feasibility

## Findings And Scope

On 2026-10-03, the daji 3.4.4 source overlay based on `155d6bc6` was checked
on macOS ARM64. Test-only joint admission now distinguishes a residual-qualified
duplicate from an independent direction in a repeated eigenspace. Sampled
rotated bases, perturbed directions and physical mode-record JSON readback
retain both the original `1e-8` residual gate and a `1e-10` overlap gate.

This extends the [automatic-partition experiment](modal-automatic-partition-feasibility-20261003.md).
The helper is a bounded admission experiment, not a production deflated
solver. Production dispatch, formats, dependencies and the four-inverse-step
refinement budget are unchanged. The public 128-element long-bending request
still fails its original gate without returning partial modes. No live
Agent, installed application or remote execution was performed this round.

## Joint Admission Rules

The guard accepts 2 to 256 normalized coordinates and fewer prior directions
than coordinates. It privately binary-scales each finite, nonzero, matching
basis vector and verifies pairwise normalized overlap. Invalid dimensions,
dependent/nonorthogonal directions, nonfinite values and nonzero components
lost during scaling are rejected. Its configurable overlap tolerance must
be finite, positive and at most `1e-8`.

This validates the geometry of the supplied basis, not its eigenvalues or
residuals. A caller must independently qualify previously accepted modes.
The test fixtures qualify their first mode against the true operator and
the independent wide reference. No physical coordinate labels are supplied
to the correction partition selector.

An already orthogonal seed stays bitwise unchanged. Otherwise the guard
projects a private copy using the existing two-pass production projection
and independence-loss check. Losing more than half the preprojection norm
is an explicit error, not a tiny vector to normalize and pass onward. The
post-projection residual is computed afresh by the automatic block fit;
the seed's earlier residual is never inherited.

The fit still performs at most four outer cycles and four fine refits per
pass using one Gram factor. After it returns a residual-qualified candidate,
joint admission recomputes the actual residual, checks its finite matching
certificate, verifies the final overlap and polls cancellation before return.
Corrections can leave the projected subspace; the guard then rejects rather
than silently retrying or returning a duplicated direction. It does not
constrain every private fine or coarse proposal to the subspace.

Neither projection nor a near-reference frequency proves modal completeness.
The borrowed seed and basis are never overwritten, including on errors and
cancellation. There is no new production fallback or residual relaxation.

## Repeated Subspace Results

Two identical beam blocks are independently assembled from integer element
stiffnesses and original f64 masses. Each block has 32 or 64 elements, giving
128 or 256 total reduced coordinates. Their identical first roots span a
two-dimensional repeated eigenspace. Fresh original, reverse, grouped and
mixed coordinate orders exercise actual assembly, not only a vector shuffle.

For each size and order, three bases use coefficients `(1,0)`, `(0.6,0.8)`
and `(0.8,-0.6)`. The next direction uses the complementary rotation plus
`0.25` times the accepted first direction. Vector powers `2^-80`, `1` and
`2^80`, and a negated unit-scale case give 96 joint-admission samples.

Both block residuals are independently recomputed in double-double arithmetic
from their original-order assembly. A wide projector comparison measures the
whole subspace rather than requiring a particular basis rotation. Duplicate
directions instead have projector distance greater than one even though
their individual residuals pass. These wide calculations are bounded test
references, not interval certificates or a general error bound.

| Elements Per Block | Largest Actual Second Direction Residual | Largest Independent Overlap | Largest Wide Projector Distance |
| --- | --- | --- | --- |
| 32 | 1.7177328569e-10 | 1.5710318912e-17 | 1.5080894344e-16 |
| 64 | 4.8403495347e-9 | 4.4671108861e-18 | 1.5684495191e-16 |

These initial samples mostly exercise projection followed by already passing
directions. Separate perturbed tests below require actual correction; the
96 passing cases alone would not qualify repeated-mode refinement.

## Perturbed Correction Results

Eight additional 64-element-per-block cases perturb one fixed original
rotational coordinate by `1e-13` or `1e-11` times the second direction norm.
The same coordinate is mapped through all four fresh assembly orders.
Projection alone leaves residuals above the original gate, so the test
cannot pass through the fit's initial convergence shortcut.

| Perturbation | Post Projection Residual | Corrected Residual Across Orders | Largest Independent Overlap | Largest Wide Projector Distance |
| --- | --- | --- | --- | --- |
| 1e-13 | about 1.54552e-5 | 3.92446e-9 to 4.22022e-9 | 9.05653e-17 | 2.57086e-16 |
| 1e-11 | about 1.54546e-3 | 3.05295e-9 to 3.19939e-9 | 6.59747e-15 | 9.33198e-15 |

Every case uses seven actual residual callbacks, including the guard's final
check, and passes independent per-block residual checks. These are sampled
small perturbations, not proof of convergence for arbitrary seeds, large
contamination or connected/heterogeneous clusters.

Sixteen reordered diagonal examples also separate close but distinct roots
at gaps `1e-2`, `1e-5`, `1e-8` and `1e-10`. They preserve the second root's
independent direction while removing prior-mode and higher-mode components.
This analytic check does not establish those relative gaps for general beams.

## Physical Shape Readback

Eight two-mode sets at 64 elements per block exercise four assembly orders
and two rotations. Each normalized pair is expanded to full physical dofs,
unit-shape normalized and checked by the unchanged physical-shape polish
and residual gates before constructing existing `ModalFrame2dModeResult`
records. The entire pair and every shape bit survive JSON readback.

After readback, tests independently recheck physical block residuals, both
roots, frequency/period consistency, unit participation norms and zero
root/axial dofs. Double-double physical mass overlap has a sampled maximum
of `1.9237658777e-18`. Whitened shape projector distance has a sampled maximum
of `2.6248130016e-16`, below the test's `1e-10` subspace gate.

These are constructed records from test-only candidates, not successful
public spectrum results, full result envelopes, SDK calls or Engine tasks.
Normalization and serialization cannot inherit the internal overlap gate;
both physical residual and mass orthogonality are checked again.

## Failure And Cancellation Checks

Residual-qualified duplicates, including sign changes and binary scaling,
are rejected for independence loss before any residual callback. Primitive
tests cover multiple scaled prior directions, accepted signed-zero bits,
subnormal/maximum single-component bases, invalid seeds and scaling loss.

A deliberately non-eigenvector prior basis demonstrates both stale-certificate
hazards: projection invalidates an initially zero residual, and unconstrained
correction restores a zero-residual direction that fails final orthogonality.
Joint admission rejects it. This is a boundary counterexample, not a claim
that production accepts that invalid prior basis.

A test-side `1e-30` request fails explicitly after 34 residual callbacks,
below a conservative cap of 81 for one bounded fit plus final check. Replaying
at `1e-8` with the same prepared fit passes independent checks. Instrumentation
observes one Gram factor across preparation, failure and replay.

Cancellation is injected after 64 components of seed scaling and actual
first-pass projection, and at the final joint return checkpoint. The test
asserts the specific checkpoint and update pass reached. Basis preparation
also cancels at entry and completed return. No partial direction or basis
is returned, borrowed inputs stay intact and fresh control scopes replay.
Malformed or stale final residual certificates fail explicitly.

## Remaining Integration Boundaries

The dense selector, Gram fit and wide projector remain bounded test helpers
with substantial preparation cost. This report establishes no production
performance budget or large-scale speedup. Previously accepted basis roots
and completeness need their own certification. Connected and heterogeneous
spatial clusters, more than two physical modes, arbitrary coordinate units,
full spectra/envelopes and installed/distributed execution remain open.

Runtime integration must maintain joint residual/subspace admission during
general candidate construction with explicit work and memory budgets. These
results add only scoped numerical validation evidence to the tensor. They
do not promote runtime execution, recovery, benchmark or industrial readiness.

This is bounded joint-subspace candidate feasibility, not a production modal solver or general multimode qualification.

## Reproduction

From `workers/rust`:

```sh
cargo test -p kyuubiki-solver --lib subspace_joint_admission --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --locked --offline
cargo test -p kyuubiki-cli --test modal_spectrum_operator --locked --offline
cargo clippy -p kyuubiki-solver -p kyuubiki-cli --all-targets --locked --offline -- -D warnings
```

The existing registered refinement target includes these child tests.
Profile registration remains distinct from execution of its live Agent lane.
Counts across overlapping targets are not a coverage percentage.

## Validation Summary

| Check | Result |
| --- | --- |
| New joint-subspace tests | 11 passed, 0 failed |
| Full Solver suite on the final source | 1380 passed, 0 failed, 9 existing ignored tests across 186 result groups |
| Headless modal operator suite | 35 passed, 0 failed |
| Solver and CLI all-target Clippy with warnings denied | Passed |
| Workspace formatting and patch whitespace | Passed |
| Operator profile self-test and registration | Passed; 59 profiles, `executed=false` |
| Tensor self-test and structural validation | Passed; 13 modules, 11 paradigms, 0 structural gaps |
| Documentation book and inventory | Passed; 26 HTML files, development/shipping 3.4.4 |
| Project organization audit | Passed; source limit 800, documentation limit 2000, tracked debt 0 |

No new ignored test was added. The headless suite checks the unchanged
production path, not execution of the test-only guard. Tensor readiness
remains blocked with 4 maturity gaps, 16 evidence-grade gaps and 11 P0 gaps.
No version bump, commit, push or packaging was performed.

## Source Fingerprints

SHA-256 paths pin the current source overlay rather than a committed revision.
The lattice fit and production refinement are unchanged this round.

| Source | SHA-256 |
| --- | --- |
| `workers/rust/crates/solver/src/modal_frame_subspace_reference.rs` | `b6553a598926241562926aea98430921158c984f7c80f8cb453992e020480772` |
| `workers/rust/crates/solver/src/modal_frame_subspace_tests.rs` | `258bde61ce26796e29dfdfd728de96788ef30991f744287798977ea9ad6157f9` |
| `workers/rust/crates/solver/src/modal_frame_automatic_tests.rs` | `d0c00865ecd81893679af1a6d4ea4c3f88b131aad597c701703f97a342c17f63` |
| `workers/rust/crates/solver/src/modal_frame_lattice_reference.rs` | `2064316a3fa577bcab5f89ed908c79286715bdf4b15af664183cdd0fdf1abbe3` |
| `workers/rust/crates/solver/src/modal_frame_refinement.rs` | `7cb3bbb92287010bd0eab5b1c351fb2ca7ac906621465d39cbbbacb72d774724` |
