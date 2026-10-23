# Projected Thermal-Frame Equation Admission

Date: 2026-10-23

Version line: daji 3.6.x, after `520e3371` (`daji 3.6.9`). This extends the
uncommitted [homogeneous mechanical gate](homogeneous-mechanical-equation-admission-20261023.md)
without treating directional supports as coordinate-zero constraints.

## Contract

`solve.thermal_frame_3d` now admits its restored physical solution before node,
element, spring or directional-reaction results are built. The gate borrows the
original assembled stiffness, complete force vector and restored unknowns.
Thermal axial/curvature loads and directional translational/rotational springs
remain part of that original system.

The checks are:

- Complete dimensions, finite unknowns and loads, and ordered in-range finite
  matrix entries, including rows that only carry support reactions.
- Exactly zero coordinate-fixed components; normalized directional displacement
  and rotation identities checked with a local `1e-10` relative tolerance.
- For each independently rebuilt admissible direction `q` in a translational or
  rotational block, the original equations satisfy `q^T (K u - f) = 0` under the
  existing `1e-8` sparse backward-error policy. Reactions along constrained
  directions are allowed, rather than requiring every physical row to vanish.

The free basis is recomputed from the original support directions using the
existing reorthogonalized QR helper, not inferred from the reduced solve output.
This is a second equation check, not an independent implementation of the QR
algorithm or an exact-assembly proof.

## Range and Cost

Coordinate free directions retain the shared original-row evaluator and range
recovery. Oblique directions use a three-way merge of original sparse rows.
Power-of-two normalization precedes weighted products, and compensated sums
include product tails. Coefficients for the same physical unknown are combined
before computing the row error scale, so large cancelled support coefficients
do not provide a spurious equilibrium allowance for a weak free equation.
Fixed zeros cannot contribute free-unknown representation allowance.

If normalization cannot round-trip an original operand or a nonzero weighted
product becomes subnormal, the oblique gate fails closed with a range error.
It does not silently erase that term, relax the tolerance or introduce another
solver fallback. This conservative gate can reject an extreme mixed-range
model that needs a wider projected-arithmetic representation; it is not an
arbitrary-range projection implementation. Normal product rounding and
ill-conditioning are not converted into a forward-error certificate.

The gate scans O(n+nnz) data with bounded block work, a reused sparse-row buffer
and small free-basis allocations. It does not retain another complete matrix,
factorization or residual vector. The added scan and allocations have a cost;
no speed, memory-peak or new million-node qualification is claimed.

Cancellation uses the existing `ConstraintValidate` and `ConstraintValidateRow`
stages, with bounded wide-row checkpoints and final completion at `3 * DOFs`.
Failure/cancellation returns before successful result publication. No checkpoint
enum was renumbered or added.

## Fixed Defect

The solve map previously discarded basis components with absolute value at most
`1e-15`. A guide with direction `[1, 2^-60, 0]` therefore lost the small but
nonzero x response during restoration, effectively changing that constraint.
The map now preserves every nonzero basis component. Tests assert the small
response through projection, solve, restoration, JSON dispatch and workflow
execution, and reject a manually clipped reconstruction.

## Added Tests

Fourteen new tests comprise nine solver unit cases and five engine integrations:

- Translational and rotational oblique supports, signed loads and coefficient
  scales `2^-600`, `1` and `2^600`, with nonzero legitimate reactions.
- Wrong free responses, changed original loads, support drift, two nonorthogonal
  directions and signed/rescaled support definitions.
- Coordinate-gate parity, all-fixed reactions, malformed full dimensions,
  invalid matrix columns and non-finite input on constrained rows.
- Coefficient cancellation before residual scaling and conservative projection
  range failures, plus exact axis and scaled oblique support identities.
- An unrelated huge third displacement component cannot set the normalization
  scale for a two-axis support identity or erase its small constrained motion.
- Near-axis support preservation through solve/restore and real workflow use.
- Wide-row, coefficient-merge and completion cancellation, followed by reuse.
- Real axis/translation/rotation/near-axis consumers with thermal expansion,
  thermal curvature and both spring types; serialized dispatch, direct/workflow
  entry/row/completion cancellation and equal replay artifacts.
- All-fixed thermally loaded frames remain valid and produce nonzero stress;
  removing temperature/gradients changes real element results.

An initial observer test attempted to mutate an `Fn` capture and did not compile;
the counter was corrected to use `Cell`. Production observer semantics were not
weakened to accommodate the test.

## Validation

Local macOS AArch64 and remote Linux x86_64, pinned Rust 1.88.0, offline locked
dependencies, final source on both platforms:

| Final-source scope | macOS passed | Linux passed | Other per platform |
| --- | ---: | ---: | --- |
| Solver library excluding `modal_frame_spectrum::` | 742 | 742 | 9 ignored, 292 filtered |
| Engine library | 711 | 711 | 1 ignored |
| Nine solver/engine integration targets | 39 | 39 | No ignored or filtered tests |
| Non-overlapping total | 1,492 | 1,492 | No failures |

The fourteen new tests are included once; focused reruns are not added to the
total. Existing directional closed forms, spring closed forms, thermal-frame
reviews, rigid-rotation objectivity, mesh convergence and nineteen homogeneous
mechanical consumer gates remain green. The previous report's wider 122-target
run is historical evidence, not represented as a rerun here.

Solver/engine all-target Clippy passed with warnings denied. Workspace formatting
and whitespace checks passed. New files are below 800 lines. Existing local
build roots were reused, with no package, deployment, result dump or additional
backup tree created. Version, Git staging and commits remain unchanged.

Five source/test files were synchronized into the existing remote test copy,
without removing remote files or changing server credentials/configuration.
Native operator metadata validation passed for 82 profiles with `executed=false`;
this registration check is separate from numerical execution. Coverage-tensor
structure and command checks passed for 13 modules and 11 paradigms, with zero
structural gaps, two maturity gaps, 23 evidence-grade gaps and 18 P0 gaps. Overall
readiness remains `blocked`; this bounded claim does not promote the whole solver
module or all projected formulations to qualified status.

## Reproduction

From the repository root:

```sh
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver --lib projected_original_equations -- --nocapture
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine --lib -- --skip modal_frame_spectrum:: --quiet
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine \
  --test thermal_frame_constraint_admission --test mechanical_constraint_admission \
  --test thermal_frame_directional_constraint --test thermal_frame_directional_spring \
  --test thermal_frame_3d_closed_form --test thermal_frame_3d_review \
  --test thermal_frame_objectivity --test thermal_frame_mesh_convergence \
  --test operator_thermal_structural_reliability --no-fail-fast -- --quiet
```

## Limits

The residual gate qualifies the assembled f64 static thermal-frame formulation
within its stated checks. It is not a separate physical reference solution,
an independent assembly/reaction certificate or coverage of arbitrary multipoint
constraints. Other projected formulations, nonlinear tangent/increment systems,
dynamic effective systems, modal/buckling equations, full modal-spectrum matrices
and opt-in benchmarks are not exercised by this change. Cooperative cancellation
and clean replay are not process-kill, power-loss or durable-restart evidence.

This is bounded original projected thermal-frame equation admission, not general forward accuracy, arbitrary-precision projection, physical qualification or durable restart.

Follow-up: [thermal-frame reaction range recovery](thermal-frame-reaction-range-reliability-20261023.md)
addresses residual accumulation and directional reaction back substitution after
this admission gate. The scope and validation totals above remain historical.
