# Modal Axial-Chain Fast-Path Reliability: 2026-10-02

## Scope And Reproduction

Base revision: `99e494d1`, daji 3.4.3, with the current uncommitted assembly,
component-spectrum and mode-shape reliability overlay. This follow-up repairs
the existing uniform fixed-free axial-chain shortcut for single-mode requests.
The change is private to Solver. Engine routes, request/result schemas,
lumped masses, mode normalization and physical formulations are unchanged.

Two numerical tests and two public tests failed before the repair. The public
tests first failed in their planar branch; after repair they execute both
planar and spatial branches. Assertions were not weakened to obtain a pass.

The first defect was an absolute floor in uniformity comparisons. The old
bound used `tolerance * max(abs(left), abs(right), MIN_POSITIVE)`. For
subnormal stiffness or mass, this could classify substantially different
chains as uniform. The wrong closed-form vector was then rejected by the
residual check. A public chain with element moduli `[1e-318, 2e-318]` and
common density `1e-318` returned a residual error near `0.7071103` in
single-mode mode, although its complete-spectrum solve succeeded.

The second defect was forming `4 * (stiffness / interior_mass)` before the
dimensionless trigonometric factor. A two-element chain with modulus `5e307`,
unit density/area/length and only axial DOFs free has finite matrix entries
and both eigenvalues are representable. Complete-spectrum mode succeeded,
but the shortcut formed infinity and returned an infinite residual error.

These are four failing cases for two numerical defects, not four independent
physics defects. The input extremes are arithmetic stress fixtures, not
recommended physical material parameters or experimental validation data.

## Retained Contract

Uniformity now compares the two values after division by their largest
absolute magnitude. Both values must be finite; two zeros compare equal.
There is no absolute floor that overwhelms subnormal values. The existing
relative admission tolerance of `1e-10` is unchanged. Tiny genuinely uniform
binary-scaled chains remain eligible, while nonuniform stiffness/mass goes
through the existing general tridiagonal path rather than the shortcut.

For `n >= 2` free axial nodes, the retained closed form is:

```text
theta = pi / (2n)
lambda_1 = (k / m_interior) * [4 sin(theta / 2)^2]
v_i proportional to sqrt(m_i) * sin((i + 1) theta)
```

The bracketed factor is below one. Computing it before multiplying by the
stiffness/mass ratio avoids the reproduced intermediate overflow without
changing the physical formula. Zero, negative or non-finite eigenvalues and
invalid mode norms are rejected explicitly before result construction.
Residuals must themselves be finite; invalid tolerance values cannot select
a shortcut or general tridiagonal path. There is no clamping of a failed
eigenvalue into a successful result and no new regularization.

The retained residual policy, including the `2e-4` uniform-chain floor for
large-chain cancellation, is not loosened. Existing sparse product checkpoints
must propagate cancellation through the shortcut before returning success;
the new 128-DOF test checks interruption at completed row 64 and clean replay.
This verifies propagation, not new cancellation coverage for every preparation
loop, durable recovery, checkpoint restart or external process supervision.

## Evidence

Nine numerical cases cover large finite frequencies, subnormal admission,
normal/subnormal/opposite-sign comparisons, six common/independent scale pairs,
nonuniform masses, invalid tolerances, truly unrepresentable frequencies and
residual cancellation/replay. Independent two-DOF references include:

- Uniform chain: eigenvalues `(k/m) * (2 +/- sqrt(2))` and the first normalized
  eigenvector `(1, 1) / sqrt(2)` in mass-normalized coordinates.
- Equal masses with normalized matrix `[[2,-1],[-1,1]]`: lowest eigenvalue
  `(3-sqrt(5))/2`.
- Normalized matrix `[[2,-sqrt(2)],[-sqrt(2),4]]`: lowest eigenvalue
  `3-sqrt(3)`.

An additional near-uniform fixture perturbs the final stiffness diagonal by
two subnormal ULPs at scale `1e-318`. Its closed-form two-DOF reference is
`2 + delta - sqrt(2 + delta^2)`, where `delta` is the relative perturbation.
The former uniform assumption has a relative frequency-squared error above
`1e-6`, despite a residual below the shortcut's retained `2e-4` floor. The
test proves this with the analytic residual and requires the general solver
to match the reference to `1e-12`. Public and headless cases also compare a
two-element chain with a two-ULP stiffness perturbation against its complete
spectrum. This case was added after repair; it is not counted among the four
initial pre-repair test failures.

Seven public tests cover 2D/3D single-mode versus complete-spectrum agreement,
frequency/period/shape consistency across common scales, heterogeneous density,
node permutation with reversed member direction, owned/borrowed equality and
error/replay. The heterogeneous-density chain additionally uses the independent
reference `(5-sqrt(13))/6`. Shape comparisons account for the arbitrary mode
sign; no repeated-eigenvalue direction uniqueness is asserted.

Two new Rust headless cases extend the modal route target to eleven tests.
They construct a batch/plan, resolve the registered bridge and execute the
Engine operator, checking both successful extreme-scale paths and explicit
unrepresentable-frequency failure followed by healthy replay. This is an
in-process non-mock route, not an installed App or distributed Agent test.

| Completed lane | Passed | Failed | Ignored |
| --- | --- | --- | --- |
| macOS ARM64 numerical chain target | 9 | 0 | 0 |
| macOS ARM64 public chain target | 7 | 0 | 0 |
| macOS ARM64 modal headless target | 11 | 0 | 0 |
| macOS ARM64 modal profile, all fourteen commands | 88 | 0 | 0 |
| macOS ARM64 full Solver, debug | 1167 | 0 | 9 |

The full Solver suite completed all 183 result groups for the final source
snapshot. The nine ignored opt-in tests provide no fresh benchmark evidence.
An earlier partial full-suite run
was stopped when the near-uniform case was added and the warning cleanup was
applied; it is not counted as a completed validation lane. Overlapping lanes
must not be added into a coverage percentage.

Solver all-target and modal CLI-target Clippy passed with warnings denied.
Formatting, whitespace, document book/inventory, operator/tensor self-tests
and the source-800/document-2000 organization gates passed. The reliability
guide has 1998 newline-terminated lines and remains within the checker limit.
Tensor structure and command checks pass, but the overall status remains
blocked: 0 structural gaps, 4 maturity gaps, 16 evidence-grade gaps and 11 P0
gaps. This bounded local claim does not remove broader qualification gaps.

The new numerical/public targets join `modal-frame-sanity`; the bounded claim
is registered in the runtime-frame-fields tensor evidence. These executable
assertions are not machine-checked formal proofs or universal solver coverage.

Local SHA-256 values, relative to `workers/rust/crates/`:

| File | SHA-256 |
| --- | --- |
| `solver/src/modal_sparse.rs` | `4483616e067425202f20e313685c260de0927678d8d655f591de14a179fb2691` |
| `solver/src/modal_sparse_chain_tests.rs` | `34ad50f3bf675f03445b0a86e04747eb6bdfcf13e117907c120a217759322317` |
| `solver/tests/modal_chain_reliability.rs` | `2047f49551ecb3ef4b0d73ff9c1ea7164e666332e7000767e153c3fec6ad9954` |
| `cli/tests/modal_spectrum_operator.rs` | `cc8b91e41518c32e9fc835c2e2a1e3f81a1c4c82bdf10e5ae87f2fa59feea2aa` |

## Limits And Reproduction

This is bounded axial-chain fast-path validation, not general modal qualification.
It does not redesign the general Sturm solver, certify arbitrary ill-conditioned
connected matrices, change sparse probe minimality guarantees, or add free-free,
nonlinear, experimental or large-mesh qualification. No speedup or million-node
benchmark claim is made; the repair concerns correct admission and arithmetic.

Linux is intentionally not attempted in this round at the user's request.
No server connection, deployment, cache change or source transfer is performed.
The previous report's pending remote staging cleanup remains unchanged.
Windows, installed-GUI and Python/Elixir routes are not newly verified here.
Affected old results need recalculation; stored results are not rewritten.
Version remains daji 3.4.3, with no commit, push or App rebuild in this round.

Run from the repository root with pinned Rust 1.88.0:

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib modal_sparse::chain_tests
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --test modal_chain_reliability
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-cli --test modal_spectrum_operator
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver
cargo clippy --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --all-targets -- -D warnings
workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile modal-frame-sanity --execute --out tmp/modal-chain-validation.json
workers/rust/target/debug/kyuubiki-script-runner check-module-function-coverage-tensor
```
