# Weakness Roadmap For Moxi 2.x

This document turns the current weak spots into a concrete roadmap for the
remaining `moxi 2.x` hardening line.

It complements:

- [minimal-industrial-closure.md](minimal-industrial-closure.md)
- [commercial-readiness-2.0.md](commercial-readiness-2.0.md)
- [release-prep-1.9-to-1.20.md](release-prep-1.9-to-1.20.md)

## Roadmap Principle

The goal after `moxi 2.0.0` is not to maximize feature count.

The goal is to make the strongest current capabilities repeatable, explainable,
recoverable, and honest enough for selected early research and industrial
partners.

## Current Tensor Status

The module/function/evidence tensor is now the first navigation gate for this
roadmap. Run `make check-module-function-coverage-tensor` before claiming a
roadmap area is closed.

Current moxi baseline:

- `gap_count`: `0`
- `blocking_gap_count`: `0`
- `maturity_gap_count`: `0`
- `thin_evidence_count`: `0`
- `evidence_grade_gap_count`: `0`
- required cells meeting their grade target: `77 / 77` (`100.0%`)
- evidence progress toward configured targets: `100.0%`
- release-critical P0 cells meeting target: `55 / 55` (`100.0%`)
- release-profile P1 cells meeting target: `22 / 22` (`100.0%`)
- `daji 3.0.0` release state: `blocked` solely by the independently controlled
  external usability release gate

No configured coordinate remains below its current evidence target. This does
not grant a daji release claim: packaged cross-platform recovery and upgrade
tiers remain independently open, and static or local evidence cannot close
them. The moxi 2.15 recalibration separated the Rust-only Worker/Operator SDK
from both Installer and the three-language Headless SDK family, added ABI
compatibility as an evidence dimension, and promoted security, persistence,
validation, and benchmark coordinates that were previously optional.

The remaining release queue is now external rather than coordinate-local.
Agent package execution is qualified on native macOS aarch64 and physical
Linux x86_64, while Windows installed external-package operation spans SDK,
Engine, Agent, and Installer. Workbench, Engine, and Headless
persistence/provenance now meet their local `verified` targets, while Workbench
and Headless security meet the local `qualified` target. Headless manifest
generation now has a current-line 1000-repeat qualification run. Protocol now
qualifies TaskIR preview and typed workflow graph round trips with 1000 repeats
each. Orchestra retains three semantically stable rounds across 256, 512, and
1024 pass-through graph sizes. Every P1 coordinate now meets its target.
Operator SDK workflow dispatch now proves same-process recovery after rejecting
an unknown extension, while the retained dynamic qualification binds tamper
rejection and Installer lifecycle recovery into the verification coordinate.
No required coordinate remains maturity-thin.
The core workflow graph/dataset contract
now has scoped, repeatable positive and rejection-boundary qualification.
Ordinary lane execution no longer counts
as asserted verification, and two native hosts do not imply an untested Windows
ABI journey.

The earlier Agent, Engine, and verification benchmark qualification remains
valid. The native current-line route executes a
10k Engine solve three times under a release build, serializes the 49-route
direct-FEM manifest 1000 times, and exercises both Protocol hot paths 1000
times. The retained remote archive covers all 19
expected 500k cases and all 39 expected 1M cases across six matrices, with every
1M case meeting the one-million-node threshold. It also records 117 retained
runs and classifies all 10 historical failures as resolved by later success.
The direct-mesh baseline contributes three repeated Agent journeys and six
subtest samples through the current native comparator. Current-line evidence
lives at `releases/usability-evidence/2.15.0/benchmark-qualification.json` and
is checked by `make check-benchmark-qualification`. Its contract inherits and
revalidates the retained 500k/1M scale archive from the earlier qualification,
so a clean checkout does not depend on an untracked `tmp/` index. Historical
scale numbers remain tied to their prior Linux hosts; the report explicitly
does not make a hardware-independent performance guarantee.

The required Headless and Protocol benchmark coordinates now use scoped
current-line runs rather than inheriting Engine scale numbers. Orchestra has a
separate native-controlled qualification at
`releases/usability-evidence/2.15.0/orchestra-benchmark-qualification.json`.
It binds the Elixir workload sources by SHA-256, enforces catastrophic-regression
ceilings, compares timing-independent semantics across three rounds, and is
rechecked by `make check-orchestra-benchmark-qualification`. Focused Operator
SDK latency measurement remains useful follow-up work, but it is not a required
benchmark coordinate in the current Daji release profile.

The Agent portion of `upgrade_and_rollback` now has fresh operational evidence.
The native controller builds distinct debug and release payloads on the remote
Linux qualification host, delegates installation and activation to Installer,
runs the Agent before and after the update, rolls back, runs it again, verifies
the original SHA-256 payload was restored, retains the report, and removes the
managed run root. The semantic validator intentionally rejects the older
`2.12.6` remote report because its rollback version and payload digest disagree.
The accepted evidence is
`releases/usability-evidence/2.14.3/agent-update-operational-qualification.json`
and is rechecked by `make check-agent-update-operational-qualification`. The
parent release tier remains open until packaged desktop update rollback is
proven across the remaining required platforms.

The Runtime payload portion now also has remote Linux operational evidence.
Installer seals distinct Debug and Release native payloads, installs them into
an isolated immutable store, executes all three declared service entries during
initial install, upgrade, and rollback, and verifies that the rollback content
digest exactly restores the first payload. Runtime activation now uses a
managed update lock and monotonic history-derived generations rather than wall
clock values. The accepted report is
`releases/usability-evidence/2.14.3/runtime-payload-operational-qualification.json`
and is rechecked by `make check-runtime-payload-operational-qualification`.
This closes only `upgrade_and_rollback/runtime-payload-remote-linux`; packaged
desktop update and rollback on Linux and Windows remain open.

The coordinated fleet portion now has retained remote Linux evidence as well.
Installer applies one Runtime payload and two Agent packages as a single
aligned-version transaction. It rejects aliased or overlapping component
stores, serializes controllers with a fleet-level lock, injects a failure before
the second Agent switch, compensates the Runtime and first Agent, executes every
component after compensation, completes a clean upgrade, and then rolls the
entire fleet back.
The Release-built Installer keeps repeated SHA-256 verification fast, and the
successful qualification removes its 451 MB temporary payload tree before the
small report is retained. Evidence lives at
`releases/usability-evidence/2.17.0/fleet-update-operational-qualification.json`
and is rechecked by
`./scripts/kyuubiki check-fleet-update-operational-qualification --verify-report releases/usability-evidence/2.17.0/fleet-update-operational-qualification.json --require-remote-linux`.
This closes `upgrade_and_rollback/installer-managed-fleet-remote-linux`, not the
parent tier: packaged desktop rollback on Linux and Windows remains open.

Live workload continuity during Agent replacement now has its own retained
remote Linux qualification. Each Rust Agent exposes a fenced admission lifecycle
with an exact active-execution count and immutable process-instance identity.
Lifecycle mutation is fail-closed to an operating-system-confirmed loopback
peer; remote callers may inspect state but cannot drain or resume an Agent over
the unauthenticated solver socket. Installer therefore executes the replacement
controller on the target host through the managed SSH boundary.
Installer drains one of two Agents to quiescence, replaces its binary, requires
a new accepting process identity, and repeats for the peer. During both
replacement windows the other Agent completes a real bar solve; initial and
final probes prove the complete two-node fleet remains executable. The run also
requires changed payload digests and removes its isolated remote work root.
Evidence lives at
`releases/usability-evidence/2.17.0/agent-rolling-replacement-operational-qualification.json`
and is rechecked by
`./scripts/kyuubiki check-agent-rolling-replacement-operational-qualification --verify-report releases/usability-evidence/2.17.0/agent-rolling-replacement-operational-qualification.json --require-remote-linux`.
This closes only
`upgrade_and_rollback/live-agent-rolling-replacement-remote-linux`; the parent
upgrade tier remains open until packaged desktop update and rollback are proven
on Linux and Windows as well.

The packaged desktop set now has its first operational update/rollback tier on
macOS. Installer hashes every file in Hub, Installer, and Workbench, stores the
three shells as one immutable versioned unit, records monotonic atomic
activations, rejects content drift and unmanaged files, and rolls back to the
exact original aggregate and component digests. The host qualification makes
two differently marked and ad-hoc re-signed variants from the current release
bundles, then launches all three shells after initial install, upgrade, and
rollback. All nine boot-receipt probes pass, and the isolated store, staging
tree, update lock, and GUI processes are removed afterward. Evidence lives at
`releases/usability-evidence/2.17.0/desktop-bundle-update-operational-qualification.json`
and is rechecked by
`./scripts/kyuubiki check-desktop-bundle-update-operational-qualification --verify-report releases/usability-evidence/2.17.0/desktop-bundle-update-operational-qualification.json --require-platform macos`.
The `2.16.9` and `2.17.0` labels identify the two qualification package
generations; both contain the current `2.17.0` runtime, so this proves package
switching and exact rollback rather than historical binary compatibility. It
closes only `upgrade_and_rollback/packaged-desktop-set-macos`.

The first Installer-managed end-to-end runtime subtier is now operational on
remote Linux. A sealed installed payload starts Orchestra and two Rust Agents
without loading the frontend; an installed Rust Headless client submits a real
bar solve, observes `rust-agent-rpc` dispatch, and verifies displacement and
stress. The completed result survives two managed restarts, including one after
the synchronized source tree is removed. Shutdown closes every qualification
port, removes managed PID files and the remote experiment root, and leaves zero
residue. The sanitized evidence is retained at
`releases/usability-evidence/2.14.6/installed-runtime-operational-qualification.json`
and is rechecked by
`make check-installed-runtime-operational-qualification`. This closes only
`remote_agent_orchestra_round_trip/installer-managed-linux`; the parent tier
still requires multi-host package acquisition, network-loss/rejoin behavior,
fleet scheduling, and installed operation on the remaining supported
platforms.

Agent control-link recovery is now explicit rather than silent. The Rust Agent
records registration and heartbeat attempts through the shared
`kyuubiki.agent-control-link/v1` contract, reports only sanitized failure codes,
falls back from a failed heartbeat to full registration, and uses bounded
exponential retry without delaying shutdown for an entire backoff window.
Orchestra retains the latest link snapshot and summarizes link states in its
registry diagnostics. A native TCP fault-injection test proves
register -> rejected heartbeat -> re-register -> clean unregister, while the
Orchestra registry test proves the degraded-to-registered diagnostic
transition. This is local verified recovery evidence; the parent remote tier
now also has native two-physical-host operational evidence. The qualification
builds an isolated Release Agent on remote Linux, starts a protected in-memory
Orchestra on macOS, observes registered and heartbeat state, kills Orchestra,
proves that the same Agent process reports a sanitized degraded state, recreates
Orchestra on the same endpoint, and requires registration count growth before
accepting recovery. The retained run moved from registration count 1 to 2,
continued heartbeat progress, closed every qualification port, removed both
secret files and the managed remote root, and retained no host identity or
address. Evidence lives at
`releases/usability-evidence/2.14.7/agent-control-link-operational-qualification.json`
and is rechecked by `make check-agent-control-link-operational-qualification`.
This closes control-link network-loss/rejoin, not Installer-managed package
acquisition, fleet scheduling, or installed operation on every supported
platform. In-flight process-loss recovery now has its own independent two-host
operational qualification rather than being inferred from this link test. That
qualification uses a visible, default-disabled, exact-job execution barrier,
requires the remote watchdog to show the job active, then kills the remote Rust
Agent before result commit. It proves idempotent fallback execution,
checkpoint-required replay blocking, checkpoint-authorized continuation, three
same-endpoint rejoins, follow-up closed-form solves, and zero residue. Evidence
lives at
`releases/usability-evidence/2.14.8/distributed-task-recovery-operational-qualification.json`
and is rechecked by
`make check-distributed-task-recovery-operational-qualification`. Shared
PostgreSQL Orchestra process-crash takeover is now independently retained for
both source runtime and a source-detached Installer-managed Linux production
release. Network-partition fencing now has separate physical-database evidence:
the owner remains alive but loses only its database tunnel, fails closed, the
standby takes token 2, and the former owner rejects writes after rejoining.
Long-running workflow takeover is now independently operational as well. An
exact remote Agent barrier holds generation-one work while the active Orchestra
is lost: idempotent work is claimed as generation two, dispatches exactly
twice, and commits one verified result; uncheckpointed `checkpoint_required`
work remains generation one, is blocked without redispatch, and ignores the
orphan completion. Both former owners rejoin as standby and the retained run
leaves no managed process, port, tunnel, database, or work-root residue.
Evidence lives at
`releases/usability-evidence/2.18.3/orchestra-long-workflow-takeover-operational-qualification.json`
and is rechecked by
`make check-orchestra-long-workflow-takeover-operational-qualification`. This
closes that subtier and leaves nine explicit release subtiers open. Full host
power loss, Installer-led fleet package acquisition, and the installed
cross-platform matrix remain among them.
Persisted in-flight workflow state now has a
separate local qualification: a digest-bound execution envelope survives a
complete OTP application stop/start, a fresh session claims a higher
generation, stale writers are fenced, idempotent work resumes, uncheckpointed
side effects stop, and tampered envelopes fail closed. Runner loss and missing
Task Supervisor paths are also injected. This remains application-level
SQLite evidence, not host-loss or HA-database operational proof. The earlier
control-link probe also
exposed and fixed an HTTP write-half-close incompatibility with Cowboy; a
native regression requires the Agent to keep the request connection open until
the response arrives.

The former P0 runtime API tie now meets its `verified` target. The current-line
protocol report executes 101 tests, proves 56 advertised methods have 56 unique
wire round trips, rejects unknown methods and malformed envelope states, and
retains TaskIR tamper rejection. The Headless report executes 271 tests across
Python, Elixir, and Rust, including real loopback workflow and operator-task
routes plus 14 cross-language failure-parity cases. Both reports are retained
under `releases/usability-evidence/2.15.0/` and are revalidated together by
`make check-runtime-api-verification`. This is local current-line verification,
not installed-package, remote-host, or cross-platform operational proof.

The persistence/provenance P0 cluster now meets its local targets across six
modules. Installer and Workbench guarded mutations share a SHA-256 chained
ledger that rejects tamper before side effects and records both successful and
failed actions. Engine solver results bind canonical JSON content to a SHA-256
digest, expected operator identity, and result type; Headless operator tasks
verify summary, execution preview, and retained lineage as one fail-closed
profile. Orchestra still recovers verified generations and fenced in-flight
workflows, while native Installer retains atomic journal resume and tamper
rejection. Seven suites repeat twice with all 14 suite rounds and 28 assertions
passing in
`releases/usability-evidence/2.15.0/persistence-provenance-qualification.json`.
Engine, Headless, and Workbench are claimed only at `verified`; this local
evidence does not imply remote database durability, installed cross-platform
operation, or multi-host recovery.

The former first coordinate, `sdk-headless / sdk_headless`, now meets its
`operational` target. Its remote Linux qualification installs the release Rust
Headless tools with Cargo into an isolated prefix, records both binary digests,
deletes the synchronized source tree, and executes with an empty isolated
runtime `PATH`. The installed tools discover 35 templates; initialize,
validate, render, and execute a three-step Headless workflow; run a three-
candidate heat-spreader study through native real solver kernels without mock
or fallback; reject a missing workflow before execution; and validate the
original workflow again after failure. The managed remote root is removed with
zero residue, and the retained report contains no host identity or absolute
path. The installed package identifies itself as shipping version `2.7.0`; the
qualification belongs to the current `2.14.1` development evidence line and is
retained at
`releases/usability-evidence/2.14.1/headless-sdk-operational-qualification.json`.
Python and Elixir parity remain supported by the separate 263-test qualified
suite; this Rust installation evidence does not claim registry-installed
operation for those languages.

The former first coordinate, `orchestra-control-plane / workflow_composition`,
now meets its `operational` target. Its remote Linux qualification executes the
orchestrated and offline-mesh paths, proves a 23-node heat-to-thermo workflow
across two registered Agents, retains the completed result across an Orchestra
restart, rejects unauthorized and malformed submissions without creating jobs,
rejects tampered Agent TaskIR, recovers execution, verifies process-loss replay
policy, and closes all qualification ports without residue. The retained report
contains no host identity or absolute path and lives at
`releases/usability-evidence/2.14.1/orchestra-workflow-operational-qualification.json`.

The former tied coordinate, `workbench-shell / workflow_composition`, now meets
its `qualified` target alongside `workbench-shell / validation`. Its native
qualification repeats 38 PWDT control-plane tests, one three-viewport browser
layout regression, and two isolated real-Next workflow journeys twice. All 82
test executions pass with stable semantic digests across rounds. The browser
journey proves catalog discovery, builder entry, operator insertion, draft
storage, submission, polling, completed result rendering, and fail-closed
invalid JSON with zero backend submissions. Across the three suites, 18
acceptance assertions and five explicit rejection boundaries are retained in
`releases/usability-evidence/2.14.0/workbench-validation-qualification.json`.
This local qualification does not imply installed-package or cross-platform
operational proof.

The former first coordinate, `contracts / validation`, now meets its
`qualified` target. Its native qualification repeats 14 checks twice across
seven contract families. All 28 rounds pass with stable output digests, covering
eight acceptance paths and six explicit rejection boundaries: duplicate
workflow values, TaskIR integrity tampering, material-chain round drift,
operator-model schema and side-effect violations, and repository path
traversal. The machine-validated report is retained under
`releases/usability-evidence/2.13.9/contracts-validation-qualification.json`.

The former first coordinate, `runtime-engine-solver / solver_execution`, now
meets its `operational` target. The retained remote Linux journey builds the
release Agent and Installer in an isolated lab run, seals and atomically
activates the Agent package through Installer-owned storage, and executes the
same closed-form TaskIR through two distinct Agent processes. Both processes
prove numerical equality, capability rejection, digest-tamper rejection,
watchdog quiescence, and recovery; the qualification work root is removed with
zero residue. The machine-validated report is retained under
`releases/usability-evidence/2.13.8/agent-solver-operational-qualification.json`.
This evidence does not promote Orchestra fleet scheduling or installed
Headless SDK operation.

The P0 security cluster now meets its `qualified` target for contracts,
shared desktop UI, Hub, Workbench, all three official Headless SDK bindings,
Installer shell, Orchestra, Agent, Engine, and native Installer. The native
qualification maps 20 asserted checks onto 28 exact module/security-lane
coordinates and repeats every check twice. Its 40/40 retained rounds cover UI
capability boundaries, in-memory Workbench secrets, route-scoped auth, hostile
language-pack rejection, SDK debug redaction and header-injection rejection,
runtime fuzz boundaries, dependency and component integrity, credential
storage, remote deployment metadata, and data contracts. The machine-validated
report is retained under
`releases/usability-evidence/2.15.0/system-security-qualification.json`.
Installed cross-platform penetration testing and multi-host adversarial testing
remain separate operational tiers and are not implied by this local
qualification.

The web control plane also retains an upstream dependency residual: the latest
published Cowlib 2.19.0 release is still marked with unresolved Hex security
advisories. Kyuubiki has lock-bound tests for invalid response-header rejection
and non-reachability of the affected Link serializer, but these do not replace
an upstream patch. The retained installed-package Orchestra takeover journey is
loopback-only and therefore does not qualify public-network exposure. Pin the
first patched upstream release, rerun dependency and control-plane security
lanes, then add adversarial reverse-proxy and direct-listener evidence before
promoting this boundary.

The former leading coordinate, `sdk-headless/workflow_composition`, now meets
its `qualified` target. Its native qualification requires all 230 Rust
Headless workflow-core tests and all 16 CLI execution-boundary tests. It proves
template normalization, service-action coverage, bounded same-job wait
recovery, explicit per-run wait-budget overrides, zero-execution rejection of
contract-invalid batches, retained runtime failure causes, and standard
non-empty execution reports for malformed documents and incompatible
executors. The
machine-validated result is retained under
`releases/usability-evidence/2.13.1`; installed package and remote multi-host
operation remain separate evidence tiers.

The former leading coordinates, `hub-shell/deployment_update` and
`installer-shell/deployment_update`, now meet their `qualified` targets. The
native qualification combines all 60 Installer tests with an eight-test
browser and IPC suite. It proves the Hub-to-Installer handoff, update-source
configuration, digest-verified download, digest revalidation before apply,
installation-integrity follow-up, catalog traversal rejection, tamper
rejection, and managed-path symlink rejection. The machine-validated result is
retained under `releases/usability-evidence/2.12.6`; installed cross-platform
self-replacement and remote fleet rollout remain operational proof tiers and
are not implied by this local qualification.

The former leading coordinate, `sdk-headless/validation`, now meets its
`qualified` target. Its native qualification runner requires all 263 official
SDK tests across Python, Elixir, and Rust, including live loopback transport
checks, 14 paired cross-language failure boundaries, and four shared contract
fixture digests. The machine-validated result is retained under
`releases/usability-evidence/2.12.6`; package-registry installation and remote
service execution remain separate operational proof tiers.

The former leading coordinate, `runtime-protocol/validation`, now meets its
`qualified` target. Its native qualification runner requires all 94 protocol
tests, four configured fuzz profiles totaling 1280 cases, five digest-checked
TaskIR examples spanning Rust and Elixir authoring, all 56 advertised RPC
method round trips, structured TaskIR rejection codes, and strict request,
response, and progress envelope boundaries. The machine-validated result is
retained under `releases/usability-evidence/2.13.1`; installed and multi-host
protocol proof remains a separate operational tier.

The `runtime-protocol/security` and `verification-evidence/security`
coordinates now meet their `qualified` targets through the same retained v2
qualification. It requires 9 named fail-closed boundaries across RPC envelope
state, unknown-method admission, JSON and byte ingress, Task IR digest and
structure rejection, and solver-capability admission. This result is narrowly
scoped: it does not promote GUI, credential, control-plane, Agent, Engine, or
Installer security without independent retained evidence for those surfaces.

The former leading coordinate, `desktop-shared-ui/validation`, now meets its
`qualified` target. The native qualification runner executes the cross-shell
browser suite, requires all 33 tests to pass, preserves Hub, Installer, and
Workbench action counts, and verifies UI-to-native closure, PWDT parity,
workspace-dominant layouts, reversible navigation, regression panels, live
Workbench chunk mounting, controlled Model/Study, System, and Library deep-page
round trips, Store request ownership, shared GUI/PWDT Store manifest mutation,
no-click PWDT Store/Workflow navigation, and Pwdt session recovery. The machine-validated
result is refreshed under `releases/usability-evidence/2.18.3`; installed-package
and cross-platform proof remain separate operational tiers.

The former leading coordinate, `installer-shell/validation`, now also meets
its `qualified` target without inventing a parallel validation framework. The
current cross-shell contract is explicitly mapped back to Installer and its
retained moxi 2.18.3 report reruns all 27 browser/call-chain tests, observes 53
Installer actions with zero missing or failed actions, and preserves four
intentional fail-closed guards. Deployment/update handoff, capability routing,
workspace priority, reversible navigation, and regression-critical panels are
all asserted. The report is retained at
`releases/usability-evidence/2.18.3/desktop-ui-validation-qualification.json`
and is now rechecked by `make check-desktop-ui-validation` as part of
`architecture-check`.

The former top two coordinates, `runtime-agent-cli/solver_execution` and
`runtime-protocol/solver_execution`, now meet the `qualified` target. Their
shared v2 qualification sends a `solve.bar_1d` TaskIR through a live TCP Agent
into the Rust Engine, checks the closed-form displacement, rejects a digest
tamper, verifies same-process recovery, and retains the machine-validated
report under `releases/usability-evidence/2.12.5`. The Agent TaskIR allowlist is
intentionally limited to `solve.bar_1d`; the broader direct solver RPC surface
is not yet claimed as equivalent TaskIR execution coverage.

The runtime API client calibration promotes Hub, Workbench, Installer, the
native Installer service, Protocol, and Headless SDK to required `runtime_api`
coordinates. Desktop clients retain UI-to-native execution closure, the native
Installer retains its serializable Rust API manifest and stable exports, and
Protocol plus Headless now carry scoped current-line verification. Ordinary
runnable lanes still stop at `exercised`; only exact-coordinate retained claims
can promote a runtime API coordinate further.

## 1. Numerical Trust

Current weak point:

- all release-gated solve operators are now `qualification` level, but several
  qualifications are still scoped around compact retained fixtures
- readiness v2 now separates reference, convergence, robustness, and retained
  release evidence for all 23 qualification candidates; the current measured
  baseline is `23 complete / 0 partial / 0 thin`, with reference, convergence,
  robustness, and retained-release gaps cleared
- `beam-frame-classic` now links its independent canonical reference to the
  existing `1/2/4/8/16` beam/frame refinement regression, making it the first
  candidate combining complete four-dimensional evidence with an independent
  canonical reference
- `solve.nonlinear_spring_1d` now retains convergence and robustness evidence
  for the current Cardano hardening scope, clearing the last thin
  release-gated operator candidate in readiness v2
- `line-field-closed-form` now retains a 1, 2, 4, 8, and 16 element refinement
  regression for axial, thermal, heat, and electrostatic 1D operators, clearing
  its convergence evidence dimension
- `electromagnetic-plane-patch` now ties its retained electrostatic and
  magnetostatic triangle/quad patch evidence to manufactured linear-field
  refinement tests, clearing the plane electromagnetic convergence dimension
- `modal-frame-sanity` now retains an independent linear generalized
  eigenproblem reference note for Rayleigh stiffness/density scaling, clearing
  its reference evidence dimension
- `screening-cfd-boundary` now retains a Stokes-only manufactured linear-field
  reference note for divergence, shear, viscosity, density, and explicit
  non-Navier-Stokes scope, clearing the final reference evidence gap in
  readiness v2
- `acoustic-bar-closed-form` now retains a 1, 2, 4, 8, and 16 element acoustic
  pressure-field refinement regression for pressure, pressure-gradient,
  particle velocity, and wave number, clearing its convergence dimension
- `advection-diffusion-bar-closed-form` now retains a 1, 2, 4, 8, 16, and 32
  element pure-diffusion refinement regression for concentration, diffusive
  flux, total flux, and zero Peclet number, clearing its convergence dimension
- `magnetostatic-bar-closed-form` now retains a 1, 2, 4, 8, and 16 element
  magnetic-potential refinement regression for field strength, flux density,
  stored energy, and nodal potential, clearing its convergence dimension
- `spring-1d-closed-form` now retains a 1, 2, 4, 8, 16, and 32 element
  equivalent-chain refinement regression for tip displacement, member force,
  element extension, and total strain energy, clearing its convergence dimension
- `spring-vector-closed-form` now retains 1, 2, 4, 8, and 16 element orthogonal
  axis refinement regressions for 2D and 3D free-node displacement, member
  force, strain energy, and axis-projected node displacement
- `thermal-beam-1d-closed-form` now retains a 1, 2, 4, 8, and 16 element
  free-curvature refinement regression for quadratic displacement, linear
  rotation, retained curvature, near-zero internal force, and zero energy
- `contact-gap-1d-closed-form` now retains 1, 2, 4, 8, and 16 element active
  and inactive penalty-stop refinement regressions for displacement, spring
  force, penetration, contact force, and branch activation
- `truss-2d-closed-form` and `truss-3d-closed-form` now retain 1, 2, 4, 8, and
  16 member area-partition refinement regressions for apex displacement, stress,
  strain, axial-force summation, and total strain energy
- `thermal-truss-2d`, `thermal-truss-3d`, `thermal-frame-2d`, and
  `solid-tetra-3d` now retain explicit boundary-regression robustness artifacts,
  clearing the last readiness v2 numerical-validation gaps
- benchmark-backed accuracy exists across the covered matrix, but the next
  trust jump depends on deeper convergence, perturbation, and reference-tool
  evidence
- some limitations are documented in evidence packets, but they still need to
  become more product-visible in workflow previews and reports

Current moxi hardening focus:

- keep every release-gated physics family at `qualification` without lowering
  the manifest minimum
- turn compact qualification fixtures into richer evidence ladders across
  mechanical, thermal, electromagnetic, CFD/transport, and coupled workflows
- surface explicit failure, assumption, and limitation notes in user-facing
  workflow and export paths

Current progress:

- the composite thermo-electric material loop now cross-validates every
  electrostatic candidate against an independent layered-dielectric closed
  form, retains the maximum relative error as a quality gate, and promotes the
  analytic baseline into the research bundle only when all candidates pass;
  the same loop now retains real `1/2/4/8` electrostatic mesh refinement for
  every candidate
- the composite heat subproblem now retains an independent layered
  thermal-resistance check plus real `1/2/4/8` heat-quad refinement for main
  and materialized candidates; its source is no longer a fixed fixture:
  solved RMS dielectric fields are converted through
  `q = 2*pi*f*epsilon_0*epsilon_r*tan_delta*E^2`, volume-weighted onto heat
  nodes, and guarded by an explicit energy-balance gate
- dielectric permittivity, loss tangent, and each declared thermal-region
  conductivity now close a relaxed temperature feedback loop around the
  electrostatic, loss-projection, and heat solvers; all three retained candidates
  converge in `9-11` iterations with worst final temperature residual
  `9.79e-8 C`, loss change `1.83e-10`, and conductivity change `3.91e-10`, while
  the built-in coefficients remain explicitly labeled screening sensitivities
  rather than qualification-grade material curves
- the converged heat field now projects local mean temperatures into all three
  structural-region expansion coefficients before every main, regularized, and
  interface-graded structural solve; retained coverage is `100%`, the largest
  coefficient change is `3.64e-4`, and the largest resulting peak-stress change
  is `1.65e-4`, while validated nonlinear expansion curves remain open
- the composite thermal-structural subproblem now runs real two-dimensional
  `1/2/4/8` quad refinement for main and materialized candidates, preserving
  the solved heat profile and material parameters; node identity and coordinate
  checks now guard the heat-to-thermal projection before structural execution
- structural follow-up should still regularize the clamp, add interface
  mechanics and local refinement/stress recovery, then correlate against an
  external solver for deeper confidence; this is no longer a tensor blocker
- a retained roller-edge/vertical-anchor diagnostic now proves that restraint
  sensitivity is small for displacement while strain-energy drift remains above
  `22%`; the machine-readable
  diagnosis is restraint-sensitive but persistently energy-nonconvergent and
  cannot override the primary gates
- area-weighted von Mises RMS and P95 recovery now rejects the simpler
  explanation that only one peak element is unstable: finest-pair drift remains
  about `1.6%` and `29.8%`; local refinement,
  higher-order recovery, and independent structural correlation remain open
- cosine grading at the clamp, layer interfaces, and free edges remains
  diagnostic: P95 drift is about `34.1%` and strain-energy drift about `11.9%`;
  this narrows
  the localization diagnosis without promoting structural qualification
- four-level observed-order analysis now refuses displacement extrapolation
  because its sequence is oscillatory, reports about `12.7%` fine-grid GCI for
  asymptotically converging strain energy, and
  classifies peak stress as monotonically divergent; independent correlation
  and a better structural formulation remain the qualification blockers
- the SPD profile path now recomputes residuals against the original matrix
  instead of reporting synthetic zero for dense solves; retained composite
  thermal solves pass at or below `2.15e-14` relative residual in the current
  three-candidate baseline, so
  the remaining failure is isolated to discretization or modeling rather than
  algebraic nonconvergence
- `solve.solid_tetra_3d` now retains parameter-perturbation, rigid-rotation,
  and `1/2/4/8` multi-element affine patch checks in its active qualification
  profile; additive nodal reactions, free-DOF residual, and resultant force
  balance make equilibrium machine-visible while preserving legacy result
  deserialization. A self-equilibrated pure-bending manufactured solution now
  adds strict `2/4/8/16` non-affine displacement, stress, and energy contraction,
  reaching `3.08%`, `14.60%`, and `2.82%` on `24,576` tetrahedra. Distorted and
  interior geometry now retains a separate `4/8/16` convergence ladder with
  minimum mean-ratio quality above `0.2827`; scale-independent quality summaries
  expose distortion counts and near-incompressible locking risk. Per-component
  topology preflight now rejects orphan nodes, hidden rigid rotations, and
  floating disconnected domains; independently restrained components solve in
  one block system, and remapped node/element indices retain the same physical
  response. A general unstructured mesh generator and broad connectivity-family
  corpus, a stabilized incompressible formulation, and independent external 3D
  correlation are now the next-depth boundary
- `solve.plane_quad_2d` now uses a native bilinear isoparametric Q4 kernel with
  full `2x2` Gauss integration instead of two constant-strain triangles;
  distorted `1x1`, `2x2`, and `4x4` affine patches remain exact, and inverted
  connectivity is rejected by Gauss-point Jacobian guards
- `solve.thermal_truss_2d` and `solve.thermal_truss_3d` now retain coupled
  thermal-mechanical rigid-rotation checks with free response degrees of
  freedom; arbitrary assemblies and nonlinear thermal mechanics remain outside
  the qualified scope
- `solve.thermal_frame_2d` now retains a thermally graded and mechanically
  loaded rigid-rotation check; `solve.thermal_frame_3d` now has an optional
  explicit `local_y_axis` contract plus arbitrary 3D rigid-rotation evidence,
  while omitted orientation retains the legacy global-reference behavior
- `solve.frame_3d` now has the same optional `local_y_axis` contract for
  asymmetric sections, with arbitrary rigid-rotation objectivity, cantilever
  mesh-convergence, perturbation scaling, and invalid-axis rejection retained
  in the active qualification evidence
- both thermal frame operators now retain manufactured quadratic-field mesh
  convergence across 1, 2, 4, 8, and 16 elements; axial expansion and all
  represented bending directions demonstrate second-order error reduction
- `solve.thermal_frame_3d` now also retains full-response objectivity for a
  non-collinear three-member spatial chain with independent member orientation,
  thermal fields, and terminal mechanical loading
- branched 3D thermal-frame evidence now covers two fully fixed supports, a
  shared three-member junction, redundant thermal restraint, and load
  redistribution under arbitrary rotation
- `solve.thermal_frame_3d` now supports arbitrary-direction translational
  springs with normalized directions, exact `k n n^T` assembly, reported
  displacement/reaction/energy, axial closed-form evidence, and rotated branch
  objectivity
- arbitrary-axis rotational springs now follow the same projector contract on
  rotational degrees of freedom, report rotation/reaction moment/energy, and
  retain torsion closed-form plus rotated branch evidence
- exact arbitrary-direction translation and rotation constraints now use
  orthonormal nullspace elimination rather than penalty stiffness, recover
  reactions from the full residual, reject dependent directions, and retain
  coupled closed-form plus rotated branch evidence
- `solve.buckling_beam_1d` now provides the first geometric-stability slice:
  linear eigenvalue beam-column buckling, critical reference-load factors,
  normalized modes, Euler-column convergence, and dimensional scaling checks
- `solve.buckling_frame_2d` now derives geometric stiffness from a static frame
  preload, retains portal-frame objectivity and beam-formulation cross-checks,
  and resolves sparse repeated modes with oversampled block iteration
- `solve.frame_2d_p_delta` now carries a selected eigenmode imperfection through
  an elastic precritical load path, accepts explicit measured-shape profiles,
  supports linearized P-Delta and incremental corotational kinematics, and
  retains secant-amplification plus multi-member objectivity evidence; its
  experimental spherical arc-length control exposes both force residual and
  path-constraint error, adaptively cuts back oversized radii, continuously
  targets a visible Newton iteration count, and retains a shallow-arch
  limit-point, descending-branch, and segmented member-instability mesh sequence
  with explicit load increments and limit-point events while preserving load
  control as the legacy default
- stability remains screening-only while complex-frame switched-branch depth,
  automatic problem-scale radius bounds, cross-host adaptive-integration
  qualification, cyclic axial-bending experimental qualification, and broader
  independent external correlation remain incomplete

Qualification focus:

- add convergence checks beyond the retained closed-form or patch fixtures
- add cross-checks against analytic, literature, manufactured-solution, or
  independent reference cases where practical
- keep retained evidence bundles release-addressable for every promoted
  operator family

Moxi readiness standard:

- Kyuubiki can clearly separate release-qualified, scoped qualification,
  experimental, and deferred solver claims without weakening the mainline
  coverage contract.

Primary docs:

- [accuracy-plan.md](accuracy-plan.md)
- [accuracy-baselines.md](accuracy-baselines.md)
- [operator-reliability.md](operator-reliability.md)

## 2. Rust Operator SDK Industrialization

Current weak point:

- the Rust-only operator SDK has descriptors, manifests, readiness checks, and
  preflight; external-local and bound-Orchestra packages now execute through a
  real Agent, including Installer lifecycle coverage
- `cache_scope: none` now holds an exact host-generation lease through dispatch,
  evicts immediately after success or failure, refetches on later demand, and
  is covered under concurrent requests
- `cache_scope: job` now has an explicit terminal RPC, shared-owner retention,
  idempotent release, cancellation cleanup, and final-owner generation eviction
- the same six-stage package journey now passes on native macOS aarch64 and
  physical Linux x86_64, with content-bound smoke/preflight attachments and
  residue-free cleanup
- native Windows installed-package operation now has retained six-stage evidence,
  including MSVC dynamic loading, Agent RPC dispatch, bound-Orchestra rotation,
  tamper recovery, Installer lifecycle, and residue cleanup
- the remaining third-party gap is forward compatibility across future operator
  SDK API and ABI revisions, not initial Windows installation coverage

Current moxi hardening focus:

- keep the operator crate template green with descriptor readiness tests
- expose package readiness in Installer preflight JSON and CI gates
- keep TaskIR package identity and entrypoint digest checks fail-closed
- document the separation between operator SDK and headless SDK everywhere it matters

Qualification focus:

- retain the six-stage external-local qualification from authoring through
  Agent dispatch, tamper rejection, and recovery
- retain Installer-managed activation/removal and bound-Orchestra pull without
  copying the complete central library to every Agent
- retain job-scoped cache retirement at the explicit workload boundary,
  including shared ownership and cancellation
- retain the macOS/Linux multihost report and its four SHA-256-bound child
  attachments under `releases/usability-evidence/2.16.4/`
- preserve the native Windows installed-package report and its bound evidence
  under `releases/usability-evidence/2.15.0/`; rerun the same journey whenever
  the operator SDK API, ABI, package format, or Installer lifecycle changes
- add operator package compatibility fixtures for future SDK API changes

Moxi readiness standard:

- a competent Rust developer can write, package, preflight, and run a custom
  operator without private project knowledge.

Primary docs:

- [operator-sdk.md](operator-sdk.md)
- [operator-library-centralization.md](operator-library-centralization.md)

## 3. Agent, Orchestra, And Mesh Reliability

Current weak point:

- authority boundaries are documented, but long-running failure behavior still
  needs more evidence
- distributed execution must prove recovery from partial failure, package
  fetch failure, node loss, and stale authority state
- persisted workflow transitions now use backend-atomic compare-and-swap; an
  eight-writer SQL regression admits one generation owner and rejects seven
  stale snapshots, and the local memory backend follows the same contract
- active-owner lease expiry is now implemented across memory, SQLite, and
  PostgreSQL; a second Orchestra remains standby, expired ownership increments
  a fencing token, and a stale owner cannot enter protected persistence writes
- the native operational lane now retains a two-Orchestra PostgreSQL journey:
  the owner is killed without graceful release, standby takes token 2, the
  former owner identity is fenced back to standby, and all temporary database,
  tunnel, process, port, and log state is removed
- the remaining gap is package and duration depth: repeat the journey from
  Installer-managed packages, then add long-running workflows and explicit
  database-network disruption

Current moxi hardening focus:

- keep agent and orchestra authority modes explicit
- ensure every agent execution failure reports a machine-readable reason
- continue remote-server tests through Installer-owned paths instead of ad-hoc
  SSH operations

Qualification focus:

- retain fault-injection-style tests for package rejection, node loss,
  watchdog-visible failure reasons, and scheduler retry
- record scheduler, agent, package, engine, and workflow versions in run
  provenance
- prove centralized and decentralized mesh modes without treating one as a
  second-class fallback

Moxi readiness standard:

- one bounded workflow can survive ordinary distributed-system failures without
  cascading into an unexplained global failure.

Primary docs:

- [agent-orchestrator-boundary.md](agent-orchestrator-boundary.md)
- [headless-agent-contract.md](headless-agent-contract.md)
- [installer-remote-control.md](installer-remote-control.md)

## 4. Executable Task IR Stability

Current weak point:

- Elixir can remain the fast authoring layer, but the executable structure that
  reaches agent engines must be language-neutral
- the TaskIR surface still needs more golden examples and compatibility gates

Current moxi hardening focus:

- keep TaskIR independent of UI, Phoenix, React, and Elixir-only runtime state
- make package fetch, readiness, dispatch, and result serialization visible in
  task previews

Qualification focus:

- freeze the first executable TaskIR compatibility surface
- add golden TaskIR examples for Rust-authored and Elixir-authored tasks
- add digest and replay checks for representative workflows

Moxi readiness standard:

- agent engines execute a stable task representation, not a private frontend or
  language-runtime convention.

Primary docs:

- [operator-task-ir-digest.md](operator-task-ir-digest.md)
- [workflow-graph.md](workflow-graph.md)
- [workflow-dataset.md](workflow-dataset.md)

## 5. Frontend And Runtime Consistency

Current weak point:

- the architecture says GUI, headless SDKs, agent, and orchestra should share
  backend capabilities, but experience parity is not fully proven
- Workbench still needs an obvious main workflow loop for serious users

Current moxi hardening focus:

- keep GUI actions, headless flows, and Installer preflight aligned around the
  same backend reports
- continue modular UI loading and layout safety work without hiding backend
  state behind UI-only behavior

Qualification focus:

- add one obvious Workbench path: prepare model, choose workflow, preflight,
  run, inspect, export, recover
- make mobile/WebView frontend constraints compatible with remote runtime use

Moxi readiness standard:

- the GUI is a first-class client of the same system, not a special runtime
  that secretly owns core behavior.

Primary docs:

- [app-runtime-boundaries.md](app-runtime-boundaries.md)
- [ui-architecture-migration.md](ui-architecture-migration.md)
- [mobile-gui-runtime-boundary.md](mobile-gui-runtime-boundary.md)

## 6. Security And Fuzz Coverage

Current weak point:

- security checks exist, but fuzz and hostile-input coverage should become more
  systematic around manifests, TaskIR, workflow datasets, credentials, and
  package loading

Current moxi hardening focus:

- keep dynamic library loading behind explicit host policy
- keep credential storage sandboxed and visible
- add more manifest and workflow malformed-input fixtures

Qualification focus:

- fuzz TaskIR, workflow dataset contracts, operator manifests, and package
  preflight parsing
- add red-line tests for path traversal, stale authority, invalid certificates,
  and unexpected runtime residue

Moxi readiness standard:

- common malformed or hostile inputs fail closed with useful diagnostics and no
  hidden residue burden.

Primary docs:

- [security.md](security.md)
- [architecture-red-lines.md](architecture-red-lines.md)
- [packaging-and-deployment.md](packaging-and-deployment.md)

## 7. Automated Material Research Loop

Current weak point:

- the research loop is real enough to be promising, but it still needs one
  flagship repeatable example that explains why Kyuubiki is different
- optimization metrics and reports need to feel like product primitives, not
  demo notes

Current moxi hardening focus:

- keep the heat-spreader example reproducible
- expand score contracts and feasibility explanations
- connect headless SDK output, evidence bundles, and report artifacts
- require explicit Headless executors and reserve `research` posture for the
  no-mock service path
- retain execution-authority evidence proving that local material exploration
  used real Rust solver kernels without fallback
- qualify generic Headless research rounds with contiguous input fingerprints,
  guarded patch lineage, complete service execution, and numeric domain metrics

Qualification focus:

- add a coupled multiphysics material exploration example
- include parameter sweep, optimization objectives, ranking, failure
  explanations, and exported report artifacts
- run the same example through CLI/headless and Workbench-facing paths

Current progress:

- retained material research bundles now reject missing, mock, or fallback
  execution authority across the initial run, next run, and every chained run
- generic non-material workflows now have a first-class round evidence contract;
  repeated batches and unrelated `n/a` report columns no longer qualify as
  research progress
- KCore now retains a complete generic research series rather than loose JSON
  files: export and verify both recheck every service report, metric digest,
  ancestry link, and replayable parameter patch before accepting the package
- the native `kcore research-export` path now converts an ordered minimal round
  list into that profile, removing hand-authored role and entrypoint wiring
- the composite thermo-electric panel bundle is still correctly classified as
  screening-only until external validation, failed quality gates, and
  low-confidence material cards are addressed
- a dedicated two-dimensional steady-current operator now feeds solved
  `sigma|E|^2` bulk loss into the temperature fixed point and regional heat-mesh
  checks; lumped contact resistance and finite-impedance terminals are now
  solver-supported, while geometric crowding baselines, validated interface
  parameters, and contact-to-heat mappings remain the electrical-physics gap

Moxi readiness standard:

- Kyuubiki can show one honest automated materials-research loop that is
  repeatable, inspectable, and useful even if still scoped.

Primary docs:

- [material-research-roadmap.md](material-research-roadmap.md)
- [automated-material-research-example.md](automated-material-research-example.md)
- [material-score-contract.md](material-score-contract.md)

The first recovery subtier is now executable: native workflow fault injection
proves branch isolation and fail-fast behavior. The Rust agent watchdog now also
proves failure-reason retention, execution-slot release, and a healthy follow-up
execution. Its stale-timeout sub-tier proves heartbeat refresh, timeout reason
retention, cooperative cancellation, late-result deduplication, and slot reuse.
Orchestra process-loss injection now proves post-dispatch Agent disconnects,
idempotent failover, duplicate-export prevention, and checkpoint-authorized
replay through retained evidence. Installer journal replay now has a v2 state
machine, digest-bound plan identity, atomic main/next/previous storage, and
retained process-loss evidence proving that completed steps are not replayed.
This still does not close distributed recovery: remote host kill/rejoin needs
retained fault-injection evidence on a managed physical deployment.

## Priority Order

The recommended order is:

1. deepen numerical trust beyond compact qualification fixtures
2. executable TaskIR stability and replay compatibility
3. operator SDK end-to-end package example
4. agent/orchestra/mesh recovery
5. automated material research flagship
6. security fuzz expansion
7. Workbench main-loop polish and product-visible limitations

Workbench polish matters, but it should not outrun the runtime and numerical
trust foundations.

Current nonlinear-structure progress includes sampled limit-point events and
bounded symmetric-tangent inertia diagnostics. Nonzero inertia changes outside
a limit-point neighborhood now produce explicit bifurcation-candidate brackets.
Transitions up to 128 reduced DOFs now retain a normalized critical mode for
branch construction. Candidate intervals can now be narrowed by configurable,
equilibrium-corrected inertia bisection. Opt-in positive and negative
critical-mode constraints now recover independently equilibrated branch seeds
without mutating the primary path. Both retained seed directions now support
isolated 64-point arc-length continuation with independent radius adaptation,
cutbacks, failure diagnostics, switched-path event fields, rigid-rotation
objectivity, and typed engine JSON transport. An explicit positive finite branch
radius now carries both retained directions through a physical load minimum and
onto the subsequent rising segment without contaminating the primary path. The
optional dimensionless minimum-radius ratio now bounds both adaptive shrinkage
and failed-step cutbacks relative to that visible nominal branch radius. The
same machinery now retains objective positive and negative paths on a branched
arch topology and contains a later non-distinct seed as a local failure. The
transition observer and engine route now also retain one to four ordered modes;
an exact repeated two-mode twin-arch subspace produces four mode-attributed
branch families. An explicit pairwise-combination option additionally probes
the normalized sum and difference of every retained mode pair, records the
component weights and solved projections, and continues all four added
twin-arch directions. A separate caller-weighted vector now selects an
arbitrary direction over two to four retained modes without combinatorial
growth; an exact three-mode repeated subspace retains both signed equilibria,
actual component projections, and continuation. A bounded automatic fan now
adds four deterministic projective directions for three modes and up to sixteen
for four modes, prioritizes full-dimensional combinations, and retains solved
component attribution through the engine route. One or two optional adaptive
layers now refine nearest changing-response boundaries with normalized,
projectively unique midpoints and a per-layer base-sample budget. Refinement is
now hierarchical: child intervals retain only changing endpoint responses and
halve their parent's projective angle. Probe origin, refinement level, and
parent angle remain visible through the engine route. An independent analytic
boundary at 37% of a 90-degree projective arc now proves eight levels of exact
angle halving while the bracket keeps containing the reference, separating the
refinement convergence claim from any one FE fixture. A ten-element Williams
toggle-frame path now correlates the first external snap-through limit event
against the published analytic load within 5%. A separate 8/16-element pinned
Euler column correlates a sampled bifurcation candidate and two signed,
continued branches against `pi^2 EI / L^2`; the finer switched seed moves
toward the analytic load. An exact repeated pair now recovers the analytic
double eigenvalue and uses orthogonal gauge columns to keep local, same, and
opposite pairwise branches distinct without freezing caller-weighted nonlinear
rotation. A midpoint-coupled pair now externally correlates the symmetric and
antisymmetric eigenvalue split and continues the first connected symmetric
branches. Its antisymmetric invariant path now also resolves the ordered first
and second inertia transitions and continues the secondary opposite-direction
branches against the raised Rayleigh load. A 0.5% right-column stiffness
perturbation now removes those symmetry invariant subspaces: an independent
two-coordinate Rayleigh reduction verifies both mixed eigenloads and
eigenvectors, and the lower and upper mixed single-mode branches continue with
bounded errors. The same case exposed and closed a false-positive boundary:
pairwise branch probes are now solved only inside a degenerate critical
eigenspace. Separated modes return an explicit local rejection instead of four
nominal combinations collapsing onto one physical branch. A three-column
complete midpoint-coupling graph now supplies the complementary positive case:
its uniform mode retains the Euler factor, its two graph-Laplacian modes share
the external `pi^2 EI / L^2 + 6 k L / pi^2` factor, and all eight individual
and pairwise probes in that connected mixed repeated subspace are distinct and
continue with bounded errors. A five-point parameter grid now perturbs the
third-column inertia and one coupling edge independently; all three factors
and the repeated-root split track a closed-form reduced 3-by-3 spectrum. This
completes the linear two-parameter spectral unfolding. Two combined-parameter
points on opposite sides first established external mixed-eigenvector
attribution, signed nonlinear branches, and bounded continuation. A five-point
semicircular path now tracks that identity around the repeated origin by
maximum neighboring mode overlap; every FE critical direction and branch seed
retains the corresponding attribution and bounded continuation. The test was
split into a dedicated triplet submodule before this expansion. Primary
arc-length paths now export a full reusable state contract, validate all DOFs
and constrained components on import, correct the displacement to equilibrium
under changed model parameters, preserve generalized branch orientation, and
return the next reusable state through the engine JSON route. A retained
three-stage regression seeds a selected nonlinear mixed branch on one side,
crosses the exact repeated point, and exits after eigenvalue ordering exchanges
without losing its physical mode identity. This state-seeding capability is
now backed by a typed parameter-path operator. It carries the last accepted
state across compatible models, retains failed attempts, and recursively
inserts interpolated midpoint models under bounded depth and minimum-fraction
controls. The operator is available through Engine and CLI RPC, and a forced
convergence-basin regression proves that failed large jumps recover through
visible quarter-point insertions. Fixed-load state correction now falls back
to a joint displacement/load hyperplane corrector. Optional state-tangent and
target-shape overlaps remain visible; an explicit target shape transports the
predictor only on active shape DOFs and is authoritative over a turning
tangent. A non-singular 3-by-3 serpentine surface now retains external mixed
mode alignment above 0.75 and both numerical error gates below `1e-7`; the
exact repeated point remains covered separately as a subspace crossing. The
lower asymmetric connected branch now also carries a 64-point trajectory from
`1.36e-4 L` to `1.75e-2 L` against an independent complete-elliptic-integral
two-column reduction. Its maximum load error is 2.326%, its minimum mixed-mode
alignment is 0.999989, and every FE point retains both `1e-7` gates. The next
equivalent-topology gate now replaces the direct coupler with two series
members and a free intermediate node, adds an unloaded spectator branch, and
retains a 64-point external trajectory to `1.72e-2 L` with the same 2.33% load
and 0.999989 direction bounds. This covers series/spectator isolation. A
four-column degree-three star now closes the interacting-topology gap: unequal
inertias and three unequal live couplers retain all-column participation over
64 externally seeded points to `1.83e-2 L`, with 2.321% maximum load error,
`4.46e-5` maximum external residual, and 0.999977 neighboring direction
overlap. This work also added a continuation-state identity guard so a
fixed-load Newton correction cannot silently replace a nontrivial imported
branch with the trivial equilibrium. The remaining stability depth is now
partially entered: `solve.frame_2d_material_p_delta` applies element-scoped
incremental bilinear axial return mapping with linear kinematic hardening
inside the corotational Newton assembly. A 16-element column tracks an external
pre/post-yield reference within `2e-7` for shortening, stress, signed plastic
strain, backstress, accumulated plastic strain, and tangent modulus. Trial
states are rollback-safe and only converged accepted substeps commit history.
An explicit cyclic load-factor schedule now exposes compression, unload,
reverse tension, unload, and reload histories at every requested point; a
five-point external bilinear reference verifies residual strain, backstress
reversal, and accumulated plasticity. Element materials now also accept an
initial axial stress that participates in the return mapping, internal force,
and material-geometric tangent. Opposite parallel residual forces retain an
explicit zero-load state, while imbalance on any free DOF and initial states
outside the yield surface are rejected before Newton iteration. The same
zero-state material tangent now defines the material operator's linear
eigen-buckling baseline: self-equilibrated compression lowers and tension
raises the retained critical factor, while a zero initial stress exactly
recovers the ordinary elastic result. The remaining material gates are
no longer blocked on a first fiber-section implementation: optional
section fibers now integrate independent material histories at two Gauss
stations, carry self-equilibrated residual stress, and produce coupled axial
force, end moments, and a full generalized consistent tangent. Discrete elastic
`EA`/`EI`, pure-axial bilinear response, partial axial-bending yield, residual
stress resultants, and the 6-by-6 element Jacobian are retained checks. The next
gate now includes a 4/8/16/32-fiber sequence that converges monotonically to
the analytic rectangular elastoplastic bending moment, plus a committed
axial-bending reversal path whose moment changes sign and whose accumulated
plasticity grows after reversal. Longitudinal integration is now explicitly
configurable at 2, 3, or 4 Gauss points; a nonuniform plastic-curvature field
approaches a 50,000-sample midpoint reference monotonically, with the four-point
error below half the two-point error. An opt-in bounded p-adaptive mode now
evaluates fixed-identity 2/3/4/8/12-point candidates, selects the lowest order
within the requested generalized-force tolerance, and retains all 29 candidate
histories per fiber so order changes cannot remap plastic state. Results expose
the active order, active and evaluated fiber-point counts, and retained error
estimate; elastic fields downshift to two points while a nonuniform plastic
front promotes to twelve. A first protocolized section library now expands
tagged rectangle, I-section, circular, hollow-box, and asymmetric T-section
dimensions into the same unified fiber execution IR while preserving exact
area, centroid, and second moment; explicit and library fibers are mutually
exclusive. A bounded `layered` form now covers custom nonoverlapping
piecewise-width and asymmetric profiles with deterministic sorting and exact
continuous section properties. A native `polygon` form now validates and
discretizes finite simple three-to-64-vertex rings, including concave profiles,
while preserving exact continuous area, centroid, and bending inertia in the
same fiber IR; self-intersection and degenerate geometry are rejected. The next
step now adds a bounded fiber-material catalog: explicit fibers and layered
regions resolve material IDs into numeric per-fiber elastic/plastic properties,
the zero-state composite tangent replaces the parent elastic buckling baseline,
and retained two-phase references cover transformed elastic stiffness plus
selective soft-phase yielding. This remains a perfectly bonded uniaxial model.
A reusable `self_equilibrated_quadratic` residual-stress template now projects
its generated field against the discrete constant and linear section modes,
then normalizes it to a caller-visible peak. The full solver retains zero
section axial force and bending moment, reports the initial fiber stress range,
rejects mixed explicit/template sources, and checks each generated stress
against its referenced phase yield surface. A first bounded phase-local damage
law now uses equivalent plastic strain to degrade nominal
fiber stress and its consistent tangent. Retained tests cover the active
damage tangent by central difference, frozen unloading, rollback-safe trials,
protocol visibility, and a two-phase section in which only soft-phase points
damage. A separate `solve.cohesive_interface_1d` operator now contributes a
history-dependent bilinear traction-separation contract with damaged unloading,
complete tensile failure, and compressive closure. A four-node zero-thickness
`solve.cohesive_interface_2d` kernel now adds independent displacement-jump
interpolation, rotated local/global tractions, directional tangents, and
two-point Gauss histories with a complete self-balanced `8 x 8` element
tangent. Retained rigid-motion, antisymmetric-jump, and central-difference
checks now guard its direct assemblability. A separate
`solve.cohesive_interface_mesh_2d` path now performs constrained multi-element
assembly, incremental Newton equilibrium, shared-node force accumulation, and
rollback-safe Gauss-point history commit. Proportional non-zero displacement
constraints now carry retained softening and complete-failure paths through
peak traction. Explicit per-step controls now add cyclic unload/reload and
non-proportional normal/shear paths with visible step summaries. Its retained
one- and two-element closed forms and singular-rigid-mode fixture move the
interface line beyond a single prescribed history. Optional linear component
connector springs now share the same global DOF assembly. A retained
connector-and-cohesive series reference verifies common force equilibrium,
displacement decomposition, and connector energy through solver, Agent RPC,
and workflow paths. Small-displacement linear 2D host trusses now reuse the
public truss element/result contract and contribute physical `EA/L` stiffness
to the same global system. A retained length-one series reference verifies
interface/axial-force balance, displacement decomposition, strain, stress, and
energy density across the same execution layers. Constant-strain plane-stress
triangles now add the first continuum host using the public plane-element
contract. A prescribed-apex series reference verifies analytic stiffness
partition, common force, strain, stress, and energy through Solver, Agent RPC,
and Engine Workflow. Fully integrated bilinear plane-stress quads now add a
second continuum host using the native Q4 `2 x 2` Gauss kernel. A rectangular
series reference independently verifies the same interface opening, extension,
force, stress, and energy through all three execution layers, while positive
Gauss-point Jacobians are enforced before assembly. Linear Euler-Bernoulli 2D
frames now add rotational host DOFs without changing existing translation
indices. A tip-loaded bending reference retains exact relative deflection,
rotation, root moment, stress, and energy through Solver, Agent RPC, and Engine
Workflow; invalid sections and orphan rotational data fail before iteration.
The same solver now performs sparse global tangent assembly and sparse
free-DOF projection through the shared `MatrixAssembler` contract. A retained
96-element model reports 3,072 nonzeros across 768 DOFs (`0.005208` fill) and
uses the observable symmetric-band Cholesky path. Softening or wide tangents
retain a bounded pivoted dense fallback. A separate
`solve.cohesive_interface_mesh_3d` path now closes the first six-node triangular
surface and tetrahedral solid-host co-assembly gate with a shared global
residual, three-direction history, rotated-coordinate closed form, and retained
1,440-DOF sparse regression. The next interface gates are shell-host
co-assembly, scalable sparse-indefinite factorization,
fill-reducing ordering, arc-length and adaptive-step continuation,
coupled mixed-mode/friction laws, experimental references, repeated cross-host
performance qualification, and larger localization-sensitive meshes. The
retained five-state independent
20,000-station cyclic reference promotes from order 2 to 12 and holds the
maximum generalized-force error to 0.218395%; the retained regression gate is
now 1%. A paired
120-element release benchmark now measures the adaptive path against fixed
two-point integration with identical response and 12 Newton iterations:
adaptive median time is conservatively 18.91% higher and peak RSS increases by
4.44 MiB in independent three-repeat processes on the retained Mac host. This
is now paired with a retained three-repeat Linux lab screen: fixed and adaptive
medians are 902.874 ms and 914.646 ms respectively, with identical response and
12 Newton iterations. The remote summary path rejects pair or response drift.
This establishes reproducible Mac/Linux screening; repeated multi-host
performance qualification and larger localization-sensitive cases remain open.
The first internal
complex-topology isolation reference also proves
single/multi-mode spectral consistency and fixed-load host-response invariance
after adding an unloaded free branch; the 128-mode and 256-inertia caps are
observability limits, not arc-length solver size limits.

## 2.0 Boundary Rule

If a capability cannot be made repeatable, inspectable, and honestly scoped
before `moxi 2.0.0`, it should ship as an experimental or deferred `2.x`
capability rather than weakening the first trust line.
