# Research boundary and model-connection follow-up

Date: 2026-11-04. Source and installed service line: daji 3.7.7.
This round continues the [first live research experiment](model-driven-thermal-research-20261104.md).

## Research Findings

Expanding the synthetic layered thermal experiment to the feasibility boundary
exposed a case-identity collision: the shared example formatted temperature rise
with zero decimal places. The 21.999, 22 and 22.001 K cases therefore had the same
identifier. The model-driven runner caches successful cases by that identifier,
so accepting these new loads without correcting it could reuse the wrong evidence.
The previous approved integer-load set was not affected.

A failing regression reproduced the collision before the fix. Case IDs now retain
the round-trippable fractional rise; existing integer-case IDs remain unchanged.
The live follow-up requires different case and actual job identities for all three
loads. This is a research-runner identity fix, not a solver-algorithm change.

The engineering classifier now uses maximum tip displacement and a predeclared
1e-9 m numerical band around the 110-micrometre limit. A result in the band is
`boundary`, never automatically approved feasible. This avoids letting arithmetic
rounding alone decide engineering acceptance. The numerical band is not a bound
on material uncertainty, geometric uncertainty or experimental error.

| Temperature rise | Observed tip displacement, micrometres | Classification |
| ---: | ---: | --- |
| 21.999 K | 109.995000000014 | feasible |
| 22 K | 110.000000000014 | boundary |
| 22.001 K | 110.005000000014 | infeasible |

The negative-rise case also passed the analytical temperature, displacement,
flux, element-mean field and interior interpolation-error checks. Negative
displacement satisfying this one-sided expansion limit is not qualification of
contraction, bond strength or stress safety.

### Live Execution

- Sol made four observation-driven batches covering all 16 conductivity pairs.
- The official Rust SDK dispatched 26 actual seven-node workflows: 16 exploration
  jobs and 10 additional verification jobs, including the three distinct boundary
  loads and negative rise. Cached identical checks did not submit replacement work.
- All 26 passed numerical validation, output-manifest validation and exact second
  result reads. The selected pair remained [100, 200] W/(m K).
- Six API requests were made: five Sol calls and one independent Astra review.
- The standard-rate estimate for this round was USD 0.432344. It is not a provider
  invoice or remaining-account balance. The per-run estimate ceiling remained USD 8.
- Astra reported no blocking issue within the bounded synthetic scope. Its review
  remains advisory and cannot authenticate execution or replace deterministic gates.

The next physics gap is reference decoupling: current cases tie the cold boundary
to the strain-reference temperature. Uniform heating above a fixed reference and
changing only the strain reference are not yet tested here. Pointwise stresses,
mechanical residuals and support reactions remain separate qualification work.

## Model Configuration UI

The Workbench assistant now has grouped service and model selection sections,
side-by-side on wide panels and stacked on narrow ones. Successful connection
setup collapses automatically; editing remains one action away. Model discovery
supports search and renders at most 40 choices, with a content-sized bounded list
and a separate manual-ID mode. Advanced transport/privacy detail is collapsed.
The selected model and next action remain together in the footer.

Two existing defects were reproduced in browser tests before correction:

- A failed recheck previously left the last successful model catalog visible.
  Discovery is now cleared before every attempt and on trust revocation; late
  responses cannot repopulate another provider or a revoked connection.
- A connection notice was stored as translated text, so changing language left
  that notice in the old language. State now retains a notice code and renders
  it in the active language, including already-completed checks.

No credential is persisted. Tests use fake credentials and controlled model
responses; no GUI test requests paid inference or launches simulation work.
The real research runs are separate SDK-to-installed-service tests. The GUI's
Chat Completions interface was not silently converted into the Responses tool
runner, and a loaded catalog does not certify model compatibility.

## Verification And Retention

- Frontend unit suite: 1,983 passed; workflow subset: 1,981 passed (not additive).
- Rust SDK default suite: 208 passed, 51 explicitly ignored native Agent/modal
  recovery cases. The new research example has 13 offline tests.
- Browser interaction/layout checks: 1440x1000, 1024x768 and 390x844, including
  English/Chinese notice switching, catalog filtering, manual entry, stale
  results, revoked trust, key isolation and the next-action focus handoff.
- Frontend type and file-size checks, Rust Clippy and formatting passed.

Compact research evidence is under ignored `tmp/model-thermal-20261104-c`.
Disposable per-case request/result/terminal payloads are removed after verified
completion; hashes, checks, job IDs, API usage and model reviews remain. Test logs
are `tmp/model-connection-frontend-20261104.log`,
`tmp/model-connection-browser-20261104.log`, and
`tmp/model-research-sdk-boundary-20261104.log`.

This round does not claim full physical qualification, restart durability,
Linux/Windows acceptance or a newly installed GUI package. No commit or push
was performed; existing unrelated changes remain intact.
