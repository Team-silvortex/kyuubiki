# moxi 2.x

This is the single entrypoint for the current Kyuubiki product line.

Use it when you want the shortest answer to:

- what `moxi 2.x` means
- what `moxi 2.0.0` inherits from the `tamamono 1.x` bridge
- where to go next inside the current documentation set

## What Defines This Line

`moxi 2.x` is the point where Kyuubiki treats its core contracts as product
baseline rather than loose prototypes.

The current development point in this line is `moxi 2.9.0`.

The 2.x line optimizes for:

- numerical trust over demo breadth
- repeatable validation over anecdotal confidence
- explicit workflow and task contracts over ad hoc payloads
- runtime recovery over cascading failure
- visible installer/update/integrity behavior over hidden residue
- GUI/runtime decoupling so headless SDKs can exercise the same capabilities

## What Carries Forward

`moxi 2.0.0` carries forward the final `tamamono 1.20.x` closeout work:

- versioned desktop shells and release metadata
- operator reliability shards and qualification evidence
- TaskIR, workflow graph, workflow dataset, and material study schemas
- installer integrity, update, cleanup, and remote deployment contracts
- language pack catalog and product-owned UI automation selectors
- benchmark and fuzz-smoke entrypoints
- documentation inventory, book manifest, and module coverage tensor

The old `tamamono` documents are still useful as historical preparation
records. They are no longer the active version line.

## What Not To Expect

`moxi 2.0.0` does not mean every solver is equally mature.

The current rule is:

- keep broad physics coverage visible
- mark weak operators honestly
- improve reliability evidence before widening claims
- keep GUI convenience separate from runtime authority
- keep agent/orchestra/mesh behavior protocol-driven

## Current 2.9 Checkpoint

`moxi 2.9.0` treats the next trust jump as product usability and automation
closure rather than raw feature sprawl:

- GUI navigation and backend calls must form complete, testable user journeys
  instead of isolated panels that look finished but do not execute
- the native `check-desktop-usability-journeys` probe now guards those journeys
  against slipping back to Node integration shims; `make
  build-usability-readiness-report` executes the 8 blocking paths and records
  the current `baseline_pass` evidence
- `create-open-project` now executes a real native bundle round trip. Hub and
  `kyuubiki project create|inspect|validate|normalize|pack|unpack|diff` share
  `workers/rust/crates/project-bundle` instead of maintaining separate storage
  implementations
- `kyuubiki project automation-presets|automation-render|automation-run` now
  flows through `workers/rust/crates/project-automation` and the Rust headless
  SDK. Dry runs are side-effect free, live execution is service-only, and an
  unconfirmed destructive step halts the remaining plan
- `kyuubiki macro actions|inspect|validate|normalize|render|run` now shares that
  native protocol instead of invoking the frontend Node CLI. Static macro
  checks preserve payload/state templates, while render/run resolve them into
  the executable plan
- `kyuubiki headless templates|suggest|init|inspect|validate|render|plan|run`
  now dispatches to the official Rust binary. Its native contract covers the
  complete template-to-plan-to-run journey, while research posture rejects
  mock and hybrid executors instead of silently downgrading execution authority
- native service execution now covers every action declared with the service
  engine, including direct mesh and saved-model-version solve/wait/result
  chains; the executor matrix is guarded against drifting from that contract
- direct mesh execution resolves inline payloads, saved models, and saved model
  versions without a frontend bridge, and stable model-version/endpoint fields
  remain visible in composite solve results
- the former frontend Node project/macro/headless CLI graph has been removed;
  npm compatibility commands and Headless CI names now dispatch to Rust, while
  the native script audit rejects new non-UI Node scripts under the frontend
- full Workbench language coverage, translation planning, batch export, and
  reviewed batch application now run through Rust; Make/CI cannot directly
  restore a `node scripts/*.mjs` operational path, and the obsolete network
  machine-translation chain has been removed
- Pwdt should become the deterministic frontend automation surface for Hub,
  Workbench, and Installer, while headless SDKs remain backend/control clients
- the module-function coverage tensor is the shared map for deciding which
  weak coordinate to harden next
- component integrity, installation/update visibility, language packs, and
  documentation entrypoints must keep moving with the active line
- protocol and checker code should stay split into bounded modules so new
  contracts can be extended without recreating large-file debt

The 2.7 cohesive-interface coassembly checkpoint is still retained as a
calculation baseline. The 2.9 boundary is also explicit: Pwdt parity is not yet
complete, app shipping metadata is separate from this development checkpoint,
and broader industrial solver qualification still needs deeper retained
fixtures, external correlation, larger-scale evidence, and packaged GUI/Pwdt
round trips beyond the current native project-bundle and automation probes.

## Current Reading Path

1. [version-line.md](version-line.md)
   Formal version-line note, codename, and major-version policy.
2. [commercial-readiness-2.0.md](commercial-readiness-2.0.md)
   Trust-gate checklist for deciding whether the line is credible as an early
   commercial product.
3. [minimal-industrial-closure.md](minimal-industrial-closure.md)
   Minimum industrial loop for research, validation, recovery, and packaging.
4. [weakness-roadmap.md](weakness-roadmap.md)
   Current weak-spot roadmap for the active `moxi 2.x` trust boundary.
5. [accuracy-plan.md](accuracy-plan.md)
   Accuracy roadmap, benchmark targets, and verification priorities.
6. [material-research-roadmap.md](material-research-roadmap.md)
   Reliability roadmap for turning material studies from runnable prototypes
   into reproducible screening, review, and qualification-oriented workflows.
7. [physics-coverage-map.md](physics-coverage-map.md)
   Solver-family coverage map and the benchmark lane used to keep broad
   physics support visible.
8. [accuracy-baselines.md](accuracy-baselines.md)
   Concrete benchmark baselines already enforced in automation.
9. [operator-sdk.md](operator-sdk.md)
   Current extension-contract direction for growing operator capabilities
   without turning every family into a one-off vertical slice.
10. [workflow-graph.md](workflow-graph.md)
    Multi-operator composition direction for shader-like workflow growth.
11. [workflow-dataset.md](workflow-dataset.md)
    ONNX-like cross-operator data contract for workflow-carried values.
12. [installer-remote-control.md](installer-remote-control.md)
    Installer-owned remote deployment and runtime-control surface.
13. [language-packs.md](language-packs.md)
    Local-first multilingual extension path for product-owned UI surfaces.
