# Daji 3.4.5 Coverage Tensor Recalibration

Date: 2026-10-04.
Source: daji 3.4.5 working-tree overlay based on `431b2ac7`.
Method: retained-evidence and current-scenario review, not a new product execution
campaign, numerical algorithm change or release qualification.

## Decision And Boundaries

Retain the existing v5 evaluator and its dimension-local, named-scenario rules.
Update the advisory profile from `daji-3-2-hardening` to `daji-3-4-hardening`,
with checkpoint `daji 3.4.5` and target `daji 3.4.x`. Do not change thresholds,
priority weights, evidence grades, module ownership or platform obligations.
The independent usability release gate remains unsatisfied.

The axes remain 13 modules, 11 function paradigms and scoped evidence depth.
There are 77 required module/paradigm coordinates; optional intersections are
not required target cells. All 224 evidence claims remain: 222 marked `proven`
and two `partial`. These statuses describe retained claims within their declared
scope, not fresh proof for every capability of the module.

The main correction is to separate recently retained modal candidate research
from current production obligations. Historical installed truss/thermal and
scale claims had qualified broader solver dimensions but did not separately
constrain heterogeneous modal recovery or its downstream journeys. Explicit
named scopes now prevent those stronger unrelated claims from closing them.

## Before And After

The before column uses the same working tree and retained claims, immediately
before adding the twelve current solver scopes. It is not the older September
13 snapshot, which had fewer retained claims and one additional dimension gap.

| Measure | Before scope review | Current calibration |
| --- | ---: | ---: |
| Retained claims | 224 | 224 |
| Required coordinates | 77 | 77 |
| All-target coordinates | 61 / 77 (79.2%) | 58 / 77 (75.3%) |
| Structural gaps | 0 | 0 |
| Required-dimension presence gaps | 4 | 4 |
| Below-target or scope-incomplete coordinates | 16 | 19 |
| P0 at target | 44 / 55 | 41 / 55 |
| P1 at target | 17 / 22 | 17 / 22 |
| Dimension-score progress | 91.3% | 91.3% |
| Named scenario requirements met | 6 / 20 | 10 / 32 |
| Release readiness | blocked | blocked |

The 22 open scenarios overlap the 19 coordinate gaps. Neither count is additive
with the four dimension-presence gaps. Percentages are configured target
attainment or dimension-score progress, not code coverage, physics coverage,
reliability probabilities or an exhaustive inventory of every configuration.
The lower coordinate attainment exposes previously unnamed obligations; no
functionality was removed by this review.

## Current Solver Scopes

The new registry is
`config/architecture/module-function-coverage-evidence/current-solver-qualification-scopes.json`.
It preserves four verified scopes and adds eight unmet qualified obligations.
Each specifies a module, paradigm, required dimension, exact scope, acceptance
condition, rationale files and named claims. An empty binding remains open.
Rationale files are not automatic proof bindings.

| Named scope | Retained grade | Decision |
| --- | --- | --- |
| `modal-public-bounded-single-mode-baseline` | verified | Retained public bounded successes and explicit failures; not general modal qualification. |
| `modal-normalized-candidate-research` | verified | Five-of-six test-only candidate recovery; not production admission. |
| `modal-wide-beam-candidate-research` | verified | Eight accepted order proposals across five fixtures, with bounded beam checks and cancellation; not a sixth recovered fixture. |
| `modal-candidate-pipeline-local-cost` | verified | Six isolated optimized local candidate processes, one warmup and three samples; not beam timing, production throughput or a memory-budget certificate. |
| `modal-heterogeneous-production-recovery` | unassessed | All six retained graded/layered cases must qualify through the actual public production path. |
| `modal-public-cancellation-budget-replay` | verified, below qualified target | Retained bounded fault tests do not qualify the new production recovery pipeline. |
| `modal-production-physical-publication` | unassessed | Qualify original physical residual, strict unit norm and independent JSON readback over heterogeneous and unequal-length failures. |
| `modal-production-multimode-recovery` | unassessed | Qualify the retained long-bending six-mode request, clustered/subspace checks and late-mode failure isolation. |
| `modal-independent-renumbered-assembly` | unassessed | Rebuild and assemble requests independently; jointly permuting a fixed seed/operator is narrower proof. |
| `modal-production-whole-pipeline-budget` | unassessed | Bound and measure complete preparation, eigensolve, ordinary polishing, both corrections and publication. |
| `modal-current-agent-production-journey` | unassessed | Retain actual TaskIR/RPC production recovery with authority, independent results and healthy replay. |
| `modal-current-headless-production-journey` | unassessed | Retain an official Rust Headless study-to-production/export/readback/recovery journey, not an ad-hoc research wrapper. |

`unassessed` here means no named qualifying binding for that exact obligation,
not absence of all modal, Agent or SDK functionality. The cancellation scope
deliberately binds its narrower verified baseline so partial strength remains
visible without satisfying the qualified target. Python/Elixir installed
research and macOS/Windows product journeys remain separately tracked.

The three newly below-target coordinates are
`runtime-engine-solver/solver_execution`, `runtime-engine-solver/validation`
and `runtime-agent-cli/solver_execution`. Existing solver benchmark and Headless
gaps also gain explicit modal obligations. No general numerical validation
claim can substitute for a named production scope.

## Global And Mainline Priorities

The unchanged release weighting still ranks `hub-shell/product_surface`,
`installer-shell/product_surface` and `workbench-shell/product_surface` first.
Their weakest dimensions are execution/contract declarations, with separately
open Windows installed scenarios. This is a scoped proof bottleneck, not an
assertion that the installed macOS applications cannot run. Audit and bind real
evidence; do not raise claim grades merely to restore a green report.

The complete nineteen-coordinate queue also retains Headless language journeys,
Worker protocol contracts/security, Workbench recovery, Installer activation and
rollback, PostgreSQL checkpoint replay, whole-generation restore, desktop-shared
product proof, and the Windows Agent round trip. No deferred platform or data
lifecycle obligation is removed.

For the current calculation development mainline, the ordered acceptance work is:

1. Qualify heterogeneous modal production recovery and strict physical publication
   without relaxing the original residual, normalization or root gates.
2. Qualify multimode behavior and independent request reassembly, including
   cancellation, no partial publication and fresh healthy replay.
3. Qualify cumulative work, allocation ownership and measured whole-pipeline
   cost; per-fit planning bounds and process-high-water RSS are not equivalent.
4. Revalidate that admitted behavior through real Agent TaskIR/RPC and official
   Rust Headless study/export/readback, retaining Engine/Solver ownership.

This mainline recommendation does not replace the global release queue or
automatically certify unreviewed physics, geometries, material systems or scale.
The 32-scenario registry remains a reviewed subset and should grow when a new
production capability introduces an independent acceptance boundary.

## Recalibration Verification

The focused evaluator tests cover minimum dimension grading, non-proven claims,
unrelated modules/dimensions, exact named bindings, malformed or duplicated
scopes, zero-grade-step scope gaps, exact-count release thresholds and readable
limits. New cases ensure verified candidates cannot close qualified production
or installed scopes even beside operational historical evidence, and verify
the current registry preserves four solver successes while leaving all eight
new production obligations unmet.

| Final check | Result |
| --- | --- |
| Focused tensor source tests | 21 passed, 0 failed, 0 ignored; 379 unrelated tests filtered out |
| Script-runner all-target strict Clippy | Passed with `-D warnings` |
| Tensor validator and self-test | Passed structurally; 4 dimension-presence, 19 evidence-grade and 14 P0 gaps remain |
| Operator profile validator and self-test | Passed; 59 profiles, registry-only `executed=false` |
| Documentation inventory and book | Passed; 26 HTML files, development/shipping metadata 3.4.5 |
| Organization audit and self-test | Passed; source <=800, docs <=2000, tracked debt 0 |
| Workspace formatting and diff whitespace | Passed |

Product research, remote execution, installed GUI and full numerical
qualification are not rerun as part of this review. Historical solver reports
retain their original results. The book's shipping metadata is not proof of a
new local installation; the last retained installed baseline remains 3.4.0.

Reproduction uses the native project entrypoint:

```text
./scripts/kyuubiki check-module-function-coverage-tensor
./scripts/kyuubiki check-module-function-coverage-tensor --self-test
```

From `workers/rust`, focused source checks are:

```text
cargo test -p kyuubiki-script-runner module_function_tensor --locked --offline
cargo clippy -p kyuubiki-script-runner --all-targets --locked --offline -- -D warnings
cargo fmt --all -- --check
```

Generated `tmp/module-function-coverage-tensor.json` and its Markdown sibling are
recomputable outputs, not shipped evidence or backups. The advisory command may
pass while readiness remains blocked and `release_claim_allowed=false`.
Current human-readable navigation is the
[HTML architecture chapter](../docs/book-ch03-architecture-boundaries.html#tensor-calibration),
[module architecture](../docs/module-architecture.md) and
[weakness roadmap](../docs/weakness-roadmap.md#current-tensor-status).

## Subsequent Production Follow-Up

The [bounded public request-reassembly correction](modal-request-reassembly-reliability-20261004.md)
was executed after this calibration snapshot. It adds one verified claim,
bringing current retained claims to 225, and raises independent reassembly
from unassessed to verified, below its qualified target. The snapshot's 224
claims and pre-follow-up grades above remain historical. Coordinate and scenario
target counts, release thresholds and blocked readiness are unchanged.

A later [banded-inverse candidate comparison](modal-banded-inverse-candidate-reliability-20261004.md)
recovers six of six retained candidate fixtures, adds a verified validation
claim (226 current claims), and extends the existing candidate-research binding.
It does not change public admission or any qualified production target. The
five-of-six candidate snapshot above retains its original algorithm and scope.
