# Commercial Readiness For 2.0

Use this page as the readiness gate for deciding whether Kyuubiki can honestly
ship a `2.0` early-commercial / research-partner line.

This is not a marketing checklist.

It is a trust checklist.

The paired machine-readable source is
[commercial-readiness-2.0.manifest.json](commercial-readiness-2.0.manifest.json).
Keep the checklist and manifest aligned with
`node ./scripts/validate-commercial-readiness.mjs`.

## Commercial Posture

`2.0` is allowed to be commercial, but it should be scoped as:

`credible early commercial trust line`

That means:

- selected early customers can run bounded real workflows
- research partners can evaluate programmable FEM automation seriously
- internal R&D teams can use headless workflows without treating the system as
  a toy
- limitations are documented instead of hidden

`2.0` should not claim mature parity with incumbent FEM suites.

`3.0` is the strategic line for that direct challenge.

## Release Gate

Before `2.0`, every item below should be classified as one of:

- `required`
- `acceptable limitation`
- `defer to 2.x`
- `blocker`

No `blocker` item should remain open at the `2.0` boundary freeze.

The manifest also tracks each gate's current `readiness_state`:

- `ready`: sufficient for the current `2.0` trust line
- `partial`: present but still needs closure work before boundary freeze
- `watch`: high-risk area that needs deliberate attention before `2.0`
- `blocked`: cannot ship as `2.0` until resolved

For the current `1.x` industrialization line, `partial` and `watch` are
acceptable planning states. `blocked` is not acceptable at the final `2.0`
freeze.

## 1. Numerical Trust

Required:

- core solver families have benchmark-backed baselines
- benchmark docs include reference values, tolerances, and automation status
- solver limitations are visible in docs and product-facing descriptions
- result drift is detectable across releases

Acceptable limitation:

- some advanced solver families can remain experimental if clearly labeled

Blocker:

- claiming industrial trust from demos or hand-picked screenshots alone

Evidence docs:

- [accuracy-plan.md](accuracy-plan.md)
- [accuracy-baselines.md](accuracy-baselines.md)
- [solver-matrix-optimization-pack.md](solver-matrix-optimization-pack.md)

## 2. Workflow And Asset Durability

Required:

- workflow graphs and dataset contracts are stable enough to export, import,
  replay, and explain
- results can be traced back to model inputs, material data, workflow version,
  operator version, runtime path, and agent context
- snapshots, reports, and result bundles have visible lifecycle and cleanup
  rules
- optimization metrics are first-class report contracts, not informal notes

Acceptable limitation:

- project persistence can still be file-first if the file contract is clear

Blocker:

- important result meaning depends on hidden UI state or untracked local state

Evidence docs:

- [workflow-graph.md](workflow-graph.md)
- [workflow-dataset.md](workflow-dataset.md)
- [headless-sdks.md](headless-sdks.md)

## 3. Headless SDK Credibility

Required:

- Rust headless SDK supports real workflow initialization, plan, run, and
  report paths
- material research examples include optimization profiles and candidate-level
  score explanations
- CLI and SDK surfaces produce machine-readable outputs suitable for AI agents
  and CI
- frontend wasm Python automation and backend headless SDK responsibilities are
  clearly separated

Acceptable limitation:

- Python and Elixir parity can lag behind Rust for advanced material research
  if the gap is documented

Blocker:

- headless paths are only demos and cannot run without frontend-only behavior

Evidence docs:

- [headless-sdks.md](headless-sdks.md)
- [headless-agent-contract.md](headless-agent-contract.md)

## 4. Installer, Update, And Disk Hygiene

Required:

- standard install paths are documented for supported platforms
- update source, download, staging, apply, rollback, and cleanup behavior are
  visible
- old residue cleanup is explicit and safe
- large build/download artifacts do not silently accumulate on developer or
  user machines

Acceptable limitation:

- some platform packagers can remain manual if the support level is clearly
  marked

Blocker:

- users cannot predict where files go or how to recover from a failed update

Evidence docs:

- [packaging-and-deployment.md](packaging-and-deployment.md)
- [desktop-release-checklist.md](desktop-release-checklist.md)
- [installer-remote-control.md](installer-remote-control.md)

## 5. Agent, Mesh, And Runtime Authority

Required:

- agent authority rules are explicit
- one agent cannot be ambiguously controlled by multiple orchestrators
- offline direct mesh and orchestrated control-plane modes are separated
- operator pull/cache/cleanup behavior is visible and auditable
- remote runtime actions are logged with enough context for diagnosis

Acceptable limitation:

- direct mesh can remain narrower than orchestrated mode if its boundary is
  honest

Blocker:

- runtime authority is ambiguous or hidden behind UI convenience

Evidence docs:

- [agent-orchestrator-boundary.md](agent-orchestrator-boundary.md)
- [agent-control-authority.md](agent-control-authority.md)
- [operator-library-centralization.md](operator-library-centralization.md)

## 6. Security And Audit

Required:

- certificate, token, and remote-control behavior is documented
- sensitive configuration is not committed into project data
- privileged installer and remote actions are audited
- user-visible configuration distinguishes editable and read-only settings

Acceptable limitation:

- enterprise policy integrations can wait for `2.x` if local trust behavior is
  solid

Blocker:

- secrets, certificates, or remote credentials are handled as ordinary project
  files

Evidence docs:

- [security.md](security.md)
- [installation-integrity-contract.html](installation-integrity-contract.html)
- [installer-remote-control.md](installer-remote-control.md)

## 7. Workbench And UX Stability

Required:

- core UI flows avoid overlapping or unusable layouts at supported resolutions
- workbench and SDK use the same concepts for workflows, materials, results,
  and reports
- UI automation selectors stay product-owned and stable
- error recovery paths are visible to users

Acceptable limitation:

- advanced rendering can remain staged if current visualization limits are
  clear

Blocker:

- successful workflows require fragile UI-only gestures that cannot be
  automated or recovered

Evidence docs:

- [frontend-style.md](frontend-style.md)
- [frontend-implementation.md](frontend-implementation.md)
- [ui-automation-contract.html](ui-automation-contract.html)
- [rendering-roadmap.html](rendering-roadmap.html)

## 8. Documentation As Product Infrastructure

Required:

- the central docs book is coherent enough for humans and large-model agents
- release posture, architecture boundaries, SDK paths, installer behavior, and
  troubleshooting are findable from the docs map
- limitations are documented near the relevant capability

Acceptable limitation:

- some desktop-facing HTML mirrors can lag briefly if repository docs remain
  source-of-truth

Blocker:

- critical operational knowledge exists only in chat history or local memory

Evidence docs:

- [README.md](README.md)
- [book.html](book.html)
- [navigation-matrix.html](navigation-matrix.html)
- [troubleshooting.md](troubleshooting.md)

## 2.0 Exit Statement

Kyuubiki can call `2.0` commercially ready only when this sentence is true:

`A bounded real FEM workflow can be installed, run, audited, explained,
updated, and repeated without relying on hidden state or private knowledge.`

If that is true, `2.0` can be a credible early commercial product.

If it is not true, the release should remain in the `1.x` industrialization
line.
