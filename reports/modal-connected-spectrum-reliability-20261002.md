# Connected Sparse-Modal Reliability: 2026-10-02

## Scope And Reproduced Failure

Base revision: `99e494d1`, daji 3.4.3, with the preceding uncommitted modal
reliability overlay. This round fixes the general sparse single-mode search
and adds fifteen regressions: seven numerical, six public and two Rust
headless cases. The public fixture is a connected structure with 134 free
DOFs, not a collection of independent beams and not a large-mesh benchmark.

Before the fix, two tests failed with the original 128-step, `1e-6` relative
residual contract:

- An eight-dimensional diagonal operator with eigenvalues
  `[1, 1.0001, 4, 10, 25, 50, 80, 100]` exhausted the budget at approximately
  `lambda=1.000049`, residual `4.999590e-5`.
- A connected two-row 3D frame with weak rungs exhausted the budget at
  `lambda=1.000012`, residual `4.676217e-5` when requesting one mode. The same
  model's dense two-mode output passed its independent analytic reference.

The old method required both independent scalar inverse probes to converge.
With nearby low eigenvalues, a mixed probe could converge too slowly even
when the uniform probe already represented the lowest mode. Silently ignoring
the unresolved probe would reintroduce the higher-mode false-success risk.

## Numerical Change

Both deterministic starting probes and their mandatory convergence are kept.
Each inverse step now minimizes the Rayleigh quotient in a space spanned by
the normalized inverse image, current vector and previous vector. Two-pass
orthogonalization rejects numerically dependent search directions. The dense
projection is at most three by three; original-operator vectors remain
linear in the number of free DOFs. No full dense matrix or extra inverse
solve per iteration is introduced, but projection adds up to three operator
applications plus vector work. This is not a measured throughput improvement.

Rayleigh-Ritz projection is the standard small-subspace extraction described
in the [SLEPc projection-method documentation](https://slepc.upv.es/release/documentation/manual/eps.html#projection-methods).
The implementation remains the project's native Rust routine; SLEPc is a
method reference, not a new runtime or build dependency. This is not a claim
of implementing or matching SLEPc's complete eigensolver stack.

A two-direction trial still stalled in both reproductions. Retaining the
previous direction was needed. A further regression caught cancellation of
a resolved soft eigenvalue when the projection basis started with the old
mixed vector. The final basis is anchored on the inverse image, so soft
directions are not recovered by subtracting large projected diagonal values.
Diagonal tests retain eigenvalues `1e-20`, `1e-100` and `1e-200` next to
order-one/stiffer directions.

All accepted results still pass the original-operator relative residual
check. The default remains 128 iterations per probe and tolerance `1e-6`.
There is no hidden extended budget or relaxed tolerance. Invalid inverse or
projection callback dimensions, zero inverse vectors, non-finite values,
nonpositive projected spectra and cancellation remain errors. The existing
unresolved-second-probe rejection test is unchanged.

## Independent Connected Reference

The 268-node fixture has two rows of 67 tips. Each tip has one free x
translation and is grounded by a unit-length x-directed beam to a fully
fixed root. Neighboring tips within a row are coupled along y, and matching
tips across rows are coupled along z. All tip rotations and other
translations are fixed. There are 333 members: 134 grounding members, 132
row members and 67 rungs. Every tip belongs to one connected free-DOF graph.

Members have area one, `Iy=Iz=J=1/12`, and `G=0.4E`. Unit-length couplers
therefore contribute transverse stiffness `12EI=E`; grounding members
contribute axial stiffness `EA=E`. Row stiffness is `c=10000`, rung stiffness
is `w`, and grounding stiffnesses are `1-delta` and `1+delta`. Coupler density
is `0.1`; grounding densities account for row degree and the rung, leaving
each free tip with unit lumped mass.

The reduced discrete reference is a path Laplacian plus a two-by-two row
coupling matrix, independent of the production eigensolver:

```text
M = I
B = [[1-delta+w, -w], [-w, 1+delta+w]]
lambda_low/high = 1+w +/- hypot(delta,w)
mu_j = 2*(1-cos(j*pi/67)), j = 0,...,66
complete roots = lambda_low/high + c*mu_j
```

In the tested range, the first two roots have `j=0`, with constant amplitude
along each row. All dimensions, member connectivity and restraints are
constructed procedurally; no large input/output file is tracked.

- Rung strengths `1e-2`, `1e-4`, `1e-6` are crossed with grounding contrasts
  `-0.25`, `-0.01`, `0`, `0.01`, `0.25`: fifteen parameter combinations.
- Single-mode and two-mode requests both agree with their independent roots
  within `1e-8` relative error. Observers confirm entry to `ModalIteration`
  for the sparse request, above the existing 128-DOF dense fallback limit.
- The sparse physical shape has unit norm, exact restrained zeros,
  within-row variation below `1e-6` and row-pair residual below `1e-6`.
- Common stiffness/density scales `1e-60` and `1e60`, reversed node numbering,
  reversed member endpoints and member-order reversal preserve the reference.
- Separate prescribed eight-dimensional rotated spectra cover gaps `1e-2`
  through `1e-8` at common scales `1e-200`, `1`, `1e200`, including a uniform
  seed that is an exact higher mode rather than the first mode.

These are discrete element/mass references, not continuum convergence,
experimental material calibration or automatic branch tracking through
nonlinear changes of model topology. For gaps below the `1e-6` residual
tolerance, the single-mode tests bound lowest-frequency error; they do not
certify that individual nearby directions or their splitting are resolved.

## Execution And Recovery

Two new headless cases use a real Rust execution batch, plan, bridge route
and in-process Engine operator. They check the connected sparse mode and
dense pair, dimensions, shape constraints, frequency fields and cancellation
inside the projected eigensolve. The operation must return an error before
observer-scope exit, and the same valid request must replay successfully.
Numerical and public tests cover the same cancellation boundary separately.

`ModalSweep` can now be observed for the tiny projected solve inside a sparse
iteration; it does not by itself identify full-model dense fallback. Engine
dispatch, schemas, constraints, mass formulation and output normalization
are unchanged. No GUI logic or operator physics is moved into the Engine.

## Validation

| Lane | Result |
| --- | --- |
| macOS ARM64 sparse iteration numerical target | 10 passed, 0 failed |
| macOS ARM64 connected public target | 6 passed, 0 failed |
| macOS ARM64 repeated-cluster public target | 7 passed, 0 failed |
| macOS ARM64 modal Rust headless target | 20 passed, 0 failed |
| macOS ARM64 Solver library | 434 passed, 0 failed, 9 ignored |
| macOS ARM64 complete modal profile, nineteen commands | 148 passed, 0 failed |
| macOS ARM64 full Solver suite, including library/integration/doc harnesses | 1,218 passed, 0 failed, 9 ignored |
| Strict Clippy, formatting and repository gates | Passed |

The full Solver run exited successfully with 185 completed result groups;
the final doc-test harness completed with zero tests. Source and test files
were finalized before this run. The complete modal profile also exited
successfully and its generated artifact has `ok=true`. Strict Clippy passed
for all Solver targets and the modal CLI
target with warnings denied. Formatting, operator-validation and tensor
self-tests, documentation book/inventory, whitespace and the source-800 /
document-2000 organization checks passed. The reliability guide remains at
1999 newline-terminated lines. The nine ignored library tests are the existing
eight opt-in benchmarks and one retained physical-reference comparison, not
new performance evidence. Overlapping lanes must not be summed into coverage.

Tensor structure and command checks pass, but overall status remains blocked:
0 structural gaps, 4 maturity gaps, 16 evidence-grade gaps and 11 P0 gaps.

Final code SHA-256 values, relative to `workers/rust/crates/`:

| File | SHA-256 |
| --- | --- |
| `solver/src/modal_sparse_iteration.rs` | `43b1b325bcf6fe2b6d59b2463e2b6311d0dacfd145d60f59e2c2144ddb4fee0d` |
| `solver/src/modal_sparse_iteration_tests.rs` | `f05520a8e8b38b53d646b170be58a2794aa732a9327fdfc0b0031cc5d093c01b` |
| `solver/tests/modal_connected_reliability.rs` | `e729ff777695e154a2d114d0d64b6360fd6b12f9b3b78367c3595d2fa5dbbe1f` |
| `cli/tests/modal_spectrum_operator.rs` | `3584e95e2a37f022dc1ceadd6d2a5e78d47f0c787a86e5fcc2d3e0ad2f81af21` |
| `cli/tests/modal_spectrum_operator/connected.rs` | `cf7a26c521ed5ae948cd216e229914fa37d6e98245af3a68da190e7016b9d7d1` |

## Boundaries

This is bounded connected sparse-modal validation, not general modal qualification.
Two finite deterministic probes and small residuals are not a mathematical
certificate that the globally lowest mode was found for every matrix. This
does not qualify arbitrary spectral multiplicities, ill-conditioning,
non-symmetric/indefinite systems, nonlinear dynamics or million-node modal
performance. Tightly clustered individual directions need not be unique.

Coverage evidence is registered for `sdk-headless` and `runtime-engine-solver`,
under solver execution/validation and numerical validation/recovery. It
does not promote unrelated tensor coordinates. Generated execution evidence
belongs in ignored `tmp/modal-connected-validation.json`; test logs belong
outside the repository. Historical reports keep their original counts and
source snapshots.

All work is local macOS ARM64 with pinned Rust 1.88.0. Linux is intentionally
not tested. No server access, deployment, Windows run, installed GUI test,
Python/Elixir route verification, new dependency, commit, push, version bump
or App rebuild is part of this round.

Run from the repository root:

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib modal_sparse_iteration
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --test modal_connected_reliability
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-cli --test modal_spectrum_operator
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver
cargo clippy --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --all-targets -- -D warnings
env CARGO_NET_OFFLINE=true workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile modal-frame-sanity --execute --out tmp/modal-connected-validation.json
workers/rust/target/debug/kyuubiki-script-runner check-module-function-coverage-tensor
```
