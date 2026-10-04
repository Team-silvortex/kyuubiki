# Changelog

## daji 3.4.5

October 4, 2026. Modal cancellation safety and bounded numerical correction.

- Extract physical tridiagonal bands once, validate symmetry and range, and
  propagate cancellation or preparation errors rather than falling back silently.
- Add bounded cooperative checks to modal vector arithmetic, refinement and
  final physical validation without changing ordinary arithmetic order.
- Keep candidate spectra private until the original stiffness/mass and physical
  shape checks complete. Failed or cancelled later modes discard earlier modes.
- Admit complete dense spectra for nontridiagonal single-mode problems up to
  256 active degrees of freedom, using the unchanged 1e-8 physical residual gate.
- Apply bounded, automatically partitioned coordinate correction to unresolved
  single-mode shapes. Keep the eigenvalue and original root neighborhood fixed.
- Extend independent physical reassembly, failure replay, Rust Headless and
  Agent-stage regression coverage while retaining Solver numerical ownership.
- Record multimode, projection, coherent-phase and quantized-QR comparisons as
  test-only candidates, not public runtime fallbacks or installed qualification.
- Align source, SDK, language-pack, installation and current documentation
  metadata to 3.4.5 while preserving protocol identifiers and report provenance.

See `releases/snapshots/3.4.5.json` and the dated modal reports for verification.
The tiny-scale 128-element public solve remains fail-closed; a successful
test-only candidate is not a successful public solve. Recompute affected modal
results before research reuse. No desktop rebuild, package publication, new
remote platform test or large-scale qualification is included. The installed
baseline remains 3.4.0, and tensor readiness gaps remain open.

## daji 3.4.4

October 2, 2026. Modal numerical reliability and bounded Agent execution.

- Check full physical frame assembly before support reduction, and share
  range-aware mass normalization between dense and prepared inverse paths.
- Preserve soft modal components, weak couplings, repeated-root subspaces,
  physical shape normalization, and original stiffness/mass residual gates.
- Improve bounded sparse inverse iteration, prepared factor reuse, chain
  recognition and tridiagonal recovery without relaxing convergence budgets.
- Separate exceptional sparse-product scales into binary mantissas/exponents;
  avoid representable-result loss from intermediate underflow or overflow,
  while ordinary products retain their existing arithmetic order.
- Enable only planar and spatial modal built-ins alongside the bar route in
  Agent TaskIR admission. Validate matching built-in entrypoints and dispatch
  through Engine rather than adding numerical implementations to Agent.
- Preserve numeric JSON round trips and task digests; test live TCP admission,
  cancellation, failure isolation, same-connection recovery and fresh replay.
- Add native development-cache inspection and allowlisted cleanup. Reduce
  development debug data without changing release profiles or installed apps.
- Align product, SDK, language-pack, installation and documentation metadata
  to 3.4.4; keep protocol versions and historical test provenance unchanged.

See `releases/snapshots/3.4.4.json` and the dated modal reports for verification.
Recompute affected historical modal results before research reuse. The retained
128-element bending case still fails closed within its original budget. Local
debug microbenchmarks are not release throughput or whole-solve speedup claims.
This source release does not rebuild desktop apps, publish packages or certify
new platform/scale coverage. The installed baseline remains 3.4.0, and tensor
readiness gaps remain open.

## daji 3.4.3

September 28, 2026. Structural and thermal-structural numerical reliability.

- Harden truss and thermal-truss geometry, temperature averaging, recovered
  fields, displacement norms, and strain energy against range loss.
- Split beam and frame element/system assembly into checked Solver-local
  modules while keeping the Engine and operator interfaces independent.
- Validate assembled and recovered values even for fully restrained systems;
  invalid arithmetic fails instead of becoming a successful null-valued result.
- Recover full planar bending-field energy and avoid subtractive cancellation
  in spatial thermal-frame elastic energy.
- Normalize spatial directions without squared-norm overflow; use a stable
  exact-constraint basis for thermal-frame supports and reaction recovery.
- Reorthogonalize explicit spatial section hints without changing the default
  reference-axis convention; retain rigid modes and equivalent-hint fields.
- Add analytical, orientation, refinement, cancellation, and fresh-replay tests,
  including Rust Headless plan/bridge/Engine execution on macOS and Linux.
- Align package, SDK, language-pack, installation, and documentation metadata
  to 3.4.3 without changing protocol versions or relabeling historical evidence.

See `releases/snapshots/3.4.3.json` for verification and limits. Recompute affected
historical energy, extreme-range, support, or near-parallel section-hint results
before research reuse. This source release does not rebuild desktop applications,
publish registry packages, or certify new physical/scale coverage; the last local
desktop/runtime installation remains 3.4.0 and tensor readiness gaps remain open.

## daji 3.4.2

September 27, 2026. Atomic workflow administration and solver-kernel reliability.

- Admit workflow jobs and initial recovery state atomically; reject duplicate or
  invalid initialization without overwriting existing execution receipts.
- Delete jobs and results together under the Orchestra lease, stop tracked local
  runners only after commit, and reject late publication of deleted work.
- Guard result editing/deletion with one storage mutation: retain workflow
  identity, reject private recovery injection, and never recreate missing results.
- Correct signed CST triangle gradients for clockwise/mixed-orientation meshes,
  including thermal expansion and recovered mechanical fields.
- Stabilize Q4 and tetrahedral geometry under large coordinate translations;
  reuse stiffness intermediates and reject nonfinite element coefficients.
- Harden thermal-plane temperature interpolation, recovered fields, energy totals,
  and displacement norms against representable-range overflow and cancellation.
- Add closed-form, permutation, fault-injection, headless, and Linux regression
  evidence. Kernel benchmark gains are not whole-solve performance claims.
- Make update-catalog self-tests independent of retained local desktop builds,
  with explicit declared/present artifact rendering coverage.
- Align product, SDK, desktop, language-pack, and current documentation metadata
  to 3.4.2; preserve protocol identifiers and original evidence versions.

See `releases/snapshots/3.4.2.json` for verification and scope. Recompute historical
clockwise/mixed-orientation CST results before research reuse. This is a source
release, not a desktop rebuild, binary download, or registry publication; the last
locally built and installed desktop/runtime baseline remains 3.4.0.

## daji 3.4.1

September 27, 2026. Workflow publication and storage recovery hardening.

- Aligned Rust and Elixir workflow conditions and named-input routing through
  shared fixtures, including exact numeric comparisons and branch isolation.
- Made job updates compare the full observed snapshot, preserving concurrent
  metadata and preventing stale watchdog transitions from overwriting new work.
- Publish solver completion and its result atomically. Premature Agent progress
  stays in postprocessing until the final response is validated and committed.
- Commit workflow runtime, recovery envelopes, and job progress together, with
  stale-claim rejection and rollback coverage on SQLite and memory backends.
- Consolidated memory job/result persistence into one integrity-checked snapshot
  generation, with bounded recovery files and one-time legacy import.
- Keep recovery coordination and watchdog scans alive through scoped storage
  outages. Health reports degraded state and unknown counts rather than false
  success; normal programming errors are not swallowed.
- Aligned product, SDK, desktop, language-pack, and current documentation metadata
  to 3.4.1 without changing protocol identifiers or relabeling historical evidence.
- Show an explicit empty state when a source release declares no desktop packages.

See `releases/snapshots/3.4.1.json` for verification and scope. This is a source
release, not a new desktop installation, binary download, or registry publication.
The last locally built and installed desktop/runtime baseline remains 3.4.0.

## daji 3.4.0

September 27, 2026. Workflow reliability and coordinated local packaging.

- Added bounded graph preflight and operator-output publication contracts,
  with shared Rust/Elixir fixtures and API recovery coverage.
- Made diagnostic bundle ordering independent of serde_json feature unification.
- Fixed CI metric-contract scanning after module splitting and updated the
  headless terminal-receipt assertion to the current completion contract.
- Added one bounded retry for the identified Chromium/libdbus pre-page startup
  crash, retaining failure diagnostics without retrying application assertions.
- Isolated live operator-task test databases, retained startup diagnostics, and
  made parallel Agent test directories exclusive even at identical timestamps.
- Cleared native runtime, Agent, and test/example Clippy warnings without lint
  suppression; the four-package strict all-target check passed.
- Aligned desktop shells, native runtime, SDK metadata, language packs, and
  current documentation to 3.4.0; protocol versions remain unchanged.

See `releases/snapshots/3.4.0.json` for actual build and verification outcomes.
Historical evidence is not relabeled, and this is not registry publication,
notarization, or automatic deployment to other machines.

## daji 3.3.0 local packaging follow-up

- Grouped study setup, modeling, inline checkpoint save, execution, and result
  review near the modeling viewport, reusing existing save and PWDT contracts.
- Added duplicate-submit protection across panel switches, live cancellation,
  failed-call recovery, compact-layout, and RTL browser regression coverage.
  Fullscreen uses the existing controls without duplication.
- Aligned desktop/runtime packages, SDK metadata, and local language packs to
  3.3.0. This is a local macOS rebuild, not public or registry publication.

Build and installation outcomes belong to `releases/snapshots/3.3.0.json`.
The preceding documentation-only checkpoint below retains its original scope.

## daji 3.3.0 development checkpoint

September 20, 2026. This entry summarizes recent source changes carried forward
from 3.2.x; it is not a package publication or installed-release qualification.

- Added a focused Hub bundle-creation flow with native directory selection.
- Reorganized PWDT into a dedicated workspace with an embedded editor and
  grouped controls; compacted dense material, runtime, and security panels.
- Corrected mixed-language Workbench root and extended language-pack copy and
  completed the bounded Arabic, Persian, and Spanish PWDT copy contracts.
- Added localization regressions for locale switching, retained drafts and
  execution state, accessible labels, and installed macOS Pyodide controls.
- Aligned the central HTML book, Hub documentation shelf, model onboarding,
  and current architecture metadata to the 3.3.0 development checkpoint.

The packaged baseline remains `3.2.0`. Historical evidence, schema versions,
and open readiness gates are unchanged. Translation is not yet certified for
every surface in all 30 locales. See [current progress](docs/current-line.md)
and [language-pack scope](docs/language-packs.md) for the detailed boundaries.

Older entries below are retained snapshots, not a complete intervening history.
Use Git history and the linked current-line documentation for later development.

## tamamono 1.8.1 workflow reliability snapshot

### Changed

- started formalizing workflow-run lifecycle handling instead of leaving run
  state as ad hoc frontend-only strings
- introduced shared frontend job-status helpers for active, terminal, failed,
  and detached workflow-run states
- workflow runs now retain explicit polling attachment state, so the Workbench
  can show when a run has detached from active polling instead of only dropping
  a transient message
- added structured `status_detail` metadata to job payloads so stalled,
  watchdog-timeout, execution-timeout, and operator-cancelled failures are no
  longer hidden only inside freeform message strings
- carried the new structured job-status detail into:
  - workflow run trace cards
  - workflow history reopen flows
  - headless job fetch / wait flows
  - library and admin job list surfaces

### Notes

- this is a `1.8.1` hardening step inside the current `1.8.x` line, not the
  start of the planned `1.9.x` task-system expansion
- formal release metadata and generated update-catalog artifacts are still on
  the published `1.8.0` snapshot until the repository-wide version contract is
  advanced together

## tamamono 1.8.0 security hardening snapshot

### Changed

- stopped the desktop installer from returning plaintext `.env.local` secrets to
  the Tauri renderer during env reload; sensitive fields now round-trip as
  configured-state plus explicit overwrite intent
- moved workbench operator secrets out of browser-persisted storage and into
  in-memory session state; legacy local/session storage tokens are scrubbed on
  load
- removed the cluster-route fallback from `KYUUBIKI_CLUSTER_API_TOKEN` to
  `KYUUBIKI_API_TOKEN`; remote cluster registration, heartbeat, and removal now
  require the dedicated cluster token
- replaced direct string token equality in the Phoenix security helper with
  constant-time comparison

### Verified

- `mix test test/kyuubiki_web/api/cluster_security_api_test.exs test/kyuubiki_web/api/control_plane_api_test.exs`

## v0.4

Kyuubiki `v0.4` is the release where the system becomes much more explicitly multi-program: browser workbench, desktop shells, orchestrator, direct mesh routes, and headless solver agents can now be reasoned about as cooperating surfaces instead of one blurred stack.

### Added

- shared desktop runtime crate for Tauri installer and Tauri workbench shells
- Tauri desktop workbench shell logs/status workflow
- direct-mesh result chunk API for large result review without Phoenix on the solver hot path
- stable frontend `typecheck` command that prepares missing Next route type artifacts before `tsc`
- integration smoke coverage for:
  - local orchestrator + agent + API solve flow
  - protected cluster register / heartbeat / unregister flow
  - `direct_mesh_gui` LAN discovery + direct solve + chunk retrieval
- aggregate `make test-integration` entrypoint

### Changed

- tightened startup/restart behavior in `scripts/kyuubiki` with explicit port release and listener wait logic
- aligned `direct_mesh_gui` input normalization with the main frontend job contracts
- continued hardening remote cluster security with:
  - dedicated cluster token
  - allowlists
  - fingerprint binding
  - replay-window timestamp checks
- continued documenting the stack as independent but cooperating programs with shared contracts
- clarified multi-platform desktop packaging with staged `macos / linux / windows`
  desktop manifests, Windows `.ico` support, and a formal desktop release checklist

### Direction after v0.4

- extend integration testing into the Tauri workbench shell end-to-end path
- keep pushing visible-window-driven chunking instead of page-style browsing
- deepen distributed and peer-mesh execution without re-coupling the frontend to Phoenix
- keep refining the desktop and direct-mesh surfaces as first-class runtime modes

## v0.3

Kyuubiki `v0.3` is the release where the system starts to behave like an engine-backed FEM workstation under real scale, not just a coherent local-first prototype.

### Added

- formal benchmark scaling tiers for `10k`, `15k`, and `20k`
- checked-in single-machine baselines for `medium`, `10k`, `15k`, and `20k`
- benchmark comparison reports and regression gates
- progressive/lazy rendering for large viewport result windows
- adaptive chunk windows with jump navigation for large result browsing
- watchdog-backed job timeout, stale detection, heartbeat status, and cancel flows
- runtime remote-agent registration and heartbeat APIs for distributed deployments
- explicit control-plane and solver-RPC protocol descriptors
- Rust agent self-description and generic runtime RPC methods (`ping`, `describe_agent`)
- headless agent runtime metadata for standalone, orchestrated, and peer-mesh cluster modes
- gossip-lite peer discovery for LAN solver meshes
- explicit frontend runtime split in the architecture:
  - `orchestrated_gui`
  - `direct_mesh_gui`
- direct-mesh frontend API routes that let the Next.js shell inspect and solve
  against LAN Rust agents without going through Phoenix

### Changed

- pushed sparse-first solver performance further for `2D truss`, `2D plane triangle`, and `3D truss`
- improved single-machine `M2 + 16GB` behavior through `10k` and into the `15k`/`20k` node class
- tightened the frontend toward a denser editor-style layout with more segmented tabs and less card sprawl
- continued separating engine, orchestrator, installer, and workbench responsibilities
- made the GUI, control plane, and solver agents more explicitly deployable as independent programs
- clarified that the future frontend can run either through Phoenix or directly
  against a LAN peer mesh while sharing the same contracts

### Scale snapshot

- `10k` is now the practical comfort tier
- `15k` is a stable upper tier
- `20k` is a real single-machine stretch tier, with model-family-dependent cost

### Direction after v0.3

- push viewport-driven chunk loading beyond page-style result windows
- keep improving sparse solver stability and performance before chasing larger raw node counts
- deepen distributed orchestration and remote deployment workflows without coupling them to any single frontend mode

## v0.2

Kyuubiki `v0.2` is the first release where the system behaves like a coherent local-first FEM workbench rather than a loose prototype.

### Added

- Next.js workbench with:
  - `1D axial bar`
  - `2D truss`
  - `2D plane triangle`
  - `3D space truss`
- immersive `3D` workspace mode
- direct `2D` and `3D` node drag editing
- `3D` box selection, focus, frame selection, link editing, duplication, mirror, and nudge tools
- multi-material model support for `2D truss`, `3D truss`, and `2D plane triangle`
- external material import from `JSON` and `CSV`
- project / model / model-version CRUD
- job / result CRUD
- portable project formats:
  - `.kyuubiki.json`
  - `.kyuubiki`
- chunked result browsing for large result sets
- Rust engine facade crate
- benchmark profiles: `medium`, `large`, `v2`
- Rust installer CLI
- Tauri installer GUI

### Changed

- moved toward engine-first separation between frontend, orchestrator, and solver
- added multi-agent Rust RPC execution with round-robin dispatch and failover
- added dual database support:
  - local-first `SQLite`
  - distributed/cloud `PostgreSQL`
- improved 3D workspace layout so the viewport can fully occupy space when auxiliary docks are closed
- reworked frontend into a denser, more ergonomic workbench with tabbed panels and virtualized lists

### Persistence

- persisted projects
- persisted models
- persisted model versions
- persisted jobs
- persisted results
- database snapshot export

### Tooling

- `make start-local` / `make restart-local`
- `make start-cloud` / `make restart-cloud`
- `make doctor`
- `make validate-env`
- `make export-db`
- `make installer-gui-dev`
- `make installer-gui-build`

### Direction after v0.2

- single-machine `10k`-node workflows on `M2 + 16GB`
- stronger sparse-first solver paths
- more engine-style result chunking and viewport-driven loading
