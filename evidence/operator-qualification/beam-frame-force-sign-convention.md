# Beam, Frame, And Torsion Sign Convention Note

Status: collecting evidence for `beam-frame-classic`.

This note defines the sign conventions used by the first classic structural
qualification packet. It is scoped to the compact linear fixtures in:

- `workers/rust/crates/solver/tests/beam_frame_classic_regression.rs`
- `workers/rust/crates/solver/tests/beam_1d_review.rs`
- `workers/rust/crates/solver/tests/frame_2d_review.rs`
- `workers/rust/crates/solver/tests/torsion_1d_review.rs`

## Coordinate And Displacement Signs

- Positive `x` follows the element direction from `node_i` to `node_j`.
- Positive frame `y` is the global upward direction used by `Frame2dNodeInput`.
- Positive beam/frame transverse displacement follows positive `y`.
- A downward point load is represented as negative `load_y`.
- Positive rotation `rz` is the positive right-handed rotation about the
  out-of-plane `z` axis.

The cantilever fixtures therefore expect a downward tip load to produce
negative tip `uy` and negative tip `rz` with the current solver convention.

## Beam And Frame Force Signs

For the cantilever beam and equivalent 2D frame fixture:

- `moment_i` is positive at the fixed/root end for a downward tip load.
- `moment_j` is zero at the free/tip end for the point-load fixture.
- `shear_force_i` and `shear_force_j` have opposite signs, with the pair
  balancing the applied nodal load.
- Bending stress is compared by magnitude through `max_bending_stress` and
  `max_combined_stress`.

The qualification regression uses the closed-form magnitudes:

- tip displacement: `P L^3 / (3 E I)`
- tip rotation: `P L^2 / (2 E I)`
- root moment: `P L`
- peak bending stress: `P L / S`

where the explicit sign check stays on `uy` and `rz`, while stress and moment
summary fields are treated as non-negative review magnitudes.

## Torsion Signs

For the prismatic shaft fixture:

- Positive `torque_z` produces positive `rz` at the free end.
- The element `torque` summary is compared as a positive applied torque
  magnitude.
- Shear stress is compared as `T / S_t`, where `S_t` is the torsional section
  modulus used by the fixture.

This note does not cover warping torsion, non-prismatic shafts, or section
principal-axis transformations. Those require separate scope notes before any
stronger claim.

## Promotion Constraint

The `beam-frame-classic` candidate still needs retained release provenance for
the multi-case regression output before any manifest entry can move from
`review` to `qualification`.

