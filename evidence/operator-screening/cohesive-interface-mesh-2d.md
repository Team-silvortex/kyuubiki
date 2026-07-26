# Cohesive Interface Mesh 2D Screening Evidence

Operator: `solve.cohesive_interface_mesh_2d`

Release line: `moxi 2.x`

## Retained assembly

The operator assembles multiple four-node zero-thickness cohesive elements into
one two-dimensional translational equilibrium system. Elements resolve a
material ID from a request-level catalog and retain two independent Gauss-point
histories. Nodes expose zero-displacement constraints and proportional external
loads.

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
- every retained load step reports iterations, residual, load factor, and
  convergence
- an underconstrained rigid mode is detected as a singular reduced tangent
- the singular step leaves displacement and damage at the committed zero state
- unknown materials, duplicate IDs, invalid connectivity, non-finite inputs,
  and unbounded controls are rejected
- protocol serialization, Agent RPC, engine workflow, result chunking,
  Rust headless discovery, and self-hosted Web submission use one request

## Scope boundary

This is a real global cohesive-element equilibrium path, not merely a UI or
single-element history wrapper. The current screening implementation is bounded
to 512 nodes and uses a dense reduced solve. It does not yet co-assemble host
solid, shell, beam, or frame elements. Proportional load control also cannot
reliably traverse snap-back after peak traction; displacement control,
continuation, coupled mixed-mode damage, friction, sparse assembly, and
experimental calibration remain open.
