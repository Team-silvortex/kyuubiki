# Advection Diffusion Service Chain Verification

## Source Scope

October 8, 2026. Source macOS working-tree overlay on `f5a4247a`
(`daji 3.5.3` commit label); packaged metadata remains `3.5.0`. No version bump,
commit, push or installed App rebuild is part of this round. Previous scalar,
entity-ID and native result-file readback changes remain in the working tree.

## Closed Gaps

Rust already implemented the steady 1D advection-diffusion solver, engine
workflow execution and `solve_advection_diffusion_bar_1d` Agent RPC. Orchestra
lacked the submit route, normalizer, analysis delegate, advertised method and
workflow solver catalog entry. Native Headless control and all three official
SDKs lacked the corresponding action/HTTP/RPC table entries.

This round connects those existing mechanisms through the normal asynchronous
job lifecycle. No solver algorithm, RPC version, TaskIR execution-capability
whitelist, frontend controller or engine dependency is changed.

Actual saved-version CLI testing also exposed an old mismatch: native batch
preflight required `endpoints` although the service executor already supported
Orchestra-routed saved solves. Both native saved-version action contracts now
require only `model_version_id`. The existing direct-mesh fallback still checks
for endpoints. The frontend/PWDT direct-mesh contract is not changed.

Initial focused test failures also caught a fixture's missing required
`fix_concentration` flag and the established HTTP validation status of 422;
fixtures/assertions were corrected instead of weakening the physical schema
or altering the service's error contract.

## Actual Transport Chain

The new regression runs an isolated real source Orchestra and two Rust Agents,
with disposable state. Nine admissions cover:

1. Zero-velocity diffusion: concentrations `[2, 3, 4]`, total flux -2.
2. Reverse velocity -0.5: constant concentration 2, total flux -1.
3. Velocity 0.5 with middle source 4: concentrations `[2, 3, 2]`, signed total
   fluxes -0.75 and 3.25, maximum flux 3.25 and element Peclet number 0.125.
4. The source case through actual native CLI file upload and physical result
   readback. Approximately 8 MB disposable metadata selects transport; it is
   not a large numerical mesh or committed fixture.
5. An immutable saved model version solved through native CLI with no explicit
   Agent endpoints. Its expected source descriptor remains exact; retained
   research evidence for middle concentration reloads identically without
   another calculation.
6. A three-node input/solve/output workflow through the service SDK and actual
   Agent RPC. All nodes complete, no nodes fail, and its physical result matches
   the reverse-velocity case.
7. An admitted zero-diffusivity model through native SDK: terminal failed job,
   no result, no later guarded project creation and no automatic retry.
8. The same invalid model through native CLI: nonzero exit, structured failure
   and persisted report agreement, again no downstream write or retry.
9. A healthy zero-velocity CLI solve on the same Agents after the failures.

Independent checks derive middle concentration from the two-element Galerkin
equation and derive gradient, diffusive/advective/total flux and Peclet values
from the element formulas. The flux difference equals source divided by area.
They are not merely equality checks between two paths sharing one solver.
Final Orchestra job count and both Agent execution counters total exactly nine.
Both Agents return to accepting work; guarded project state is unchanged.

## Extended Entity Contract

The formerly protocol-only transport family now participates in all shared
HTTP cases: Rust and Elixir each check 240 model/edit combinations across ten
families. The ten-family actual inline/file regression checks complete physical
result equality and analytic values. It retains literal whitespace IDs,
rejects ambiguous IDs and tuple-shaped entities, stops later writes and reloads
physical results without another solve.

Final admission count for that separate regression is exactly 31: 22 paired
successful solves, eight terminal failed file decodes and one healthy recovery.
Its eight invalid inline attempts create no jobs. The original nine-family,
29-admission report remains historical evidence at its original scope.

## Selected Regression

| Suite | Passed |
| --- | ---: |
| Native Headless SDK | 528 unit and 25 integration |
| CLI | 16 unit and 98 selected integration, including 57 live/harness tests |
| Engine | 637 unit; one existing ignored |
| Protocol | 125 unit and seven integration |
| Solver transport closed-form/reliability/refinement/review | 13 integration |
| Orchestra selected normalizer/API/workflow/Agent regressions | 35 |
| Official Rust SDK | 106 integration |
| Official Python SDK | 97 |
| Official Elixir SDK | 91 |

This is 1778 distinct selected passing tests, not full repository or code
coverage. Focused repeats and qualification reruns are not added to the total.
Official SDK smoke tests verify the new HTTP path and exact RPC method through
mock servers; only the native Rust service chain above executes real physics.

## Static and Coverage Checks

All-target Clippy for Protocol, CLI, native Headless and official Rust SDK passes
with warnings denied. Native Rust format checks and changed official Rust file
format checks pass. Elixir changed-file format checks pass; Orchestra compiles
with warnings treated as errors.

Headless qualification passes and its report verifies at 544 tests across two
suites. Protocol qualification passes and its report verifies at 125 tests,
61 RPC methods and five TaskIR examples. Small local reports remain ignored
under `tmp`; no large fixture, credential or server configuration is added.

Topology, module/function matrix, extension standard, runtime API surface,
documentation inventory, book check (26 HTML files) and exact version audit
(224 checks) pass. Project organization reports zero tracked debt at source/
document limits of 800/2000 lines; the new live helper is 315 lines.

Tensor self-test and actual validation pass. Structural gaps remain zero,
maturity gaps four, evidence-grade gaps 19 and P0 gaps 14. Daji readiness remains
blocked: this follow-up adds a scoped contract claim, not release qualification.

## Acceptance Limits

These are small source macOS cases. No installed App, remote platform, GUI/PWDT,
explicit-endpoint desktop transport gateway, million-node performance or
cross-language numerical producer/consumer acceptance is established. Simple
analytic cases do not certify every advection-dominated regime or a general CFD
solver. Source and byte identities are consistency receipts, not authenticated
execution provenance. Tensor evidence remains scoped contract evidence and must
not promote installed/remote/release readiness.
