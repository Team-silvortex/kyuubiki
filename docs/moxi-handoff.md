# Moxi 2.0.0 Handoff

This document is the focused status baseline for the `moxi 2.0.0` line and the
next `moxi 2.x` hardening work.

The paired machine-readable source is
[moxi-handoff.manifest.json](moxi-handoff.manifest.json). Keep this document
and the manifest aligned with `make check-moxi-handoff`.

## Handoff Statement

`Moxi 2.0.0 starts from stable contracts; the next work should preserve that
baseline while hardening evidence, recovery, and user-facing trust.`

This is not a feature wishlist. It is the bridge from the first moxi baseline
into a more reliable `2.x` engineering line.

## Patch Posture

The moxi hardening line should prefer:

- contract freeze over new payload shapes
- retained evidence over anecdotal confidence
- explicit limitations over broad claims
- release identity clarity over scattered version drift
- operational recovery over hidden local heroics

The moxi hardening line should avoid:

- treating historical tamamono snapshots as active version truth
- adding new product claims before the coverage tensor and evidence reports can
  back them
- adding major UI, solver, or runtime surfaces without matching checks
- leaving important operational knowledge only in chat history

## Gate States

- `ready`: sufficient for the handoff boundary
- `active`: work is underway and should close before the patch is considered done
- `watch`: risk-bearing area that needs explicit evidence or a documented limit
- `defer_to_2x`: intentionally outside the immediate moxi baseline and tracked
  as later `2.x` work

## Handoff Gates

### 1. Release Identity

Question:
Can a human or tool tell that the repository is now on the moxi 2.0.0 line?

Must close:

- keep moxi 2.0.0 release identity aligned across docs, catalogs, app metadata, and version checks
- keep historical tamamono snapshots as provenance instead of active version truth
- make future patch bumps flow through the same version-line audit

Evidence:

- [version-line.md](version-line.md)
- [current-line.md](current-line.md)
- [release-prep-1.9-to-1.20.md](release-prep-1.9-to-1.20.md)

### 2. Contract Freeze

Question:
Are workflow graphs, datasets, TaskIR, operator descriptors, and material study
bundles stable enough to preserve during moxi hardening?

Must close:

- turn remaining shared payload shortcuts into documented contracts or explicit deferred work before they spread
- keep SDK, agent, and Workbench meanings aligned for workflow and operator data
- ensure moxi-facing task and bundle examples are machine-checkable

Evidence:

- [workflow-graph.md](workflow-graph.md)
- [workflow-dataset.md](workflow-dataset.md)
- [operator-task-ir-digest.md](operator-task-ir-digest.md)
- [material-score-contract.md](material-score-contract.md)

### 3. Research Loop Evidence

Question:
Can the flagship automated material research loop be reproduced and audited
without private context?

Must close:

- retain machine-readable material research bundles for at least one bounded study
- keep optimization metrics, score explanations, and next-step planning visible
- separate Rust-led headless proof from Python and Elixir parity gaps

Evidence:

- [automated-material-research-example.md](automated-material-research-example.md)
- [material-research-roadmap.md](material-research-roadmap.md)
- [headless-sdks.md](headless-sdks.md)

### 4. Runtime Authority

Question:
Can agent, orchestra, direct mesh, and installer-controlled remote modes be
distinguished without UI-only assumptions?

Must close:

- keep one-orchestrator authority and offline mesh mode documented as product rules
- make installer remote deployment and runtime-control boundaries explicit
- keep GUI shells decoupled from runtime ownership so mobile WebView control remains plausible

Evidence:

- [agent-control-authority.md](agent-control-authority.md)
- [agent-orchestrator-boundary.md](agent-orchestrator-boundary.md)
- [installer-remote-control.md](installer-remote-control.md)
- [mobile-gui-runtime-boundary.md](mobile-gui-runtime-boundary.md)

### 5. Qualification And Benchmark

Question:
Are numerical claims bounded by retained evidence instead of broad ambition?

Must close:

- keep review-level physics coverage separate from qualification claims
- retain release evidence for qualification candidates before promotion
- keep 500k and 1m benchmark evidence labeled as exploratory unless promoted into scheduled gates

Evidence:

- [accuracy-plan.md](accuracy-plan.md)
- [accuracy-baselines.md](accuracy-baselines.md)
- [operator-reliability.md](operator-reliability.md)
- [testing-and-ci.md](testing-and-ci.md)

### 6. Install Update Security

Question:
Can installation, update, cleanup, credential, and remote artifact behavior be
audited during moxi hardening?

Must close:

- keep standard paths and cleanup rules visible
- avoid committing local server credentials or machine-specific config
- keep privileged deployment and update actions covered by audit-friendly reports

Evidence:

- [packaging-and-deployment.md](packaging-and-deployment.md)
- [desktop-release-checklist.md](desktop-release-checklist.md)
- [security.md](security.md)
- [installation-integrity-contract.html](installation-integrity-contract.html)

### 7. Documentation Entrypoint

Question:
Can a teammate or model enter the project and understand what moxi 2.0.0 can do
and what remains for 2.x?

Must close:

- keep docs book, navigation matrix, current line, commercial readiness, and this handoff note linked
- keep limitations close to the feature surfaces that expose them
- make moxi status explain what is closed, deferred, or intentionally scoped as 2.x hardening

Evidence:

- [README.md](README.md)
- [book.html](book.html)
- [navigation-matrix.html](navigation-matrix.html)
- [commercial-readiness-2.0.md](commercial-readiness-2.0.md)

## Relationship To Commercial Readiness

[commercial-readiness-2.0.md](commercial-readiness-2.0.md) answers whether
`moxi 2.0.0` can honestly ship as a credible early commercial trust line.

This handoff document answers the narrower moxi-baseline question:

`What must stay stable after moxi 2.0.0 so 2.x hardening can proceed without
reintroducing inherited ambiguity?`

If a task does not map to one of these gates, it should be deferred, retired,
or moved into an explicit `2.x` backlog rather than squeezed into the current
hardening pass.
