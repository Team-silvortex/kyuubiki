# Connected Modal Cluster Candidate Feasibility

## Findings And Scope

On 2026-10-03, the daji 3.4.4 source overlay based on `155d6bc6` was checked
on macOS ARM64. Connected two-beam fixtures now exercise the test-only
[joint admission experiment](modal-joint-subspace-feasibility-20261003.md)
with different branch stiffness and generalized inertia. A single inverse
proposal with fresh deflation resolves sampled perturbations that stall
under coordinate-only correction. The original residual gate stays `1e-8`.

The experiment also demonstrates a limitation: two individually qualified
and mutually orthogonal vectors can represent a correct modal cluster without
uniquely resolving its individual directions when the gap is below tolerance.
Duplicate directions still fail joint admission.

All new code is reachable only through test modules. Production dispatch,
dependencies, formats and the four-inverse-step budget are unchanged. The
public 128-element long-bending request still fails its original gate without
partial modes. There was no live Agent, remote or installed application run.

## Connected Fixture And Independent Reference

Each branch has 16 or 32 reduced beam elements, with two active coordinates
per free node and its root fixed. The whole connected model has 64 or 128
reduced coordinates. The branches use the same integer beam stiffness entries
but coefficients `1` and `4 * (1 + detune)`. Corresponding translation and
rotation coordinates are joined by positive springs. Their nonzero negative
cross-branch entries are retained, not cut into separate components.

Generalized diagonal inertias are `1` on the first branch and `4` on the
second, including rotational coordinates. This is a synthetic algebraic
fixture, not a calibrated density distribution, continuum material law or
general finite-element mass model. Inertia whitening uses exact binary
weights `1` and `0.5`. No coordinate labels reach the automatic fit selector.

Let `mu` and `q` be the first component root and direction from the independent
bounded double-double beam reference with unit generalized inertia. Ideal
component arithmetic reduces this particular separable fixture to:

```text
H(mu) = [[mu + c, -c / 2],
         [-c / 2, (1 + detune) * mu + c / 4]]
c = relative_spring * round_f64(mu)
```

The two roots and mixing weights supply reference candidates spanning the
first component cluster. These are not asserted to be exact eigenpairs of
the final rounded assembly. That assembly retains f64 coefficients after
stiffness multiplication and diagonal spring addition.

A second reference constructs those coefficients by separate closed-entry
formulas, never by dumping the production matrix. It applies the normalized
or physical operator in double-double arithmetic to each actual candidate.
This distinguishes ideal component roots from residuals of the retained
f64 operator. The reference is bounded test arithmetic, not an interval
certificate, formal error bound or independent general spectrum solver.

Twenty-four compact calibrations at four elements per branch check every
normalized matrix entry, compensated matrix products against the independent
reference and the first two Jacobi roots against the component construction.
Product discrepancies are below `1e-13` absolute; root discrepancies are
below `1e-10` relative. Spectral ordering is calibrated only on these compact
fixtures, not certified for every larger model.

## Baseline Cluster Results

The baseline crosses two branch sizes, three relative springs (`1e-8`,
`1e-4`, `1`), three detunings (`0`, `1e-4`, `0.25`) and four freshly assembled
coordinate orders: original, reverse, grouped and mixed. That gives 72
two-mode cases and 144 direction checks. The second seed contains `0.25`
times the accepted first direction, so joint admission must remove it.

Actual and independently recomputed normalized residuals pass `1e-8` and
agree to within `1e-5` relative. Double-double overlap and projector checks
use `1e-10` gates. Borrowed seeds remain unchanged.

| Elements Per Branch | Largest Actual Residual | Largest Independent Residual | Largest Overlap | Largest Projector Distance |
| --- | --- | --- | --- | --- |
| 16 | 1.4309981840e-11 | 1.4309994097e-11 | 2.8274340190e-17 | 1.5537699492e-16 |
| 32 | 3.1184085760e-10 | 3.1184085970e-10 | 9.5596624497e-18 | 3.5203959811e-16 |

These seeds pass after projection; the baseline alone does not demonstrate
correction of a failed candidate. The perturbed cases below do.

## One Inverse Proposal After Coordinate Stalls

Twelve cases at 32 elements per branch use three `(relative_spring, detune)`
pairs and four assembly orders. The second seed also perturbs original
coordinate 63 by `1e-11` times its direction norm. Projection leaves every
case above the gate.

For the weakest mixed-order case, the original coordinate-only fit uses 36
true residual callbacks and stops at `3.4084272219e-7`. The retained negative
test requires that failure; it does not raise a budget or weaken the gate.

The new test helper makes one generic proposal. It projects a private seed,
checks the true residual, solves that residual with the existing prepared
normalized inverse, and subtracts the correction. It then unconditionally
reorthogonalizes against prior directions before computing a fresh true
residual. A proposal must improve that residual, pass the overlap check and
poll cancellation before return. It is not admitted merely because it improves.

Mandatory post-inverse deflation matters: inverse correction can restore a
prior-mode component even when overlap is individually below its gate. Joint
admission still recomputes the true residual and overlap after the proposal.
No beam labels or additional adaptive inverse loop are introduced.

| Relative Spring | Detune | Post Projection Residual | Corrected Actual Residual | Independent Residual | Largest Projector Distance |
| --- | --- | --- | --- | --- | --- |
| 1e-8 | 0 | 8.6903739402e-6 | 1.5949130830e-10 | 1.5949133310e-10 | 1.9422761329e-14 |
| 1e-4 | 1e-4 | 8.6890815592e-6 | 2.1687493912e-10 | 2.1687493575e-10 | 5.6422322310e-13 |
| 1 | 0.25 | 3.7637070744e-6 | 8.9953489495e-11 | 8.9953481744e-11 | 4.5597156255e-14 |

All four orders pass independently recomputed residuals, `1e-10` overlap
and projector gates; their differences are limited to the reported rounding
digits. The largest perturbed overlap is `1.0291846403e-17`. Every fit uses
two true residual callbacks after the proposal. The proposal itself uses two
additional true residual evaluations; those are not included in `fit_calls`.
The inverse solve has its own existing linear validation and bounded refinement.

Prepared-inverse instrumentation observes one preparation and one dense
factorization across injected cancellation and fresh-control proposal replays.
The coordinate fit has a separately prepared Gram factor. This is not a
claim that the whole pipeline uses a single factor or has a production work
budget. The dense test selector and reference remain expensive bounded tools.

## Physical Shape And JSON Readback

Twelve two-mode sets use the same three parameter pairs and four assembly
orders. Qualified internal directions expand to full physical coordinates,
undergo existing unit-shape normalization and become constructed
`ModalFrame2dModeResult` records. These initial samples already pass the
physical residual gate; they do not demonstrate additional physical polishing.

After JSON readback, the entire pair and every shape bit match. Tests recheck
indices, eigenvalue bits, frequency/period consistency, unit participation
and shape norms, and zero root/axial coordinates. All 24 physical residuals
pass both the true operator and independent frozen-coefficient reference,
with relative agreement better than `1e-5`.

The largest physical residual is `2.3690212303e-10`. Original-mass overlap
has a sampled maximum of `1.9275887811e-17`; whitened projector distance has
a sampled maximum of `3.2406617788e-16`. These are constructed records from
test-only candidates, not public result envelopes, SDK calls or Engine tasks.

## Unresolved Directions And Failure Recovery

One weakly connected mixed-order case has relative root gap
`1.2500001034e-10`, below the `1e-8` residual tolerance. Orthogonal rotations
with coefficients `(0.6, 0.8)` and `(-0.8, 0.6)` pass actual and independent
residual gates and retain projector distance `4.7112150492e-16`. Yet the first
vector's alignment with the ideal low-root direction is only `0.6`.

This verifies cluster membership, not uniqueness of individual mode directions.
The same first vector even passes the second root's individual residual gate,
but attempting to admit it again loses independence and fails before any
fit callback. Future production integration must distinguish these claims;
this experiment introduces no new spectral certificate or metadata format.

Cancellation is injected at dense substitution step 64 and the proposal's
final modal validation checkpoint. Specific checkpoints are asserted, no
partial candidate is returned, and borrowed seeds and bases remain unchanged.
Independent fresh control scopes replay bitwise-identical proposals using
the same prepared inverse. A deliberately strict `1e-30` fit fails; replay
with the same fit and the original `1e-8` gate succeeds independently.

## Remaining Integration Boundaries

The experiment covers two branches with uniform within-branch properties and
separable corresponding-coordinate connectors. Spatial/nonseparable coupling,
within-branch heterogeneity, more than two modes, arbitrary units, general
seeds and full spectrum/envelope admission remain open. Compact root checks
and a correct projector cannot certify completeness or individual uniqueness.

Production integration still needs explicit work/memory budgets and full
SDK/Engine/Agent execution evidence. The public 128-element failure remains
unchanged. Only scoped numerical validation is added to the tensor; runtime
execution, recovery, benchmark and industrial readiness are not promoted.

This is bounded connected-cluster candidate feasibility, not a production modal solver or general heterogeneous spectrum qualification.

## Reproduction

From `workers/rust`:

```sh
cargo test -p kyuubiki-solver --lib subspace::connected --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --locked --offline
cargo test -p kyuubiki-cli --test modal_spectrum_operator --locked --offline
cargo clippy -p kyuubiki-solver -p kyuubiki-cli --all-targets --locked --offline -- -D warnings
```

The registered refinement target includes these child tests. Registration
does not execute the profile's live Agent lane. Counts from overlapping
targets are not a coverage percentage.

## Validation Summary

| Check | Result |
| --- | --- |
| New connected-cluster tests | 6 passed, 0 failed |
| Full Solver suite on the final source | 1386 passed, 0 failed, 9 existing ignored tests across 186 result groups |
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
refinement is unchanged this round. Earlier reports retain their historical
fingerprints; helper registration and visibility changed in this follow-up.

| Source | SHA-256 |
| --- | --- |
| `workers/rust/crates/solver/src/modal_frame_connected_reference.rs` | `202b233b2385150262a4af3b40a65db334f2123271982a5f76e21aea77cfeaec` |
| `workers/rust/crates/solver/src/modal_frame_connected_tests.rs` | `12f36e3ba5e31ea2d98f551e9440fd1bb23f79b9b1850197a3b9c59f0289faa0` |
| `workers/rust/crates/solver/src/modal_frame_subspace_reference.rs` | `25f1e69bfce2443c0cc432c633ddc746c320418f5e9587a779466d9d8abafdd9` |
| `workers/rust/crates/solver/src/modal_frame_subspace_tests.rs` | `4aba91a83db76c51f29868ddc37cbb3802a8db9a8d8d89528e149799bb98100b` |
| `workers/rust/crates/solver/src/modal_frame_conditioning_reference.rs` | `f2e51c80bfe7dd5ce9bff5adcb436be5593f7eca7376df508a3edc4ddd36c6a1` |
| `workers/rust/crates/solver/src/modal_frame_refinement.rs` | `7cb3bbb92287010bd0eab5b1c351fb2ca7ac906621465d39cbbbacb72d774724` |
