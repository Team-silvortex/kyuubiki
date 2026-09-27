# Mechanical Q4 Kernel: 2026-09-27

## Scope And Reproduced Defects

This change optimizes the native bilinear plane-stress Q4 kernel shared by
`solve.plane_quad_2d`, `solve.thermal_plane_quad_2d` and the plane-quad host in
`solve.cohesive_interface_mesh_2d`. It does not change the constitutive model,
quadrature, input/output schemas, engine dispatch or frontend.

Base revision: `d7de8968d0eba5e3ed5c08c2ed82ab67ba8aed36`, daji 3.4.1. The
working-tree overlay includes the preceding tetrahedral-kernel changes, which
are separate from this Q4 measurement. Existing Orchestra changes are untouched.

Three new unit tests and three public-solver tests failed before the fix:

- The old Jacobian summed derivatives against absolute coordinates. Exact
  translations changed area, gradients and stiffness despite unchanged local
  edges. On a skew Q4 translated by `2^40`, an affine x-strain of `0.002`
  became `0.0020001868879655636` at a Gauss point.
- Public mechanical and thermal solves inherited that error. A free thermal
  expansion displacement of `0.001` became `0.0010000513283831827` after the
  same exact coordinate translation.
- Finite material parameters could produce infinite constitutive coefficients
  and NaN stiffness while precompute still returned success. The public
  mechanical solver failed later with a generic nonfinite-matrix error rather
  than identifying the element before assembly.

Precompute now subtracts one element origin before Jacobian evaluation. The
stiffness loop calculates `D * B` once per Gauss point, integrates the upper
triangle of `B^T * D * B`, and mirrors each contribution because the existing
isotropic constitutive matrix is symmetric. This removes repeated nine-term
products while preserving all four Gauss points.

Nonfinite local differences, constitutive coefficients, shape gradients,
integrated area and stiffness are rejected before assembly. Mechanical,
thermal and cohesive callers retain element context in error messages.

## Correctness Evidence

- Exact positive/negative `2^20` and `2^40` shifts preserve every Gauss-point
  determinant, gradient, shape function and the complete stiffness matrix.
- Four cyclic node orderings recover analytic affine strain, stress and energy
  on the translated skew cell. Inverted ordering remains rejected by the
  existing positive-Jacobian contract.
- Forty-eight skew/material/cyclic-order cases compare against the previous
  nine-term accumulation, with stiffness symmetry and all three zero-force
  planar rigid-body modes checked. The test-only reference shares unchanged
  shape/Jacobian and constitutive helpers; it is not an independent full solver.
- Public mechanical patch solves check analytic displacement, stress and
  energy. Thermal solves check free expansion and the integrated response of
  a restrained, spatially varying temperature field after translation.
- Cohesive-host displacement, reaction, traction and energy retain their
  reference values. Invalid host coefficients fail and a valid request replays.
- The headless SDK execution plan dispatches the registered Q4 engine route,
  retains analytic response after translation, identifies coefficient overflow
  and successfully replays. This is in-process dispatch, not a network or
  installed-Agent test.

Local macOS ARM64 full solver regression: **997 passed, 0 failed, 8 ignored**
opt-in tests. The mechanical headless target passed all **4** cases, including
the existing extreme-modulus and output-overflow checks. Remote Linux release
regression passed **365** unit tests and **52** integration tests over seven
selected targets, with 0 failures and 8 opt-in unit tests ignored. The new
benchmark was separately executed twice. Overlapping runs are not summed as a
unique coverage percentage.

Strict solver `--all-targets` and mechanical headless-test Clippy checks passed
with `-D warnings`, as did workspace formatting. The structural and thermal
validation profiles executed all **12** and **16** commands successfully.
Tensor self-test/structure, documentation book/inventory and the 800-line source
/ 2000-line document organization gates passed. The tensor records bounded
`verified` numerical/recovery and benchmark evidence only: 0 structural gaps,
4 maturity gaps, 16 evidence-grade gaps and 11 P0 gaps remain, with the overall
release status still blocked rather than promoted by this change.

This is bounded Q4 kernel validation, not general mechanical or thermoelastic qualification.
The existing dimensional Jacobian threshold and 2x2 integration are unchanged.
This does not add tiny-cell scale invariance, nonlinear material response,
large deformation or general contact qualification. Input coordinates must
still retain their local detail in `f64`; origin subtraction cannot restore
information already rounded away. Extreme finite inputs may remain outside
the representable arithmetic range. Historical translated-Q4 results that will
be reused should be recomputed; this change does not mutate stored artifacts.

## Remote Paired Precompute

Host: Linux x86_64 physical server, AMD Ryzen 7 7735H, 8 cores / 16 logical CPUs.
Compiler: rustc 1.88.0, standard Cargo release profile. No CPU affinity or
frequency lock was imposed, so these are host-specific point estimates.

Both paths run in the same process/build on 16 ordinary shape/scale fixtures.
Each run uses 3 warm-ups and 9 measured samples of 100,000 complete element
precomputations. Old/new order alternates. Coordinates, material parameters and
returned kernels are black-boxed; non-inlined wrappers are used on both sides.
Fixture construction and correctness comparisons are outside timing. The new
path includes the added range checks; the old path omits them as before.

| Run | Previous median ns/element | Current median ns/element | Time reduction |
| --- | ---: | ---: | ---: |
| First | 421.443 | 299.398 | 28.96% |
| Repeat | 427.998 | 302.320 | 29.36% |

Repeat samples, ns/element, chronological measured order:

```text
previous_ns=[421.50198,422.45702,423.27841,426.98655,429.93461,431.5216,428.80652,427.99805,433.24719]
current_ns=[301.1572,300.70218,299.53214,303.22246,303.34337,302.40176,302.31951,301.56238,302.58924]
```

This is a Q4 precompute microbenchmark, not end-to-end solver throughput or a 1M-node qualification.
It excludes global assembly/factorization, thermal equivalent-load and result
recovery, nonlinear interface iteration, transport and UI. No global speedup
or memory-capacity claim is made.

## Reproduction And Provenance

Run from the repository root, with the pinned toolchain:

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-solver
cargo test --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-cli --test mechanical_output_operator
cargo test --manifest-path workers/rust/Cargo.toml --locked --release -p kyuubiki-solver --lib q4_precompute_paired_benchmark -- --ignored --nocapture --test-threads=1
cargo clippy --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-solver --all-targets -- -D warnings
workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile plane-2d-patch-closed-form --execute --out tmp/q4-mechanical-validation.json
workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile thermal-plane-patch --execute --out tmp/q4-thermal-validation.json
```

The remote release correctness run selected `--lib` and these seven integration
targets: `plane_quad_geometry_reliability`, `plane_quad_isoparametric`,
`plane_2d_closed_form`, `thermal_plane_quad_isoparametric`,
`thermal_plane_mesh_refinement_regression`, `cohesive_interface_mesh_2d_closed_form`
and `cohesive_mesh_convergence_reliability`.

The remote run used a source-only scratch directory and the existing managed
Cargo cache. No installed services or server configuration were changed.
Local and remote source SHA-256 values matched, with paths relative to
`workers/rust/crates/solver/src/`:

| File | SHA-256 |
| --- | --- |
| `plane_2d_quad.rs` | `9ed66eaac86030a9727bbf6628edaaffde320aaec9acb2b4b63ab126c5446e3c` |
| `plane_quad_kernel_reference.rs` | `f2c15f89e997580cf82c377e349e743da096f72319072bef3a7e9f273d081d6c` |
| `plane_quad_kernel_benchmark.rs` | `86bd613a2d98d8d34f8a473877ec9726dc2401876e3e85e534c88ec0144a53ed` |
