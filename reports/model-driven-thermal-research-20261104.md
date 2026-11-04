# Model-driven thermal research, 2026-11-04

## Scope

Two live, caller-authorized synthetic research rounds used OpenAI Responses API,
the public Rust Headless SDK and the installed local daji 3.7.7 Orchestra/Agent
services. Sol selected parameters, observed real tool results and selected a
design. Astra reviewed the evidence separately. No mock replaced live execution.

The example is `sdks/rust/examples/model_driven_thermal_research.rs`; its runnable
guide is [research-model-driven.html](../docs/research-model-driven.html).
Provider integration is a consumer of the SDK, not a new SDK authority or GUI
gateway. The production solver implementation was not changed in this round.

## Measured Outcome

| Round | Model-selected designs | Real workflows | API calls | Estimated USD | Numerical gates | Model review |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| A, initial closed loop | 6 | 12 | 4 | 0.18938 | Pass | No blocking issues within scope |
| B, strengthened field checks | 8 | 14 | 4 | 0.25673 | Pass | No blocking issues within scope |
| Total | Not a unique-case count | 26 | 8 | 0.44611 | All admitted cases passed | Advisory, not certification |

Each workflow has seven graph nodes, two solver stages and one temperature-field
bridge. The total is 26 real submissions, not 26 different physical phenomena.
Repeated winner checks reuse an already validated identical case within a round.
All jobs completed with validated output manifests and identical second result
fetches. The two runs independently selected the same constrained optimum.

| Quantity | Observed / independently checked result |
| --- | --- |
| Conductivity selection | Left 100, right 200 W/(m K) |
| Axial flux | Approximately 2666.666666667 W/m2 |
| Through-section heat rate | Approximately 2.666666667 W |
| Tip expansion | Approximately 100 micrometres |
| Design limit | 110 micrometres at the specified 20 K rise |
| Nominal margin | 10 micrometres, not an uncertainty allowance |
| Discrete oracle | Exhaustive analytical evaluation of 16 pairs, not 16 SDK submissions |

The maximum-conductivity pair (200,200) expands about 125 micrometres and is
infeasible. The reversed pair (200,100) expands about 150 micrometres. The model
did not mistake higher flux or a successful solve for a feasible design.

## Verification Blind Spot Addressed

The first independent review correctly noted that exact nodal displacement in
this reference problem does not establish pointwise field accuracy. The follow-up
adds a separate field checker and negative regressions rather than relaxing or
relabeling the existing nodal gates.

For the selected pair, actual Q4-interpolated midpoint displacement errors were:

| Refinement | Nodes | Maximum midpoint error, micrometres |
| ---: | ---: | ---: |
| 1 | 6 | 8.3333333333 |
| 2 | 15 | 2.0833333333 |
| 4 | 45 | 0.5208333333 |
| 8 | 153 | 0.1302083333 |

Halving the element length reduces this interpolation error by approximately
four, consistent with the quadratic analytical displacement and linear Q4
interpolation along the strip. The `1e-9 m` error-law tolerance checks agreement
with the expected discretization error; it does not claim the interior error
itself is below `1e-9 m`.

The strengthened checker also verifies element connectivity, element-mean axial
strain, axial/shear/ transverse-restraint stress, and boundary-integrated heat
rates. It deliberately distinguishes flux from heat rate and element averages
from pointwise stress. Both mesh and constitutive checks remain specific to the
declared synthetic plane-stress, zero-Poisson-ratio setup.

Across round B, the largest original-gate errors were approximately:

- Nodal temperature and bridged temperature rise: `1.56e-12 K`.
- Nodal axial displacement: `2.49e-17 m`.
- Axial/transverse heat flux: `1.27e-9 W/m2`.

Temperature-origin shift, zero rise and doubled rise were checked at refinement
8. The doubled-rise case becomes infeasible by design but passes the physical
reference checks. Numerical validity and engineering feasibility remain separate.

## API Use And Safeguards

| Model | Calls | Input tokens | Output tokens | Estimated USD |
| --- | ---: | ---: | ---: | ---: |
| gpt-6.1-sol | 6 | 15,760 | 1,490 | 0.04642 |
| gpt-6-astra | 2 | 14,889 | 5,016 | 0.39969 |

Estimates use the verified 2026-11-04 standard input/output prices without
cached-input discounts, not an account balance or final invoice. Model capability
and pricing references are the official [Sol](https://developers.openai.com/api/docs/models/gpt-6.1-sol)
and [Astra](https://developers.openai.com/api/docs/models/gpt-6-astra) documentation.
Both exact model IDs were confirmed in the authorized account before inference.

Each round reserves conservative estimated cost before a request and enforces a
USD 8 estimate ceiling, explicit call/job limits and opt-in. Unknown transport
outcomes stop without retry. A model can only supply typed, bounded synthetic
design parameters; it cannot execute commands, read arbitrary files, change the
service origin, loosen numerical gates or grant itself approval. Credentials
remain outside the repository and are not printed or retained in evidence.

Compact reports, API usage records, model selections/reviews and tool traces are
under ignored `tmp/model-thermal-20261104-a` and `tmp/model-thermal-20261104-b`.
Large per-case request/result/terminal payloads were removed after verification;
their hashes and numerical evidence remain in the compact records (about 560 KiB
across both rounds, down from about 22 MiB). This retention
policy is not a restart-durability or power-loss guarantee. Source generators and
checks remain in Git; paid model responses are not deterministic fixtures.

## Local Regression

- Default Rust SDK test suite: 206 passed, 0 failed, 51 explicitly ignored.
- The 51 ignored cases are existing opt-in native Agent/modal recovery tests;
  they were not executed or promoted to passing evidence in this round.
- The new example contributes 11 offline tests, including pre-dispatch policy,
  budget reservation, incomplete-response rejection, discrete optimality,
  malformed field rejection and interior interpolation-error convergence.
- Targeted model approval/execution and workflow contract/result suites: 25 passed
  before the complete SDK run; these are included in, not additional to, 206.
- Clippy with warnings denied, formatting, changed-file whitespace checks and
  local guide links passed. An exact credential-value scan of 187 newly created
  source/report/evidence files found no leaked key; the value was never printed.
- The first full SDK attempt was blocked by local listener permissions. The
  completed run used permitted local test sockets; it did not call paid APIs.

The SDK test log is `tmp/model-research-sdk-tests-20261104.log`. This was not a
repository-wide GUI, Linux, CI or package-installation acceptance run.

## Remaining Work

No solver defect was reproduced in this bounded case. The concrete finding was
an acceptance-coverage gap, now supplemented by interior-error and field checks.
This evidence does not promote general material-research or tensor maturity.

- Add quadrature-point/norm-based errors, support reactions and mechanical residuals.
- Check uniform heating with zero flux, reference-only shifts, negative rise and the 22 K feasibility boundary.
- Extend fault tests to stale cross-job fields, wrong material mapping and uncertain submissions.
- Add uncertainty and experimentally supported material/interface properties before making real-material claims.

The independent reviews remain advisory critiques of supplied evidence. They do
not authenticate local execution or replace caller-owned deterministic checks.
