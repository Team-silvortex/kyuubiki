# Model-driven non-affine thermal bending

Date: 2026-11-04. Rust Headless SDK and installed macOS runtime: daji 3.7.8.
Continues [free-transverse and shear patches](model-research-planar-patches-20261104.md).
Runnable entry: [HTML bending guide](../docs/research-model-driven.html#thermal-bending).

Follow-up: the [bilayer energy diagnosis](model-research-bending-energy-20261111.md)
isolates the 4.92% energy gap, extends directional refinement, and repairs the
large structural-seed input path. Results below retain their original scope.

## Scope And Boundary Conditions

The previous affine patch tests were almost exactly representable by Q4 elements.
This round deliberately uses a piecewise-quadratic displacement field, nonuniform
temperature and through-height material jumps. It separates numerical consistency
from continuum approximation error and tests convergence over four meshes.

The real chain is model tool selection -> official Rust SDK -> local Orchestra
-> native Rust Agent -> steady Q4 heat -> node-to-node temperature bridge ->
plane-stress thermoelastic Q4. No solver mocks, GUI dependence or arbitrary model
code execution is used. Model-provider integration remains example-only, outside
the public SDK and engine. Dispatch/readback/manifest handling is shared with the
earlier runner rather than implemented as a separate backend.

Geometry: x in [0, 1] m, y in [0, 0.25] m, thickness 0.01 m. Heat fixes bottom
and top temperatures, insulates vertical sides and has no source. Mechanics fixes
only origin ux/uy and bottom-right uy to remove three rigid modes. These are not
clamped-cantilever conditions. Temperature increments are in K; shifting the
affine temperature origin changes no constant-property reference physics.

| Case | Bottom/top/reference temperature | Material and end condition |
| --- | --- | --- |
| gradient_free | 20 / 40 / 20 | homogeneous, zero mechanical loads |
| gradient_free_reverse | 20 / 0 / 20 | homogeneous, zero mechanical loads |
| bilayer_gradient | 20 / 40 / 20 | two y-layers, analytical end tractions |
| bilayer_gradient_reverse | 20 / 0 / 20 | two y-layers, analytical end tractions |
| bilayer_uniform | 30 / 30 / 20 | two y-layers, analytical end tractions |
| bilayer_shift | 293.15 / 313.15 / 293.15 | shifted bilayer-gradient reference |

Homogeneous properties: E = 70 GPa, nu = 0.3, alpha = 12e-6/K,
k = 20 W/(m K). Equal-height bottom/top layers use E = [70, 20] GPa,
nu = [0.2, 0.35], alpha = [10e-6, 30e-6]/K and k = [10, 40] W/(m K).
All are synthetic, linear, temperature-independent, perfectly bonded materials.

**The bilayer cases are not fully traction-free strips.** Their end tractions
are nonzero locally but have zero resultant force and moment on each end.
This distinction makes an exact two-dimensional reference available without
pretending that a beam approximation is an exact free-end elasticity solution.

## Independent Continuum Reference

Starting from the
[plane-stress constitutive law](https://www.mathworks.com/help/pde/ug/3-d-linear-elasticity-equations-in-toolbox-form.html),
the following solution is derived for the declared heat field and end tractions.
No production element routines are imported by the reference or checker.

Temperature follows two series thermal resistances. In each layer, write thermal
eigenstrain as t(y) = alpha (T(y) - Tref) = a + b y. Obtain e0 and kappa from:

```text
A = integral E dy; B = integral E y dy; D = integral E y^2 dy
NT = integral E t(y) dy; MT = integral E t(y) y dy
[A B; B D] [e0; kappa] = [NT; MT]

ex = e0 + kappa y
sx = E (ex - t(y)); sy = tau = 0
ey = t(y) - nu sx/E
ux = (e0 + kappa y) x - kappa y/2
uy = integral_0^y ey(s) ds + kappa x(1-x)/2
```

Integrals are evaluated layerwise. This produces continuous displacement,
compatible strains, zero stress divergence, continuous interface traction and
the three required zero constrained displacements. A jump in sx at a horizontal
interface is admissible because interface traction uses sy and tau, both zero.
End loads are the consistent nodal integrals of sx(y) n_x, not pointwise stress
samples. Zero section resultants do not imply pointwise zero end traction.

The homogeneous gradient gives e0 = 0, kappa = 0.00096/m, bottom-centre bow
120 micrometres and zero continuum elastic energy. The bilayer gradient gives
e0 = -7.74297188755e-5, kappa = 0.00287614457831/m and bow 359.518072289
micrometres. Uniform bilayer heating gives bow 134.939759036 micrometres.
These corner/flux references are separately hard-coded in the matrix verifier.

Unlike an [affine patch test](https://openseesdigital.com/2021/04/04/did-you-pass-your-patch-test/),
quadratic displacement is not Q4-exact. Exact continuum nodal samples alone do
not satisfy the discrete equilibrium equations; a regression explicitly rejects
such a fabricated exact result.

## Numerical Gates And Their Meaning

The checker reconstructs strain, stress, internal forces and energy using separate
rectangular Q4 2 x 2 Gauss quadrature. It checks returned stress/strain means,
invariants of mean stress, energy density, geometry, connectivity, node identity,
bridge loads/restraints/materials, nodal temperature and vertical heat flux.
It does not qualify pointwise peaks from mean stress outputs.

With e_h and e_exact denoting total strain, define:

```text
Uerr = 0.5 integral (e_h-e_exact) : C : (e_h-e_exact) dV
U_h - U_exact - f^T (u_h-u_exact) = Uerr
2 U_h = f^T u_h - integral sigma_h : thermal_strain dV
```

These identities distinguish quadratic strain-error energy, discrete elastic
energy and the difference between discrete and continuum elastic energies.
The thermal term has a minus sign in the last identity. The final load-displacement
pairing is not path-integrated work. Reconstructed force residuals are not
solver-returned reactions or a validation of that output interface.

Normalizations are explicit, not division by a possibly zero continuum energy:

- Nodal error: sqrt(sum squared displacement errors / sum squared exact nodal
  displacements), over both components at all nodes.
- Stress scale S: E_max alpha_max max(max absolute temperature increment, 1 K).
- Stress error: sqrt(integral (dsx^2 + dsy^2 + 2 dtau^2) / volume) / S.
- Energy scale: S^2 volume / E_min. Normalized error energy is Uerr / this scale.
- Relative total elastic-energy error is separately reported only when its
  continuum denominator exceeds 1e-20 J; otherwise it is null, not zero.

Before execution, the matrix required nodal error and normalized error energy to
decrease by at least 20% at every mesh doubling. At r = 8, nodal RMS and midpoint
bow errors must each be <= 2%, normalized error energy <= 0.001 and normalized
stress L2 error <= 0.1. These are catalog gates, not universal error bounds.
Per-case passing flags require consistency and a broad nodal-error bound; they
do not mean a coarse mesh meets the final accuracy gates. No limits were loosened.

## Live Results

The completed run is `tmp/model-bending-20261104-b`. Sol selected all six scenarios
at all four mesh levels in six batches, then returned findings on its seventh
call. The caller's fixed 24-case matrix reused those retained results without
extra submissions. Astra accepted the bounded scope with no blocking issues,
while explicitly identifying the continuum-energy and normalization limitations.
The reviewer did not independently authenticate job execution.

Meshes use 4r x 2r rectangular cells. Refinements 1/2/4/8 have 15/45/153/561
nodes and 8/32/128/512 Q4 elements. All 24 actual workflow jobs completed with
valid output manifests, matching second result reads and passing matrix gates.

| r | Free-gradient nodal error | Bilayer-gradient nodal error | Free-gradient bow, micrometres | Bilayer-gradient bow, micrometres |
| ---: | ---: | ---: | ---: | ---: |
| 1 | 21.5139% | 32.1177% | 94.2690 | 239.2577 |
| 2 | 7.2559% | 11.5180% | 111.6048 | 317.4462 |
| 4 | 2.0651% | 3.3347% | 117.6824 | 347.6514 |
| 8 | 0.5599% | 0.8994% | 119.3895 | 356.3944 |

Reversal changes the displacement sign and preserves energy/error magnitude to
numerical precision. Shifting both temperatures and reference preserves the
bilayer response. The uniform bilayer reaches 0.8342% nodal error and 0.9373%
bow error at r = 8. Passing exact temperature checks does not make displacement
exact, and a successful job on the coarsest mesh is not an accurate continuum solve.

The largest remaining accuracy gap is elastic energy:

| r=8 case | Continuum elastic energy, J | Computed energy, J | Ordinary relative energy error |
| --- | ---: | ---: | ---: |
| free gradient | 0 | 0.00431509894 | undefined |
| bilayer gradient | 0.30305220884 | 0.31795988396 | 4.9192% |
| uniform bilayer | 0.12650602410 | 0.12840262087 | about 1.50% |

The bilayer-gradient Uerr is 0.01611619066 J, not the total-energy difference
0.01490767513 J. Passing the separately normalized Uerr gate must not be advertised
as sub-percent total-energy accuracy. This is a remaining discretization weakness,
not a reproduced inconsistency in the energy postprocessor.

Maximum consistency errors across all meshes: temperature 7.11e-14 K, heat flux
6.82e-11 W/m2, strain postprocessing 3.26e-18, stress postprocessing 3.64e-8 Pa,
energy reconstruction 5.00e-16 J, variational identity 8.05e-16 J and constitutive
balance 2.97e-11 J. All-DOF force residual reaches 8.50e-6 N at the finest mesh.
The separate maxima are 8.50e-6 N on free DOFs and 5.45e-6 N on constrained DOFs;
the bilayer force gate is 0.00105 N. This finest-mesh increase merits follow-up
on solver tolerances/reproducibility; it is not explained away as a support-only
effect. No solver-tolerance change was made. The four control executions described
below reproduce the finest-mesh fields, including the residual increase; this is
not a many-run, cross-platform reproducibility qualification.

## Fixes And Regression

Attempt A completed one real solve and was rejected by the new checker. Its
constitutive balance mistakenly used a plus thermal term on the right side.
Independent strain-energy reconstruction, discrete equilibrium and the variational
identity agreed. A failing hand-calculated unit-bar regression isolated the sign
error; correcting it made the retained real result pass offline before run B.
The production solver was not changed and there was no automatic retry.

The independent review highlighted that normalization definitions were absent
from its compact observations. Those definitions, actual scales, ordinary total
energy error and split free/constrained residuals are now explicit output fields.
All 24 original results passed offline replay after this evidence-only enrichment.
The original review remains unchanged; it did not see the later enriched fields.
Supplemental audit: `tmp/model-bending-offline-audit-20261104.json`.

Input review found a separate test-design weakness: the original 24-case matrix
prefilled free thermal nodes with the analytical temperature. Although these
degrees of freedom were not constrained, an input-echo defect could evade that
part of the checker. A failing regression now requires free nodes to start at
the reference temperature; boundary nodes alone carry prescribed temperatures.
Synthetic checker fixtures explicitly populate solved temperatures separately.

Four new actual r=8 controls in `tmp/model-bending-unseeded-20261104` covered
`gradient_free`, `bilayer_gradient`, `bilayer_uniform` and `bilayer_shift`.
Each had 495 free heat nodes, all starting away from the analytical solution and
all changing to the solved field. Every case passed. Compared with its original
run-B result, the maximum temperature, ux, uy and total-energy differences were
all exactly zero in the retained JSON values. Job IDs were distinct and digests
were reconciled. This excludes simple input echo in these four controls; the
entire 24-case matrix was not rerun with the new initial temperatures. These
controls made zero model API calls, and the original reviewer did not see them.
Comparison: `tmp/model-bending-unseeded-comparison-20261104.json`.

The enriched review payload estimate, including 4096 bytes of framing margin,
is 77,166 bytes, below the unchanged 128 KiB limit. Models still cannot change
gates, execute arbitrary code or authorize additional scope.

- Final Rust SDK all-target regression: 288 passed, 0 failed, 57 explicitly ignored.
- Research examples: 39 and 33 passed, included in the total above.
- Offline opt-in replay: the initial result and all 24 final results passed.
- Separately invoked opt-in live control: four new workflow jobs passed; the
  ignored-test total above does not count this invocation as an automatic test.
- Tests reject wrong energy signs, fake exact quadratic outputs, altered bridge
  properties, wrong flux/stress/displacement, missing/aliased/reused evidence,
  nondecreasing errors and failed finest-mesh accuracy.
- Clippy with warnings denied, formatting and diff whitespace checks pass.
- Touched source files remain below 800 lines. No production solver change,
  reinstall, commit or push. Other existing worktree changes are retained.
- Post-run health: two ready Agents, zero active/queued jobs, no blocked recovery
  work and a healthy watchdog.

Live definition SHA-256:
`5e10282e4dc159b9c603af2df752167fe56623a023694e75f12fb2f09b6853ab`.
Enriched replay definition SHA-256:
`e1c5eccb79fd66a849ee4d3f424d9086b93cd5a120d66b6ed50098aabd90d5d2`.
Unseeded-control definition SHA-256 (control-specific source set):
`ba506d87be241d5a27d89e4227d37d65c59d382694a13e3db122cdb1138d8f9c`.

## Cost, Retention And Next Work

Run B used seven Sol calls and one Astra call, estimated USD 0.429870. Including
the first attempt: nine completed API calls, 70,860 input-plus-output tokens,
29 actual workflow executions (one initial checker rejection, 24 matrix jobs,
four unseeded controls) and estimated USD 0.433000. These are pinned
standard-rate estimates, not an invoice or remaining account balance. No timeout,
unknown API charge or uncertain workflow outcome was retried.

After replay and hash/job-ID reconciliation, removed only the 87 owned generated
request/result/terminal payloads from attempts A/B and the four controls:
108,023,366 bytes (103.02 MiB). All 29 jobs were terminal, and post-control service
health showed no active or queued work. Compact checks, original approvals/usage/
model reviews, real job IDs, digests, the offline audit, control comparison and
reproducible source remain. Cleanup manifest and receipt are
`tmp/model-bending-cleanup-manifest-20261104.json` and
`tmp/model-bending-cleanup-receipt-20261104.json`. No installed service data or
other research runs were deleted. Cleaned directories cannot replay raw files
without regeneration.

Next priorities are tighter continuum-energy accuracy, finest-mesh residual
repeatability and truly traction-free heterogeneous end effects. Nonlinear,
transient, contact, arbitrary/distorted meshes, nonmatching transfer and general
peak-stress accuracy remain unqualified. No production solver defect was reproduced
inside the declared 24-case reference suite.
