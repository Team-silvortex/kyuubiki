# Bilayer bending energy diagnosis and bridge input repair

Live experiments: 2026-11-04. Offline interpolation audit and report: 2026-11-11.
Runtime and Rust Headless SDK: daji 3.7.8. Continues the
[non-affine bending study](model-research-thermal-bending-20261104.md).
The [HTML guide](../docs/research-model-driven.html#bending-energy) describes reproduction.

## Findings

The 32 x 16 Q4 bilayer-gradient mesh still has 4.919177% ordinary relative
elastic-energy error. This was not eliminated by changing a tolerance, replacing
a result or changing the physics. At 64 x 32 the error falls to 1.235404%.
Directional refinement, strain-error decomposition and an independently assembled
direct solve identify predominantly longitudinal Q4 interpolation error in this
benchmark, rather than energy reporting or linear-solve error.

The larger workflow also exposed a real product defect: the heat-to-thermo bridge
required the structural seed mesh in its small configuration area. At 64 x 32,
the configuration contained 39,798 JSON value nodes and exceeded the 20,000-node
safety budget. The first attempt completed eight workflows, then received HTTP
422 before the ninth job could be created. It stopped without retry.

Both Rust and Orchestra now accept `heat_result` and `seed_model` as separate
bridge input ports. Configuration retains only the transfer contract. The bridge
records both data dependencies, rejects conflicting/missing/extra envelope fields,
and preserves legacy single-input calls. Neither the configuration budget nor
the separate 500,000-node input-artifact budget was increased. The shared Rust
research builder now uses these ports; a recovery test also stopped relying on
the structural node's array position. See the
[bridge contract](../docs/workflow-graph.md#heat---thermo_mechanical).

## Experiment boundary

The model selected four scenarios in three batches, observed all twelve actual
SDK -> Orchestra -> Rust Agent heat/thermo workflows, and then produced findings.
The caller's fixed coverage matrix reused those completed results without new
submissions. A second model accepted the bounded scope with no blocking issues.

Geometry, loading and materials are unchanged from the bilayer reference:
1 x 0.25 m, thickness 0.01 m, two equal y-layers, E = [70, 20] GPa,
nu = [0.2, 0.35], alpha = [10, 30]e-6/K and k = [10, 40] W/(m K).
Bottom/top/reference temperatures are 20/40/20, except the uniform control uses
30/30/20. Three restraints remove rigid-body modes. Nonzero analytical end
tractions are self-equilibrated in force and moment. This is not a fully free
bilayer, a clamped cantilever, a distorted mesh or nonlinear material qualification.

| Scenario | Mesh sequence | Ordinary relative elastic-energy error |
| --- | --- | --- |
| Gradient, both directions | 16x8 -> 32x16 -> 64x32 | 19.337438% -> 4.919177% -> 1.235404% |
| Gradient, x only | 16x16 -> 32x16 -> 64x16 | 18.363770% -> 4.919177% -> 1.467906% |
| Gradient, y only | 32x8 -> 32x16 -> 32x32 | 5.857466% -> 4.919177% -> 4.684433% |
| Uniform, both directions | 16x8 -> 32x16 -> 64x32 | 5.911229% -> 1.499215% -> 0.376040% |

The finest mesh has 2,145 nodes and 2,048 elements. Both isotropic sequences
pass their predeclared 20%-per-step error reduction and final <=2% ordinary
relative energy-error gates. Directional probes have no final accuracy gate.
Approximately quadratic decay of energy error is not quadratic convergence of
the strain-error norm, and these gates do not bound pointwise stress error.

## Discrete solve and energy checks

The reference assembles rectangular Q4 stiffness and thermal loads independently,
then uses banded Cholesky without regularization. It imports no production solver
or element routines. It shares the declared case/model inputs and the Q4
discretization, so it is not a second continuum model.

Across the twelve cases, production versus direct energy differs by at most
4.4555e-11 J and displacement by 1.4860e-10 in relative L2 norm. The corresponding
predeclared limits are 1e-6 J and 1e-5. Independently reconstructed reported
energy differs by at most 9.9921e-16 J. The largest reconstructed force residual
is 8.1325e-6 N, within the case gate. No production solver tolerance was changed.
Runtime outputs do not certify backend choice or iteration counts; a controlled
tolerance sweep remains unperformed.

At 32 x 16, continuum energy is 0.30305220883534134 J and computed energy is
0.31795988396059743 J. The 0.01490767512525609 J excess is distinct from strain-error
energy, 0.01611619066094847 J. Of the latter, 0.015087830884642044 J (93.6191%)
is shear and 0.0010283597763064626 J is coupled normal error. The normal component
retains x/y Poisson coupling; 93.6% is not a fraction of the elastic-energy excess.

The two checked identities are also distinct:

```text
U_h - U_exact - f^T (u_h - u_exact) = Uerr
2 U_h = f^T u_h - integral sigma_h : thermal_strain dV
```

Their maximum residuals are 1.4721e-14 J and 3.1132e-11 J, respectively. Sol's
last prose finding accidentally attached the smaller residual to the second
identity; the retained numerical fields and this report use the correct labels.
Model prose never overrides numerical gates. Final f^T u is not path work,
and reconstructed equilibrium does not validate a solver reaction-output API.

## Offline interpolation experiment

On November 11, followed the models' proposed interpolation experiment using
the twelve retained real results, without new workflows or model requests.
Let Iu be the Q4 nodal interpolant of the exact continuum displacement. Within
an interface-aligned rectangular element of width hx and height hy:

```text
strain_x(Iu) - strain_x(exact) = 0
strain_y(Iu) - strain_y(exact) = -s_i (y - y_mid)
gamma_xy(Iu) = kappa (x - x_mid)
s_i = (1 + nu_i) * slope(thermal_eigenstrain_i) - nu_i * kappa

Uerr_I_normal = hy^2 / 24 * sum_i V_i E_i/(1-nu_i^2) s_i^2
Uerr_I_shear  = hx^2 / 24 * sum_i V_i G_i kappa^2
```

The closed forms match separate Q4 energy quadrature. Doubling nx quarters the
interpolant shear term without changing its normal term; doubling ny quarters
the normal term without changing the shear term. This agrees with the observed
directional sensitivity. The general bilinear-Q4 inability to represent bending
without parasitic shear is described in the
[University of Alberta element derivation](https://engcourses-uofa.ca/books/introduction-to-solid-mechanics/finite-element-analysis/two-dimensional-solid-elements/quadrilateral-elements/).
The quantitative attribution here is specific to our declared reference.

For the baseline mesh, interpolant error energy is 0.0162685337399395 J and
equilibrated FE error energy is 0.01611619066094847 J. Their reduction is accounted
for by the 0.00015234307972359062 J energy of the displacement correction.
All twelve cases satisfy the Galerkin decomposition
`Uerr_I = Uerr_h + 0.5 integral strain(Iu-u_h):C:strain(Iu-u_h) dV`
within 1.4872e-12 J (predeclared audit tolerance 1e-9 J). Exact nodal samples
are therefore not an equilibrium solution and must not be substituted for one.
This offline audit was not sent back to a model reviewer.

## Runtime and regression evidence

With user approval, Installer replaced the idle local 3.7.8 runtime after a sealed
rebuild. The GUI was not repackaged. Existing state and research records were
preserved; the superseded immutable payload was removed after healthy startup.
Installed manifest SHA-256 is
`8aa3bc00cb40bf40234bae56774d13708eee5b3ec7d6986ac476ba01f0ea41f0`.
Two Agents were ready after installation and remain healthy on November 11.

Regression results: 967 Rust engine library/integration tests passed (one ignored),
307 Rust SDK tests passed (59 opt-in tests ignored), and 112 Orchestra
bridge/input/preflight tests passed. Strict Rust linting, formatting and
documentation checks also passed. Red/green tests cover the new seed contract, backward
compatibility, dual dependency lineage, conflicting inputs, unchanged budgets,
the shared builder and the interpolation closed forms. This is not an all-platform
or complete repository qualification.

Live attempt A used three model requests and eight completed workflows; its next
submission was rejected. Completed attempt B used four Sol requests, one Astra
request and twelve workflows. Combined: eight API requests, 59,198 tokens,
twenty completed workflows and estimated USD 0.383236 at the pinned standard
rates. This is not an invoice or account balance. No paid calls were made on
November 11. Approvals, usage, checks, job IDs, digests and model reviews remain
under `tmp/model-bending-energy-20261104-a` and `tmp/model-bending-energy-20261104-b`.

After checking all twenty completed jobs against the current service, matching
request/result hashes and reading an original pre-update result back unchanged,
removed only the 61 owned request/result/terminal payload files (215,267,955 bytes,
205.30 MiB), including the known preflight-rejected request. Compact evidence and
the offline interpolation audit remain. The cleanup audit and receipt are
`tmp/bending-energy-cleanup-audit-20261111.json` and
`tmp/bending-energy-cleanup-receipt-20261111.json`. No installed database, external
credential or unrelated run was deleted. Raw replay now requires regeneration.

The subsequent [Q9 order comparison](model-research-bending-order-20261111.md)
tests a bounded higher-order reference at equal cell and equal DOF counts.
Production integration, actual backend/iteration telemetry, and fully
traction-free bilayer end effects remain separate work. Mesh refinement reduced
this discrepancy; this round did not improve the production Q4 approximation order.
