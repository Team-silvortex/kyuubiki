# Packaging And Deployment

This document is the packaging map for `kyuubiki tamamono 1.x`.

Use it when you need to answer:

- which component builds what
- where artifacts land
- which command should be used for local packaging
- which output is source-of-truth vs generated

Use this page for build and artifact mechanics:

- component build entrypoints
- packaging entrypoints
- generated output paths
- staging layout and output semantics

Do not use this page as the main source for:

- runtime operating modes
- environment-switch troubleshooting
- the final human release checklist

Those belong to:

- [operations.md](operations.md)
- [desktop-release-checklist.md](desktop-release-checklist.md)

## Component matrix

| Component | Role | Main build command | Main output path |
| --- | --- | --- | --- |
| `apps/frontend` | browser workbench | `make build-frontend` | `apps/frontend/.next` |
| `apps/web` | Phoenix orchestrator / control plane | `make build-orchestrator` | `apps/web/_build` |
| `workers/rust/crates/cli` | headless Rust solver agent | `make build-agent` | `workers/rust/target/release/kyuubiki-cli` |
| `apps/hub-gui` | Tauri desktop hub shell | `make build-hub-gui` | `apps/hub-gui/src-tauri/target` |
| `apps/installer-gui` | Tauri installer shell | `make build-installer-gui` | `apps/installer-gui/src-tauri/target` |
| `apps/workbench-gui` | Tauri desktop workbench shell | `make build-workbench-gui` | `apps/workbench-gui/src-tauri/target` |
| `workers/rust/crates/installer` | release staging / portable layout generator | `make package-runtime` | `dist/<platform>` |

## Build entry points

Use these commands when working component-by-component:

- `make build-frontend`
- `make build-orchestrator`
- `make build-agent`
- `make build-hub-gui`
- `make build-installer-gui`
- `make build-workbench-gui`
- `./scripts/kyuubiki build-hub-gui macos|linux|windows`
- `./scripts/kyuubiki build-installer-gui macos|linux|windows`
- `./scripts/kyuubiki build-workbench-gui macos|linux|windows`

These are thin wrappers over the component-native toolchains:

- frontend: `npm run build`
- orchestrator: `MIX_ENV=prod mix compile`
- agent: `cargo build -p kyuubiki-cli --release`
- desktop shells: Tauri build wrappers

## Packaging entry points

Use these commands when building deployable layouts:

- `make desktop-status PLATFORM=macos|linux|windows|all`
  Prints host-aware desktop packaging readiness, including staged runtime
  scaffold state, desktop manifest presence, icon readiness, and host bundle
  visibility
- `make package-runtime`
  Builds the staged runtime scaffold under `dist/<platform>`
- `make package-desktop`
  Builds the Tauri Hub GUI, installer GUI, and workbench GUI packaging outputs
- `make desktop-stage PLATFORM=macos|linux|windows|all`
  Stages the release scaffold and desktop manifests under `dist/<platform>`
- `make desktop-build-host`
  Builds the `hub-gui`, `installer-gui`, and `workbench-gui` bundles for the current host
- `make desktop-release PLATFORM=macos|linux|windows|all`
  Runs `desktop-stage`, host-native desktop bundle builds, and desktop verification
- `make desktop-verify PLATFORM=macos|linux|windows|all`
  Verifies staged manifests and required icon inputs for each desktop app
- `./scripts/kyuubiki package-desktop macos|linux|windows`
- `./scripts/kyuubiki package-desktop all`
- `./scripts/kyuubiki desktop-status macos|linux|windows|all`
- `./scripts/kyuubiki desktop-stage macos|linux|windows|all`
- `./scripts/kyuubiki desktop-build-host`
- `./scripts/kyuubiki desktop-release macos|linux|windows|all`
- `./scripts/kyuubiki desktop-verify macos|linux|windows|all`

`make package-runtime` is the cleanest entry point when you want a portable
runtime layout that keeps component outputs organized in one generated tree.

Current staged runtime layout:

- `dist/<platform>/bin`
- `dist/<platform>/config`
- `dist/<platform>/data`
- `dist/<platform>/desktop/hub-gui`
- `dist/<platform>/desktop/installer-gui`
- `dist/<platform>/desktop/workbench-gui`
- `dist/<platform>/desktop/<app>/artifacts`
- `dist/<platform>/desktop/<app>/artifacts.json`
- `dist/<platform>/desktop/artifacts-summary.json`
- `dist/<platform>/desktop/build-summary.json`
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

### Shared install contract

The desktop installer integrity report and repair workflow now read from one
human-owned source file:

- `deploy/installation-integrity-contract.json`
- `make build-installation-docs` regenerates the HTML documentation views that
  mirror this contract under `docs/` and `apps/hub-gui/ui/docs/`

That file defines:

- required repo-local install layout roots
- protected paths that repair must not remove
- allowlisted residue patterns that repair may clean
- the visible behavior contract surfaced in the installer GUI
- the expected desktop shipping version for the current line

### Unified update contract

Unified updates now follow the same source-of-truth posture:

- `deploy/update-channels.json`
  human-owned channel, tag, and rollout contract
- `releases/update-catalog.json`
  generated channel-to-version registry consumed by installer/runtime tooling
- `docs/update-catalog.html`
  generated operator-facing HTML reference for the current channel map

This gives the project a Docker-like update model:

- human-facing tags such as `tamamono:stable`
- concrete immutable shipped versions such as `1.6.0`
- visible rollout rules instead of hidden cleanup or migration behavior
- one shared update description for CLI, installer GUI, and docs surfaces

### Generated paths

These are tool outputs and should be treated as disposable:

- `apps/frontend/.next`
- `apps/web/_build`
- `workers/rust/target`
- `apps/hub-gui/src-tauri/target`
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

## Recommended operator flow

When packaging desktop deliverables, the smoothest path is now:

1. inspect current readiness:
   `make desktop-status PLATFORM=all`
2. stage or refresh rollout scaffolds:
   `make desktop-stage PLATFORM=all`
3. build host-native desktop bundles:
   `make desktop-build-host`
4. run the integrated release pass for the current host:
   `make desktop-release`
5. re-check descriptors and icon coverage:
   `make desktop-verify PLATFORM=all`

`desktop-status` is intentionally the first stop. It gives operators one place
to see:

- current host platform
- whether `dist/<platform>` scaffolds are already present
- whether each desktop app has a staged manifest
- whether required icon inputs are ready for each platform
- whether host-native Tauri bundle directories already exist
- which next command makes sense from the current state

## Recommended desktop release flow

Use one of these two operator-facing flows:

- inspect readiness first:
  `make desktop-status PLATFORM=all`
- stage only:
  `make desktop-stage PLATFORM=all`
- full host release pass:
  `make desktop-release`

`desktop-release` intentionally does three things in one stable order:

1. stage `dist/<platform>` layout and desktop manifests
2. build the host-native `hub-gui`, `installer-gui`, and `workbench-gui` bundles
3. collect host-native desktop bundle artifacts back into `dist/<host>/desktop`
4. verify desktop manifests plus platform-specific icon inputs

After a successful host build or host release pass, operators should expect:

- copied desktop deliverables under:
  - `dist/<host>/desktop/hub-gui/artifacts`
  - `dist/<host>/desktop/installer-gui/artifacts`
  - `dist/<host>/desktop/workbench-gui/artifacts`
- one per-app artifact manifest:
  - `dist/<host>/desktop/<app>/artifacts.json`
- one platform summary:
  - `dist/<host>/desktop/artifacts-summary.json`
- one host build status summary:
  - `dist/<host>/desktop/build-summary.json`

If one desktop shell fails but others succeed, `desktop-build-host` now keeps
the successful artifacts staged under `dist/` and writes the partial result to
`build-summary.json` before returning a non-zero exit code.

`build-summary.json` uses a small operator-facing status vocabulary:

- `built`
  every expected host bundle kind for that app is present
- `partial`
  at least one host bundle kind was staged, but the full expected set is not present
- `failed`
  no host bundle was staged for that app

On macOS, a common `partial` shape is:

- `.app` present
- `.dmg` missing

That usually means the host session could compile and bundle the application,
but the disk-image step could not run to completion. In headless, restricted,
or sandboxed macOS sessions, `hdiutil` itself may be unavailable for full DMG
creation even when `.app` bundling succeeds.

For tamamono 1.x, treat these as two different validation modes:

- `automated session result`
  the packaging command was run from an automated, sandboxed, or otherwise
  controlled execution context. This is enough to validate manifest staging,
  `.app` bundling, artifact collection, and summary generation.
- `full desktop terminal result`
  the same command was run from a normal macOS desktop terminal session. This
  is the authoritative place to confirm whether `.dmg` output can be produced
  on the real host.

If an automated session reports `partial` on macOS, but a normal Terminal.app
 session can create DMGs with `hdiutil`, treat that as an execution-context
 limitation, not as evidence that the Mac host itself is incapable of building
 the release image.

This keeps the current host honest while still preserving all three rollout
paths inside `dist/`.

The platform-specific release checklist lives in:

- [docs/desktop-release-checklist.md](desktop-release-checklist.md)

## Related docs

- [README.md](../README.md)
- [docs/operations.md](operations.md)
- [docs/desktop-release-checklist.md](desktop-release-checklist.md)
- [docs/system-overview.md](system-overview.md)
- [deploy/README.md](../deploy/README.md)
- [releases/README.md](../releases/README.md)
