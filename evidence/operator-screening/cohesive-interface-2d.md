# Cohesive Interface 2D Screening Evidence

Operator: `solve.cohesive_interface_2d`

Release line: `moxi 2.0.x`

## Retained element

The operator evaluates one four-node zero-thickness line interface. The lower
surface is `lower_i -> lower_j`; the upper surface uses coincident
`upper_i -> upper_j` nodes. The lower geometry defines the local tangent, and
its left normal defines positive opening. At every retained history step:

```text
jump = average(upper displacement) - average(lower displacement)
delta_s = jump dot tangent
delta_n = jump dot normal
```

Two Gauss points interpolate the upper/lower jump with linear shape functions,
and each point retains independent scalar shear and normal histories. Normal
compression uses a separate closure penalty and does not heal opening damage.
Tractions rotate back to global coordinates and integrate into four nodal
forces. The same quadrature assembles the symmetric `8 x 8` material tangent
in the fixed lower-i/lower-j/upper-i/upper-j translational DOF order.

## Retained checks

- pure opening and shear match their bilinear closed forms
- the active directional tangents match the softening slopes
- rotating the interface preserves local response and rotates global traction
- antisymmetric endpoint jumps retain non-zero stiffness instead of becoming a
  center-integration zero-energy mode
- rigid translation produces zero jump, traction, and nodal internal force
- every column of the assembled tangent matches a nodal-force central
  difference on a mixed elastic/softening integration state
- all four element nodal internal forces sum to zero
- unloading freezes independent normal and shear histories
- non-coincident pairs, repeated indices, degenerate length, incomplete
  displacement vectors, and non-finite data are rejected
- protocol, Agent RPC, engine workflow, Rust headless discovery, and
  self-hosted Web submission retain the same public request

## Scope boundary

This is a real element-response and nodal-force kernel. It evaluates one
prescribed displacement history; the separate
`solve.cohesive_interface_mesh_2d` operator now assembles the same trial/commit
kernel into a bounded multi-element global nonlinear equilibrium solve.
Normal and shear damage are uncoupled, so this is not a Benzeggagh-Kenane,
power-law, frictional post-failure, or experimentally calibrated mixed-mode
delamination claim.
