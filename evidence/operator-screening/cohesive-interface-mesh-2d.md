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
substitute.

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
- every retained load step reports iterations, residual, load factor, and
  convergence, including its maximum connector force
- an underconstrained rigid mode is detected as a singular reduced tangent
- the singular step leaves displacement and damage at the committed zero state
- unknown materials, duplicate IDs, invalid connectivity, non-finite inputs,
  mutually active control modes, free-DOF prescriptions, unbounded controls,
  and invalid connector IDs, nodes, or component stiffness are rejected
- protocol serialization, Agent RPC, engine workflow, result chunking,
  Rust headless discovery, and self-hosted Web submission use one request

## Scope boundary

This is a real global cohesive-element equilibrium path, not merely a UI or
single-element history wrapper. The current screening implementation is bounded
to 512 nodes and uses a dense reduced solve. Linear component connector springs
are the first retained heterogeneous element contract, but the operator does not
yet co-assemble solid, shell, beam, or frame elements. Proportional displacement
control can traverse the retained monotonic softening path, while explicit
histories cover cyclic and non-proportional prescribed paths. Arc-length and
adaptive step control remain open alongside coupled mixed-mode damage, friction,
sparse assembly, and experimental calibration.
