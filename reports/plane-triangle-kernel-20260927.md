# Mechanical CST Triangle Kernel: 2026-09-27

## Scope And Reproduced Defects

This repair covers the native constant-strain plane-stress triangle shared by
`solve.plane_triangle_2d`, `solve.thermal_plane_triangle_2d` and the triangle
host of `solve.cohesive_interface_mesh_2d`. Engine dispatch, task schemas,
constitutive laws and UI are unchanged. No new runtime dependency is introduced.

Base revision: `d7de8968d0eba5e3ed5c08c2ed82ab67ba8aed36`, daji 3.4.1.
The working-tree overlay includes the preceding tetrahedral and Q4 repairs;
the existing Orchestra edits are separate and untouched.

Before the repair, all three initial kernel regressions and all four new
public-solver regressions failed, as did the extended cohesive-host test:

- Both orientations were accepted, but the strain-displacement matrix divided
  by the absolute area. Clockwise connectivity reversed strain and signed
  stress. An affine x-strain of `0.002` became `-0.002`; a tensile patch stress
  of `4` became `-4`. Stiffness uses B twice, so displacement and energy alone
  could conceal the error.
- Thermal equivalent forces use B once. A mixed-orientation patch returned an
  x-displacement of `-0.00025` instead of the free-expansion value `0.001`.
  A nonuniform-temperature case also changed with element orientation.
- The cohesive host reported a y-strain of `-0.01` instead of `0.01` even when
  the displacement/reaction solution still looked correct.
- Finite extreme elastic parameters produced infinite D and NaN stiffness
  while element precompute returned success. Public solve failed later with
  a generic nonfinite-matrix error, without the responsible element ID.

Gradients now retain signed area; integration still uses positive area.
Precompute rejects nonfinite area, gradients, constitutive coefficients and
stiffness. Mechanical and thermal callers attach element context to the error.
The stiffness calculation uses the known zero entries of isotropic D and CST B
to form symmetric 2x2 nodal blocks rather than full dense matrix products.

## Correctness And Limits

- All six node orders on a skew cell recover independently specified affine
  strain, signed stress and energy, including exact positive/negative `2^40`
  translations. Thermal forces preserve affine virtual work and DOF permutation.
- Seventy-two skew/material/order combinations match the old positive-orientation
  dense stiffness after DOF permutation. All three planar rigid modes and exact
  stiffness symmetry are checked. This frozen reference shares unchanged area
  and constitutive helpers; it is not an independent full solver.
- Two-element public patches exercise neither, either or both triangles
  reversed. Mechanical response matches analytic displacement, signed stress
  and energy. Thermal checks cover uniform free expansion, nonuniform nodal
  temperatures and restrained compressive stress.
- All six cohesive-host orders retain displacement, reaction, traction, signed
  strain/stress and energy. Overflow errors identify the mechanical/thermal
  element, and subsequent valid solves succeed.
- The Rust headless execution plan resolves the registered mechanical engine
  route and checks signed stress for all four mesh-orientation combinations.
  This is in-process dispatch, not a network or installed-Agent test.

Final macOS ARM64 full solver regression: **1005 passed, 0 failed, 9 ignored**
opt-in tests. The mechanical headless target passed all **5** cases, including
the retained extreme-modulus and overflow checks. Remote Linux release
regression passed **369** unit tests and **53** integration tests over the nine
selected targets, with 0 failures and 9 opt-in unit tests ignored. The final
paired benchmark was executed separately twice. Overlapping runs are not
summed as a unique coverage percentage.

Strict solver `--all-targets` and mechanical headless Clippy checks passed with
`-D warnings`, as did workspace formatting. Structural and thermal validation
profiles executed all **14** and **17** commands successfully. Tensor self-test
and structure, documentation book/inventory, and the 800-line source / 2000-line
document organization gates passed. Tensor evidence is bounded `verified`
numerical/recovery and benchmark evidence only: 0 structural gaps, 4 maturity
gaps, 16 evidence-grade gaps and 11 P0 gaps remain; overall release status is
still blocked and is not promoted by this repair.

This is bounded CST kernel validation, not general mechanical or thermoelastic qualification.
The existing dimensional minimum-area threshold (`1e-12`), constant-strain
model and mean nodal-temperature approximation are unchanged. This does not
qualify tiny-cell scale invariance, large deformation, nonlinear materials or
arbitrary contact. Arithmetic outside the representable range can still be
rejected. The optimization depends on the current symmetric isotropic plane-
stress D; a future general constitutive law must not reuse the sparse formula
without revisiting its nonzero pattern and tests.

Previously computed clockwise or mixed-orientation triangle results must be
recomputed before research reuse, including thermal and cohesive-host outputs.
No stored results are silently rewritten or deleted. Prior compact-patch
qualification is not evidence for these previously untested orientations.

## Remote Paired Precompute

Host: Linux x86_64 physical server, AMD Ryzen 7 7735H, 8 cores / 16 logical CPUs.
Compiler: Rust 1.88.0, standard Cargo release profile. No CPU affinity or
frequency lock was imposed. These timings are host-specific point estimates.

Both paths run in the same process/build on 16 ordinary positive-orientation
shape/scale fixtures. Each run has 3 warm-ups and 9 measured samples of 200,000
complete element precomputations. Old/new order alternates. Inputs and returned
kernels are black-boxed; both wrappers are non-inlined. Fixture setup and
correctness comparisons are excluded from timing. Only the new path includes
the additional finite-coefficient checks.

An intermediate dense `D * B` / upper-triangle version took 49.612 ns versus
45.636 ns for the old implementation, an 8.71% increase. It was replaced by the
zero-aware nodal-block implementation, without removing the new checks.

| Final implementation run | Previous median ns/element | Current median ns/element | Time change |
| --- | ---: | ---: | ---: |
| First | 45.459 | 45.476 | +0.04% |
| Repeat | 46.157 | 46.331 | +0.38% |

Repeat samples, ns/element, chronological measured order:

```text
previous_ns=[46.293645,46.261785,46.29645,46.156985,49.216125,45.864885,45.8439,46.07974,45.69712]
current_ns=[46.33146,46.49998,46.54351,46.42554,46.6728,46.09687,45.890935,46.189645,45.92465]
```

No measurable speedup over the original is claimed. The final implementation
recovers the initial repair's measured overhead on these fixtures and is close
to the original cost while adding correctness/range protections. This is not
a statistical equivalence test or a global overhead guarantee.

This is a CST precompute microbenchmark, not end-to-end solver throughput or a 1M-node qualification.
It excludes global assembly/factorization, thermal force and result recovery,
nonlinear interface iteration, transport and UI.

## Reproduction And Provenance

Run from the repository root with the pinned toolchain:

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-solver
cargo test --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-cli --test mechanical_output_operator
cargo test --manifest-path workers/rust/Cargo.toml --locked --release -p kyuubiki-solver --lib triangle_precompute_paired_benchmark -- --ignored --nocapture --test-threads=1
cargo clippy --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-solver --all-targets -- -D warnings
workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile plane-2d-patch-closed-form --execute --out tmp/triangle-mechanical-validation.json
workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile thermal-plane-patch --execute --out tmp/triangle-thermal-validation.json
```

The remote release correctness run selects `--lib` and these nine integration
targets: `plane_triangle_orientation_reliability`, `plane_2d_closed_form`,
`plane_input_reliability`, `plane_triangle_2d_review`,
`thermal_plane_triangle_2d_review`, `thermal_plane_mesh_refinement_regression`,
`thermal_plane_input_reliability`, `cohesive_interface_mesh_2d_closed_form` and
`cohesive_mesh_convergence_reliability`.

Remote execution uses source-only scratch space and the existing managed Cargo
cache, without changing installed services or server configuration. The scratch
source was removed after completion; the shared cache was retained. Local and
remote source SHA-256 values matched, relative to `workers/rust/crates/solver/src/`:

| File | SHA-256 |
| --- | --- |
| `plane_2d_math.rs` | `b91940875ce08a3f4f592bafde92432c4ae12da8fa8d95d405d2544871b954e6` |
| `plane_triangle_kernel_reference.rs` | `f7d6e63c7446b05d1bec128c5035558ff7bdbab0aab4f5348b3b175503a215e7` |
| `plane_triangle_kernel_benchmark.rs` | `e95f59e4343de8e73886704dcda07381e74b1a6716ce8d8e75c5ec3adb0ae2d9` |
