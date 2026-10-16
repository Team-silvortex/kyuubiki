# Daji 3.6.x Recovery Tensor Recalibration

Date: 2026-10-16. Source: the working-tree overlay based on `301ba02c`.
The commit subject says `daji 3.6.4`; Cargo/frontend package metadata still says
`3.6.0`. This review records the actual source metadata as its checkpoint and
uses the `daji-3-6-hardening` profile. It does not bump or reconcile product
versions. The last documented installed baseline remains 3.4.0.

Method: review current configuration, retained reports, matching tests and
named acceptance boundaries; recompute using the unchanged v5 evaluator.
This is not a new numerical, installed, physical-server or power-loss campaign.
Existing uncommitted recovery fixes are part of the reviewed overlay.

## Calibration Decisions

- Keep 13 modules, 11 paradigms, 77 required coordinates, the existing numerical
  targets, priority weights, platform requirements and advisory release gate.
- Preserve all 386 pre-review claims: 383 proven, two partial and one open. Add
  one narrowly scoped `verified` claim for the already-run local desktop suite,
  producing 387 claims, 384 proven. Counts are registrations, not test coverage.
- Bind the retained 126-case desktop GUI suite to product-surface execution,
  contract and UX closure in Hub, Installer, Workbench and shared UI. Its IPC
  effects are mocked; static route reachability is not a native backend execution.
  Hub and Installer rise from declared to verified; shared UI reaches its verified
  target. No installed or operational grade is awarded.
- Add thirteen exact recovery/control-mode scenarios: seven preserve achievements
  within their existing scope; six are explicit unmet qualification obligations.
  Require recovery evidence in Orchestra validation, where typed continuation was
  recorded but not included among required dimensions.
- Keep historical installed Orchestra evidence separate from native standalone
  and decentralized Mesh evidence. A process-kill test is not host power loss;
  HTTP gateway authorization is not native TCP peer authentication; local browser
  and native-file tests do not compose automatically into one installed journey.
- Preserve old reports and their dated counts. Replace current-status prose in
  the book, architecture overview, weakness roadmap and active-line entrypoint.

## Before And After

Both columns use this same source overlay and its retained evidence. The before
column is the October 16 pre-edit computation, not the October 4 snapshot.

| Measure | Before recalibration | After recalibration |
| --- | ---: | ---: |
| Required coordinates | 77 | 77 |
| Retained claims | 386 | 387 |
| Coordinates meeting every configured target | 57 / 77 (74.0%) | 55 / 77 (71.4%) |
| Structural gaps | 0 | 0 |
| Required-dimension presence gaps | 4 | 2 |
| Below-target or scope-incomplete coordinates | 20 | 22 |
| P0 coordinates at target | 40 / 55 | 38 / 55 |
| P1 coordinates at target | 17 / 22 | 17 / 22 |
| Dimension-score progress | 92.0% | 94.0% |
| Named scenarios at target | 10 / 33 | 17 / 46 |
| Named scenarios still open | 23 | 29 |
| Release readiness | blocked | blocked |

One coordinate closes: `desktop-shared-ui/product_surface`. Three previously
target-met coordinates become explicitly incomplete: `runtime-agent-cli/security`,
`runtime-agent-cli/benchmark` and `orchestra-control-plane/workflow_composition`.
Their existing dimension grades remain visible, but unrelated stronger evidence
cannot satisfy the new named scopes. Other new obligations fall within already
open coordinates, so scenario gaps must not be added to coordinate gaps.

The two remaining required-dimension gaps are
`runtime-protocol/sdk_operator/security` and
`workbench-shell/sdk_headless/execution`. In the latter, GUI PWDT is not the
Headless SDK: audit the actual cross-surface execution contract instead of using
PWDT success to relabel it or removing the requirement to improve the score.

The 71.4% figure is configured coordinate target attainment. The 94.0% figure is
dimension-score progress that does not encode every open scenario. Neither is
line/branch test coverage, supported physics, an exhaustive configuration census
or a probability of reliable computation. Forty-six scenarios remain a reviewed
subset, not proof that no other acceptance boundary is missing.

## New Named Scenarios

Registry: `config/architecture/module-function-coverage-evidence/current-recovery-qualification-scopes.json`.

| Scenario | Grade / Target | Decision |
| --- | --- | --- |
| `orchestra-retained-agent-control-rejoin` | operational / operational | Retained two-host control rejoin, original version and process scope only. |
| `standalone-agent-local-process-crash-recovery` | verified / verified | Native local process-kill, ledger reopen and unknown-outcome isolation. |
| `mesh-local-discovery-owner-isolation` | verified / verified | Loopback native peers, endpoint replacement and no cross-peer replay. |
| `workbench-local-direct-caller-recovery` | verified / verified | Browser/mock gateway and separately tested native metadata persistence. |
| `workbench-local-checkpoint-reservation-boundaries` | verified / verified | Real browser/IndexedDB with mocked checkpoint service; conflicts block before POST. |
| `desktop-local-journal-session-fencing` | verified / verified | Real cross-origin iframe lifecycle with mocked native invocation. |
| `orchestra-source-typed-postgres-continuation` | verified / verified | Actual remote PostgreSQL with source macOS Orchestra/Agents and bounded chains. |
| `native-agent-lan-ingress-authorization` | unassessed / qualified | Native peer/controller authentication, revocation and execution/recovery authorization. |
| `mesh-authenticated-multihost-recovery` | unassessed / qualified | Real authenticated hosts, partition/rejoin and original-owner recovery. |
| `workbench-installed-direct-owner-recovery` | unassessed / qualified | Matching installed macOS shell/frontend/journal plus real Agent and two-window faults. |
| `agent-durable-ledger-host-power-loss` | unassessed / qualified | Disposable-host/VM power cut around durable commit boundaries. |
| `orchestra-installed-typed-workflow-recovery` | unassessed / qualified | Managed current artifacts, default database policy, remote Agent and typed continuation. |
| `mesh-sustained-ingress-and-recovery-budget` | unassessed / qualified | Sustained traffic/churn with tail latency, memory, descriptors and recovery availability. |

An empty binding means no named qualifying proof for that exact scenario, not
absence of every relevant feature. Six open scenarios cannot be closed merely
by increasing the grade of any existing unrelated claim. No test against active
user data or disruptive physical-host action is authorized by this registry.

## Priorities After Review

The unchanged release-weighted queue now starts with:

1. `sdk-headless/sdk_headless`: qualified to operational; installed Python/Elixir
   studies and the current heterogeneous modal SDK journey remain open.
2. `runtime-protocol/sdk_operator`: declared to qualified, limited by missing
   protocol-specific security evidence. Worker authoring and Headless control
   must remain distinct.
3. `workbench-shell/validation`: verified to qualified; native interrupted writes,
   sustained installed recovery and Windows qualification remain open.
4. `runtime-engine-solver/validation`: dimension grade is qualified, but physical
   modal publication, multimode recovery and independently rebuilt requests
   still lack their named qualifying proof.

This weighted release list is not an exploit-risk ranking or a command to abandon
the current recovery mainline. For that mainline, prioritize native LAN admission
authorization before advertising an authenticated Mesh; qualify a matching
installed macOS direct-recovery chain; then test real multi-host isolation and
sustained recovery capacity. Qualify current typed Orchestra continuation and
Agent host-power-loss durability separately. Never substitute automatic migration,
blind replay, larger caps or relaxed numerical gates for missing evidence.

Historical modal, migration, whole-generation restore, Installer activation,
Windows and cross-language obligations stay open at their previous targets.
Decentralized durable DAG scheduling, consensus and ownership transfer are not
implied by peer discovery or this recalibration; they need their own reviewed
acceptance scopes before being claimed. This review does not expand the feature
contract to promise automatic failover of uncertain work.

## Verification And Reproduction

Focused Rust tensor tests pass 24/24, including three new checks for separate
control-mode evidence, non-substitution by unrelated operational claims, and
local desktop mapping without installed release promotion. The structural
validator passes with two dimension gaps, 22 coordinate gaps and 17 P0 gaps.
The independent usability release gate remains false; advisory command success
does not grant release permission.

The tensor self-test, strict script-runner Clippy (`--all-targets`, `-D warnings`),
HTML book and documentation inventory checks, project organization audit and
`git diff --check` also pass. The organization audit reports no tracked line-limit
debt against the 800-line source and 2000-line documentation limits.

Use the native entrypoint to regenerate the two existing bounded outputs:

```text
./scripts/kyuubiki check-module-function-coverage-tensor
./scripts/kyuubiki check-module-function-coverage-tensor --self-test
```

Focused evaluator validation from the repository root:

```text
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-script-runner module_function_tensor --locked --offline
```

`tmp/module-function-coverage-tensor.json` and its Markdown sibling are disposable
derived outputs, not new retained benchmark payloads. No product rebuild, install,
remote deployment, product test campaign, version bump, commit or push occurred
as part of this recalibration. The previous turn's 267 frontend, 126 desktop,
25 checkpoint-browser and eight Rust journal results remain dated retained
evidence, not tests rerun by the tensor checker.

See the [current HTML calibration](../docs/book-ch03-architecture-boundaries.html#tensor-calibration)
and [weakness queue](../docs/weakness-roadmap.md#current-tensor-status).
