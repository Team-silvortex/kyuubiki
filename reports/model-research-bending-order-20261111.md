# Full-integration Q9 comparison for bilayer bending

Date: 2026-11-11. Rust research example: daji 3.7.8.
Continues the [Q4 energy diagnosis](model-research-bending-energy-20261111.md)
and its model-proposed higher-order comparison. Reproduction is in the
[HTML research guide](../docs/research-model-driven.html#bending-order).

## Finding and scope

The local Q9 reference resolves the piecewise-quadratic displacement of the
interface-aligned bilayer benchmark to floating-point accuracy. For the gradient
case, Q4 at 32 x 16 has 4.919177% relative elastic-energy error. Q9 at 16 x 8 has
the same 561 nodes and 1,119 free mechanical DOFs, with absolute energy error
1.0991e-13 J and relative energy error 3.6268e-13 (dimensionless).

This supports changing approximation order for this case rather than tightening
the Q4 linear-solver tolerance. It does not change the production Q4 result or
certify arbitrary meshes. Q9 is an independent, bounded research implementation,
not a registered operator, public SDK solver, or installed Agent capability.
No new Agent workflows, model API requests, installations or provider reviews
were performed in this round. Existing model findings motivated the experiment;
its numerical gates and new results are local and deterministic.

## What was solved

Geometry, material properties, analytical end tractions and three rigid-mode
restraints match the earlier reference: 1 x 0.25 m, thickness 0.01 m,
two equal y-layers, E = [70, 20] GPa, nu = [0.2, 0.35],
alpha = [10, 30]e-6/K, k = [10, 40] W/(m K). Gradient temperatures are
bottom/top/reference = 20/40/20; the uniform case uses 30/30/20.
These heterogeneous cases have nonzero self-equilibrated end tractions; they
are not fully traction-free strips or clamped cantilevers.

Q9 uses the tensor product of the three nodal quadratic Lagrange polynomials
on [-1, 1]. The resulting nine-node basis spans Q2, including x*y, x^2 and y^2.
The [DefElement Lagrange definition](https://defelement.org/elements/lagrange.html)
gives the quadrilateral tensor-product polynomial space and (k+1)^2 DOFs.
The interpretation here is specific to straight-sided rectangles with a mesh
line on the material interface, not arbitrary isoparametric cells.

The prototype performs these operations, without receiving exact displacements:

1. Assemble and solve the Q9 steady conduction system, prescribing only the
   bottom/top temperature differences. Interior temperatures are unknowns.
2. Transfer all nine nodal temperatures on the matching mesh into the thermal
   strain load, including midside and centre nodes.
3. Integrate stiffness, thermal loads and recovered energy with full 3 x 3 Gauss
   integration. No reduced integration, hourglass penalty or stiffness damping.
4. Integrate analytical end tractions with quadratic edge functions and three
   Gauss points. Midside forces are not replaced by Q4 corner loads.
5. Solve the mechanical system with three zero rigid-mode restraints and banded
   Cholesky, without regularization. Reconstruct heat/mechanical residuals and
   energy checks from the solved fields.

The new Q9 and the existing independent Q4 reference share the banded linear
solver and physical case definition, not element kernels. This is an
approximation-order comparison, not independent validation of the linear solver.
The local Q4 comparison uses the analytical layerwise-linear temperature field;
Q9 solves it. The earlier live study separately verified actual Q4 heat/bridge
fields and matched the independent Q4 mechanical reference within 4.46e-11 J.

## Results

Four Q4 mechanical solves were compared with both equal-cell and equal-DOF Q9
solves. Four additional Q9 controls cover homogeneous free bending, reversed
free bending, reversed bilayer gradient and a shifted temperature reference.
There are twelve Q9 heat/mechanical solve pairs in total, including repeated
meshes used in the two comparison bases; these are not twelve distinct meshes.

| Case | Q4 mesh | Q4 relative energy error | Equal-DOF Q9 mesh | Q9 absolute energy error |
| --- | --- | --- | --- | --- |
| Gradient | 16 x 8 | 19.337438% | 8 x 4 | 9.4369e-15 J |
| Gradient | 32 x 16 | 4.919177% | 16 x 8 | 1.0991e-13 J |
| Uniform | 16 x 8 | 5.911229% | 8 x 4 | 3.0531e-16 J |
| Uniform | 32 x 16 | 1.499215% | 16 x 8 | 1.3212e-14 J |

Equal-cell Q9 comparisons also passed. The largest Q9 system has 2,145 nodes
and 4,287 free mechanical DOFs. Across all twelve Q9 pairs:

| Check | Largest observed value | Predeclared limit |
| --- | --- | --- |
| Absolute elastic-energy error | 3.0743e-13 J | 1e-8 J |
| Relative nodal displacement error | 4.0326e-12 | 1e-8 |
| Temperature error | 8.4200e-13 K | 1e-8 K |
| Strain-error energy | 2.0883e-23 J | 1e-14 J |
| Free mechanical residual | 2.8008e-9 N | 1e-4 N |
| Reconstructed restraint reaction | 1.8930e-8 N | 1e-4 N |
| Free heat residual | 8.8201e-14 W | 1e-8 W |
| Boundary heat-rate error | 3.1886e-12 W | 1e-8 W |
| Constitutive energy identity residual | 9.6584e-12 J | 1e-8 J |

The identity is `2U = f^T u - integral sigma:thermal_strain dV`;
final force/displacement pairing is not loading-path work. Exact equilibrium
reactions are zero for these manufactured end loads, so the small reconstructed
values are numerical residuals, not measured support forces.
Homogeneous free-bending controls use absolute energy gates and report ordinary
relative energy error as null because reference energy is zero.

The exact solution is in this Q9 approximation space. Near-roundoff agreement
is therefore a polynomial reproduction result, not a general accuracy estimate.
The small error increase on finer Q9 meshes is not an approximation convergence
rate. No speedup claim or performance benchmark follows from these runs.

## Regressions and evidence

Eight new regression tests check the nodal basis and derivatives of all nine
tensor monomials, six physical scenarios, invalid mesh/material bounds, exactly
three element rigid modes, midside/centre thermal participation, quadratic
traction virtual work, corrupted fields, and malformed comparison evidence.
The element rank test finds rank 15 for its 18 mechanical DOFs under full
integration. Evidence with missing geometry, inconsistent DOF counts or negative
absolute error is rejected rather than accepted through null equality.

Rust SDK all-target tests: 315 passed, 59 opt-in tests ignored. Strict linting
and formatting passed. The initial restricted test attempt could not open a
local test port; the permitted full rerun passed. No production Rust solver,
Orchestra or GUI code changed, and no full cross-platform qualification is claimed.

Only compact numerical evidence is retained in `tmp/bending-order-final-20261111`;
no full field payloads, database copies or credentials are written. The report
records the source definition SHA-256:
`abbdf092b53f9fe40f227db0c1896dfbca41839ce6e512ae996b38bddd2a058c`.
The superseded local trial output was removed after the final rerun.

## Next integration boundary

Production use still needs an explicit quadratic-element task schema and Rust
operator implementation, shared-node connectivity and Jacobian validation,
high-order temperature/traction contracts, cancellation and size budgets, and
actual SDK -> engine -> result tests. Operator math should remain outside the
generic execution engine and headless control SDK. Distorted cells, nonmatching
thermal meshes, fully free bilayer end effects, nonlinear materials and external
solver cross-validation remain separate work.
