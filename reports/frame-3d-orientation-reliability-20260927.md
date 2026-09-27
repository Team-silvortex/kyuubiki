# Spatial Frame Orientation Reliability: 2026-09-27

## Scope And Baseline

Base revision: `6ab4c2dd`, daji 3.4.2, plus the current working-tree overlay.
This follows the [support-basis repair](frame-3d-support-reliability-20260927.md).
The implementation change is confined to the Solver's shared spatial frame
coordinate transform. Mechanical and thermal static frames use the explicit
section hint; modal spatial frames share the implicit-axis path. There is no
Engine, task schema, GUI, installed service or public input/output change.

Five of six initial regression cases failed: both internal mathematical tests
and three of four public Solver tests. These are cases, not independent defects.
The invalid-hint rejection and clean replay control already passed.

The preceding implementation projected a normalized `local_y_axis` hint off
the member axis once, then normalized the small remainder. Axial roundoff
could be amplified enough to leave the local basis nonorthogonal:

- With member delta `[1,1,0]` and hint `[1+epsilon,1-epsilon,0]` at
  `epsilon=1e-3`, the off-diagonal entry `(R*R^T)[0,1]` was
  `1.5698553568199713e-13`, beyond the `8e-15` test bound.
- With delta `[1,2,3]`, hint `[1+epsilon,2-epsilon,3]` and `epsilon=1e-6`,
  a linearized rigid-rotation displacement field produced a spurious local
  end action of `3.296520389994839e-8`, beyond the `1e-10` patch-test bound.
- Equivalent section hints changed mechanical and thermal coupled fields.
  The equivalence is analytic: for delta `[1,1,0]`, every hint
  `[1,1+epsilon,0]` with positive epsilon has the same perpendicular direction
  as `[-1,1,0]`.
- Free uniform thermal expansion returned `1.0000000001570096e-3` for a
  component whose closed form is `1e-3`, outside the `2e-14` absolute bound.

The public sweep retains `epsilon=1e-3, 1e-6, 1e-9, 1e-11, 4e-12`, checks
actual outputs rather than successful return alone, and keeps its original
assertion tolerances after the fix.

## Coordinate Contract

The member direction uses scale-first normalization. An explicit section hint
is normalized, projected twice off that direction with fused multiply-add
updates, and rejected if its remaining norm is at or below `1e-12`. Local `z`
is the normalized cross product, and local `y` is reconstructed from `z` and
`x` to close a right-handed orthonormal triad.

```text
x = normalize(node_j - node_i)
y_candidate = reorthogonalize(normalize(hint), x)
z = normalize(x cross normalize(y_candidate))
y = z cross x
R = rows(x, y, z)
R*R^T = I, det(R) = +1
```

The primary [OpenSees linear transformation documentation](https://opensees.github.io/OpenSeesDocumentation/user/manual/model/geomTransf/Linear.html)
describes constructing beam-local axes from the member axis and a nonparallel
orientation vector. Its supplied vector defines the local x-z plane, whereas
Kyuubiki's hint specifies local y; these are not interchangeable conventions.
This reference supports the coordinate construction, not an executed external
solver comparison or a new physical formulation.

The default path retains its prior reference-axis predicate
`abs(dz/length) < 0.9`. An expanded boundary test caught an intermediate repair
using the renormalized `x[2]` for this choice: `0.9` became
`0.8999999999999999`, unexpectedly switching the implicit section roll.
Preserving the original predicate fixes that regression. Tests cover both
poles, both sides and exact ties of the threshold, and geometry scaling.
An initial full test run was interrupted after this finding; only the final
snapshot is used for final execution evidence.

The raw direction magnitude floor remains `1e-12`, as does the relative
parallelism floor after normalization. Large finite hints do not bypass the
parallelism check. Negative hint scaling flips both local y and z; the thermal
comparison reverses both signed local gradients and signed section outputs.
This differs from reversing member connectivity with local y held fixed,
which reverses local z only.

## Coverage And Evidence

Four internal tests check the orthonormal/right-handed triad, linearized rigid
rotation with no elastic action beyond tolerance, legacy default-axis behavior,
and scale-independent acceptance/rejection. The rigid patch uses
`u_i=0`, `u_j=omega cross (node_j-node_i)` and endpoint rotations `omega`.
It is a small-strain rigid mode, not a finite-rotation/corotational solve.

Seven public tests cover mechanical and thermal equivalent-hint fields,
uniform free-expansion closed form, invalid hints plus replay, 2/4/8-member
refinement, local-axis reversal with signed thermal gradients, and all six
frequencies of a rotated free-tip cantilever through the modal operator.
The modal check concerns an isolated fixed-root/free-tip member with isotropic
nodal translational/rotary mass, not arbitrary assembled modal objectivity or
new explicit-axis support in the modal request schema. Borrowed/owned static
results are also compared.

Two new Rust headless cases extend the spatial target to nine tests. They
validate a batch and plan, resolve the bridge route and call the registered
Engine operator for both static families. Equivalent hints must retain fields;
invalid hints must fail and allow a fresh valid solve.

| Completed final-snapshot lane | Passed | Failed | Ignored |
| --- | --- | --- | --- |
| macOS ARM64 orientation unit tests | 4 | 0 | 0 |
| macOS ARM64 public orientation target | 7 | 0 | 0 |
| macOS ARM64 spatial headless target | 9 | 0 | 0 |
| macOS ARM64 full Solver, debug | 1114 | 0 | 9 |
| Linux x86_64 selected Solver, release | 480 | 0 | 9 |
| Linux x86_64 spatial headless target, release | 9 | 0 | 0 |

The final complete macOS run finished 180 result groups with no failures.
Solver all-target and spatial CLI-target Clippy passed with warnings denied.
Workspace formatting, whitespace, document book/inventory, tensor self-test
and source-800/document-2000 organization gates passed. The reliability guide
remains below its limit at 1998 lines. Profiles `frame-3d-closed-form` and
`thermal-frame-3d-closed-form` executed all nine and thirteen commands,
respectively, including the four unit invariants and seven public regressions.

The separate claim is registered in
`config/architecture/module-function-coverage-evidence/runtime-frame-fields.json`.
Tensor status remains 0 structural gaps, 4 maturity gaps, 16 evidence-grade
gaps and 11 P0 gaps, with overall release status blocked. These are executable
numerical assertions, not machine-checked formal proofs or an expansion of
general physical qualification.

Both hosts use pinned Rust **1.88.0** with `--locked --offline`. Linux executes
`--lib` plus `frame_3d_orientation_reliability`, `frame_3d_support_reliability`,
`frame_3d_output_reliability`, `thermal_frame_3d_closed_form`,
`thermal_frame_3d_review`, `thermal_frame_objectivity`,
`thermal_frame_mesh_convergence`, `thermal_frame_directional_constraint`,
`thermal_frame_directional_spring`, `frame_input_reliability`,
`frame_3d_closed_form`, `frame_3d_objectivity`, `frame_3d_review`,
`mechanical_convergence`, `modal_frame_3d_review`, `modal_frame_sanity_regression`
and `frame_2d_output_reliability`. The 18 groups contain 376 unit tests and 104
selected integration tests. Overlapping lanes are not additive coverage
percentages; the nine skipped opt-in tests are not benchmark evidence.

Local and remote SHA-256 match for these paths relative to `workers/rust/crates/`:

| File | SHA-256 |
| --- | --- |
| `solver/src/frame_3d_math.rs` | `5947cd7dd25c0a7929995b6d94c2d69ffe697ec4f4565bed877ffa9fe636f030` |
| `solver/src/frame_3d_orientation_tests.rs` | `6e47d93349d809f5a6cfefac57e864001145014523f5ca8ad88bf067f706ba05` |
| `solver/tests/frame_3d_orientation_reliability.rs` | `c1f4e4b97ec212ef55e2140a1bc7b06ad0c7e754d3708b95b0e65e1a8849b760` |
| `cli/tests/frame_3d_output_operator.rs` | `b877a0b4b5d8721340a4d302eb6fe84f9f7b482fd65d4011cf4c9163ad6e630b` |

## Limits And Reproduction

This is bounded spatial-frame orientation validation, not general frame qualification.
Existing physical qualification scopes remain unchanged. Reorthogonalization
removes axial contamination but cannot recover section roll information lost
when an input hint is too close to the member axis. A clearly nonparallel hint
remains preferable. The implicit-axis rule does not promise continuous roll
across its reference switch; use an explicit hint when section orientation is
physically prescribed. No new nonlinear, shear-deformation, warping-torsion,
contact, GUI, Windows, installed-Agent, performance or million-node claim is made.

Previously saved results using sensitive near-parallel hints need recalculation
before research reuse. Stored data is not rewritten. No version bump, commit,
push or application rebuild is part of this change. Remote testing uses
temporary source-only scratch space and the existing managed Cargo cache;
service configuration and credentials are not changed or stored in the repo.
The temporary remote source directory was removed after all jobs completed;
the managed cache was retained.

Run from the repository root:

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-solver --lib frame_3d_math::orientation_tests
cargo test --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-solver --test frame_3d_orientation_reliability
cargo test --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-cli --test frame_3d_output_operator
cargo test --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-solver
cargo clippy --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-solver --all-targets -- -D warnings
workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile frame-3d-closed-form --execute --out tmp/frame3d-orientation-validation.json
workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile thermal-frame-3d-closed-form --execute --out tmp/thermal-frame3d-orientation-validation.json
workers/rust/target/debug/kyuubiki-script-runner check-module-function-coverage-tensor
```
