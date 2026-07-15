# Modal Frame Normalization Policy

This note records the current review-level modal-frame interpretation for
`solve.modal_frame_2d` and `solve.modal_frame_3d`.

## Current Solver Contract

The modal frame operators assemble a lumped mass vector and a global frame
stiffness matrix, reduce the system to free degrees of freedom, and solve a
mass-normalized eigenproblem.

For each retained mode, the solver reports:

- `eigenvalue_rad_s_squared`
- `natural_frequency_rad_s`
- `natural_frequency_hz`
- `period_s`
- `shape`
- `participation_norm`

The reported `shape` is expanded back to the full model DOF layout. Restrained
DOFs are reported as zero. The current participation norm is the Euclidean
norm of the expanded shape vector and is expected to be `1.0`.

## Ordering And Degenerate Modes

Modes are sorted by positive finite eigenvalue. For 2D cantilever review
fixtures, the retained frequencies are expected to be strictly increasing. For
3D symmetric cantilever fixtures, near-degenerate bending modes are allowed, so
frequency order is non-decreasing rather than strictly increasing.

Any future degeneracy-aware mode tracking must be explicit. This policy does
not claim stable mode-shape identity across symmetric degenerate pairs.

## Qualification Boundary

This policy is review evidence. It documents the shape and ordering contract
for retained modal-frame tests, but it is not a broad vibration qualification
claim for arbitrary frame meshes, damping, nonlinear joints, or experimental
modal correlation.
