# Model-driven free-transverse and shear patches

Date: 2026-11-04. Official Rust SDK and installed macOS runtime: daji 3.7.8.
Continues [Poisson/material coupling](model-research-poisson-coupling-20261104.md).
Runnable entry: [HTML planar-patch guide](../docs/research-model-driven.html#planar-patches).

## Why This Round

The previous two-material cases restrained every transverse displacement. They
could not establish free Poisson contraction, transverse thermal expansion or
engineering shear behavior. This round removes those restraints and adds
regular/distorted Q4 pairs. It is a homogeneous affine-field verification suite,
not an extension of the heterogeneous layered reference to arbitrary free bodies.

The chain remains Rust Headless SDK -> Orchestra -> native Rust Agent -> steady
Q4 heat -> temperature/reference bridge -> plane-stress thermoelastic Q4.
Models propose bounded catalog experiments, not arbitrary solver code or loads.
The graph builder, dispatch, manifest checks, second result read and evidence
retention are shared with the earlier research runner. The public SDK and
production engine acquire no dependency on the model provider or these cases.

## Physical Reference

The rectangle is 1 x 0.25 m with thickness 0.01 m, E = 70 GPa,
alpha = 12e-6/K and conductivity 20 W/(m K). Uniform boundary temperature is
20 + theta; the strain reference is 20. Interior temperature DOFs are solved.
Only origin ux/uy and bottom-right uy are fixed, removing exactly three rigid
modes. The consistent nodal loads represent t = sigma n on every boundary edge.
They are self-equilibrated; this is not a cantilever with an end load.

The affine solution follows from the
[plane-stress constitutive law](https://www.mathworks.com/help/pde/ug/3-d-linear-elasticity-equations-in-toolbox-form.html)
with isotropic thermal eigenstrain. The following reference is derived for
these boundary conditions, independently of production implementation:

```text
ex = (sx - nu sy)/E + alpha theta
ey = (sy - nu sx)/E + alpha theta
gamma_xy = 2(1 + nu) tau/E
ux = ex x + gamma_xy y
uy = ey y
energy density = [sx(ex - alpha theta) + sy(ey - alpha theta) + tau gamma_xy]/2
```

Regular and irregular mesh consistency is a limited application of the
[finite-element patch test](https://openseesdigital.com/2021/04/04/did-you-pass-your-patch-test/).
Interior-node perturbations preserve the straight rectangular boundary. The
checker independently reconstructs the Q4 Jacobian, displacement derivatives,
constitutive stress, internal force and energy with 2 x 2 Gauss integration.
It does not import production element or constitutive routines. Positive
Jacobians are required at the integration points; this does not certify arbitrary
mesh quality. Separate polygon-area fixtures test the quadrature implementation.

All-DOF internal forces are compared with the submitted loads, including the
three pinned DOFs. Analytical extra support reactions are zero for these balanced
tractions. These are reconstructed residuals, not solver-returned reactions.
The elastic part of the final traction-displacement pairing subtracts stress-free
thermal displacement before its half is compared with elastic energy. It is not
path-integrated work. No general thermodynamic balance is claimed.

## Real Execution

The successful numerical round is `tmp/model-planar-20261104-b`. Sol selected
all 32 scenario/mesh pairs in eight batches, observing each batch before the
next. Its ninth call returned findings. The caller's mandatory matrix reused
those exact 32 results without extra submissions. Refinements 1, 2, 4 and 8
have 9, 25, 81 and 289 nodes, and 4, 16, 64 and 256 elements respectively.

| Scenario | nu | theta, K | Stress [sx, sy, tau], MPa | Corner ux/uy, micrometres | Elastic energy, J |
| --- | ---: | ---: | --- | --- | ---: |
| free_hot | 0.3 | 10 | [0, 0, 0] | 120 / 30 | approximately 0 |
| free_cold | 0.3 | -10 | [0, 0, 0] | -120 / -30 | approximately 0 |
| tension | 0.3 | 0 | [1, 0, 0] | 14.2857143 / -1.07142857 | 0.0178571429 |
| tension_distorted | 0.3 | 0 | [1, 0, 0] | 14.2857143 / -1.07142857 | 0.0178571429 |
| shear | 0.3 | 0 | [0, 0, 0.5] | 4.64285714 / approximately 0 | 0.0116071429 |
| shear_distorted | 0.3 | 0 | [0, 0, 0.5] | 4.64285714 / approximately 0 | 0.0116071429 |
| mixed_hot | 0.49 | 10 | [1.2, -0.4, 0.3] | 143.1357143 / 26.4714286 | 0.0417607143 |
| mixed_auxetic | -0.2 | -10 | [1.2, -0.4, 0.3] | -102.2857143 / -30.5714286 | 0.0277142857 |

The table uses rounded observations at refinement 8. Free hot/cold and both mixed
cases also use distorted meshes. The mixed cases change both nu and temperature;
they do not isolate either variable individually. A plane-stress nu = 0.49 test
is not a nearly incompressible 3D qualification.

All 32 workflows completed with valid output manifests and identical second
result reads. Every returned nodal displacement, temperature, bridge constraint,
load and material value is checked. Stress components, principal stresses,
maximum in-plane shear, von Mises stress, total/mechanical strain and energy
are checked element by element. Hard-coded corner and energy references also
cross-check the model builder's formulas.

Maximum observed errors across the 32 cases:

| Check | Maximum error |
| --- | ---: |
| Temperature / temperature delta | 1.78e-15 K |
| Nodal displacement | 9.75e-18 m |
| Heat flux | 0 W/m2 |
| Stress components/invariants/reconstruction | 3.10e-6 Pa |
| Strain | 3.41e-17 |
| Energy against independent references | 7.28e-15 J |
| All-DOF force balance | 6.18e-10 N |

These are consistency errors for affine, synthetic cases, not real-material
uncertainty or evidence of this accuracy on non-affine problems. Gates were
fixed before execution: displacement 1e-10 m, temperature 1e-7 K, energy 1e-9 J,
stress 1e-8 times the declared case stress scale and force 1e-8 times that scale
times thickness. No tolerances were loosened after observing results.

## Defect Found And Fixed

The initial attempt, `tmp/model-planar-20261104-a`, stopped after one completed
workflow. The newly added bridge checker compared JSON values structurally:
request `0.0` became response `0`, and E = `70000000000.0` became
`70000000000`. It incorrectly classified equivalent numbers as load/material
drift. This was a verification-harness false rejection, not a reproduced solver
or bridge mutation.

A failing regression reproduced the issue. Load/material fields now require
finite numeric values and compare their f64 values exactly. Boolean constraints
remain strict. Null/string numbers, changed loads, changed E and missing or
numeric restraint flags still fail. No physical tolerance was added to hide
contract drift. The original real result passed the corrected checker offline
before a new, separately recorded live round began. There was no automatic
retry or uncertain workflow resubmission.

The replay test now supports an explicitly selected retained-result directory.
It rechecked the first failed-at-checking result and all 32 subsequent results
without paid calls or new solves. It requires at least one payload, but does not
mistake partial replay for complete matrix coverage.

## Automation And Verification

The bounded tool accepts only eight scenario names and four refinement levels.
Unknown/mixed-suite input is rejected before dispatch. The existing 16-model-call,
40-job, 10-planning-turn and estimated USD 8 per-run limits remain unchanged.
Fresh execution and cached coverage now have explicit false/true reuse flags.
A compact request/observation audit carries scenario, refinement, real job ID
and reuse state to the independent reviewer without raw reasoning or credentials.

The local audit reconciled eight batches, 32 unique actual job IDs and 32 cached
coverage checks. Every retained request/result digest matched its evidence row.
This is local reproducibility evidence, not independent execution attestation.

- Rust SDK `--all-targets`: 275 passed, 0 failed, 53 explicitly ignored tests.
- Two research example test targets: 32 and 27 passed, included in that total.
- The retained-result replay was separately enabled: 1 original result, then
  all 32 successful-round results passed. It remains ignored by default.
- Mutated evidence tests reject zeroed transverse displacement, halved engineering
  shear, wrong stress/invariants/energy, changed constraints and incomplete,
  reused, aliased or missing execution evidence.
- Clippy across all SDK targets with warnings denied, formatting and diff
  whitespace checks pass. All touched source files remain below 800 lines.
- Post-solve health: two ready Agents, zero active/queued jobs, no blocked or
  tracked recovery work and healthy watchdog state.
- No production solver algorithm change, runtime reinstall, commit or push.

Executed definition SHA-256:
`297835f8cbfacd3658baec8a3c350f8dcb3797c5b49f70f22429e24ecaf61682`.

## Limits And Next Gaps

No production solver defect was reproduced within these 32 bounded cases.
They validate homogeneous affine response, not non-affine mesh convergence,
heterogeneous free expansion, bending, contact, nonlinear material behavior,
transient coupling, nonmatching meshes or general pointwise stress peaks.

## Model Review And Evidence Clarification

The independent Astra review accepted the bounded scope with no blocking issues.
It explicitly did not qualify invariants from separate observed outputs: the
original tool returned only aggregate stress error, although the deterministic
checker already validated invariants. Its comment that invariant postprocessing
remained unaudited describes the reviewer's evidence limit, not absence of those
local checks. The models' responses are retained unchanged.

The review also identified a misleading field name: `mechanical_external_work_j`
contained the thermal-subtracted pairing, not the full final force/displacement
pairing. This was an observation-contract ambiguity, not a numerical energy bug.
A second red/green regression led to these explicit replacement fields:

- `traction_displacement_pairing_j`: final nodal traction loads dotted with u.
- `thermal_traction_pairing_j`: loads dotted with stress-free thermal displacement.
- `elastic_traction_pairing_j`: the thermal-subtracted difference, equal to 2U
  for the declared affine reference, not a general loading-path integral.
- `path_integrated_work_evaluated`: false.
- `observed_stress_fields_pa`: separate minima, maxima, expected values and
  maximum errors for sx, sy, tau, both principal stresses, max shear and von Mises.
- `expected_stress_pa`: explicitly labels the catalog values formerly called `stress`.

The new checker also compares the full pairing with
`2U + volume * alpha * theta * (sx + sy)` without relaxing earlier gates.
All 32 original result payloads passed offline revalidation. At refinement 8:

| Case | Full pairing, J | Thermal part, J | Elastic part, J |
| --- | ---: | ---: | ---: |
| mixed_hot | 0.323521428571436 | 0.24 | 0.083521428571436 |
| mixed_auxetic | -0.184571428571422 | -0.24 | 0.055428571428578 |

Maximum pairing error is 7.19e-15 J. The retained pure-shear von Mises range at
refinement 8 is approximately [866025.403784329, 866025.403784446] Pa, compared
with reference 866025.403784439 Pa. These are now observed values, not a relabeling
of analytical expectations. Mutation tests confirm altered invariant values
change the observed range and fail acceptance.

This clarification used zero additional model calls or workflows. It does not
retroactively change the original model review or claim the models saw the new
fields. Supplemental evidence is `tmp/model-planar-offline-audit-20261104.json`,
bound to the original job IDs and result digests. Its replay definition SHA-256 is
`51eacf696409cc345ea03bf3c43d7415dde69948e9028b660ae6cd49acab986b`.
Replacing the original review observations with these fields leaves the measured
request-size estimate, including 4096 bytes of additional framing margin, at
99,857 bytes, below the existing 128 KiB cap. Future calls still enforce the cap.

## Cost And Retention

The successful round completed nine Sol calls and one Astra call, with recorded
usage of 102,732 input-plus-output tokens and pinned standard-rate estimate
USD 0.515824. The first failed-at-checking attempt used one Sol call, estimated
USD 0.003580, and one completed workflow. Together: 11 model calls, 33 actual
workflow executions and estimated USD 0.519404. This is not the provider invoice
or remaining balance. There was no model timeout, unknown cost or automatic retry.

Original approval, API reservations/usage, model findings/review, compact case
checks, job IDs and hashes are retained, along with the supplemental offline audit.
After successful review, full replay and digest verification, the 99 owned
request/result/terminal payloads across attempts A/B were removed, reclaiming
49,268,252 bytes. They are reproducible from source; offline replay of these two
directories now requires regeneration of raw results. This cleanup does not touch
installed service databases, other runs, credentials or source history.
