# Model-driven Poisson and material coupling

Date: 2026-11-04. Official Rust SDK and installed macOS runtime: daji 3.7.8.
Continues [temperature/reference decoupling](model-research-reference-decoupling-20261104.md).
Runnable entry: [HTML research guide](../docs/research-model-driven.html#material-coupling).

## Question And Scope

The previous two layers had equal expansion/conductivity ratios and zero
Poisson ratio. This round deliberately separates those assumptions: change
Poisson ratio alone, expansion ratio alone, and both together; include positive
and negative Poisson ratios, a plane-stress nu=0.49 case, and uniform hot/cold
controls. It does not test a nearly incompressible three-dimensional solver.

Geometry remains a 1 x 0.1 m strip, thickness 0.01 m, with two equal x-layers,
conductivities [100, 200] W/(m K), and Young moduli [70e9, 3e9] Pa. All y
displacements are restrained; x is fixed only at the left boundary. Materials
are synthetic, linear, perfectly bonded and temperature-independent. The
gradient cases use boundary temperatures 40/20 and strain reference 20.

The real chain is Rust Headless SDK -> Orchestra -> native Rust Agent ->
steady Q4 heat -> node-to-node temperature/reference bridge -> plane-stress
thermoelastic Q4, with heat, bridge and structural artifacts retained for checks.
There is no GUI dependency, shell model tool or mock solver substitution.

## Independent Reference

Starting from isotropic plane-stress elasticity, add thermal eigenstrain
`alpha * theta`, where `theta = T - Tref`. Enforcing zero transverse strain
and zero axial traction gives the following continuum reference, derived for
this test rather than copied from the production solver:

```text
epsilon_x = (1 + nu) * alpha * theta
sigma_x = 0
sigma_y = -E * alpha * theta
```

The constitutive starting point is documented in
[MathWorks' plane-stress Hooke law](https://www.mathworks.com/help/pde/ug/3-d-linear-elasticity-equations-in-toolbox-form.html).
The temperature field is obtained independently from the two series thermal
resistances. Piecewise integration of the axial strain gives hand-calculated
tip references for all eight scenarios. These relations are not valid for an
unrestrained transverse boundary and are not flexural-bending references.

For rectangular Q4 elements, let `m` and `v` be the element mean and variance of
theta. The discrete axial strain is constant within each x interval, yielding:

```text
discrete energy density = E * alpha^2 / 2 * (m^2 + 2 * v / (1 - nu))
continuum mean energy density = E * alpha^2 / 2 * (m^2 + v)
midpoint displacement error = abs((1 + nu) * alpha * q / k) * hx^2 / 8
```

The example implements independent rectangular quadrature of the full
plane-stress constitutive response, using returned displacements and analytical
temperature. It does not import production solver routines. Reconstructed
support resultants and free-x-DOF residuals are not solver-returned reactions.
Force and moment residuals are reported separately in N and N m.

## Live Results

Sol selected all 32 approved scenario/mesh combinations in eight batches of
four, observing each batch before selecting the next. Its ninth call returned
findings. The caller's mandatory matrix then reused those exact retained case
results, without dispatching additional jobs. Every scenario ran at refinements
1, 2, 4 and 8; the largest mesh had 153 nodes and 128 Q4 elements.

Ten paid API calls completed: nine Sol calls and one independent Astra review.
Recorded usage gives a pinned standard-rate estimate of USD 0.579824, below
the USD 8 per-run estimate ceiling. This is not a provider invoice or remaining
credit balance. Astra reported no blocking issues within the bounded scope;
its review cannot authenticate execution or replace numerical gates.

All 32 distinct actual workflow jobs completed and passed manifest validation,
exact second result reads, temperature/bridge/displacement gates, element-mean
field checks, independent mechanical reconstruction and discrete-energy checks.
No production solver defect was reproduced within these bounds. This is not
evidence that the full thermoelastic solver is bug-free.

Observed tip displacements below are rounded; the other refinements agree
within the predeclared tolerance.

| Scenario | Expansion coefficients, 1e-6/K | Poisson ratios | Tip displacement, micrometres |
| --- | --- | --- | ---: |
| control | 10 / 20 | 0 / 0 | 100 |
| poisson_only | 10 / 20 | 0.3 / 0.3 | 130 |
| ratio_only | 10 / 35 | 0 / 0 | 125 |
| coupled | 10 / 35 | 0.2 / 0.4 | 161.666667 |
| coupled_hot | 10 / 35 | 0.2 / 0.4 | 305 |
| coupled_cold | 10 / 35 | 0.2 / 0.4 | -305 |
| coupled_auxetic | 18 / 6 | -0.2 / 0.3 | 109 |
| coupled_high_nu | 10 / 35 | 0.49 / 0.25 | 172.25 |

Uniform hot/cold cases use 30/30 and 10/10 boundaries with reference 20.
Both have zero heat flux and 0.266875 J strain energy, but opposite tip
displacements and reconstructed top reactions of -40250/+40250 N. All gradient
cases have axial heat flux approximately 2666.666667 W/m2 and heat rate 2.666667 W.
Equal total heat rate alone would not rule out swapping equal-length layers;
layer-resolved temperatures and mechanical fields are checked separately.
The continuum tip displacement also does not depend on Young's modulus for
these restraints. Stress and energy checks are therefore essential to distinguish
modulus mistakes. Unchanged transverse resultants when only nu changes are
expected here, not evidence that the solver ignored nu.

Maximum observed free-x-DOF residual was 1.31e-10 N, force-check error 1.95e-9 N,
net moment 1.37e-11 N m, temperature-delta error 7.82e-13 and discrete-energy
error 5.05e-14 J. These are numerical consistency errors, not uncertainties in
real material properties.

For the combined material case, errors no longer share the same curvature:

| Refinement | Left midpoint error, micrometres | Right midpoint error, micrometres |
| ---: | ---: | ---: |
| 1 | 10 | 20.4166667 |
| 2 | 2.5 | 5.1041667 |
| 4 | 0.625 | 1.2760417 |
| 8 | 0.15625 | 0.3190104 |

Both layer errors reduce by four when spacing halves. Discrete energy decreases
from approximately 0.397476852 to 0.351379847 J, toward continuum energy
0.350648148 J. Correct nodal displacement and element-mean stress do not imply
exact interior displacement or correct pointwise peak stress.

## Implementation And Regression

The existing research example now accepts an optional `coupling` suite instead
of adding a new control CLI. The caller validates the entire batch against that
suite, keeps the same job/API/cost budgets, and does not automatically retry
ambiguous work. Expanded material parameters are passed through workflow input,
execution IDs and retained observations rather than hidden in engine code.

Failing tests preceded the material/model/checker extension. New tests cover
parameter-sensitive identities, hand-integrated tip references, uniform
nonzero-Poisson energy and reactions, and rejection of ignored Poisson coupling,
wrong layer expansion, incomplete matrices, altered metrics, aliased job IDs
and mixed-suite batches. Those initial failures were missing research-example
capabilities, not newly reproduced production solver failures.

- SDK default suite: 240 passed, 0 failed, 51 explicitly ignored tests.
- Research examples: 25 and 20 tests, included in that SDK total.
- All SDK examples compile; Clippy with warnings denied passes.
- Formatting and whitespace checks pass. The final formatted research examples
  were tested again without another paid run.
- Source files remain below 800 lines; no production solver algorithm changed.
- Post-run services are healthy: two ready Agents, zero active/queued jobs,
  no tracked/blocked recovery jobs, and a healthy watchdog.
- No runtime reinstall, commit or push was performed; existing changes remain.

## Evidence And Remaining Limits

Evidence is under ignored `tmp/model-coupling-20261104-a`. The executed definition
SHA-256 is `cea8871df0febc07624be25facc94a832cf890e09c378308c37bb8c87a34a8b3`.
It records the source at launch; later formatting does not alter the experiment's
physical or execution behavior. Keep compact case checks, real job IDs, digests,
usage and model review rather than permanent generated payload copies.
After successful review and digest verification, 96 disposable request/result/
terminal files were removed, reclaiming 25,938,561 bytes. Only this run's owned
generated files were removed; installed service state and earlier evidence remain.

The model reviewer received compact case and coverage summaries, not the full
tool transcript, and correctly did not claim to authenticate execution order.
A separate local audit matched all eight retained requests to their four returned
job IDs, reconciled the union with the 32 report jobs and checked all ten usage
records completed. Original per-case reuse fields are null in this runner;
explicit reuse flags are available in the caller's matrix coverage records.
Those limitations are not silently interpreted as independently attested history.

The one-sided 110 micrometre expansion classifier remains visible for continuity,
but is not this study's optimization objective or a safety assessment. It labels
contraction feasible without checking a contraction allowable. Arbitrary shear,
free transverse deformation, distorted meshes, nonmatching transfer, nonlinear
materials, contact and pointwise stress peaks remain outside this round.
