# Cohesive Interface Mesh 3D Screening Evidence

Operator: `solve.cohesive_interface_mesh_3d`

Release line: `moxi 2.x`

## Retained model

The operator assembles six-node, zero-thickness triangular interfaces into one
three-dimensional translational equilibrium system. The lower and upper
triangles must contain distinct node IDs whose initial coordinates coincide in
pairs. A local orthonormal frame is derived from the lower triangle. Three-point
triangle integration retains two independent tangential histories and one
normal history at every integration point. The normal law keeps an independent
compression penalty while opening and both shear directions use the retained
bilinear damage law.

Optional `SolidTetra3dElementInput` hosts share the same node indices and global
degrees of freedom. They reuse the exact `B`, elasticity, stiffness, stress,
von-Mises, and energy kernel used by `solve.solid_tetra_3d`; no adapter solve or
post-processing force transfer is involved. Interface and host internal forces
therefore enter the same Newton residual and sparse tangent.

Each accepted load step reports residual, iterations, reactions, maximum
traction, directional damage, host stress, tangent nonzero count, fill ratio,
and the selected linear solver. Trial histories are copied from the last
accepted state and committed only after the free-DOF residual converges.

## Retained checks

- uniform opening of a unit right triangle recovers separation `0.005`, traction
  `5`, and equal nodal reactions from the independent triangular closed form
- a prescribed load-unload path reaches damage `0.75`, keeps it on unload, and
  returns the damaged-secant traction and unloading regime at every Gauss point
- rotating the triangle from the global XY plane to the YZ plane rotates the
  local normal while preserving opening, traction, and nodal displacement
- a one-free-DOF interface plus linear tetra host matches the independent
  stiffness sum: displacement `0.006`, average interface traction `2`, solid
  stress `-6`, two shear stresses `-3`, and von Mises stress `sqrt(90)`
- separated node pairs, unknown materials, missing host nodes, degenerate
  triangles, invalid material laws, and invalid controls fail before iteration
- an 80-element block retains 480 nodes, 1,440 global DOFs, 8,640 tangent
  nonzeros, fill ratio `1/240`, and the reported
  `symmetric_band_cholesky` path
- protocol serialization, Agent RPC, Engine workflow execution, result
  chunking, Rust headless discovery, and self-hosted Web submission expose the
  same request and result contract

## Scope boundary

This is a small-strain triangular cohesive surface with uncoupled directional
damage and linear tetrahedral hosts. The implementation is bounded to 512 nodes
and 4,096 interface elements. Narrow positive-definite systems stay sparse;
indefinite or wide systems retain the bounded 1,536-free-DOF pivoted dense
fallback. Coupled mixed-mode criteria, frictional closure, finite rotations,
higher-order faces, shells, scalable sparse-indefinite factorization,
fill-reducing ordering, arc-length continuation, adaptive load stepping, and
experimental calibration remain outside this screening claim.
