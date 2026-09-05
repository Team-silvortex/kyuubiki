# daji 3.x

This is the single entrypoint for the active Kyuubiki product line.
The current development point in this line is `daji 3.0.0`.

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
