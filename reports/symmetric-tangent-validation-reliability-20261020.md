# Symmetric Tangent Validation Reliability

Date: 2026-10-20

Version line: daji 3.6.x. Baseline commit: `ec3260d5` (`daji 3.6.8`).
The Rust workspace manifest remains at `3.6.0`; this change does not bump it.

The shared tangent solver used by corotational frames and 2D/3D cohesive
interfaces now checks original equations with the existing range-aware,
componentwise residual implementation. Both banded and dense paths use bounded
iterative refinement. Invalid sparse columns are rejected before factorization.

## Reproduced failures

Four negative controls failed against the old validation:

- With `h = 2^1023`, `A = [h]`, `b = [h]`, and the wrong `x = [2]`,
  residual and scale overflowed. The resulting `NaN` was ignored by `f64::max`.
- With the same equation and `x = [1.5]`, a finite wrong residual divided by
  an infinite scale became zero.
- With `q` the smallest positive subnormal, `A = [q]`, `b = [0]`, and
  `x = [0.25]`, the nonzero product vanished in the old residual calculation.
- For `A = [[1, 1e-30], [1e-30, 1]]`, `b = [0, 1e20]`, and
  `x = [0, 1e20]`, the connected maximum displacement inflated the first
  equation's denominator and hid its local imbalance.

The fourth control strengthens the acceptance policy from a connected-row
normwise bound to a componentwise one; it is not an arithmetic defect in the
definition of a normwise bound.

An internal sparse matrix with a column outside its declared dimension also
reproduced an index-out-of-bounds panic. Columns equal to the dimension and
`usize::MAX` now fail admission, with dense fallback both enabled and disabled.

## Repair and recovery

The tangent solver reuses the shared finite-solution scan and componentwise
residual check. Its denominator is based on each equation's actual products,
with the existing row-local subnormal representation allowance.
Extreme products use the existing exponent-aware residual recovery.

Both backends validate the original matrix and RHS, not factorized or scaled
equations. Numeric tolerance constants are unchanged:
`clamp(128 * EPSILON * dofs, 1e-9, 1e-7)`. The metric is deliberately stricter
than the removed connected-maximum check.

The stricter gate exposed a real shallow-arch continuation regression:
`branched_arch_continuation_is_objective_and_contains_local_seed_failures`
failed at backward error `1.003327e-9`. Dense pivoting alone did not fix it.
Reusing the dense LU factor for residual corrections restored the test without
relaxing the tolerance or changing its physical assertions.

A second exact control uses `A = [[8, -7], [-7, 8]]` and `b = [h, h]`.
The solution is `[h, h]`, but banded substitution overflows an intermediate
numerator. Failed banded solves or validation can now enter the existing
size-limited dense fallback. Both RHS signs recover a validated finite solution;
disabling the dense fallback still returns an error.

Follow-up on 2026-10-23: [banded substitution range recovery](banded-substitution-range-reliability-20261023.md)
now solves that same control on the banded path with dense fallback disabled.
The results above describe the initial tangent-validation repair.

Each backend performs at most four corrections after its initial solve.
Factors are reused rather than rebuilt for each correction. A correction
residual vector is formed only when validation requires refinement.
Validation itself still allocates its existing row metadata; this is not a
zero-allocation or throughput claim.

Cancellation is checked before fallback and during shared validation,
residual construction and substitution. Cancelled cohesive solves return an
error rather than exporting an ordinary nonconverged result. Clean replay
after interruption uses the same equations and immutable prepared factors.
No operator/engine protocol, runtime dependency or new solver backend was added.

## Controls

Fourteen new unit tests cover the four negative controls, valid signed and
subnormal equations, both backends at extreme scales, finite-solution fallback,
dimensions and non-finite unused components, invalid columns, wide-row
cancellation, band/dense refinement replay, iteration and dense size limits,
and the absence of correction-vector construction on an accepted solve.

The 2D and 3D cohesive cancellation integration tests now also interrupt
`ResidualValidate`, then compare the clean replay with the original result.

## Validation

Rust 1.88.0, offline locked dependencies, existing target caches:

| Completed group | macOS AArch64 | Linux x86_64 |
| --- | --- | --- |
| Solver unit filter `linear_` | 175 passed, 3 ignored | 175 passed, 3 ignored |
| Solver unit filter `solver_control` | 12 passed | 12 passed |
| Solver operator integration targets | 140 passed | 140 passed |
| Engine workflow targets | 15 passed | 15 passed |
| CLI operator targets | 22 passed | 22 passed |
| Total excluding repeated focused runs | 364 passed | 364 passed |

The fourteen new unit tests are included in `linear_`, not counted twice.
Three ignored tests are existing opt-in performance benchmarks.
The integration set covers frame continuation and reference solutions,
equilibrium, initial stress, explicit imperfections, material response,
output reliability, cohesive closed forms, sparse paths and convergence.

This is a targeted regression selection, not a full-project test run.

Local Clippy passed with `-D warnings` for solver/engine libraries and the
2D/3D cohesive sparse and frame-continuation integration targets.
Workspace formatting and whitespace checks passed. Changed source/config
files remain below 800 lines; this report remains below 2,000 lines.
No new build root, application package or installation was created.

Native operator validation passed with 68 profiles and `executed=false`.
Tensor structure and command checks passed: 13 modules, 11 paradigms,
0 structural gaps, 2 maturity gaps, 23 evidence-grade gaps and 18 P0 gaps.
Overall readiness remains blocked. These checks validate evidence metadata;
the numerical executions are the separate test runs tabulated above.

## Reproduction

From the repository root:

```sh
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver --lib linear_ -- --quiet
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver --lib solver_control -- --quiet
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine -p kyuubiki-cli \
  --test cohesive_interface_mesh_2d_closed_form \
  --test cohesive_interface_mesh_2d_sparse \
  --test cohesive_interface_mesh_3d_closed_form \
  --test cohesive_interface_mesh_3d_sparse \
  --test cohesive_interface_mesh_3d_input_reliability \
  --test cohesive_mesh_convergence_reliability \
  --test frame_2d_p_delta_closed_form \
  --test frame_2d_p_delta_reliability \
  --test frame_2d_branch_controls \
  --test frame_2d_branch_continuation \
  --test frame_2d_branch_reference \
  --test frame_2d_euler_branch_reference \
  --test frame_2d_coupled_euler_reference \
  --test frame_2d_williams_toggle_reference \
  --test frame_2d_p_delta_explicit_imperfection \
  --test frame_2d_material_p_delta_closed_form \
  --test frame_2d_equilibrium_reliability \
  --test frame_2d_output_reliability \
  --test frame_2d_initial_stress_reliability \
  --test frame_2d_review \
  --test cohesive_interface_mesh_2d_workflow \
  --test frame_2d_branch_workflow \
  --test frame_2d_p_delta_workflow \
  --test frame_2d_p_delta_operator \
  --test frame_2d_material_operator \
  --test frame_2d_output_operator -- --quiet
```

This is bounded tangent-equation validation, not general nonlinear convergence, conditioning, physical qualification or durable restart.
