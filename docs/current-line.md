# daji 3.x

This is the active-status entrypoint, not an accumulated release diary.
The current development point in this line is `daji 3.7.1`.
Read the [HTML chapter](book-ch02-version-line.html) for a WebView-friendly
summary, or choose a task in the [documentation map](README.md).

## Version And Evidence Ledger

Reviewed October 24, 2026. Distinguish these identities before reproducing a run:

| Identity | Recorded state | Authority |
| --- | --- | --- |
| Active product line | `daji 3.x`, hardening profile `daji-3-7-hardening` | [Version policy](../config/version-line-policy.json), [brand](../assets/brand/brand.json) |
| Package and book baseline | `3.7.1` | [Book manifest](book-manifest.json), package manifests |
| Last recorded local macOS installation | `3.7.1` | [Release snapshot](../releases/snapshots/3.7.1.json), [installation report](../reports/daji-3.7.1-local-rebuild-20261024.md) |
| Latest version-named Git checkpoint | `091b1393`, subject `daji 3.7.3` | Git history; not an artifact version |
| Subsequent committed repair | `308eb93c`, isolated CI dependency/read repair | Git history; no new desktop build |
| Latest workflow follow-ups | Source working-tree changes | Dated reports below; not installed acceptance |

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

Follow the evidence rather than treating this table as a new combined run:

- [Harmonic assembly](../reports/harmonic-assembly-20261024.md),
  [inertia tails](../reports/harmonic-inertia-precision-20261024.md),
  [residual bounds](../reports/harmonic-residual-enclosure-20261024.md) and
  [many-term residuals](../reports/harmonic-residual-many-term-20261024.md).
- [Payload ownership](../reports/workflow-payload-ownership-20261024.md) and
  [cancellation boundaries](../reports/workflow-cancellation-boundaries-20261024.md).
- [Selected-input attribution](../reports/workflow-selected-input-20261024.md),
  [output retention](../reports/workflow-output-retention-20261024.md) and
  [execution receipts](../reports/workflow-execution-evidence-20261024.md).

The latest execution-receipt report records 3,475 web tests with zero failures
and 77 skips, plus 737 passed Rust Engine/selected integration tests and one
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
