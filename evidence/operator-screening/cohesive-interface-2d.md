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

Independent scalar cohesive histories evaluate shear and normal traction.
Normal compression uses a separate closure penalty and does not heal opening
damage. The global traction is the local traction rotated back to global
coordinates. With interface area `A = length * thickness`, each lower node
receives `-A traction / 2` and each upper node receives `+A traction / 2`.

## Retained checks

- pure opening and shear match their bilinear closed forms
- the active directional tangents match the softening slopes
- rotating the interface preserves local response and rotates global traction
- all four element nodal internal forces sum to zero
- unloading freezes independent normal and shear histories
- non-coincident pairs, repeated indices, degenerate length, incomplete
  displacement vectors, and non-finite data are rejected
- protocol, Agent RPC, engine workflow, Rust headless discovery, and
  self-hosted Web submission retain the same public request

## Scope boundary

This is a real element-response and nodal-force kernel suitable for later
assembly. It currently evaluates one prescribed displacement history; it is
not yet assembled into a multi-element global nonlinear equilibrium solve.
Normal and shear damage are uncoupled, so this is not a Benzeggagh-Kenane,
power-law, frictional post-failure, or experimentally calibrated mixed-mode
delamination claim.
