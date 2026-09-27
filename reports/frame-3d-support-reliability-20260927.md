# Spatial Frame Support Reliability: 2026-09-27

## Scope And Reproduction

Base revision: `6ab4c2dd`, daji 3.4.2, plus the current working-tree overlay.
This follows the [spatial output repair](frame-3d-output-reliability-20260927.md)
and covers exact directional translation/rotation supports in
`solve.thermal_frame_3d`. It also adds spring assembly/energy range controls.
The Engine remains a dispatcher; all new numerical implementation stays in
the Solver. Public input/output schemas, runtime services and GUI are unchanged.

After correcting the new fixture's required nodal load fields, **five of six**
initial public Solver regressions failed against the preceding implementation:

- For independent supports `[1,0,0]` and `[1,1e-9,0]`, the free-motion map was
  accepted but reaction recovery failed with `system is singular`. This also
  happened when the first direction was a fixed-axis node flag. Translational
  and rotational reactions share the affected path.
- At separation `1e-3`, the reaction expected to be `10000` was
  `9999.999999275224`, already outside the retained `2e-12` relative bound.
- A rotated two-support plane and three oblique supports failed during
  reaction recovery. Single-pass orthogonalization could also retain a
  roundoff direction as an additional physical degree of freedom.
- Four rotated support directions reached `3 - constrained_basis.len()` with
  an invalid rank and panicked with integer subtraction overflow in debug.
  Invalid model input must return a normal error, not unwind this Solver path.

The initial dependent-direction rejection/replay control already passed.
These are five failing regression cases, not five independent defects.
The first fixture parse failure is not counted as a product failure.

## Numerical Contract

For the matrix `N` whose columns are normalized support directions, the local
factorization is `N = Q R`. Two-pass modified Gram-Schmidt builds orthonormal
columns before normalizing a small remainder. The same small factorization
implementation constructs the free complement and recovers reactions:

```text
N^T u = 0
u = Q_free * u_reduced
r = K*u - f
R * lambda = Q^T * r
reaction_vector = N * lambda
```

The previous normal-equation path solved `(N^T N)*lambda = N^T*r`, losing
separation information for nearly parallel directions. Reaction recovery now
uses triangular substitution directly. QR for full-column-rank least-squares
problems is documented by the primary
[LAPACK user guide](https://www.netlib.org/lapack/lug/node27.html).
This is a small Solver-local implementation, not a LAPACK dependency or an
executed comparison against LAPACK or another physical solver.

The factorization uses temporary stack-sized 3-by-3 arrays. It does not store
a dense matrix for every node or introduce an Engine cache. A block may have
at most three independent directions. The existing normalized orthogonal
remainder cutoff `1e-10` remains; related or overcomplete directions are
rejected before allocating a negative free dimension. Fully constrained
blocks produce exactly zero free basis vectors. Factorization and substitution
retain cooperative cancellation, and non-finite reaction coefficients fail
before a successful public result can contain JSON null.

The analytic two-support oracle uses a tip load `P=10` along `y`, with support
directions `n1=[1,0,0]` and `n2=[1,epsilon,0]/sqrt(1+epsilon^2)`. Both tip `x`
and `y` are restrained, so:

```text
lambda1 = P/epsilon
lambda2 = -P*sqrt(1+epsilon^2)/epsilon
lambda1*n1 + lambda2*n2 = [0,-P,0]
```

The same identity holds for a nodal `y` moment and rotational constraints.
The tests sweep `epsilon = 1e-3, 1e-5, 1e-7, 1e-9, 2e-10`. These large opposing
reactions are physically ill-conditioned: the repair avoids unnecessary
normal-equation accuracy loss, not the sensitivity inherent in the supports.
A rotated specimen retains the independent free `z` bending displacement
`Pz*L^3/(3*EIy)=0.004` and energy `0.006`; it checks support-order changes,
reaction-vector balance and constrained displacement separately.

Additional controls combine uniform temperature, both gradients, translational
and rotational springs. Reparameterizing the same support plane, including
negative and very large finite direction scales, preserves node/member/spring
results. Three finite `1e308` springs on a fixed root must not hide an invalid
assembled stiffness. An axial/torsional spring example has finite member and
spring-group energies but an unrepresentable combined total at doubled load;
the correct outcome is an explicit summary-range error.

## Execution Evidence

These counts and hashes record the support repair's snapshot. The later
[section-orientation repair](frame-3d-orientation-reliability-20260927.md) has
separate execution totals and further headless coverage.

The new Solver target contains **11 passing tests**. Three new headless cases
extend the existing spatial target to **7**. Those tests create a Rust batch,
validate its execution plan, resolve the bridge manifest and dispatch through
the registered Engine operator. They check near-parallel reactions, dependent
support rejection, unrepresentable reactions and fresh replay. This is not a
standalone installed-Agent or network-failure test.

| Completed lane | Passed | Failed | Ignored |
| --- | --- | --- | --- |
| macOS ARM64 new Solver target | 11 | 0 | 0 |
| macOS ARM64 spatial headless target | 7 | 0 | 0 |
| macOS ARM64 full Solver, debug | 1103 | 0 | 9 |
| Linux x86_64 selected Solver, release | 469 | 0 | 9 |
| Linux x86_64 spatial headless target, release | 7 | 0 | 0 |

The complete macOS Solver run finished 179 result groups successfully. Solver
all-target Clippy, spatial CLI-target Clippy (both warnings denied), workspace
formatting and whitespace checks passed. The thermal-frame-3d validation profile
executed all 11 registered commands successfully. Document book/inventory,
tensor structure/self-test and source-800/document-2000 organization gates
passed. The reliability guide is 1993 lines; the new numerical module is 112
lines and its integration test target is 429 lines.

The separate bounded claim is registered in
`config/architecture/module-function-coverage-evidence/runtime-frame-fields.json`.
The tensor retains 0 structural gaps, 4 maturity gaps, 16 evidence-grade gaps
and 11 P0 gaps; its overall release status is still blocked. This repair does
not promote general physical qualification or close unrelated readiness gaps.

Both hosts use pinned Rust **1.88.0**, `--locked --offline`. Linux runs `--lib`
plus `frame_3d_support_reliability`, `frame_3d_output_reliability`,
`thermal_frame_3d_closed_form`, `thermal_frame_3d_review`,
`thermal_frame_objectivity`, `thermal_frame_mesh_convergence`,
`thermal_frame_directional_constraint`, `thermal_frame_directional_spring`,
`frame_input_reliability`, `frame_3d_closed_form`, `frame_3d_objectivity`,
`frame_3d_review`, `mechanical_convergence`, `modal_frame_3d_review`,
`modal_frame_sanity_regression` and `frame_2d_output_reliability`.
Its 17 result groups contain 372 unit and 97 selected integration tests.
Overlapping lanes must not be added into a test-coverage percentage. The nine
ignored opt-in tests were not executed and do not constitute benchmark evidence.

Local and remote SHA-256 match for these paths relative to `workers/rust/crates/`:

| File | SHA-256 |
| --- | --- |
| `solver/src/thermal_frame_3d_constraint_basis.rs` | `95cedb8c538e7ad7fad9b650c311eb50fb9ba7c71f5d3c1766cbd1260d61f55d` |
| `solver/src/thermal_frame_3d_constraints.rs` | `a147c468039537a6b47f2c6825d3d48472ffd5b16b6079f8fa53df3deaa99cf8` |
| `solver/tests/frame_3d_support_reliability.rs` | `ab07743d5d92b1e9e4b037ad396c9f4b583ee09a38c1c947ec88177be0192724` |
| `cli/tests/frame_3d_output_operator.rs` | `81143af3108bef33a38eca3b46acae7d19e3664c4a1f07cb4dccd58becfd1433` |

## Bounds And Commands

This is bounded spatial-frame support validation, not general frame qualification.
No new physical formulation, nonlinear/contact capability, general condition-
number guarantee, GUI, Windows, installed-Agent, performance or 1M-node claim
is made. Existing qualified physical scopes remain unchanged. Closely aligned
supports require physically appropriate tolerances and accurate input geometry;
valid finite inputs do not guarantee that all derived quantities fit in `f64`.
Cancellation means interrupt and start a fresh solve, not resume an intermediate
factorization. Previous support-sensitive results should be recalculated before
research reuse. Stored results are not rewritten by this change.

Remote runs use temporary source-only scratch space and the existing managed
Cargo cache. The temporary source directory was removed after all remote runs
completed; the managed cache was retained. No service configuration, credentials or build bundles enter the
repository. No version bump, commit, push or application installation is part
of this repair.

Run from the repository root:

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-solver --test frame_3d_support_reliability
cargo test --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-cli --test frame_3d_output_operator
cargo test --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-solver
cargo clippy --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-solver --all-targets -- -D warnings
cargo clippy --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-cli --test frame_3d_output_operator -- -D warnings
workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile thermal-frame-3d-closed-form --execute --out tmp/thermal-frame3d-support-validation.json
workers/rust/target/debug/kyuubiki-script-runner check-module-function-coverage-tensor
```
