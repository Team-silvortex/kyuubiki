# Thermal-Frame Reaction Range Reliability

Date: 2026-10-23

Version line: daji 3.6.x, after `520e3371` (`daji 3.6.9`), extending the uncommitted
[projected original-equation gate](projected-thermal-frame-equation-admission-20261023.md).

## Reproduced Defects

Three tests failed before the first production change:

- A finite original reaction row was rejected because the ordinary dot-product
  accumulation overflowed before later cancellation restored a finite result.
- Four products of `0.25 * minimum_subnormal` were individually rounded to zero,
  publishing zero rather than a representable one-quantum aggregate reaction.
- Finite directional reaction coefficients were rejected because QR back
  substitution materialized an overflowing intermediate subtraction.

A fourth control was added after trying the shared ordinary residual helper.
It still failed: `(1 + 2^-27) * (1 - 2^-27) - 1` was published as zero instead
of `-2^-54`. This normal-range product-tail case requires compensated evaluation
even when no intermediate overflows or enters the subnormal range.

## Implementation

`sparse_scaled_residual_vector` is an internal opt-in final-result path. It reuses
the existing two-pass binary-scaled, product-tail-compensated sparse row evaluator.
The ordinary vector path shares only its dimension, allocation and checkpoint
loop with this path; iterative residual arithmetic and its ordinary fast path
are unchanged.

Thermal-frame support recovery uses the opt-in path with the original stiffness,
complete assembled force and restored displacements. The shared residual has
sign `f - K u`; its existing vector is negated in place to retain the physical
reaction convention `K u - f`. Operands and final values must be finite. Invalid
dimensions return an error rather than indexing past the matrix or solution.
Wide-row and completion cancellation propagate without publishing a result.

Directional reaction recovery still factors the same reorthogonalized support
basis `N = Q R`, but now evaluates each triangular row as one scaled expression:

```text
lambda[i] = (Q[:,i]^T r - sum(R[i,j] * lambda[j], j > i)) / R[i,i]
```

It does not materialize an unscaled projected RHS or intermediate numerator.
At most five operand pairs live on the stack. Existing scaled substitution
handles the combined expression and division, and requires reversible range
restoration. Non-finite inputs, an out-of-range coefficient or a coefficient
whose subnormal range restoration loses information fail explicitly. There is
no new solver fallback or relaxed tolerance. Reversible restoration is not a
claim that division, QR or the physical model is mathematically exact.

The recovered full residual still occupies one O(n) vector, as before. The final
reaction path now pays for two sparse row passes and in-place sign conversion;
it does not copy the stiffness, refactor the physical system, allocate another
complete residual vector or alter the solve iteration. No timing or RSS
improvement is claimed.

## Added Coverage

Thirteen new tests comprise ten solver unit cases and three engine integrations:

- Signed overflow/cancellation rows, signed quarter-quantum aggregation and true
  zero cancellation, normal-product tails in zero and small residuals.
- Finite reaction coefficients after overflowing intermediate arithmetic,
  independent coordinate scales and exactly representable subnormal coefficients.
- Signed binary coefficient scales, support permutations, invalid dimensions,
  non-finite residual operands and excessive support direction counts.
- Explicit rejection of irreversible erasure, overflow and rounded subnormal
  coefficient restoration, followed by clean reuse.
- Cancellation during a 2,049-entry original row, residual completion, sign
  publication, QR factorization and scaled coefficient recovery, then replay.
- Real fully restrained frames with signed loads near `1e308`: translational
  and rotational directional supports, serialized dispatch, direct/workflow
  equality, componentwise force/moment balance using scaled comparison, and
  cancellation during postprocessing followed by identical replay artifacts.
- Unrepresentable near-parallel support coefficients fail both direct dispatch
  and workflows; a subsequently corrected finite case succeeds through both.

The extreme consumer models deliberately restrain all DOFs, so the huge-load
cases isolate physical reaction recovery without requiring huge deformations.
Existing deformed thermal-frame, spring, objectivity and convergence fixtures
provide separate ordinary-physics regression coverage, not a new high-load
deformation qualification.

## Validation

Final source, Rust 1.88.0, offline locked dependencies, existing build caches:

| Scope | macOS AArch64 passed | Linux x86_64 passed |
| --- | ---: | ---: |
| Solver library excluding `modal_frame_spectrum::` | 752 | 752 |
| Engine library | 711 | 711 |
| Thirteen solver/engine integration targets | 84 | 84 |
| Non-overlapping total | 1,547 | 1,547 |

Each platform had zero final failures, ten ignored library cases and 292 filtered
modal-spectrum tests. Integrations had no ignored or filtered cases. Focused
reruns are not added to the totals; the thirteen new cases are included once.
The earlier 122-target report is historical, not represented as a rerun here.

Local solver/engine all-target Clippy passed with warnings denied; formatting
and whitespace checks passed. Source/configuration limits remain 800 lines and
documentation 2,000 lines. Six source/test files were synchronized to the existing
Linux test copy, without deleting remote files or changing service deployment,
server configuration or credentials. No release package, result dump, backup
tree, version change, staging operation or commit was created.

Native operator metadata validation passed for 83 profiles with `executed=false`;
this registration result is separate from numerical test execution. Tensor
structure and command checks passed across 13 modules and 11 paradigms: zero
structural gaps, two maturity gaps, 23 evidence-grade gaps and 18 P0 gaps.
Overall readiness remains `blocked`. All 50 changed/new source, configuration
and documentation files passed their line limits; this bounded evidence does
not qualify the full solver module or all reaction implementations.

## Reproduction

From the repository root:

```sh
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver --lib thermal_frame_reactions_ -- --nocapture
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine --lib \
  --test thermal_frame_constraint_admission --test mechanical_constraint_admission \
  --test thermal_frame_directional_constraint --test thermal_frame_directional_spring \
  --test thermal_frame_3d_closed_form --test thermal_frame_3d_review \
  --test thermal_frame_objectivity --test thermal_frame_mesh_convergence \
  --test operator_thermal_structural_reliability --test frame_3d_support_reliability \
  --test frame_3d_output_reliability --test frame_3d_orientation_reliability \
  --test frame_input_reliability --no-fail-fast -- --skip modal_frame_spectrum:: --quiet
```

## Limits

This does not prove every reaction is correct, replace an independent reference
solver or certify `N lambda = K u - f` for arbitrary inputs. Binary-scaled
normalization can still reject an extreme mixed-range expression that cannot
be aligned reversibly; this is not arbitrary-precision accumulation. Exact
range restoration concerns conversion of a computed quotient, not exact QR or
arithmetic. Other operators' reaction recovery, spring-state energy, nonlinear,
dynamic/eigen formulations, physical qualification, benchmarks and process-kill
or power-loss recovery are outside this change.

This is bounded thermal-frame reaction range recovery, not arbitrary precision, a general reaction certificate, physical qualification or durable restart.
