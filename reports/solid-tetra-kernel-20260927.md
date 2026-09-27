# Solid Tetra Kernel: 2026-09-27

## Scope And Defects

This change optimizes the native constant-strain tetrahedron used by
`solve.solid_tetra_3d` and by solid hosts in `solve.cohesive_interface_mesh_3d`.
Base revision: `d7de8968d0eba5e3ed5c08c2ed82ab67ba8aed36`, daji 3.4.1, with the
uncommitted Rust geometry/test overlay. Existing orchestration changes in the
working tree are separate from this measurement.

Four new unit tests and two public-solver tests failed before the fix:

- Exactly representable translations of a well-shaped skew tetrahedron by
  positive/negative `2^20` or `2^40` could turn the old absolute-coordinate
  cofactor determinant into zero. An unchanged cell was rejected as zero-volume.
- A right tetrahedron with edge length `1e103` has representable volume
  `1.6666666666666667e308`, but preflight overflowed the triple product before
  dividing by six. The old shape-quality expression also used dimensional
  powers that could overflow despite finite volume.
- Translated affine-strain recovery could not construct the element kernel.
- Finite input parameters could create infinite constitutive/stiffness
  coefficients without the element constructor rejecting them.

The new shared geometry helper forms local edges before normalization. It uses
three cross products rather than a 4x4 cofactor inverse, restores volume after
division by six, and calculates dimensionless mean-ratio quality. Preflight and
element construction now share the same volume calculation. Unrepresentable
edge differences, volume, shape gradients or coefficients fail before assembly,
with the element ID in the error.

There are no new schemas, engine dispatch dependencies or frontend paths. The
old cofactor kernel is retained only under `cfg(test)` for bounded reference
checks and paired timing; it is not a runtime fallback.

## Correctness Evidence

- All 24 node permutations recover analytic affine strain and stress after a
  large exact coordinate shift, including negative determinant orientations.
- Forty-eight ordinary skew/orientation/material combinations match the old
  independently calculated cofactor geometry and stiffness. Stiffness symmetry
  and all six zero-force rigid-body modes are checked separately. The reference
  shares the unchanged constitutive/matrix multiplication helpers, so it is not
  an independent implementation of the entire solver.
- Geometry scales from `1e-100` through `1e103` preserve representable volume,
  normalized quality and inverse-length gradients. Collapsed/sliver cells,
  true volume underflow/overflow and nonfinite coefficients fail closed.
- Public solid solves compare displacement, reaction, stress and energy across
  exact positive/negative translations. A huge-volume, fully fixed zero-load
  fixture tests arithmetic range only, not the accuracy of an enormous model.
- The cohesive-host regression compares translated displacement, reactions and
  traction, rejects an overflowing host material and then solves a valid replay.
- Two Rust headless SDK plan-to-Engine tests exercise the registered solid
  operator route, successful numerical fields, clear errors and clean replay.
  This is in-process dispatch, not installed-Agent or network validation.

Local macOS ARM64 final-tree full solver regression: 989 passed, 0 failed,
7 ignored opt-in tests. This includes the extended cohesive suite (6 cases).
The new headless route separately passed both cases. Overlapping runs are not
summed as unique coverage.

Remote Linux release regression: 361 unit tests and 18 tests across seven
selected solid/cohesive integration targets passed, 0 failed. Seven opt-in unit
tests were ignored in this ordinary run; the new benchmark was separately
executed twice. Solid affine patches, bending refinement and cohesive sparse
assembly remained passing. Strict solver `--all-targets` Clippy and the new
headless test's Clippy check both passed with `-D warnings`. Workspace formatting
and all 13 commands in the structural-solid validation profile passed locally.
Tensor self-test/structure, documentation book/inventory, and the 800-line
source / 2000-line document organization gates passed. The tensor still has
0 structural gaps, 4 maturity gaps, 16 evidence-grade gaps and 11 P0 gaps; the
overall release status remains blocked, not promoted by this change.

This is bounded tetrahedral geometry validation, not general mechanical or large-mesh qualification.
The existing constant-strain/isotropic/small-strain assumptions remain. There
is no plasticity, contact, large-deformation or near-incompressibility upgrade.
Normalization cannot reconstruct local detail already lost when input
coordinates were rounded to `f64`; the translation fixtures deliberately retain
their exact local edges. Shape gradients and coefficients must still be
representable, and very ill-conditioned cells still fail the existing quality
threshold. This is not a claim to accept every mathematically finite input.

## Remote Paired Measurement

Host: Linux x86_64 physical server, AMD Ryzen 7 7735H, 8 cores / 16 logical CPUs.
Toolchain: rustc 1.88.0 (`6b00bc388`, LLVM 20.1.5), standard Cargo release profile.
No CPU affinity/frequency lock was imposed; host scheduling may affect timing.

Each path uses 16 ordinary shape/scale fixtures, 3 warm-ups and 9 measured
samples of 100,000 calls. Old/new order alternates within each pair. Both
functions are non-inlined wrappers with black-boxed inputs and outputs.
Fixtures and correctness comparisons are outside timing. Both versions run in
the same process and build. The current full kernel includes its extra finite
coefficient checks; the old reference faithfully omits those new checks.

| Run | Path | Previous median ns/element | Current median ns/element | Time reduction |
| --- | --- | ---: | ---: | ---: |
| First | Geometry | 261.465 | 100.821 | 61.44% |
| First | Complete element precompute | 525.624 | 384.853 | 26.78% |
| Repeat | Geometry | 260.389 | 100.723 | 61.32% |
| Repeat | Complete element precompute | 513.171 | 377.109 | 26.51% |

Retained repeat samples, in chronological measured order, ns/element:

```text
geometry previous_ns=[268.41135,258.45485,258.29323,257.55844,262.82034,258.80762,260.38882,263.87273,264.34663]
geometry current_ns=[99.9615,100.68979,99.86171,100.96982,100.066,100.72345,100.9575,104.42077,105.76522]
kernel previous_ns=[510.69889,513.89586,513.17089,521.39671,514.49089,518.29892,509.19724,504.99186,508.89798]
kernel current_ns=[376.11224,378.81748,380.61891,382.70887,381.01365,377.10914,375.18979,374.01215,375.73743]
```

This is a precompute microbenchmark, not end-to-end solver throughput or a 1M-node qualification.
It excludes request validation, global assembly/factorization, result recovery,
transport and UI. No memory-capacity or global speedup bound is claimed.

## Reproduction And Provenance

From the repository root, using the pinned toolchain:

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-solver
cargo test --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-cli --test solid_tetra_geometry_operator
cargo test --manifest-path workers/rust/Cargo.toml --locked --release -p kyuubiki-solver --lib tetra_precompute_paired_benchmark -- --ignored --nocapture --test-threads=1
cargo clippy --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-solver --all-targets -- -D warnings
workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile solid-tetra-3d-closed-form --execute --out tmp/solid-tetra-validation.json
```

The remote release correctness run selected `--lib` and these seven integration
targets: `solid_tetra_geometry_reliability`, `solid_tetra_3d_closed_form`,
`solid_tetra_3d_mesh_patch`, `solid_tetra_3d_bending_convergence`,
`cohesive_interface_mesh_3d_closed_form`, `cohesive_interface_mesh_3d_sparse`,
and `cohesive_interface_mesh_3d_input_reliability`.

The source-only remote run reused the managed Cargo cache. No installed Agent,
Orchestra, deployment or service configuration was changed. Remote and local
source hashes matched; paths below are relative to
`workers/rust/crates/solver/src/`:

| File | SHA-256 |
| --- | --- |
| `solid_tetra_3d_geometry.rs` | `5eba71c16b4bb6506a088a4be08abfff20dd171a7462d38c1d097deabeb1f329` |
| `solid_tetra_3d_element.rs` | `7efb6b68a460ede97de138f5148e56bc64040441c330a4adcd14e07ca3185ba8` |
| `solid_tetra_3d_reference.rs` | `f7343c0c9c6e4078c397827bcdd20374039967810e705039caf33d7b68696fcd` |
| `solid_tetra_3d_benchmark.rs` | `c9bf4cd26c0b0919079d4bb251158c7ef1ac7749076995d18e76248a0a4ecca2` |

The structural-solid validation profile now includes geometry, cohesive-host and
headless route regressions. The tensor registers bounded `verified` geometry
and benchmark evidence only; existing qualification/release obligations remain.
