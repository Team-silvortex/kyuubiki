# Packaging And Deployment

This document is the packaging map for `kyuubiki v0.5`.

Use it when you need to answer:

- which component builds what
- where artifacts land
- which command should be used for local packaging
- which output is source-of-truth vs generated

## Component matrix

| Component | Role | Main build command | Main output path |
| --- | --- | --- | --- |
| `apps/frontend` | browser workbench | `make build-frontend` | `apps/frontend/.next` |
| `apps/web` | Phoenix orchestrator / control plane | `make build-orchestrator` | `apps/web/_build` |
| `workers/rust/crates/cli` | headless Rust solver agent | `make build-agent` | `workers/rust/target/release/kyuubiki-cli` |
| `apps/installer-gui` | Tauri installer shell | `make build-installer-gui` | `apps/installer-gui/src-tauri/target` |
| `apps/workbench-gui` | Tauri desktop workbench shell | `make build-workbench-gui` | `apps/workbench-gui/src-tauri/target` |
| `workers/rust/crates/installer` | release staging / portable layout generator | `make package-runtime` | `dist/<platform>` |

## Build entry points

Use these commands when working component-by-component:

- `make build-frontend`
- `make build-orchestrator`
- `make build-agent`
- `make build-installer-gui`
- `make build-workbench-gui`
- `zsh ./scripts/kyuubiki build-installer-gui macos|linux|windows`
- `zsh ./scripts/kyuubiki build-workbench-gui macos|linux|windows`

These are thin wrappers over the component-native toolchains:

- frontend: `npm run build`
- orchestrator: `MIX_ENV=prod mix compile`
- agent: `cargo build -p kyuubiki-cli --release`
- desktop shells: Tauri build wrappers

## Packaging entry points

Use these commands when building deployable layouts:

- `make package-runtime`
  Builds the staged runtime scaffold under `dist/<platform>`
- `make package-desktop`
  Builds the Tauri installer GUI and Tauri workbench GUI packaging outputs
- `zsh ./scripts/kyuubiki package-desktop macos|linux|windows`
- `zsh ./scripts/kyuubiki package-desktop all`

`make package-runtime` is the cleanest entry point when you want a portable
runtime layout that keeps component outputs organized in one generated tree.

Current staged runtime layout:

- `dist/<platform>/bin`
- `dist/<platform>/config`
- `dist/<platform>/data`
- `dist/<platform>/desktop/installer-gui`
- `dist/<platform>/desktop/workbench-gui`
- `dist/<platform>/logs`
- `dist/<platform>/manifests`
- `dist/<platform>/scripts`
- `dist/<platform>/exports`

## Output boundaries

### Source-owned paths

These are maintained by humans and should stay readable:

- `apps/`
- `workers/`
- `schemas/`
- `deploy/`
- `assets/`
- `docs/`
- `scripts/`

### Generated paths

These are tool outputs and should be treated as disposable:

- `apps/frontend/.next`
- `apps/web/_build`
- `workers/rust/target`
- `apps/installer-gui/src-tauri/target`
- `apps/workbench-gui/src-tauri/target`
- `dist/`
- `tmp/`

## Deployment shapes

### Local workstation

Recommended for single-machine use.

- frontend served locally
- orchestrator served locally
- local Rust agents
- default storage: `sqlite`

Typical command:

- `make start-local`

### Cloud control plane

Recommended for centralized HTTP/API deployments.

- frontend and orchestrator deployed centrally
- storage: `postgres`
- agents can remain remote

Typical command:

- `make start-cloud`

### Distributed control plane

Recommended when Phoenix remains the scheduler but Rust agents live on remote
machines.

- orchestrator runs centrally
- agents are discovered through:
  - `static`
  - `manifest`
  - `registry`

Typical command:

- `make start-distributed`

### Direct mesh GUI

Recommended for LAN or headless peer-mesh operation where the frontend does not
need Phoenix on the solver hot path.

- GUI talks to LAN Rust agents through direct-mesh routes
- chunked result browsing still works
- no project/job persistence requirement on the solver hot path

## Desktop packaging behavior

Desktop packaging now follows a simple rule:

- if the requested platform matches the current host platform, the Tauri shell
  is actually built
- if the requested platform is different, `kyuubiki` still stages the matching
  desktop manifests under `dist/<platform>/desktop/...`

That keeps `macos`, `linux`, and `windows` deployment paths visible and
manageable even when you are not cross-compiling on the current machine.

The platform-specific release checklist lives in:

- [docs/desktop-release-checklist.md](/Users/Shared/chroot/dev/kyuubiki/docs/desktop-release-checklist.md)

## Related docs

- [README.md](/Users/Shared/chroot/dev/kyuubiki/README.md)
- [docs/operations.md](/Users/Shared/chroot/dev/kyuubiki/docs/operations.md)
- [docs/desktop-release-checklist.md](/Users/Shared/chroot/dev/kyuubiki/docs/desktop-release-checklist.md)
- [docs/system-overview.md](/Users/Shared/chroot/dev/kyuubiki/docs/system-overview.md)
- [deploy/README.md](/Users/Shared/chroot/dev/kyuubiki/deploy/README.md)
- [dist/README.md](/Users/Shared/chroot/dev/kyuubiki/dist/README.md)
