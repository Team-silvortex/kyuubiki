# Scaled Modal Wide Proposal And Publication Boundary

Date: 2026-10-03. Source line: daji 3.4.4, local macOS ARM64, Rust 1.88.

## Scope And Decision

This is test-only scaled wide proposal diagnosis, not new runtime admission or general modal qualification.

The follow-up isolates rounding and physical shape recovery after the
[hybrid comparison](modal-hybrid-proposal-comparison-20261003.md). A bounded
double-double proposal is resolved from the actual stored physical stiffness
and mass, rather than an ideal beam or a differently rounded normalized
matrix. Its first root agrees with the production root to about one f64
rounding unit at all three coordinate scales. Its intermediate residual is
near `6e-17`, but rounding the shape back to f64 raises the residual above the
unchanged `1e-8` gate in every sampled representation.

The existing coordinate recovery can qualify the unit/large-coordinate unit
shapes. The tiny-coordinate physical shape remains unresolved, including a
chain that first corrects the internal normalized vector. Higher-precision
proposal generation alone therefore does not resolve this fixture. All new
proposal code remains test-only. Production algorithm selection, residual
gates, task/result formats, versions and dependencies are unchanged.

## Matched Proposal Model

The retained straight-beam fixture has 128 elements and 256 active bending
DOFs. It assembles the actual rounded segment lengths and production local
entries/masses at segment scales `1`, `1e14` and `1e-10`. The reduced physical
matrix is copied from those same entries before compression; no stiffness
is inferred by undoing a rounded normalized matrix.

The test supplies a positive diagonal coordinate transform `C`: one for
translations and the reciprocal segment scale for rotations. Double-double
arithmetic forms `C K C` and `C M C`, applies one bandwidth-three LDL factor,
then 32 inverse iterations with binary iterate scaling. The physical proposal
is `C q`. This supplied transform and band are fixture knowledge, not a
label-free policy or general topology/permutation qualification. Nonzero
couplings are retained, including tiny assembled cancellations.

The existing double-double scalar implementation was extracted into one
shared test module without changing its sum/product/division arithmetic.
Its TwoSum/FMA motivation follows
[Ogita, Rump and Oishi](https://www.tuhh.de/ti3/paper/rump/OgRuOi05.pdf).
This is finite-range numerical reference arithmetic, not arbitrary precision
or interval certification. The wide intermediate residual shares proposal
arithmetic and is diagnostic only, not an independent acceptance oracle.

Rounded candidates are checked with the original compensated operator and
original mass. Accepted physical unit candidates additionally pass separate
closed-entry reassembly, a complete constructed result record, typed JSON
readback, exact shape/root bit retention and the unit-norm contract. Those
tests do not constitute public, Headless or Agent execution of wide proposals.

## Measured Boundaries

| Segment scale | Wide intermediate physical residual | Root relative difference from production |
| --- | --- | --- |
| 1 | 5.88104e-17 | 2.22045e-16 |
| 1e14 | 6.52521e-17 | 1.11022e-16 |
| 1e-10 | 6.64639e-17 | 1.11022e-16 |

The different-scale roots belong to their actual rounded models. They are
not asserted to equal the ideal unit-beam root. Root agreement here is a
cross-check, not a general spectral-index or multimode certificate.

Each column below starts from the same wide physical direction and root.
The correction is the existing automatic Gram fit with unchanged bounds.

| Representation | Scale 1 initial / corrected | Scale 1e14 initial / corrected | Scale 1e-10 initial / corrected |
| --- | --- | --- | --- |
| Physical components rounded directly, no unit norm | 4.62242e-8 / reject, 1.06305e-8 | 4.08103e-8 / 8.45184e-9 | 4.34324e-8 / reject, 4.33623e-8 |
| Round first, then f64 unit normalization | 5.49396e-8 / 8.12660e-9 | 5.86491e-8 / 9.05217e-9 | 6.27497e-8 / reject, 4.28131e-8 |
| Wide unit normalization, then round | 4.29141e-8 / 7.97908e-9 | 4.77505e-8 / 7.68019e-9 | 4.62241e-8 / reject, 3.94261e-8 |
| Round mass-normalized vector, then production shape expansion | 5.49235e-8 / 8.12649e-9 | 5.86845e-8 / 9.05217e-9 | 7.04732e-8 / reject, 3.79126e-8 |

An unnormalized corrected direction is not a publishable unit shape. The
direct-round row is a diagnostic and does not receive a publication claim.
All initial rounded representations explicitly fail the gate. Successful
unit-shape corrections explicitly pass independent physical/readback checks;
failed corrections must report the unchanged residual gate.

The combined test chain rounds the mass-normalized proposal, applies existing
normalized roundoff recovery, expands it, then runs existing physical polish
and recovery. Its final physical residual is `8.95961e-9` for scale one and
`8.80877e-9` for scale `1e14`. At scale `1e-10`, the normalized step completes
but physical recovery rejects at `1.92317e-8`. This is a candidate experiment,
not a new production fallback. The two existing recovery budgets are not
merged or reset, nor represented as one 80-check budget for the whole chain.

## Bounds And Recovery

Proposal preparation rejects dimensions outside 2..=256 before factorization,
ragged/nonfinite/asymmetric inputs, nonzero entries outside bandwidth three,
nonpositive or out-of-range mass/scales and unresolved pivots. Nonzero raw
coefficient magnitudes are restricted to `1e-100..=1e100`; coordinate scales
to `1e-20..=1e20`. Balanced mass is `1e-12..=1e12`, balanced nonzero K entries
are `1e-100..=1e12`, and positive pivots must exceed `1e-12`. Unsupported
problems fail explicitly; none of these test bounds is a runtime policy.

One prepared factor is reused for exactly 32 substitutions. Factor coefficient
and substitution range guards stop an expansive SPD example before arithmetic
overflow. Cancelled preparation, factorization, substitution, iteration and
final proposal return preserve the borrowed model and replay deterministically.
Wide unit normalization has its own shape-scan/update/final cancellation
checks and rejects loss of a nonzero component during scaling or rounding.

The helper retains two dense double-double matrices, at most 2 MiB of their
numeric payload at 256 DOFs, plus linear vectors and row headers. This excludes
the caller's matrices, runtime/operator state, later correction fits, allocator
overhead and RSS. No timing, RSS or remote qualification is claimed, and the
earlier production roundoff resource model does not cover this extra helper.

## Verification

Four retained tests pass for the representation chain, known diagonal mode and
analytic unit shapes, invalid/range boundaries and bounded cancellation/replay.
The full solver regression passes 1,418 tests with zero failures and nine
pre-existing ignored tests across 186 result groups. Following final shared
module cleanup, the four new tests, two existing independent banded-oracle
tests, one scalar-tail test and nine hybrid comparison/control tests were
rerun successfully. These overlapping lanes are not added to the full total.
All 37 Headless modal integration tests pass. Strict all-target Solver/CLI
Clippy with warnings denied, formatting and whitespace checks pass.

Tensor structure and self-tests pass for 13 modules and 11 paradigms, retaining
zero structural gaps. The broader readiness gaps are unchanged: four maturity
gaps, 16 evidence-grade gaps and 11 P0 gaps; daji qualification remains blocked.
Operator profile self-test and registration pass for 59 profiles with
`executed=false`: registration/tensor checks do not run numerical commands.
Documentation book and inventory checks pass, including 26 HTML files.
Organization checks retain zero tracked debt at the source/document line
limits of 800/2,000, and the new files remain below those limits.

From `workers/rust`:

```text
cargo test -p kyuubiki-solver --lib scaled_wide_ --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --lib independent_banded_ --locked --offline
cargo test -p kyuubiki-solver --lib wide_reference_preserves_known_ --locked --offline
cargo test -p kyuubiki-solver --lib hybrid_ --locked --offline
cargo test -p kyuubiki-solver --locked --offline
cargo test -p kyuubiki-cli --test modal_spectrum_operator --locked --offline
cargo clippy -p kyuubiki-solver -p kyuubiki-cli --all-targets --locked --offline -- -D warnings
cargo fmt --all -- --check
```

Native checks run from the repository root:

```text
workers/rust/target/debug/kyuubiki-script-runner check-module-function-coverage-tensor --self-test
workers/rust/target/debug/kyuubiki-script-runner check-module-function-coverage-tensor
workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --self-test
workers/rust/target/debug/kyuubiki-script-runner check-operator-validation
workers/rust/target/debug/kyuubiki-script-runner check-doc-book
workers/rust/target/debug/kyuubiki-script-runner check-doc-inventory
workers/rust/target/debug/kyuubiki-script-runner audit-project-organization
git diff --check
```

## Next Investigation

The sampled error increase is associated with f64 vector representation and
unit recovery, not an inaccurate first root. This does not prove that every
f64 direction fails, that all unit transformations are harmless, or that wide
arithmetic can be removed from consideration. A different representable-vector
search still needs its own bounded work and independent physical acceptance.
Multimode orthogonality, clustered spectral position, generic coordinate
selection and live Agent/installed/remote behavior remain separate open work.
