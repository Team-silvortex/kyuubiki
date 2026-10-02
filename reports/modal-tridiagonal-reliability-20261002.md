# General Tridiagonal Modal Reliability: 2026-10-02

## Scope And Reproduction

Base revision: `99e494d1`, daji 3.4.3, with the current uncommitted modal
assembly, component-spectrum, shape-recovery and uniform-chain reliability
overlay. This follow-up changes the private general tridiagonal lowest-mode
solver, not the uniform-chain shortcut, dense complete-spectrum solver,
Engine routes, request/result schemas or physical mass/stiffness formulation.

Two numerical tests and one public test failed before the algorithm change:

- A normalized operator with diagonals `[1, 2e-24]` and coupling
  `-sqrt(2)*1e-24` could not resolve its low mode within the former 96
  halvings of a global interval. It failed the numerical target's `1e-9`
  residual requirement, with residual near `5.294780e-31`.
- Diagonals `[1e-24, 1]` and coupling `1e-15` exposed the absolute
  `EPSILON` pivot substitution. A small positive pivot became a much larger
  negative pivot; the target returned a residual error near `5.002510e-15`.
- A public two-member fixed-free axial frame with unit geometry/density
  and moduli `[1, 1e-24]` passed the existing `1e-6` solver residual bound
  but failed a `1e-12` relative eigenvalue comparison with its analytic
  reference. Smaller soft moduli are also exercised after repair.

The public reproduction initially compared complete spectra as well; that
revealed a separate dense-solver residual failure. The test was split before
the algorithm change into an independent analytic single-mode assertion.
Complete-spectrum comparisons now use the separately stated resolved range;
the extreme complete-spectrum gap is not counted as repaired or hidden by a
relaxed assertion. Pre-repair public execution stopped in the planar branch;
successful post-repair runs execute both planar and spatial branches.

These extreme coefficients are floating-point stress fixtures, not recommended
material parameters, experiments or material-model qualification.

## Numerical Contract

The general solver now lives in `solver/src/modal_tridiagonal.rs` with a
separate numerical test module. Sparse routing and the uniform analytical
shortcut remain in `modal_sparse.rs`. Storage remains linear in the DOF
count; no dense conversion or new dependency is introduced.

The first positive eigenvalue is bracketed between zero and the smallest
diagonal, a Rayleigh upper bound. Sturm inertia at zero must contain no
nonpositive modes. Bisection stops at adjacent floating-point endpoints or
a relative interval width of two machine epsilons, with a bounded 1152-step
safety cap covering normalized f64 exponent and mantissa range. This is
not a claim that every ill-conditioned matrix has accurately computable
eigenvalues; subtraction can still lose unresolved spectral information.

Only an exact zero pivot uses a negative minimum-subnormal one-sided
convention. Small nonzero pivots retain their magnitude and sign. The term
`b^2 / pivot` is evaluated by division before multiplication, with a binary
balancing fallback if the division overflows. This avoids losing a small
coupling's square before a compensating division. Nonzero coefficients lost
inside this solver's normalization fail explicitly instead of being silently split.

Mode recovery joins left and right eliminations at an anchor selected by the
smallest finite reciprocal inverse diagonal. It no longer starts a one-way
recurrence from an arbitrarily tiny endpoint component. The resulting unit
vector must pass the existing relative residual requirement. Nonpositive,
non-finite, disconnected, dimensionally invalid and unrepresentable cases
cannot publish a successful eigenpair. No failed eigenvalue is regularized
and no residual tolerance is increased.

The numerical background is interval/Sturm selection as described in
[LAPACK DSTEBZ](https://www.netlib.org/lapack/double/dstebz.f) and two-sided
inverse-column recovery as described in
[LAPACK DLAR1V](https://github.com/Reference-LAPACK/lapack/blob/master/SRC/dlar1v.f).
This is a bounded local implementation, not a port of LAPACK's full algorithms
or a claim of their qualification. No LAPACK runtime dependency is added.

Preparation, Sturm scans, bisection, two-sided recovery, normalization and
residual scans have cooperative checkpoints. Tests interrupt inside the
operation, not just at observer-scope exit, and then replay with fresh control.
This is in-process cancellation, not durable recovery or Agent restart.

## Evidence

Eleven numerical tests include:

- Independent two-by-two references computed as `det(A)/lambda_max` to
  avoid cancellation between almost equal roots, with soft terms down to
  `1e-300` and both diagonal orders.
- Fixed-end discrete sine references at 2, 3, 17, 128 and 257 DOFs, across
  scales `1e-200`, `1` and `1e200`, including alternating coupling signs.
- Interior-localized soft modes at three positions in a 65-DOF chain.
- Forty-eight deterministic positive-definite heterogeneous matrices
  cross-checked against the dense solver at 2, 3, 7 and 16 DOFs.
- Exact-zero and small-positive Sturm pivots, balanced quotient range,
  invalid input, nonpositive spectrum, representability and seven
  cancellation checkpoints followed by clean replay.

Three added public tests extend the chain target to ten. For a two-member
chain with moduli `[1,s]` and unit density/length/area, the reduced normalized
matrix is `[[1+s,-sqrt(2)*s],[-sqrt(2)*s,2s]]`; its determinant is `2s`.
The analytic low root is `2s/lambda_max`. For `s <= 1e-24`, this equals `2s`
to the tested relative accuracy. The recovered physical displacement ratio
at the middle node is approximately `s`, with unit tip displacement after
normalization. Both 2D/3D and reversed node/member ordering are checked.
Complete spectra are cross-checked at `s = 1e-2, 1e-4, 1e-8` only.

Two added Rust headless cases extend the route target to thirteen. They build
a batch and execution plan, resolve the bridge and execute the real Engine
operator, checking low frequency, small physical shape components and
cancellation/error propagation followed by replay. This is a non-mock
in-process route, not an installed App, external Agent or distributed run.

| Lane | Result |
| --- | --- |
| macOS ARM64 numerical tridiagonal target | 11 passed, 0 failed |
| macOS ARM64 public chain target | 10 passed, 0 failed |
| macOS ARM64 modal headless target | 13 passed, 0 failed |
| macOS ARM64 complete modal profile, fifteen commands | 104 passed, 0 failed |
| macOS ARM64 full Solver, debug | 1181 passed, 0 failed, 9 ignored |
| Strict Clippy, formatting, document/tensor/organization gates | Passed |

The final full Solver run completed all 183 result groups with exit status
zero after the source/test snapshot was frozen. The nine ignored opt-in
cases provide no new benchmark evidence. The executed profile summary is
`tmp/modal-tridiagonal-validation.json` (ignored generated evidence).

Clippy passed for all Solver targets and the modal CLI target with warnings
denied. The operator-validation and tensor self-tests passed, as did the
documentation book/inventory and source-800/document-2000 organization gates.
The reliability guide remains at 1998 newline-terminated lines. Tensor
structure and command checks pass, while overall status remains blocked:
0 structural gaps, 4 maturity gaps, 16 evidence-grade gaps and 11 P0 gaps.

The profile and tensor register this bounded numerical/recovery evidence.
Overlapping test lanes must not be added into a coverage percentage.
The assertions are executable regressions, not machine-checked formal proofs.

Local SHA-256 values, relative to `workers/rust/crates/`:

| File | SHA-256 |
| --- | --- |
| `solver/src/modal_tridiagonal.rs` | `37712b1e65591fb5a12862fa279bcc24ffbf7c81bd3a2d72e04e71f21baab622` |
| `solver/src/modal_tridiagonal_tests.rs` | `74d389fd94eca53cdb76c7764285ad7437d98e534f8597b0529aaad1c903a8c4` |
| `solver/tests/modal_chain_reliability.rs` | `ef59fa2c04385115b6573553f27a3f6dbfd6da55509427400933d378279a6db9` |
| `cli/tests/modal_spectrum_operator.rs` | `75e8afcb44af3ffbdd0182c526e448c62ee01f02b43101465c7b5e8dfb7e8e0f` |

## Remaining Boundary

This is bounded general-tridiagonal lowest-mode validation, not general modal qualification.
The dense complete-spectrum path is not changed here. At stiffness ratio
`[1,1e-24]`, its local Jacobi coupling threshold can stop before the tiny
shape component is recovered; the final relative-residual gate rejects the
result (`modal frame mode 0 failed its relative residual check`). This remains
an explicit follow-up gap, not a successful complete-spectrum case.

Subsequent work in the [complete-spectrum convergence report](modal-jacobi-convergence-reliability-20261002.md)
addresses this specific stopping-rule failure with separate tests and evidence.
The results and source hashes above describe the earlier tridiagonal snapshot.

Arbitrary ill-conditioned matrices, clustered/repeated spectra, general
sparse probe minimality, experimental material accuracy, free-free modes and
large-mesh performance remain outside this evidence. The largest new numeric
fixture is 257 DOFs, not a million-node benchmark. Existing ignored opt-in
tests do not establish new scale coverage.

Linux is intentionally not attempted at the user's request. No remote
connection, transfer, cleanup, deployment or container change is performed.
Windows, installed GUI and Python/Elixir routes are not newly verified.
Prior computed outputs are not rewritten and need recalculation if affected.
Version remains daji 3.4.3; this round has no commit, push or App rebuild.

Run from the repository root with pinned Rust 1.88.0:

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib modal_tridiagonal::tests
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --test modal_chain_reliability
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-cli --test modal_spectrum_operator
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver
cargo clippy --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --all-targets -- -D warnings
workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile modal-frame-sanity --execute --out tmp/modal-tridiagonal-validation.json
workers/rust/target/debug/kyuubiki-script-runner check-module-function-coverage-tensor
```
