# daji 3.x

This is the active-status entrypoint, not an accumulated release diary.
The current development point in this line is `daji 3.7.1`.
Read the [HTML chapter](book-ch02-version-line.html) for a WebView-friendly
summary, or choose a task in the [documentation map](README.md).

## Version And Evidence Ledger

Reviewed October 25, 2026. Distinguish these identities before reproducing a run:

| Identity | Recorded state | Authority |
| --- | --- | --- |
| Active product line | `daji 3.x`, hardening profile `daji-3-7-hardening` | [Version policy](../config/version-line-policy.json), [brand](../assets/brand/brand.json) |
| Package and book baseline | `3.7.1` | [Book manifest](book-manifest.json), package manifests |
| Last recorded local macOS installation | `3.7.1` | [Release snapshot](../releases/snapshots/3.7.1.json), [installation report](../reports/daji-3.7.1-local-rebuild-20261024.md) |
| Latest version-named Git checkpoint | `305e3009`, subject `daji 3.7.3` | Git history; not an artifact version |
| Earlier committed repair | `308eb93c`, isolated CI dependency/read repair | Git history; no new desktop build |
| Latest committed workflow repair | `e347cf8c`, skipped-node execution evidence | Git history; no new desktop build |
| Latest workflow follow-up | Summary admission, comparison and reduction range | Dated reports below; not installed acceptance |

A commit subject does not update package metadata or installed files. The last
recorded installation is not a live health check. Local builds do not publish
download artifacts or SDK packages, and protocol/schema versions do not follow
product SemVer mechanically.

## Current Source Progress

The mainline is correctness, bounded resource use and safe recovery for an
agent-driven research journey. Keep numerical algorithms in Solver and task
execution/publication in Engine and Agent; the GUI is not runtime authority.

| Area | Retained change | Scope |
| --- | --- | --- |
| Harmonic assembly and arithmetic | Preserve coefficient/product tails, original-equation checks and checked complex ranges | Selected source kernels and manufactured/physical fixtures, not general physical qualification |
| Workflow payload ownership | Move owned inputs and final-port values without redundant copies | In-process ownership; required independent retained copies remain |
| Workflow cancellation | Poll admission, node and completion boundaries even under skip policy | Cooperative local cancellation, not preemption of opaque callbacks |
| Selected-input attribution | Name the actual fallback source without weakening readiness or named-input bindings | Fresh Rust execution traces and direct lineage |
| Requested output retention | Keep requested published outputs despite optional ports, cache hints or later consumers | Local result retention, not a durable checkpoint |
| Execution receipts | Align Orchestra branch/failed-fallback traces with Rust and actual publications | Shared fixture, local API routing and SQLite result readback |
| Skipped-node receipts | Never claim consumption for nodes that did not execute; keep dependency retirement separate | Shared branch/failure fixture, retained raw outputs and local SQLite readback |
| Named-source integrity | Preserve sibling metrics and failure markers instead of selecting a same-named nested field | Shared Rust/Orchestra fixtures, legacy wrapper controls and local SQLite readback |
| Summary-result admission | Reject failed final status before reduction or scoring; never count collection status as a candidate | Six shared Rust/Orchestra operators, raw-result retention, local API jobs and SQLite readback |
| Summary comparison range | Skip disabled operations, handle zero baselines and reject unrepresentable requested metrics | Shared range fixtures, bounded rescaling, graph isolation and local SQLite readback |
| Summary reduction range | Preserve bounded aggregate tails and finite means; validate normalization before clamping and scores before ranking | Shared three-operator fixtures, malformed options, graph isolation and local SQLite readback |
| Official SDK durable modal research | Direct native Agent, four independently checked heterogeneous modes, disk export and original-identity recovery after process kill | Small local macOS source-built example; not installed acceptance or general modal qualification |
| SDK lost-admission recovery | Retain typed input intent before dispatch; reconcile the original receipt after a lost reply without resubmission | Real local Agent relay faults and process kills; failed/unknown/missing or mismatched records cannot publish results |
| Direct SDK RPC boundaries | Unique per-process request sequence, strict reply identity/state checks and configurable byte/time budgets | Local protocol fault tests plus real Agent progress/results; not authentication or cancellation authority |
| SDK failure-context recovery | Validate inner terminal receipts and retain original failure identity, code and diagnostics after lost replies or timeout | Real local Agent timeout and restart; diagnostics do not authorize replay or result publication |
| Research export publication | Independently validate before staging and after disk readback; publish result and verification together | Local single-writer example with forced export-process termination; pending files are not results or power-loss evidence |
| Retained research readback | Offline, read-only input/intent/receipt/result binding and independent physical revalidation | Agent stopped during checks; mixed, damaged and pending-only evidence rejected without repair or resubmission |
| Research evidence read boundary | Bounded regular-file reads, unique JSON object keys and truly absent-only optional admission | Real-export negative controls plus exact-limit/parser tests; example-local policy, not a global model-size cap |
| Retained terminal consistency | Compare original identity and known terminal payload across observations, not just matching handles | SDK history-aware fetch plus online/offline example conflicts; caller-retained history, not automatic replay or a global cache |
| Retained result content | Bind separately decoded result values to the original completed receipt with typed numeric comparison | SDK gate plus stopped-Agent readback; file hashes, result consistency and physical checks remain separate |
| Modal research result metadata | Recheck echoed input, angular frequencies, declared shape norms and frequency extrema | Shared research reference blocks inconsistent publication/readback; no new Solver qualification or generic SDK physics policy |
| Recovery local preflight | Reject invalid retained content and unusable output destinations before remote observation | SDK depth gate and example admission/path checks; local connection probes, no automatic replay or filesystem reservation |
| Research record publication | Stage and synchronize complete records before no-replace final-name publication | Forced writer termination before/after dispatch, original-task recovery and unchanged pending evidence; not host-power-loss durability |
| Offline interrupted-export recovery | Revalidate a retained completed receipt and regenerate derived results into a new directory without an Agent | No replay, staging promotion or source changes; existing published evidence must be verified, not bypassed |
| Durable store readiness | Typed SDK descriptor states gate new research submission before local records or dispatch | Busy/unavailable is not missing-task evidence; readiness is a snapshot, not a reservation or recovery prerequisite |
| Durable admission refusal | Preserve capacity reason and bind explicit non-admission to the actual submission attempt | Real full-store refusal, lost reply and process restart; no execution handle, historical absence proof or automatic replay |
| RPC payload ambiguity | Reject nested duplicate keys before constructing retained result, failure-detail or progress values | Raw-message fixtures and real modal receipt recovery; exact numeric representation retained, no new execution/retry authority |
| Missing-execution boundaries | Reject contradictory absence reports; preserve native missing/corrupt-record errors before export | Real orphan restart and exact-record restoration; interrupted admission remains unknown, no automatic repair or historical absence guarantee |
| Lost terminal receipts | Retained completed/failed history rejects a missing native receipt after live loss and Agent restart | Intact prior exports remain offline-verifiable; diagnostic unknown receipts are not verified results, and exact restoration is explicit |
| Native store write failures | Real admission/receipt pending-file collisions block new durable work while retaining prior result reads | Explicit restart discards pending files without completing or replaying uncertain work; not disk-full or directory-sync fault qualification |
| Native record ambiguity | Reject duplicate decoded JSON keys in checksummed Agent records before typed decoding or publication | Real result/error corruption and restart controls; checksums are integrity checks, not authentication, and exact original restoration remains explicit |
| SDK recovery index | Read one bounded, store-pinned admission page with ordered unique IDs and validated continuation | Mixed completed/failed/interrupted native work survives process restart; listings are not result validation, atomic snapshots or replay authority; restart after filesystem repair to rescan capacity |
| Durable inventory admission fence | Check finalized record names, regular-file types and encoded lengths before reserving fresh execution; latch mismatches as admission-blocked | Bounded metadata only, not content verification or concurrent-writer protection; intact reads and healthy in-flight completion remain available; explicit inspection/restart rescans counts |

Follow the evidence rather than treating this table as a new combined run:

- [Harmonic assembly](../reports/harmonic-assembly-20261024.md),
  [inertia tails](../reports/harmonic-inertia-precision-20261024.md),
  [residual bounds](../reports/harmonic-residual-enclosure-20261024.md) and
  [many-term residuals](../reports/harmonic-residual-many-term-20261024.md).
- [Payload ownership](../reports/workflow-payload-ownership-20261024.md) and
  [cancellation boundaries](../reports/workflow-cancellation-boundaries-20261024.md).
- [Selected-input attribution](../reports/workflow-selected-input-20261024.md),
  [output retention](../reports/workflow-output-retention-20261024.md) and
  [execution receipts](../reports/workflow-execution-evidence-20261024.md) and
  [skipped-node receipts](../reports/workflow-skipped-input-evidence-20261024.md).
- [Named-source integrity](../reports/workflow-named-source-integrity-20261025.md) and
  [summary-result admission](../reports/workflow-summary-admission-20261025.md).
- [Summary comparison range](../reports/workflow-summary-comparison-range-20261025.md) and
  [summary reduction range](../reports/workflow-summary-reduction-range-20261025.md).
- [Official Rust SDK durable modal research](../reports/headless-durable-modal-research-20261025.md)
  connects the public SDK, real Agent, independent physical readback and process-kill
  recovery. Its bounded scope does not close the installed cross-language or
  heterogeneous long-bending qualification requirements.
- [Lost-admission recovery and input binding](../reports/headless-durable-admission-recovery-20261025.md)
  adds real acknowledgement loss, pre-dispatch loss and changed-input negative
  controls to that same bounded SDK journey, not a new numerical qualification.
- [Direct SDK RPC boundaries](../reports/headless-rpc-transport-boundaries-20261025.md)
  covers request-ID collisions, contradictory/foreign replies, bounded framing
  and deadline enforcement without changing solver algorithms or tolerances.
- [SDK failure-context recovery](../reports/headless-failure-context-recovery-20261025.md)
  adds strict inner terminal receipts and original solver error readback after
  timeout and process restart. The fresh SDK suite has 148 passes; its 11 opt-in
  real-Agent tests also passed separately, without claiming installed acceptance.
- [Research export publication](../reports/headless-modal-export-publication-20261025.md)
  rejects invalid physical results and staged corruption, preserves old exports,
  and recovers after an export-process kill without rerunning the solver. Its
  fresh SDK run has 149 passes and separately executes all 14 opt-in live tests.
- [Offline retained modal readback](../reports/headless-modal-offline-readback-20261025.md)
  makes export consumption executable, not only a documentation requirement.
  It checks saved identity and bytes, recomputes physical diagnostics and never
  modifies the retained files. It also fixes and deterministically tests the
  fault relay's accepted-socket blocking mode; no Solver algorithm is changed.
- [Bounded modal evidence reader](../reports/headless-modal-evidence-boundaries-20261025.md)
  fixes dangling admission links being mistaken for missing records and rejects
  oversized or ambiguous evidence. This run has 156 SDK passes and separately
  executes 18 opt-in live tests on macOS; earlier counts remain dated evidence.
- [Retained terminal observation consistency](../reports/headless-terminal-observation-consistency-20261025.md)
  rejects conflicting success/failure records and replaced terminal payloads,
  including physically admissible but different mode shapes. Its SDK run has
  162 passes and separately executes all 19 live tests on macOS.
- [Retained result-content binding](../reports/headless-result-content-binding-20261025.md)
  fixes signed-zero substitutions escaping ordinary JSON equality. Rehashed
  sign, low-bit and numeric-type changes fail despite satisfying the physical
  reference; equivalent formatting remains valid. This source run has 167 SDK
  passes and separately executes all 20 opt-in live tests on macOS.
- [Modal result metadata validation](../reports/headless-modal-metadata-validation-20261025.md)
  closes unchecked reported fields rather than changing Solver numerics. Both
  publication and stopped-Agent readback reject 38 inconsistent result copies,
  including coherently rehashed records. Full-suite results are retained in
  that report, not added to earlier overlapping counts.
- [Recovery local preflight](../reports/headless-modal-local-preflight-20261025.md)
  makes known local record/path failures precede network access and export
  creation. Connection probes verify zero calls on local rejection; genuine
  missing-admission recovery and later remote-conflict detection remain intact.
- [Research record publication](../reports/headless-modal-record-publication-20261025.md)
  closes the partial-admission write window. Forced process termination before
  dispatch never creates/replays work; after dispatch, fresh inspection recovers
  the original task without promoting partial or complete pending records.
- [Offline interrupted-export recovery](../reports/headless-modal-offline-export-recovery-20261025.md)
  continues after a killed exporter while the Agent remains stopped. It reads
  only complete retained receipts, rechecks physics and publishes into a new
  directory; negative controls prevent bypassing published or conflicting evidence.
- [Durable store readiness](../reports/headless-durable-store-readiness-20261025.md)
  distinguishes descriptor busy/unavailable/disabled/admission-blocked states
  before new submission. Real lock contention preserves the original task and
  recovery error context; a known ID cannot override a broken store binding.
- [Durable admission refusal](../reports/headless-durable-admission-rejection-20261025.md)
  preserves the native capacity decision instead of misreporting a receipt schema
  error. Lost refusal replies stay uncertain; old receipts remain readable and
  explicit duplicate observation does not allocate new execution records.
- [RPC payload ambiguity](../reports/headless-rpc-json-ambiguity-20261025.md)
  rejects repeated nested keys before they can erase contradictory execution
  flags or result values. Numerical representation controls and an original
  real-Agent receipt read after process restart preserve the existing contract.
- [Missing-execution boundaries](../reports/headless-missing-execution-boundaries-20261025.md)
  prevents `not_found` from also claiming admitted execution or resumability.
  Real missing admissions, orphan receipts and checksum faults block export;
  restoring original evidence recovers only the state it actually supports.

The latest summary reduction report records 3,544 web tests with zero failures
and 77 skips, plus 758 passed Rust Engine/selected integration tests and one
ignored test. These are that report's source checks, not coverage percentages,
a new Solver-wide campaign, remote CI results or installed acceptance. Overlapping
earlier report counts must not be added.

## Current Acceptance Queue

The latest local tensor check covers 13 modules and 11 paradigms: zero
structural gaps, two maturity gaps, 23 evidence-grade gaps and 18 P0 gaps.
Its Daji status remains `blocked`; passing the structural checker is not release
qualification. Generate the current queue instead of treating this dated
summary as a permanent ranking:

```sh
make check-module-function-coverage-tensor
make check-test-coverage-posture
```

Read [the weakness roadmap](weakness-roadmap.md) for priorities and
[testing and CI](testing-and-ci.md) for commands and evidence interpretation.
Open acceptance includes external numerical correlation, installed desktop and
SDK journeys, authenticated multi-host Mesh/load recovery, and durable/typed
continuation under the required platform/backend policy. Local trace consistency
does not close those coordinates.

## History And Earlier Links

Historical prose is kept once, in the files below. Existing checkpoint headings
remain here so older links still lead to the matching record.

## Daji 3.7.1 Local Packaging

Historical scope: [open the retained record](daji-3.6-to-3.7-history.md#daji-371-local-packaging).

### Harmonic Assembly Source Follow-up

Historical scope: [open the retained record](daji-3.6-to-3.7-history.md#harmonic-assembly-source-follow-up).

### Harmonic Inertia Source Follow-up

Historical scope: [open the retained record](daji-3.6-to-3.7-history.md#harmonic-inertia-source-follow-up).

### Harmonic Residual Source Follow-up

Historical scope: [open the retained record](daji-3.6-to-3.7-history.md#harmonic-residual-source-follow-up).

### Workflow Source Follow-ups

Historical scope: [open the retained record](daji-3.6-to-3.7-history.md#workflow-source-follow-ups).

## Daji 3.7.0 Checkpoint

Historical scope: [open the retained record](daji-3.6-to-3.7-history.md#daji-370-checkpoint).

## Daji 3.6.0 Checkpoint

Historical scope: [open the retained record](daji-3.6-to-3.7-history.md#daji-360-checkpoint).

## Daji 3.5.0 Checkpoint

Historical scope: [open the retained record](daji-3.5-history.md#daji-350-checkpoint).

## Daji 3.4.7 Checkpoint

Historical scope: [open the retained record](daji-3.3-to-3.4-history.md#daji-347-checkpoint).

## Daji 3.4.5 Checkpoint

Historical scope: [open the retained record](daji-3.3-to-3.4-history.md#daji-345-checkpoint).

## Daji 3.4.4 Checkpoint (Historical)

Historical scope: [open the retained record](daji-3.3-to-3.4-history.md#daji-344-checkpoint-historical).

## Daji 3.4.3 Checkpoint (Historical)

Historical scope: [open the retained record](daji-3.3-to-3.4-history.md#daji-343-checkpoint-historical).

## Daji 3.4.2 Checkpoint (Historical)

Historical scope: [open the retained record](daji-3.3-to-3.4-history.md#daji-342-checkpoint-historical).

## Daji 3.4.1 Checkpoint (Historical)

Historical scope: [open the retained record](daji-3.3-to-3.4-history.md#daji-341-checkpoint-historical).

## Daji 3.4.0 Checkpoint (Historical)

Historical scope: [open the retained record](daji-3.3-to-3.4-history.md#daji-340-checkpoint-historical).

## Daji 3.3.0 Checkpoint (Historical)

Historical scope: [open the retained record](daji-3.3-to-3.4-history.md#daji-330-checkpoint-historical).

## What Changed

Daji succeeds the moxi 2.x development line. Version 3.0.0 aligns the product
brand, three independent desktop shells, Orchestra, Rust Agent and Engine,
Worker SDK, official Rust/Python/Elixir Headless SDKs, language packs,
installation contracts, update channels, and documentation.

This is a product-line transition, not a wholesale protocol version bump.
Existing TaskIR, workflow, dataset, material, and KCore schema identifiers stay
unchanged unless their contracts actually change.

## Early Daji Mainline

Early Daji is a hardening phase toward an agent-driven industrial research
system, not another broad feature-expansion phase. Industrial reliability is
the acceptance goal, not a status granted by the version number.

Research agents, including external AI callers, use the official Headless SDKs
to discover capabilities, propose bounded studies, submit work, observe results,
and prepare subsequent rounds. Rust Agents are separate execution processes:
they admit language-neutral tasks and run operators through their engines.
Neither role replaces the other, and neither may bypass caller-owned approval,
resource limits, or numerical quality gates.

The primary acceptance journey is:

1. Declare a research objective, constraints, metrics, and stopping budget.
2. Discover the installed runtime and validate a reproducible workflow.
3. Authorize and execute real operator tasks without a GUI prerequisite.
4. Observe progress and diagnose, cancel, or resume interrupted work safely.
5. Validate results before ranking candidates or admitting another round.
6. Export the research lineage and evidence for replay and human review.

Workbench remains first-class for modeling, inspection, intervention, and
review of the same backend state. PWDT automates the fixed GUI; it is not a
mandatory bridge for Headless SDK users. Orchestra and explicit direct/mesh
control paths keep their distinct, equally supported authority boundaries.

Prioritize blockers in this complete journey over isolated feature counts.
The [weakness roadmap](weakness-roadmap.md#priority-order) maps that work to
existing tensor coordinates and retained evidence. New physics or abstractions
should be added when they remove a demonstrated blocker, not to widen the
catalog alone.

## What Carries Forward

- Contract-driven mechanical, thermal, electromagnetic, acoustic, modal,
  transport, and bounded flow studies with explicitly different maturity.
- Serial and parallel operator workflows, typed datasets, provenance, and
  path-independent `.kcore` research exchange.
- Decoupled GUI, Orchestra, direct Agent, and offline mesh control paths.
- Rust-only Worker extensions, Rust/Python/Elixir Headless control SDKs, and
  frontend-owned PWDT automation as three distinct extension surfaces.
- Persisted research outcomes, bounded recovery, integrity-checked runtime
  installation, and native operational tooling.
- Thirty-locale local language packs shared by the product-owned UI surfaces.
- Retained scale, numerical, recovery, and usability evidence with original
  platform, version, and execution scope.

The detailed [moxi closeout](moxi-closeout.md) remains a historical record;
old evidence is not relabeled as a new Daji run.

## Readiness Boundary

A 3.0.0 version number does not certify every solver or every platform.
The coverage tensor and usability release gate remain authoritative.
Open external numerical correlation, installed-platform, recovery, and
GUI/PWDT parity coordinates must close through new retained evidence.

The planned public channel remains Reddit. This source transition does not
publish packages, create a signed tag, notarize desktop applications, or
authorize an automatic rollout. Download and apply still require the
configured source, artifact integrity, and explicit Installer policy.

## Version Cadence

The Daji line spans `3.0.0` through `3.20.9`: 21 minor positions and 10
patch positions per minor. No subsequent codename is declared.
The archived moxi stabilization window is not an active Daji restriction.

## Reading Path

- [Daji 3.0.0 release notes](daji-3.0.0.md)
- [HTML book](book.html)
- [Formal version policy](version-line.md)
- [Current architecture](current-architecture-map.md)
- [Weakness roadmap](weakness-roadmap.md)
- [Operator reliability](operator-reliability.md)
- [Headless SDKs](headless-sdks.md)
- [Worker / Operator SDK](operator-sdk.md)
- [Packaging and deployment](packaging-and-deployment.md)
- [Minimal industrial closure](minimal-industrial-closure.md)
