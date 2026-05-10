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
- local/cloud/distributed mode selection
- diagnostics and health summary
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
- `make test-hub-gui`
- `make package-desktop`
- `make desktop-build-host`
- `make desktop-verify PLATFORM=macos|linux|windows`
- `zsh ./scripts/kyuubiki build-hub-gui macos|linux|windows`
- `zsh ./scripts/kyuubiki package-desktop macos|linux|windows`

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
- [docs/packaging-and-deployment.md](/Users/Shared/chroot/dev/kyuubiki/docs/packaging-and-deployment.md)
