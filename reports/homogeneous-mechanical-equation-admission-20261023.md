# Homogeneous Mechanical Equation Admission

Date: 2026-10-23

Version line: daji 3.6.x, after `520e3371` (`daji 3.6.9`), building on the
uncommitted [prescribed-equation admission](original-prescribed-equation-admission-20261023.md).

## Contract and Implementation

Mechanical zero constraints now use the same original-equation gate as nonzero
prescriptions. `PrescribedSystemValidator::new_homogeneous` validates the original
matrix and indexes zero-valued supports directly, without a temporary vector of
`(DOF, zero)` tuples. Repeated support indices are idempotent; out-of-range
indices fail instead of being ignored.

Before postprocessing or publishing a successful result, the expanded unknowns
must have complete dimensions, finite values and exactly zero fixed components.
Signed zeros remain numerically equal. A nonzero minimum-quantum support drift
is not accepted by a relative tolerance. All free equations are checked against
the pre-elimination assembled matrix and RHS. Fixed equations may carry reactions
and are not subjected to a zero-residual condition.

No second mechanical residual implementation was introduced. Both constraint
entry points share the ordinary/range row evaluator, free-unknown representation
allowance, fixed-input allowance exclusion and existing sparse tolerance. Failure
does not trigger a new solver fallback or relaxed tolerance.

## Connected Static Sites

Fifteen reconstruction sites are connected, covering nineteen operator families:

| Reconstruction site | Families | Original data checked |
| --- | --- | --- |
| `spring.rs`, three sites | 1D/2D/3D springs | Global stiffness and nodal force |
| `bar_1d.rs`, sparse thermal-bar branch | Thermal/mechanical axial bar | Global stiffness and full thermal-equivalent force |
| `torsion_1d.rs` | Torsion | Global stiffness and torque |
| `truss.rs`, two sites | 2D/3D trusses | Global stiffness and force |
| `thermal_truss.rs`, two sites | 2D/3D thermal trusses | Global stiffness and full thermal-equivalent force |
| `beam_1d_system.rs` | Mechanical/thermal beams | Assembled stiffness and force, including element loads |
| `frame_2d_system.rs` | Mechanical/thermal 2D frames | Assembled global system, including equivalent loads |
| `frame_3d.rs` | Mechanical 3D frames | Assembled global stiffness and force |
| `solid_tetra_3d.rs` | Tetrahedral solids | Assembled stiffness and force, before reaction recovery |
| `plane_2d_profile.rs` | Triangle/quad plane mechanics | Original assembled system, after scatter |
| `thermal_plane_2d_solve.rs` | Triangle/quad thermomechanics | Original system with thermal-equivalent RHS |

The already guarded thermal-bar chain path remains unchanged; the new consumer
fixture deliberately uses a four-arm non-path model to reach its sparse branch.
Plane profiling records `validate_original_constraints` separately so that the
additional verification is visible rather than hidden in solver timing.

The original matrix, load vector and expanded solution are borrowed, not copied.
Preparation and admission are O(n+nnz), with one O(n) prescribed-value index and
constant extra row state. There is no extra factorization, dense conversion or
residual-vector allocation. Static calls pay for the added scan; no timing or
peak-RSS improvement or new million-node qualification is claimed.

## Added Coverage

Twenty-five tests were added: six solver unit tests and nineteen independently
named engine consumer tests.

- Spring equations at binary coefficient scales `2^-600`, `1`, `2^600`, both
  load signs, correct reduced solutions, corrupted expanded supports, wrong free
  responses and inconsistent original loads.
- Equivalence with explicit zero prescriptions, signed zeros, repeated indices,
  loaded all-fixed systems, empty systems and invalid support/data controls.
- Ordinary and range controls showing that a fixed zero cannot lend rounding
  allowance to a wrong weak free component.
- Cancellation during long support indexing, a 2,049-entry original row and
  final validation completion, followed by successful reuse.
- Each of nineteen real operator families runs serialized dispatch, cancellation
  at admission entry and completion through both direct and workflow calls,
  clean replay, workflow/direct artifact equality and a loaded all-fixed case.
- Shared fixtures contain distributed beam loads, nonzero thermal strain and
  curvature so the gate must use assembled loads, not only node inputs.

Two initial 3D-truss fixtures had only two nodes and were rejected before reaching
admission. They were corrected to valid connected three-node models; production
topology validation was not weakened to make those fixtures pass.

## Validation

Final source validation used pinned Rust 1.88.0 with offline locked dependencies,
existing build roots, local macOS AArch64 and the remote Linux x86_64 test copy:

| Scope | macOS passed | Linux passed |
| --- | ---: | ---: |
| Solver library, excluding `modal_frame_spectrum::` | 733 | 733 |
| Engine library | 711 | 711 |
| 122 solver/engine/CLI integration targets | 579 | 579 |
| Total | 2,023 | 2,023 |

Each platform had zero final failures, ten ignored library tests and 292 filtered
modal-spectrum tests; integration tests had no failures or ignored cases. The
25 new tests are included once. The wider run also includes 222 existing cases
outside the preceding 67-target scope. Focused reruns are not added to the totals.
Existing 10,000-node cases, 52 live Agent cancellation tests, closed-form/review
fixtures and structural input/output/orientation/equilibrium controls remain in
the integration scope. The excluded modal matrix and opt-in benchmarks were not
run for this change.

Local solver/engine all-target Clippy passed with warnings denied; workspace
formatting and whitespace checks passed. Changed source/configuration files stay
within 800 lines and documentation within 2,000 lines. No separate build root,
numerical dump, application package or service deployment was created. Existing
test sources were synchronized without changing server credentials/configuration.

Native operator metadata validation passed for 81 profiles with `executed=false`;
this registration check is separate from the numerical executions above.
Coverage-tensor structure and command checks passed across 13 modules and
11 paradigms: zero structural gaps, two maturity gaps, 23 evidence-grade gaps
and 18 P0 gaps. Overall readiness remains `blocked`; these bounded mechanical
claims do not qualify every constrained formulation or the complete system.

## Reproduction

From the repository root, with pinned Rust 1.88.0 and offline locked dependencies:

```sh
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver --lib constraint_validation::homogeneous_tests -- --nocapture
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-engine --test mechanical_constraint_admission -- --nocapture
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine --lib -- --skip modal_frame_spectrum:: --quiet
```

The expanded integration run is the union of the previous 67 targets in the
[sparse reduction report](sparse-prescribed-reduction-reliability-20261023.md#reproduction)
and these 55 additional targets. They were executed once per platform as a
122-target set, not summed from overlapping focused runs:

```sh
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine -p kyuubiki-cli \
  --test mechanical_constraint_admission --test spring_1d_review \
  --test spring_2d_review --test spring_3d_review --test spring_input_reliability \
  --test thermal_bar_1d_review --test torsion_1d_review --test beam_torsion_input_reliability \
  --test truss_2d_closed_form --test truss_3d_closed_form --test truss_2d_review \
  --test truss_3d_review --test truss_input_reliability --test truss_output_reliability \
  --test thermal_truss_2d_closed_form --test thermal_truss_3d_closed_form \
  --test thermal_truss_2d_review --test thermal_truss_3d_review \
  --test thermal_truss_input_reliability --test thermal_truss_output_reliability \
  --test beam_1d_review --test beam_output_reliability \
  --test thermal_beam_1d_closed_form --test thermal_beam_1d_review \
  --test frame_2d_review --test frame_2d_equilibrium_reliability \
  --test frame_2d_output_reliability --test frame_input_reliability \
  --test thermal_frame_2d_closed_form --test thermal_frame_2d_review \
  --test frame_3d_closed_form --test frame_3d_review --test frame_3d_orientation_reliability \
  --test frame_3d_output_reliability --test frame_3d_support_reliability \
  --test solid_tetra_3d_closed_form --test solid_tetra_3d_review \
  --test solid_tetra_input_reliability --test solid_tetra_topology_reliability \
  --test solid_tetra_geometry_reliability --test solid_tetra_workflow \
  --test plane_2d_closed_form --test plane_triangle_2d_review --test plane_quad_2d_review \
  --test plane_input_reliability --test plane_triangle_orientation_reliability \
  --test plane_quad_geometry_reliability --test thermal_plane_triangle_2d_review \
  --test thermal_plane_quad_2d_review --test thermal_plane_input_reliability \
  --test thermal_plane_output_reliability --test operator_thermal_structural_reliability \
  --test frame_2d_composite_section_closed_form --test frame_2d_fiber_section_closed_form \
  --test frame_2d_initial_stress_reliability --no-fail-fast -- --quiet
```

## Remaining Limits

This is componentwise backward-error admission in the assembled f64 system,
not an exact-assembly or forward-error certificate. Ill-conditioning, field
recovery, physical model qualification and sustained high-scale performance
remain separate concerns. Fixed reactions are allowed, not independently
certified by this gate.

Directional/projected constraints, including the generalized thermal-frame 3D
projection, are not coordinate-zero constraints and were not forced through
this validator. Nonlinear tangent/Newton increments, dynamic effective systems,
modal and buckling equations retain their formulation-specific paths. The
analytic uniform-bar shortcut does not eliminate a matrix and is also unchanged.
Cooperative cancellation/replay evidence is not durable process-restart evidence.

This is bounded original homogeneous mechanical equation admission, not general forward accuracy, projected/nonlinear/dynamic/eigen qualification or durable restart.

Follow-up: the [projected thermal-frame gate](projected-thermal-frame-equation-admission-20261023.md)
adds a separate directional-support contract and fixes nonzero near-axis basis
component clipping. The scope and test totals above remain historical.
