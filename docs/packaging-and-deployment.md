# Packaging And Deployment

This document is the packaging map for `kyuubiki moxi 2.x`.

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
| `apps/hub-gui` | Tauri desktop hub shell | `make build-hub-gui` | `target/desktop-cache/<platform>` |
| `apps/installer-gui` | Tauri installer shell | `make build-installer-gui` | `target/desktop-cache/<platform>` |
| `apps/workbench-gui` | Tauri desktop workbench shell | `make build-workbench-gui` | `target/desktop-cache/<platform>` |
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

The native desktop dev and build wrappers synchronize shared UI, brand, and
surface-scoped language-pack assets before Tauri starts. `desktop-build-host`,
`package-desktop`, and `desktop-release` prepare those assets once before building
all three host shells.

## Packaging entry points

Use these commands when building deployable layouts:

- `make check-elixir-self-host`
  Verifies the current machine's Elixir/Mix/OTP runtime and the orchestrator
  self-host environment contract before installer-managed deployment.
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
  Builds the `hub-gui`, `installer-gui`, and `workbench-gui` bundles for the current host using
  one shared, platform-scoped Cargo cache
- `make desktop-release PLATFORM=macos|linux|windows|all`
  Runs `desktop-stage`, host-native desktop bundle builds, and desktop verification
- `make desktop-verify PLATFORM=macos|linux|windows|all`
  Verifies staged manifests and required icon inputs for each desktop app
- `make desktop-linux-remote`
  Syncs the checkout to the Ubuntu lab host and runs the Linux desktop package
  build there, keeping large Linux artifacts off the Mac by default.
- `make desktop-linux-remote-install-deps`
  Runs the installer-declared apt dependency install on the lab host with
  `sudo -n`; it fails rather than prompting or storing a password.
- `make desktop-linux-remote-preflight`
  Checks the Ubuntu lab host for a Node version compatible with
  `config/toolchains.json` (installer default: Node 20.19.x), npm, Cargo/Rust,
  Make, and the Linux Tauri system packages before running the heavier remote
  bundle build.

The Linux remote preflight currently expects these Ubuntu packages to be
installer-managed on `kyuubiki-lab`:

- `libwebkit2gtk-4.1-dev`
- `libgtk-3-dev`
- `librsvg2-dev`
- `patchelf`

Use `cargo run -p kyuubiki-installer -- linux-desktop-deps` to print the
installer-owned dependency plan, including the user-scoped Node runtime path,
the apt package set, and the preflight command.
- `./scripts/kyuubiki package-desktop macos|linux|windows`
- `./scripts/kyuubiki package-desktop all`
- `./scripts/kyuubiki desktop-upload-remote macos|linux|windows|all`
- `./scripts/kyuubiki desktop-status macos|linux|windows|all`
- `./scripts/kyuubiki desktop-stage macos|linux|windows|all`
- `./scripts/kyuubiki desktop-build-host`
- `./scripts/kyuubiki desktop-release macos|linux|windows|all`
- `./scripts/kyuubiki desktop-verify macos|linux|windows|all`
- `./scripts/kyuubiki desktop-linux-remote`
- `./scripts/kyuubiki desktop-linux-remote install-deps`
- `./scripts/kyuubiki desktop-linux-remote preflight`

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
- `dist/<platform>/manifests/embedded-runtimes.json`
- `dist/<platform>/runtimes`
- `dist/<platform>/scripts`
- `dist/<platform>/exports`

## Embedded runtime posture

Self-hosted installs should not require users to manually install Elixir/OTP or
Node before Kyuubiki can run. The installer-managed release scaffold now writes:

- `dist/<platform>/manifests/embedded-runtimes.json`
- `dist/<platform>/runtimes`

The manifest is generated from `config/toolchains.json` and declares the
runtime payloads expected for self-host operation:

- `elixir-otp` for the control plane, workflow mesh checks, and live headless
  tests
- `node` for runtime scripts, frontend launch surfaces, and docs/contract
  checks

The first implementation is manifest-first: it makes versions, target paths,
and host-fallback policy visible before payload download/extraction is wired
into the installer. Missing runtime payloads should be treated as deployment
blockers for self-host releases, not as hidden user prerequisites.

Launch scripts and `scripts/kyuubiki-runtime.mjs` now resolve runtime commands
in this order:

1. installer-managed runtime paths declared by `embedded-runtimes.json`
2. host-installed tools only as visible fallback
3. hard failure when `KYUUBIKI_RUNTIME_STRICT=1` and a required embedded
   runtime command is missing

This keeps local development flexible while making self-host deployment
version choices deterministic and inspectable.

## Remote artifact retention

Generated desktop bundles are not expected to live permanently on a local
MacBook or dev workstation.

Preferred flow:

1. stage or build the release locally
2. upload the generated outputs to the remote download server
3. optionally remove local generated bundle outputs after a successful upload

Primary command:

- `./scripts/kyuubiki desktop-upload-remote macos|linux|windows|all`

Environment overrides:

- `KYUUBIKI_RELEASE_REMOTE_HOST`
  SSH host or alias for the download server. A typical example is
  `release-user@download-host.example`.
- `KYUUBIKI_RELEASE_REMOTE_DIR`
  Remote root path that will receive `releases/<version>/...`.
- `KYUUBIKI_RELEASE_REMOTE_PASSWORD`
  Temporary dev-only compatibility password for `sshpass -e` uploads when the
  remote host is not yet configured for key-based auth. This is disabled unless
  `KYUUBIKI_RELEASE_REMOTE_ALLOW_PASSWORD=1` is also set. Prefer SSH keys or an
  agent.
- `KYUUBIKI_RELEASE_REMOTE_ALLOW_PASSWORD`
  Set to `1` to explicitly allow the temporary password compatibility path.
- `KYUUBIKI_RELEASE_VERSION`
  Override the version folder. By default the script uses
  `deploy/update-channels.json` `shipping_version`.
- `KYUUBIKI_RELEASE_REMOTE_SSH_OPTS`
  Optional SSH flags. Defaults to `-o StrictHostKeyChecking=yes`. Use an
  explicit temporary override only for disposable bootstrap hosts.
- `PURGE_LOCAL=1`
  Removes uploaded local `dist/<platform>` trees and shared
  `target/desktop-cache/<platform>/release/bundle` directories for the selected platform after
  a successful upload.

This keeps the release source-of-truth on the remote server while preserving
the local repository as the place where metadata is authored and generated.

The machine-readable disk hygiene contract is:

- `deploy/install-update-disk-hygiene.json`
- `make check-install-update-disk-hygiene`

That check binds together the installation integrity contract, update channel
policy, native remote upload runner, and this document. It rejects absolute or
traversing cleanup roots, requires `PURGE_LOCAL=1` to be explicit, and keeps
rollback on the visible same-channel reinstall path.

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

- human-facing tags such as `moxi:stable`
- concrete immutable shipped versions such as `2.0.0`
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

### Shared desktop build boundary

Hub, Workbench, and Installer remain three independent Tauri applications with
their own manifests, UI trees, bundle identities, and installation targets.
They share only Rust compilation intermediates at
`target/desktop-cache/<platform>`. The platform segment prevents macOS, Linux,
and Windows artifacts from contaminating one another, while the shared cache
avoids compiling common Tauri and Kyuubiki crates three times on the same host.

## Recommended operator flow

When packaging desktop deliverables, the smoothest path is now:

1. inspect current readiness:
   `make desktop-status PLATFORM=all`
2. if the change touches workflow-heavy frontend surfaces, start `npm run dev`
   in `apps/frontend` and run:
   `make workflow-preflight`
3. stage or refresh rollout scaffolds:
   `make desktop-stage PLATFORM=all`
4. build host-native desktop bundles:
   `make desktop-build-host`
5. run the integrated release pass for the current host:
   `make desktop-release`
6. re-check descriptors and icon coverage:
   `make desktop-verify PLATFORM=all`

`desktop-status` is intentionally the first stop. It gives operators one place
to see:

- current host platform
- whether `dist/<platform>` scaffolds are already present
- whether each desktop app has a staged manifest
- whether required icon inputs are ready for each platform
- whether each platform's shared Cargo cache and host-native Tauri bundle directories already exist
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

For moxi 2.x, treat these as two different validation modes:

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
