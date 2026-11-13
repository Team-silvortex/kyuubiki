# Daji 3.7.8 Store And Native Execution Tensor Recalibration

Date: 2026-11-13. Source: the working-tree overlay based on `f04ec479`.
Package/book metadata remains `3.7.8`; the profile remains
`daji-3-7-hardening`. Existing uncommitted Store, deployment and isolation
changes are included in this review, not attributed to the last clean commit.
The last documented macOS installation is the November 4 build, not this source.

This is an evidence and acceptance-boundary review. It reuses retained test
results and runs the tensor/checker regressions, not a fresh numerical, installed,
multi-host, CI or paid-model campaign. No daily services are restarted.

## Decisions

- Keep 13 logical modules, 11 function paradigms and 77 required coordinates.
  Store distribution remains part of the existing architecture, not an execution
  authority or a new module created to inflate coverage.
- Preserve all 478 pre-review claims unchanged: 465 proven, 12 partial and one
  open. Add three narrowly scoped verified claims, bringing the total to 481,
  including 468 proven. Broad Store and native-isolation claims remain partial.
- Add 18 named scenarios: four retain bounded verified achievements; fourteen
  are still open. Require recovery as a dimension of Agent validation as well
  as execution/contract. Existing evidence already supplies that dimension.
- Do not change targets, scores, release criticality, weights or advisory mode.
  Old installed ABI execution cannot qualify current per-call isolation; a
  real SSH deployment does not prove native TLS across hosts or a Store login.
- Keep source tests, mocked bridges, installed journeys, backend draft sealing,
  published downloads, process termination and power-loss tests separate.

Registry:
`config/architecture/module-function-coverage-evidence/current-store-native-qualification-scopes.json`.
The existing general, solver and recovery registries stay active.

## Same-Tree Comparison

Both columns are November 13 computations on the same reviewed source, with
and without this new registry. They are not comparisons with an old release.

| Measure | Before | After |
| --- | ---: | ---: |
| Required coordinates | 77 | 77 |
| Retained claims | 478 | 481 |
| Coordinates meeting every configured target | 54 / 77 (70.1%) | 48 / 77 (62.3%) |
| Structural gaps | 0 | 0 |
| Required-dimension presence gaps | 2 | 2 |
| Below-target or scope-incomplete coordinates | 23 | 29 |
| P0 coordinates at target | 37 / 55 | 32 / 55 |
| P1 coordinates at target | 17 / 22 | 16 / 22 |
| Dimension-score progress | 94.0% | 94.0% |
| Named scenarios at target | 16 / 46 | 20 / 64 |
| Named scenarios still open | 30 | 44 |
| External P0 gate gaps | 1 | 1 |
| Release readiness | blocked | blocked |

Six previously target-met coordinates now expose unfulfilled current scopes:
`workbench-shell/runtime_api`, `workbench-shell/security`,
`runtime-agent-cli/deployment_update`, `runtime-agent-cli/sdk_operator`,
`runtime-installer/sdk_operator` and `sdk-operator/sdk_operator`.
The other new obligations belong to already-open coordinates. Do not add
scenario gaps to coordinate gaps as though they were independent defects.

The two dimension-presence gaps remain `runtime-protocol/sdk_operator/security`
and `workbench-shell/sdk_headless/execution`. A missing proof does not prove
missing implementation. PWDT and Headless remain separate control paradigms.

The 62.3% figure is configured coordinate target attainment, not code coverage,
physics coverage or reliability probability. The unchanged 94.0% is a dimension
score that excludes these exact-scenario gaps. Sixty-four scenarios remain a
reviewed subset, not proof that every platform and fault combination is listed.

## Bounded Achievements Retained

| Scenario | Evidence and limit |
| --- | --- |
| `sdk-native-result-local-contract` | Mac ARM64 and Linux x86_64 source SDK checks reject wrong identities and contradictory success envelopes; not physical-result validation. |
| `agent-native-result-local-recovery` | Real source Agents reject three bad result forms, preserve original task identity, reap the child and execute a separate healthy task. |
| `agent-unix-owned-process-local-recovery` | Source-native crash/hang/cancel/parent-loss and owned process-group cleanup; not escaped descendants, Windows or hostile-code sandboxing. |
| `mac-linux-ssh-variant-deployment` | Retained two-host template staging, explicit enablement, Installer-managed restart, exact digest and sum=14, plus conflict rejection; SSH tunnel, not native TLS. |

These four targets are explicitly `verified`, not `qualified` or `operational`.
The cross-host exercise predates the later isolation/result changes; this review
does not claim that the combined newest source was rerun across both hosts.
The [retained integration record](../docs/operator-store-integration.html)
provides exact commands, dates and limits. It records the latest separate
Mac/Linux source runs as 38 SDK tests, 337 Agent units and 13 real integrations
per platform, plus the Mac Engine suite. Those are prior test results, not new
tests run during recalibration or an additive lifetime test count.

## Explicit Open Acceptance

| Scenario | Required next proof |
| --- | --- |
| `workbench-real-store-account-session` | Installed browser/OIDC callback, session expiry/refresh/logout against the deployed issuer and a registered test account. |
| `workbench-real-store-authorized-download` | Actual entitlement denial, consent, quota-idempotent grant/download, reply loss and App reopen; independent trust pins. |
| `installer-public-signed-store-install` | Real published signed package to exact-platform native installation; draft sealing and local signing fixtures are insufficient. |
| `workbench-pwdt-store-agent-journey` | Matching installed GUI and WASM PWDT from catalog reference to enable/deploy/run/recover, not host Python or mocked bridges. |
| `agent-linux-variant-native-tls` | Actual Mac-to-Linux package recovery over native TLS/token, including credential rotation and changed Agent identity. |
| `agent-linux-arm64-variant-execution` | Real target library and Agent execution/recovery on Linux ARM64, not ELF-header fixtures. |
| `agent-windows-owned-process-isolation` | Current Windows process-tree lifetime/descendant cleanup and recovery, not old in-process ABI tests. |
| `agent-unix-installed-isolation` | Current isolated Agent packaged and installed into disposable owned services, with matching client artifacts and faults. |
| `agent-isolation-whole-call-performance` | Spawn/load/IPC/decode/cleanup latency and sustained capacity/resource bounds; old resident-host or 1M solver timing does not count. |
| `installer-project-operator-controlled-replacement` | Portable bundle pins, explicit managed version replacement and safe recovery without overwriting active packages. |
| `headless-package-deployment-published-parity` | New deployment contract in separately distributed Rust/Python/Elixir packages, not just the workspace Rust helper. |
| `orchestra-store-project-package-supply` | Authorized project-pin admission into Orchestra's catalog and bound-Agent pull/execution; no Store control authority. |
| `installer-multiplatform-store-publication` | Real target-native candidates through author upload, independent review and explicit publication; no automatic listing. |
| `sdk-current-isolated-abi-qualification` | Current independently built plugin/host pairs and valid old-package compatibility across the supported isolation matrix. |

Three open obligations reference existing partial claims; those references
acknowledge implementation progress but cannot satisfy a verified/qualified
target. The others explicitly bind no qualifying proof. None authorize service
restarts, production account activity or fault injection against user data.

## Priority And Next Work

The unchanged release-weighted queue now begins:

1. `sdk-headless/sdk_headless` (585): installed Python/Elixir research, current
   modal production journey and new deployment API distribution/parity.
2. `workbench-shell/validation` (500): installed recovery and a real GUI/PWDT
   Store-to-Agent journey rather than isolated mock/native layers.
3. `runtime-protocol/sdk_operator` (455): exact Worker-SDK protocol security
   evidence, distinct from Headless control and GUI automation.
4. `runtime-agent-cli/sdk_operator` (385): current installed Unix isolation,
   Windows process-tree recovery and real Linux ARM64 execution.
5. `runtime-engine-solver/validation` (380): physical modal publication,
   multimode recovery and independently reassembled requests.

This ranking is a planning heuristic: registering more distinct obligations
can increase a coordinate's priority. It is not measured defect density.
For the active Store/native workstream, first qualify native TLS deployment and
measure per-call isolation overhead on owned source test services. Then close
the installed GUI/PWDT path and published SDK deployment parity. Real deployed
account/download acceptance needs the backend handoff below; it need not block
independent execution reliability work.

Backend handoff remains public CA and fingerprint, registered OIDC public
client/callback/scopes, dedicated test account/entitlement cases, independent
signing-key fingerprint and published matching Mac/Linux variants. No private
keys or Agent tokens belong in that handoff. Store is responsible for immutable
distribution and access entitlement, not Agent lifecycle or execution authority.

Keep numerical production, authenticated Mesh, whole-generation restore,
PostgreSQL checkpoint migration, typed recovery and Windows installed scopes
open at their original targets. No numerical threshold was relaxed.

## Verification Of This Review

- Three new tensor regressions failed before the registry existed, then passed.
- All 28 tensor tests passed, including preservation of every earlier claim,
  same-policy comparison, no historical/transport evidence borrowing and
  reopening the exact obligation when its supporting proof is withdrawn.
- The full native script-runner suite passed: 419 tests, zero failures/skips.
- Tensor, matrix/topology, documentation, Store contract, formatting, strict lint
  and project organization checks passed for the final working tree; success is
  structural/configuration verification, not release qualification.

No product tests, remote CI, app installation, remote service update or paid
model API call were performed by this recalibration. The generated full tensor
stays in ignored `tmp/`; this compact record is retained instead of a source,
database or binary snapshot.
