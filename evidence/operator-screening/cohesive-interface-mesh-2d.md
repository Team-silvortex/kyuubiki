# Cohesive Interface Mesh 2D Screening Evidence

Operator: `solve.cohesive_interface_mesh_2d`

Release line: `moxi 2.x`

## Retained assembly

The operator assembles multiple four-node zero-thickness cohesive elements into
one two-dimensional translational equilibrium system. Elements resolve a
material ID from a request-level catalog and retain two independent Gauss-point
histories. Nodes expose translation constraints, optional frame rotation
constraints, non-zero target values, forces, and moments. The same load factor
advances external loads and prescribed targets. As an alternative, an explicit
control history provides an independent load factor and complete constrained
translation and optional rotation vectors at every step; the two input modes
are mutually exclusive. Optional two-node component connector springs use the same global
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
Optional linear Euler-Bernoulli frame hosts reuse `Frame2dElementInput` and
`Frame2dElementResult`. Their appended rotational DOFs preserve the existing
translation indexing while retaining the native transformed `6 x 6` frame
stiffness, axial force, shear, end moments, stress, and strain energy. Rotations
on nodes outside the frame topology are constrained automatically so they
cannot create artificial singular modes.

Each load increment uses Newton equilibrium on the reduced free-DOF system:

```text
residual = load_factor * external_load - assembled_internal_force
tangent  = assembled_element_consistent_tangents
```

Every cohesive and host kernel writes through the shared `MatrixAssembler`
contract into a sparse global tangent. Constraint projection retains only the
free sparse rows and columns. Narrow positive-definite tangents first use a
reusable symmetric-band Cholesky factor with iterative refinement; invertible
indefinite or wide tangents within the retained model bound use a pivoted dense
fallback. Every step reports `tangent_non_zero_count`, `tangent_fill_ratio`,
and `linear_solver`; the result retains maxima and the distinct solver methods
used across the accepted path.

Every Newton trial starts from the last committed Gauss-point histories.
Histories and displacements are committed only after the free-DOF residual
satisfies the configured tolerance. A singular tangent, non-finite update, or
iteration-limit failure returns the last converged state and a visible failure
reason.

The relative residual scale is frozen at the start of each load step as the
maximum of one, the norm of the actual `load_factor * external_load` on free
DOFs, and the initial free-DOF residual. The initial residual supplies the scale
for prescribed motion and unloading. Support loads cannot loosen this test.
Norms use scaled sum-of-squares accumulation, and convergence compares the
residual-to-scale ratio without multiplying the tolerance by a large load.
Non-finite force residuals, including on constrained DOFs, fail the step before
history commit. An unrepresentable failed residual norm is reported as
`f64::MAX` with a non-finite failure reason so the report remains serializable.
Failed-step state summaries, including reactions, use the last accepted load
factor; the step's control fields still identify the attempted command.

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
- a tip-loaded frame host independently recovers the Euler-Bernoulli relative
  deflection `P L^3 / (3 E I)`, rotation `P L^2 / (2 E I)`, root moment `P L`,
  bending stress, and strain energy while its translating root remains in
  equilibrium with the cohesive interface
- a 96-element, 384-node, 768-DOF block interface model retains 3,072 tangent
  nonzeros, a `0.005208` fill ratio, and the reported
  `symmetric_band_cholesky` solve path
- every retained load step reports iterations, residual, load factor, and
  convergence, including its maximum connector force
- an underconstrained rigid mode is detected as a singular reduced tangent
- the singular step leaves displacement and damage at the committed zero state
- support loads up to `1e180` leave the free opening and traction unchanged
- equivalent base-load/factor parameterizations from `1e-180` to `1e180`
  recover the same elastic equilibrium; changing force units by `1e160`
  preserves both force-driven and partially constrained displacement-driven
  solutions without overflowing the retained norms
- an overflowing later free or support load preserves the accepted
  displacement, reactions, and damaged history; its failure report round-trips
  through JSON without null numeric fields
- unknown materials, duplicate IDs, invalid connectivity, non-finite inputs,
  mutually active control modes, free-DOF prescriptions, unbounded controls,
  and invalid connector IDs, nodes, or component stiffness are rejected
- invalid host-truss IDs, connectivity, area, modulus, and length are rejected
- invalid host-plane IDs, connectivity, thickness, modulus, Poisson ratio,
  triangle area, and Q4 Gauss-point Jacobians are rejected
- invalid host-frame IDs, connectivity, section properties, and orphan
  rotational loads or prescribed rotations are rejected
- protocol serialization, Agent RPC, engine workflow, result chunking,
  Rust headless discovery, and self-hosted Web submission use one request

## Scope boundary

This is a real global cohesive-element equilibrium path, not merely a UI or
single-element history wrapper. The current screening implementation is bounded
to 512 nodes and uses sparse global assembly and sparse constraint reduction.
Narrow positive-definite systems remain sparse through the banded solve;
invertible indefinite or wide systems retain a pivoted dense fallback bounded
to 1,536 free DOFs. Linear component connector springs
establish the heterogeneous element contract, and small-displacement linear 2D
host trusses are the first retained structural host element. Constant-strain
plane-stress triangles now provide the first retained continuum host. The
same public host contract now includes fully integrated bilinear plane-stress
quads and linear Euler-Bernoulli 2D frames. The operator does not yet
co-assemble shells or 3D solids. Proportional displacement control can traverse the retained monotonic
softening path, while explicit histories cover cyclic and non-proportional
prescribed paths. Arc-length and adaptive step control remain open alongside
coupled mixed-mode damage, friction, scalable sparse-indefinite factorization,
fill-reducing reordering, and experimental calibration.
