# Complete-Spectrum Jacobi Convergence: 2026-10-02

## Scope And Reproduction

Base revision: `99e494d1`, daji 3.4.3, with the current uncommitted modal
assembly, component-spectrum, shape-recovery and chain reliability overlay.
This follow-up changes the private Jacobi stopping rule in `modal_math.rs`.
It does not change single-mode routing, Engine dispatch, public schemas,
mass/stiffness assembly, the physical model or the installed applications.

Three regression tests failed before the stopping-rule change:

- The local convergence predicate treated a coupling `-sqrt(2)*s` between
  diagonals `[1,2s]` as resolved at `s=1e-24`.
- The complete-spectrum numerical test then retained a coordinate-basis
  vector instead of its small coupled component. Its residual was about
  `1.414213562373095e-24` against an eigenvalue near `2e-24`, giving a
  relative residual near `0.7071` rather than the required `1e-12`.
- A public two-member fixed-free axial frame with moduli `[1,1e-24]`
  failed with `modal frame mode 0 failed its relative residual check`.
  Pre-repair public execution stopped in the planar branch; successful
  post-repair tests exercise both planar and spatial branches.

These are three reproductions of one stopping-rule defect, not three
independent physics defects. The extreme coefficients are floating-point
stress fixtures, not recommended material parameters or experimental data.

## Numerical Contract

The old threshold compared each coupling with `1e-12 * sqrt(abs(a)*abs(b))`
(implemented as a product of square roots). A stiff neighbor could therefore
hide a coupling that was large relative to the soft eigenvalue. The resulting
eigenvalues could look accurate even while the soft eigenvector was unusable.

The revised predicate requires `abs(coupling) / min(abs(a),abs(b)) <= 1e-12`.
Exact zero coupling is resolved; a nonzero coupling adjacent to a zero
diagonal requires a rotation. Division avoids an absolute scale floor or
premature underflow in a tolerance-times-small-diagonal product. Absolute
diagonal values preserve shared numerical use for indefinite spectra.

In exact arithmetic, for the rotated matrix `Q^T A Q = D + E`, satisfying
`abs(E_ij) <= tau * min(abs(D_ii),abs(D_jj))` bounds each nonzero mode's
column residual by `sqrt(n-1) * tau * abs(D_jj)`. At the unchanged dense cap
of 4096 DOFs this factor is below `6.4e-11`. This is an algebraic rationale,
not a floating-point formal proof. The existing original-operator residual
gate at the frame caller remains necessary and unchanged at `1e-8`.

The 40-sweep limit, `1e-12` Jacobi tolerance, dense size limit and explicit
nonconvergence errors are unchanged. There is no diagonal regularization,
weak-coupling truncation, mass adjustment or new dependency. Numerical
Jacobi still retains zero and negative eigenvalues for its other callers;
restrained modal frames retain their existing positive-mode admission rule.

## Evidence

Eight numerical tests are isolated in `modal_math_convergence_tests.rs`:

- Two-by-two analytic soft spectra down to `s=1e-300`, in both diagonal
  orders, including original-matrix relative residuals and tiny components.
- Symmetric stopping behavior across signs and scales, subnormal values,
  exact zeros and the absence of an absolute tolerance floor.
- Zero and negative spectra without regularization, including a rank-one
  matrix and a matrix with zero diagonal but nonzero off-diagonal entries.
- A connected four-DOF star with soft scales `1e-24`, `1e-100`, `1e-200`,
  permuted coordinates, sign transforms, residuals and orthogonality.
- Common matrix rescaling, sweep-budget exhaustion, cancellation during a
  required rotation sweep, and clean in-process replay.

Four new public tests extend `modal_chain_reliability` to fourteen. For the
unit-density/area/length axial chain with moduli `[1,s]`, the reduced mass
matrix is `diag(1,0.5)` and the normalized operator is
`[[1+s,-sqrt(2)*s],[-sqrt(2)*s,2s]]`. Its determinant is `2s`; the low root
is `2s/lambda_max`, approaching `2s` to the tested relative accuracy. The
physical low-mode middle/tip ratio approaches `s`; the high-mode tip/middle
ratio approaches `-2s`. Tests compare complete and single spectra with this
independent reference rather than accepting agreement between two solvers
alone. Both 2D/3D routes, member/node reversal, common material rescaling
and normalized physical shape components are exercised.

Owned and borrowed entry points agree for the same request. Default and
explicit complete-spectrum requests preserve their distinct echoed inputs
while producing identical computed output. An initial whole-result equality
assertion was corrected to respect that provenance; production output was
not altered to erase the request difference.

Two added Rust headless tests extend `modal_spectrum_operator` to fifteen.
They create a batch and execution plan, resolve the bridge and execute the
real Engine operator. They check both frequency branches, tiny physical
shape components, cancellation before success and fresh-control replay.
This is a non-mock in-process route, not an external Agent, installed GUI
or distributed execution. Engine/operator separation is unchanged.

| Lane | Result |
| --- | --- |
| macOS ARM64 Jacobi convergence target | 8 passed, 0 failed |
| macOS ARM64 public chain target | 14 passed, 0 failed |
| macOS ARM64 modal headless target | 15 passed, 0 failed |
| macOS ARM64 full Solver library | 422 passed, 0 failed, 9 ignored |
| macOS ARM64 complete modal profile, sixteen commands | 118 passed, 0 failed |
| macOS ARM64 full Solver, debug | 1193 passed, 0 failed, 9 ignored |
| Strict Clippy, formatting and repository gates | Passed |

The final full Solver run completed all 183 result groups with exit status
zero after the source/test snapshot was frozen. The nine ignored opt-in
cases comprise eight benchmarks and one retained physical-reference check;
they are not counted as passed or as new scale/performance evidence. The
executed profile summary is `tmp/modal-jacobi-validation.json` (ignored
generated evidence). Overlapping lanes are not additive coverage percentages.

Clippy passed for all Solver targets and the modal CLI target with warnings
denied. Operator-validation and tensor self-tests, documentation book and
inventory checks, formatting and the source-800/document-2000 organization
gate passed. The reliability guide remains at 1998 newline-terminated lines.
The new bounded claim is registered in the existing tensor coordinates for
`runtime-engine-solver` and `sdk-headless`, numerical validation and recovery.
Tensor structure/command checks pass but overall status remains blocked:
0 structural gaps, 4 maturity gaps, 16 evidence-grade gaps and 11 P0 gaps.
These regressions do not close unrelated maturity or evidence-grade gaps.

Local SHA-256 values, relative to `workers/rust/crates/`:

| File | SHA-256 |
| --- | --- |
| `solver/src/modal_math.rs` | `d39d1a258db814d9b7066963cfb373d9d3f0bc77f1ffec62df86fa4f728db1f7` |
| `solver/src/modal_math_convergence_tests.rs` | `43ac98eeed30e295c4469f6f8e8ac80f080323ab642ed67295d8775e1fc712fb` |
| `solver/tests/modal_chain_reliability.rs` | `644223d2a9b011e2f633b1efa27ddd87e972f89215ecd0cdcabd0968a91e9b4d` |
| `cli/tests/modal_spectrum_operator.rs` | `e6e8ecab841ffafb38bda72cfd9da58f8e46c2ca2c258ce368960e1e57cb08d1` |

## Remaining Boundary

This is bounded complete-spectrum convergence validation, not general modal qualification.
The largest new numerical fixture is four DOFs and the public models have
three nodes and two members. This is not new large-mesh performance evidence.
Arbitrary ill-conditioning, clustered/repeated spectra, general sparse
lowest-mode certification, free-free frame modes and experimental material
accuracy remain outside this evidence. More stringent convergence cannot
recover information already lost in assembly or make an unrepresentable
connected operator representable; existing explicit range errors remain.

Subsequent [repeated-mode subspace checks](modal-cluster-subspace-reliability-20261002.md)
add bounded cluster evidence without changing the runtime solver. The test
counts and hashes above describe this earlier convergence-fix snapshot.

Linux is intentionally not attempted at the user's request. No remote
connection, transfer, cleanup, deployment or container change is performed.
Windows, installed GUI and Python/Elixir routes are not newly verified.
Prior computed outputs are not rewritten and need recalculation if affected.
Version remains daji 3.4.3; this round has no commit, push or App rebuild.

Run from the repository root with pinned Rust 1.88.0:

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib modal_math::convergence_tests
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --test modal_chain_reliability
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-cli --test modal_spectrum_operator
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver
cargo clippy --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --all-targets -- -D warnings
env CARGO_NET_OFFLINE=true workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile modal-frame-sanity --execute --out tmp/modal-jacobi-validation.json
workers/rust/target/debug/kyuubiki-script-runner check-module-function-coverage-tensor
```
