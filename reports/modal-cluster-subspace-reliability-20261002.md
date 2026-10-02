# Repeated-Mode Subspace Reliability: 2026-10-02

## Scope And Outcome

Base revision: `99e494d1`, daji 3.4.3, with the existing uncommitted modal
reliability overlay. This follow-up adds fifteen regression tests and their
validation/tensor registration. No new runtime numerical defect was reproduced
in the tested range. The solver, tolerances, iteration/size limits, dispatch,
schemas and physical assembly are unchanged. `modal_math.rs` only gains a
test-module declaration in this round; the CLI target gains a test submodule.

Repeated eigenvalues do not determine unique eigenvector directions. Comparing
mode-by-mode signed vectors can reject an equally valid basis, while comparing
frequencies alone can miss a duplicated or absent direction. These tests use
whole eigenspace projectors and orthogonality, with separate checks for the
frequency splitting of resolvable near-repeated modes.

## Independent References

Five numerical tests form symmetric matrices as `Q diag(lambda) Q^T`, using
explicit orthogonal Walsh-Hadamard columns at 4, 8, 16 and 32 DOFs. Expected
projectors are sums of the prescribed columns' outer products; they are not
derived by another call to the same eigensolver.

- Positive repeated clusters retain their full multiplicity and projector.
- Eight-DOF near-repeated clusters have prescribed gaps `1e-5`, `1e-8` and
  `1e-10`; the measured gap has relative error below `2e-4`, and the combined
  low eigenspace remains within `1e-10` Frobenius distance of the reference.
- Common scales `1e-200`, `1`, `1e200`, coordinate permutations and sign
  transforms retain the reference spectrum, residuals and eigenspaces.
- Repeated negative and zero eigenspaces are retained without filtering or
  relabelling them as positive modes. Zero-root residual/eigenvalue checks
  use the matrix scale, not a nonexistent relative zero-root tolerance.
- A checker control accepts a rotated orthonormal basis of the same subspace,
  but rejects a duplicated direction even when the nominal frequency agrees.

Seven public tests use a two-node clamped 3D beam with the solver's existing
lumped translational and rotary mass formulation. This is a **discrete element
reference**, not a continuum cantilever frequency or experimental calibration.
Let `A=1`, `E=100s`, `G=40s`, `rho=s`, `Iy=Iz=J=0.01` and length `L`.
The six exact discrete eigenvalues are:

```text
lambda_bending_low  = (60 - 12*sqrt(21)) / L^4  [multiplicity two]
lambda_torsion      = 9.6 / L^4
lambda_bending_high = (60 + 12*sqrt(21)) / L^4  [multiplicity two]
lambda_axial        = 200 / L^2
```

The values are sorted independently; changing length can change the ordering
between high bending and axial modes. Multiplying `Iy` by `1+delta` splits
each bending pair by that same factor, with `delta=1e-6` and `1e-8` tested.
Physical bending rotations satisfy `theta=(beta/L)*(n cross displacement)`,
where `beta=sqrt(21)-3` or `-sqrt(21)-3` and `n` is the beam axis.

Public shapes retain their existing unit-Euclidean normalization. Tests
separately whiten each shape with `sqrt(M)` and renormalize before checking
mass orthogonality. For a bending pair, define `T=I-n*n^T`, the cross-product
matrix `C v=n cross v`, and `N=sqrt(1/2+beta^2/24)`.
The independent mass-whitened projector is:

```text
a = sqrt(1/2)/N; b = beta/sqrt(24)/N
P = [[a^2*T, -a*b*C], [a*b*C, b^2*T]]
```

This is derived from the discrete two-by-two bending equations and rigid
coordinate rotation, not copied from the production element-assembly routine.
The tests cover axes aligned with each coordinate, oblique/reversed axes,
lengths `0.5`, `1`, `2`, common material scales from `1e-200` to `1e200`,
reversed node/member topology, and both sides of the implicit local-axis
reference switch. Full-pair projector error is below `1e-9` and pairwise
mass inner products are below `1e-10` in these cases.

Requests for 1 through 6 modes retain their requested count. A truncated
repeated cluster must return valid members of the eigenspace, not pretend
that a particular basis direction is uniquely defined. Completing the pair
must recover the full projector. No automatic cluster expansion or new
output metadata is introduced by these tests.

## Execution And Recovery

A 24-beam fixture has 144 free DOFs, above the 128-DOF single-mode dense
fallback threshold. A solver observer confirms that a one-mode request
actually enters `ModalIteration`. Its result agrees with the analytic lowest
eigenvalue and lies within `1e-5` of the dense reference's complete 48-dimensional
lowest eigenspace. The dense request returns those 48 low modes, not all 144
modes in its output. Its global projector matches the block-diagonal analytic
reference. This validates one bounded sparse-path case, not sparse lowest-mode
certification for arbitrary connected large models.

Three Rust headless regressions live in a separate `clustered.rs` submodule
of the existing modal route target. They build a batch and execution plan,
resolve the bridge and execute the real Engine operator. They verify repeated
and split spectra, physical result fields, mass orthogonality, projectors and
partial mode requests. These are non-mock in-process calls, not an installed
GUI, external Agent or distributed execution.

Both public and headless tests cancel during validation of the second mode,
assert an error inside the operation (before observer-scope exit), and then
rerun the model with fresh control. No partially validated cluster may escape
as a successful result. This is cooperative in-process cancellation/replay,
not durable recovery, process restart or checkpoint migration.

## Validation

| Lane | Result |
| --- | --- |
| macOS ARM64 numerical cluster target | 5 passed, 0 failed |
| macOS ARM64 public cluster target | 7 passed, 0 failed |
| macOS ARM64 modal Rust headless target | 18 passed, 0 failed |
| macOS ARM64 full Solver library | 427 passed, 0 failed, 9 ignored |
| macOS ARM64 complete modal profile, eighteen commands | 133 passed, 0 failed |
| Strict Clippy, formatting and repository gates | Passed |

The final profile and library runs completed with exit status zero after
dimension checks were added to prevent truncated dot-product or projector
comparisons from hiding a short vector. The nine ignored library cases are
eight opt-in benchmarks and one retained physical-reference comparison; they
are not counted as new passes or performance evidence.

Strict Clippy passed for all Solver targets and the modal CLI target, with
warnings denied. Formatting, operator-validation and tensor self-tests,
documentation book/inventory checks and the source-800/document-2000
organization gate passed. The reliability guide uses 1999 newline-terminated
lines. Tensor structure/command checks pass, while overall status remains
blocked: 0 structural gaps, 4 maturity gaps, 16 evidence-grade gaps and
11 P0 gaps. The new bounded numerical/recovery claim does not promote
unrelated modules or close their maturity/evidence-grade gaps.

The full Solver integration suite is not rerun in this test-only round.
The previous [convergence-fix report](modal-jacobi-convergence-reliability-20261002.md)
records its own full-suite run; those historical totals are not counted as
fresh results here. Overlapping lanes must not be added into a coverage
percentage. Generated profile evidence belongs in ignored
`tmp/modal-cluster-validation.json`, not a tracked large output directory.

Final source SHA-256 values, relative to `workers/rust/crates/`:

| File | SHA-256 |
| --- | --- |
| `solver/src/modal_math.rs` | `99f2eeba611627b18a8f50c872042533b5c6628030c46db5b4ccdb7229b79e3b` |
| `solver/src/modal_math_cluster_tests.rs` | `868b19be70bff85caf35911633156584cbb8c8ae528b12cb3dcdd2f577823655` |
| `solver/tests/modal_cluster_reliability.rs` | `b514cdaf61a6e2121dae4f0decca35b6ebe63cf5ddd1bc74bda12bcc68339287` |
| `cli/tests/modal_spectrum_operator.rs` | `7282c060a44cfe7fea487d00b2b8667601286cae95b2953a5302f9dee0d15a23` |
| `cli/tests/modal_spectrum_operator/clustered.rs` | `6a789442d90845736390970dfbea6c0242d5c43a29c4d977d639b881ec44cc22` |

## Boundaries

This is bounded repeated-mode subspace validation, not general modal qualification.
It does not certify arbitrary ill-conditioned, defective or non-symmetric
problems, unlimited multiplicities, nonlinear modal continuation, automatic
mode tracking across parameter paths, continuum discretization convergence,
experimental material accuracy or large-mesh performance. The largest new
public fixture has 144 free DOFs, not a million-node benchmark. Near-degenerate
individual directions can be sensitive even when the combined subspace is
stable; the tests deliberately do not impose unique directions on them.

There are no new packages, build tools or runtime dependencies. The existing
Engine/operator separation is preserved. Linux is intentionally not attempted;
no server access, deployment or container change is performed. Windows,
installed GUI and Python/Elixir routes are not newly verified. Version remains
daji 3.4.3; this round has no commit, push, version bump or App rebuild.

Run from the repository root with pinned Rust 1.88.0:

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib modal_math::cluster_tests
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --test modal_cluster_reliability
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-cli --test modal_spectrum_operator
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib
cargo clippy --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --all-targets -- -D warnings
env CARGO_NET_OFFLINE=true workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile modal-frame-sanity --execute --out tmp/modal-cluster-validation.json
workers/rust/target/debug/kyuubiki-script-runner check-module-function-coverage-tensor
```
