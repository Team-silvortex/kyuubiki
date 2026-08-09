# Cohesive Interface Mesh 2D Screening Evidence

Operator: `solve.cohesive_interface_mesh_2d`

Release line: `moxi 2.x`

## Retained assembly

The operator assembles multiple four-node zero-thickness cohesive elements into
one two-dimensional translational equilibrium system. Elements resolve a
material ID from a request-level catalog and retain two independent Gauss-point
histories. Nodes expose constraints, optional non-zero target displacements,
and proportional external loads. The same load factor advances both external
loads and prescribed displacement targets. As an alternative, an explicit
control history provides an independent load factor and complete constrained
node displacement vector at every step; the two input modes are mutually
exclusive. Optional two-node component connector springs use the same global
translational DOFs and Newton assembly as the cohesive elements. They provide a
small protocolized host proxy for heterogeneous equilibrium, not a bulk-element
substitute. Optional small-displacement linear 2D host trusses reuse the public
`TrussElementInput` and `TrussElementResult` contracts from `solve.truss_2d`.
They contribute physical `EA/L` axial stiffness, internal force, and tangent to
the same global system. Optional constant-strain plane-stress triangles likewise
reuse `PlaneTriangleElementInput` and `PlaneTriangleElementResult`, contributing
their continuum stiffness, internal force, and tangent without an adapter solve.
Optional bilinear plane-stress quads reuse `PlaneQuadElementInput` and
`PlaneQuadElementResult`. Their native `2 x 2` Gauss integration and positive
Jacobian guards participate directly in the same Newton matrix.

Each load increment uses Newton equilibrium on the reduced free-DOF system:

```text
residual = load_factor * external_load - assembled_internal_force
tangent  = assembled_element_consistent_tangents
```

Every Newton trial starts from the last committed Gauss-point histories.
Histories and displacements are committed only after the free-DOF residual
satisfies the configured tolerance. A singular tangent, non-finite update, or
iteration-limit failure returns the last converged state and a visible failure
reason.

## Retained checks

- one interface under uniform elastic opening matches `traction / stiffness`
- a two-element strip assembles the shared-node force and recovers the exact
  endpoint and middle reactions
- prescribed opening crosses peak traction, matches the softening closed form,
  and reaches complete failure with zero residual reaction
- an explicit load-unload-reload history freezes damage during unloading,
  resumes growth beyond the retained peak, and preserves the path maximum
- a non-proportional shear-then-opening history retains independent directional
  damage
- a linear connector-and-cohesive series system matches its closed form:
  connector force balances the incident cohesive nodal force, cohesive opening
  plus connector extension equals driver displacement, and connector energy is
  `force * extension / 2`
- a length-one host-truss-and-cohesive series system independently matches the
  same force and displacement decomposition while reporting exact strain,
  stress, axial force, and strain-energy density
- a prescribed apex host-plane-and-cohesive series system matches the analytic
  stiffness partition and recovers interface opening `0.005`, continuum
  extension `0.01`, common force `5`, and exact plane strain, stress, and energy
- a rectangular Q4 host-and-cohesive series system independently recovers the
  same opening, extension, common force, stress, and energy through Solver,
  Agent RPC, and Engine Workflow
- every retained load step reports iterations, residual, load factor, and
  convergence, including its maximum connector force
- an underconstrained rigid mode is detected as a singular reduced tangent
- the singular step leaves displacement and damage at the committed zero state
- unknown materials, duplicate IDs, invalid connectivity, non-finite inputs,
  mutually active control modes, free-DOF prescriptions, unbounded controls,
  and invalid connector IDs, nodes, or component stiffness are rejected
- invalid host-truss IDs, connectivity, area, modulus, and length are rejected
- invalid host-plane IDs, connectivity, thickness, modulus, Poisson ratio,
  triangle area, and Q4 Gauss-point Jacobians are rejected
- protocol serialization, Agent RPC, engine workflow, result chunking,
  Rust headless discovery, and self-hosted Web submission use one request

## Scope boundary

This is a real global cohesive-element equilibrium path, not merely a UI or
single-element history wrapper. The current screening implementation is bounded
to 512 nodes and uses a dense reduced solve. Linear component connector springs
establish the heterogeneous element contract, and small-displacement linear 2D
host trusses are the first retained structural host element. Constant-strain
plane-stress triangles now provide the first retained continuum host. The
same public host contract now includes fully integrated bilinear plane-stress
quads. The operator does not yet co-assemble shells, beams, frames, or 3D
solids. Proportional displacement control can traverse the retained monotonic
softening path, while explicit histories cover cyclic and non-proportional
prescribed paths. Arc-length and adaptive step control remain open alongside
coupled mixed-mode damage, friction, sparse assembly, and experimental
calibration.
