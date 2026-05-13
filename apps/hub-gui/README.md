# Hub GUI

This app is the unified desktop launcher and operator shell for `kyuubiki`.

It sits above:

- [installer-gui](/Users/Shared/chroot/dev/kyuubiki/apps/installer-gui)
- [workbench-gui](/Users/Shared/chroot/dev/kyuubiki/apps/workbench-gui)

Its job is not to replace the modeling workbench. Its job is to become the
everyday desktop entrypoint for project launch, runtime control, and system
overview.

## Responsibilities

- project launcher
- guided assistant entrypoint with local hints and optional OpenAI-compatible model planning
- project bundle inspect / validate / normalize / unpack / pack / diff entrypoint
- Hub-managed workload library for local bundles, imported bundle packs, and future remote catalog delivery
- remote workload catalog sync backed by a formal schema contract
- one-click sync from the local control plane workload catalog, with the Hub pre-filling the current local catalog endpoint
- downloaded remote workloads can be attached back to a local `.kyuubiki` path for unified Hub management
- remote catalog sync now rejects payloads that do not match `kyuubiki.workload-catalog/v1`
- the control plane can now serve `/api/v1/workloads/catalog` plus `/api/v1/projects/:project_id/bundle` for first-party Hub distribution
- recent bundle / compare / output path recall for repeat project operations
- recent project-bundle action history with restore / re-run controls and outcome-aware summaries
- lightweight recent-action filters for failed, inspect, normalize, and diff flows
- recent-action cleanup controls for keeping failed items only or clearing the history
- filtered recent-action JSON export for lightweight local analysis handoff
- local recent-action JSON import with lightweight merge semantics for cross-machine handoff
- lightweight pinning for favorite recent actions so common project flows stay at the top
- dedicated Favorites view above recent history so pinned flows stay immediately visible
- lightweight favorite labels for pinned flows so common routines can read like named shortcuts
- one-click CLI command copy from favorites so common bundle workflows can jump straight into shell automation
- one-click Python stub copy from favorites so common bundle workflows can jump into the front-end DSL / Pyodide path
- runtime lifecycle overview
- managed hot-reload control for the local/cloud/distributed dev loop
- local/cloud/distributed mode selection
- diagnostics and health summary
- integrated desktop readiness wall for `macos / linux / windows` staging, icons, manifests, and host bundles
- desktop release stage / verify / host-build control
- quick launch into `Workbench`, `Installer`, and future admin tools

Quick launch behavior now prefers an already-built host desktop bundle when one
exists, and falls back to the repo-local `tauri:dev` shell during development.

## Main paths

- UI shell:
  [ui/](/Users/Shared/chroot/dev/kyuubiki/apps/hub-gui/ui)
- Tauri backend:
  [src-tauri/](/Users/Shared/chroot/dev/kyuubiki/apps/hub-gui/src-tauri)
- Packaged icons:
  [src-tauri/icons](/Users/Shared/chroot/dev/kyuubiki/apps/hub-gui/src-tauri/icons)
- Product split and IA notes:
  [docs/hub-architecture.md](/Users/Shared/chroot/dev/kyuubiki/docs/hub-architecture.md)

## Commands

- `npm run sync:shared`
- `make hub-gui-dev`
- `make hub-gui-build`
- `make hot-local`
- `make hot-cloud`
- `make hot-distributed`
- `make hot-hub-gui`
- `make test-hub-gui`
- `make desktop-status PLATFORM=all`
- `make package-desktop PLATFORM=all`
- `make desktop-build-host`
- `make desktop-verify PLATFORM=macos|linux|windows`
- `zsh ./scripts/kyuubiki build-hub-gui macos|linux|windows`
- `zsh ./scripts/kyuubiki package-desktop macos|linux|windows`
- `zsh ./scripts/kyuubiki hot-status`

## Validation

- shared UI sync:
  `cd apps/hub-gui && npm run sync:shared`
- smoke test:
  `cd apps/hub-gui && npm run test:smoke`
- Tauri shell check:
  `cargo check --offline --manifest-path src-tauri/Cargo.toml`

## Output

Tauri build output lands under:

- `apps/hub-gui/src-tauri/target`

Platform-scoped staged desktop manifests land under:

- `dist/<platform>/desktop/hub-gui`

Do not treat that directory as source-owned. The source of truth is:

- the Hub Tauri shell source in this app
- the shared desktop runtime crate
- the repository-level desktop packaging flow
- [docs/hub-architecture.md](/Users/Shared/chroot/dev/kyuubiki/docs/hub-architecture.md)
- example workload catalog:
  [deploy/workload-catalog.example.json](/Users/Shared/chroot/dev/kyuubiki/deploy/workload-catalog.example.json)
- [docs/packaging-and-deployment.md](/Users/Shared/chroot/dev/kyuubiki/docs/packaging-and-deployment.md)
