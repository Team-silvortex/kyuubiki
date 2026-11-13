# Model-driven temperature/reference decoupling

Date: 2026-11-04. SDK and installed macOS services: daji 3.7.8.
Continues the [boundary investigation](model-research-boundary-and-connection-20261104.md).
Run instructions: [model-driven research guide](../docs/research-model-driven.html#reference-decoupling).

## Findings And Fixes

The previous layered research validator checked axial displacement but did not
check the prescribed zero transverse displacement. A failing regression showed
that changing a returned `uy` to 0.1 m could still pass its numerical gates.
The shared validator now requires finite transverse displacement and enforces
the prescribed constraint to 1e-12 m. This was an acceptance-check defect; the
real solver did not return a violated transverse constraint in this round.

Reference temperatures were also rounded to two decimals in execution IDs.
20.001 and 20.002 therefore collided, creating a latent wrong-evidence reuse
risk if admitted in a future experiment. IDs now preserve round-trippable
references, while keeping existing exact two-decimal IDs unchanged. The new
cold-boundary offset is also part of the identity. Previously approved reference
values were not affected by the fractional-reference collision.

The experimental case model now separates boundary temperatures from the
strain reference. Stress tolerances scale with actual temperature departure
from the reference, not boundary gradient alone: uniform heating has zero
gradient but nonzero stress. Neither model opinions nor successful job status
can bypass the numerical checks.

No production solver algorithm or installed runtime was changed for this round.
The live experiment found no temperature-reference bridge defect within its
bounded scenario set. New checks do not establish broad solver qualification.

## Real Execution

The fixed geometry is a 1 m by 0.1 m, 0.01 m thick strip with two equal layers.
Conductivities are [100, 200] W/(m K), expansion coefficients [10e-6, 20e-6]/K,
Young moduli [70e9, 3e9] Pa and Poisson ratio zero. All y displacements are fixed;
x is fixed only at the left edge. Materials are synthetic, linear and perfectly
bonded. The sides are insulated; there are no volume sources or mechanical loads.

Sol selected 20 experiments in five observation-driven batches, then returned
findings on its sixth API call. The caller completed a fixed 20-case minimum
matrix, reusing identical successful evaluations and submitting eight additional
cases. The union was 28 distinct real seven-node workflows through the official
Rust SDK, Orchestra and installed Rust Agents, not mock solver responses.

Seven paid API calls completed: six Sol calls and one independent Astra review.
Recorded usage gives a pinned standard-rate estimate of USD 0.402882, below the
per-run USD 8 estimate ceiling. This is not an invoice or remaining-account
balance. Astra reported no blocking issue within the synthetic reference scope;
its review remains advisory and cannot authenticate execution or override gates.

All 28 passed manifest checks, identical second result reads, analytical field
checks, prescribed constraints and independently reconstructed mechanics. The
largest mesh had 153 nodes and 128 Q4 elements; this was a correctness study,
not a large-scale benchmark or a throughput test.

The following observed values are rounded for readability and use refinement 2.

| Scenario | Boundary temperatures | Strain reference | Tip displacement, micrometres | Heat flux, W/m2 |
| --- | --- | ---: | ---: | ---: |
| baseline | 40 / 20 | 20 | 100 | 2666.666667 |
| reference_only | 40 / 20 | 30 | -50 | 2666.666667 |
| uniform_hot | 30 / 30 | 20 | 150 | 0 |
| uniform_cold | 10 / 10 | 20 | -150 | 0 |
| absolute_shift | 313.15 / 293.15 | 293.15 | 100 | 2666.666667 |
| uniform_shifted | 303.15 / 303.15 | 293.15 | 150 | 0 |
| reversed_gradient | 20 / 40 | 20 | 200 | -2666.666667 |
| zero | 20 / 20 | 20 | 0 | 0 |

Uniform heating and cooling both produced 0.205 J strain energy, with opposite
independently reconstructed top reactions of -38000 and +38000 N. Zero heat flux
does not imply zero thermal strain or zero elastic energy. The +150/+200
micrometre cases correctly fail the one-sided 110 micrometre expansion limit;
negative displacement satisfying that limit is not contraction-safety approval.

Across all cases, the largest free-x-DOF residual was 7.10e-11 N, structural
temperature-delta error 7.96e-13, and discrete-energy error 1.75e-14 J. Prescribed
displacement errors were zero. Support reactions are reconstructed with a
separate rectangular Q4 quadrature from returned displacements and analytical
temperature; they are **not solver-returned support reactions**.

## Discretization Limits

Accurate nodes and element means must not be presented as exact interior fields.
The baseline illustrates the remaining interpolation error:

| Refinement | Maximum midpoint displacement error, micrometres | Discrete energy, J |
| ---: | ---: | ---: |
| 1 | 8.333333333 | 0.368518519 |
| 2 | 2.083333333 | 0.348240741 |
| 4 | 0.520833333 | 0.343171296 |
| 8 | 0.130208333 | 0.341903935 |

Continuum energy is approximately 0.341481481 J. Midpoint error and excess
discrete energy decrease by a factor of four when the mesh spacing halves.
Agreement with the discrete-energy formula does not imply equality to the
continuum solution. Returned stress values are element means, not stress peaks.

Priority follow-ups are nonzero Poisson coupling, distorted/nonmatching meshes,
node permutation in the transfer bridge, and solver-reported reactions or
integration-point stress validation. These need their own independent references.

The independent review highlighted two additional identifiability limits, checked
against the declared inputs: both layers have the same alpha/conductivity ratio
(1e-7 in the corresponding units), and the field is uniform in y. Identical axial
curvature can hide layer-specific mistakes; permuting equal-temperature nodes
does not probe general transfer mapping. Zero Poisson ratio also does not test
the distinction between plane-stress and plane-strain in-plane coupling. The next
study should deliberately break these symmetries, rather than only add meshes.

For the present gradient cases, an analytical estimate of the maximum Q4 axial
stress error at an element end is 4.666667/refinement MPa, despite zero continuum
axial stress and near-zero element-mean axial stress. This is a prediction from
the element strain interpolation, not an observed stress-peak output or a new
acceptance gate. Pointwise stress can converge more slowly than displacement.

## Verification And Retention

- Rust SDK default suite: 231 passed, 0 failed, 51 explicitly ignored tests.
- Research examples: 19 reference tests and 17 selection tests, included in
  that total rather than additional tests.
- Both research examples pass Clippy with warnings denied and formatting checks.
- All SDK examples compile; changed source files remain below 800 lines.
- The first full-suite attempt was blocked by local listener permissions; the
  rerun with permission to bind loopback test sockets completed successfully.
- Post-run services remained healthy: two ready Agents, zero active/queued jobs,
  no tracked or blocked recovery jobs, and a healthy watchdog.
- No commit, push, reinstall or change to the model-configuration GUI was made.

The live run is under ignored `tmp/model-reference-20261104-a`. Its definition
SHA-256 is `5627764a1a0049c0da966a187c84664058fa9950c1029534960f72c997c08e43`.
Approval, API usage, compact case evidence, hashes, real job IDs, numerical
reports, model findings and independent review are retained. After complete
review and digest verification, 84 raw per-case request/result/terminal payloads
were removed, reclaiming 22,954,344 bytes of disposable local output.
Credentials remain outside the repository and reports. Detailed test logs are
under ignored `tmp/model-reference-*-20261104.log` and the live log is
`tmp/model-reference-live-20261104-a.log`.
