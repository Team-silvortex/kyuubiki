# moxi 2.x Minor Lines

Use this document as the durable roadmap for the active `moxi 2.x` line.

`moxi 2.0.0` is the first formal Kyuubiki 2.x baseline after the
`tamamono 1.x` industrialization bridge. The old `tamamono` roadmap is kept as
historical context only; new current-line planning belongs here.

## Line posture

`moxi 2.x` should turn the broad 1.x prototype surface into a reliable research
and engineering baseline.

The line should prioritize:

- validated solver behavior over feature count
- explicit task and data contracts over ad hoc payloads
- reproducible material-research workflows over isolated demos
- installer, integrity, update, and credential behavior that is visible and
  recoverable
- GUI/runtime decoupling so headless SDKs can exercise the same backend
  capabilities
- benchmark, fuzz-smoke, and safety gates that can be expanded by module
  topology and coverage tensor

## Suggested minor grouping

### `2.0`

Primary goal: stabilize the handoff baseline.

Expected emphasis:

- finish moxi branding and current-line documentation alignment
- preserve the working operator, workflow, installer, and language-pack
  contracts inherited from `tamamono 1.20.x`
- keep weak solver families visible but honestly marked
- make the documentation book and module coverage tensor the normal entrypoint
  for planning

### `2.1` to `2.4`

Primary goal: improve calculation trust.

Expected emphasis:

- expand accuracy baselines across mechanical, thermal, electromagnetic, CFD,
  and coupled-field paths
- add cross-validation and local formal checks where the physics model is small
  enough to reason about
- increase large-node benchmark coverage without treating scale as a substitute
  for correctness

### `2.5` to `2.8`

Primary goal: harden distributed execution.

Expected emphasis:

- make agent, mesh, and orchestra behavior protocol-first
- keep centralized and decentralized modes equally documented
- improve watchdog, retry, recovery, and failure-reason reporting
- route physical-machine deployment through Installer-owned flows instead of
  ad hoc SSH procedures

### `2.9` to `2.12`

Primary goal: mature research workflows.

Expected emphasis:

- make material-study examples reproducible from headless SDKs
- strengthen workflow datasets and TaskIR handoff between Elixir descriptions
  and Rust engine execution
- make operator and workflow template stores self-hostable and integrity-checked
- improve result provenance, comparison, and export paths

### `2.13` to `2.16`

Primary goal: prepare broader ecosystem surfaces.

Expected emphasis:

- converge Rust worker SDK, Python headless SDK, and Elixir task description
  support around the same contract vocabulary
- keep frontend WASM Python DSL separate from headless automation SDKs
- support mobile GUI shells as control surfaces without implying mobile agents
  or local orchestra runtime
- move remaining script-like maintenance paths toward native, self-hosted tools

### `2.17` to `2.20`

Primary goal: close the daji 3.0 preparation gap.

Expected emphasis:

- make the minimum industrial loop repeatable enough for research partners
- turn the coverage tensor into a release-planning gate
- make component integrity a protocol for all new components
- make safety, benchmark, and documentation checks normal before ecosystem
  expansion

## Current rule

When in doubt, new `moxi 2.x` work should answer three questions:

1. Does it improve calculation trust, runtime safety, or workflow repeatability?
2. Is the contract visible to headless SDKs and not only to GUI code?
3. Can the coverage tensor, benchmark lane, or integrity checker observe it?
