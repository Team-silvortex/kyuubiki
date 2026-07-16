# Stokes Flow Mesh Convergence Evidence

This evidence applies only to the Stokes-only screening operators:

- `solve.stokes_flow_quad_2d`
- `solve.stokes_flow_triangle_2d`

The retained convergence fixture uses the analytic linear field `u = y`,
`v = 0`, and `p = 0` on the unit square. The field is divergence-free and has a
constant shear rate of `1.0`, so both a coarse mesh and a refined mesh should
report the same screening diagnostics.

The regression file
`workers/rust/crates/solver/tests/stokes_flow_mesh_convergence.rs` checks both
quad and triangle paths. It compares the coarse and refined results for:

- total integrated element area
- maximum velocity
- pressure drop
- maximum divergence error
- maximum shear rate
- maximum viscous shear stress

This is a refinement-invariance check for the compact screening operator, not a
general Navier-Stokes validation. It does not claim turbulence, compressible
flow, transient flow, or industrial CFD accuracy.

Promotion boundary:

- This evidence is sufficient to qualify the current Stokes screening boundary
  scope.
- Any future Navier-Stokes, transient, turbulence, or production CFD claim must
  add separate convergence, benchmark, or external-reference evidence.
