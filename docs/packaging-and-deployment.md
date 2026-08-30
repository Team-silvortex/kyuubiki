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
- `make desktop-install-host`
  Installs the current three-shell set on the host. macOS stages, validates,
  and atomically replaces the fixed application bundles, restoring the prior
  bundle if activation fails. Ubuntu validates exactly three current-version
  `.deb` packages and their architecture, installs them through non-interactive
  `sudo -n apt-get`, then verifies package versions and `/usr/bin` entrypoints
- `make desktop-packaged-smoke PLATFORM=macos|linux|windows`
  Launches all three packaged desktop binaries, waits for their interactive UI
  startup receipts, validates version/surface/PID identity, and retains logs.
  Linux uses an isolated D-Bus session plus Xvfb for the WebKitGTK shells;
  Windows installed-package qualification additionally uses `--install-nsis`
- `./scripts/kyuubiki qualify-desktop-bundle-update-operational-host`
  Copies the current host bundles into an isolated qualification root, creates
  two content-distinct package generations, and executes the three-shell
  install, upgrade, and exact rollback journey. macOS variants are ad-hoc
  re-signed after their qualification marker is added; the source bundles and
  installed applications are never modified
- `./scripts/kyuubiki check-desktop-bundle-update-operational-qualification`
  Semantically revalidates a retained report, including all three payloads,
  three monotonic activation records, nine unique boot probes, exact rollback,
  and clean lock/staging state
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
installer-managed on the configured Ubuntu qualification host:

- `libwebkit2gtk-4.1-dev`
- `libgtk-3-dev`
- `librsvg2-dev`
- `patchelf`

Use `cargo run -p kyuubiki-installer -- linux-desktop-deps` to print the
installer-owned dependency plan, including the user-scoped frontend build Node
path, the apt package set, and the preflight command. That Node tree is not
copied into an installed Kyuubiki runtime.
- `./scripts/kyuubiki package-desktop macos|linux|windows`
- `./scripts/kyuubiki package-desktop all`
- `./scripts/kyuubiki desktop-upload-remote macos|linux|windows|all`
- `./scripts/kyuubiki desktop-status macos|linux|windows|all`
- `./scripts/kyuubiki desktop-stage macos|linux|windows|all`
- `./scripts/kyuubiki desktop-build-host [--bundles <bundle-list>]`
- `./scripts/kyuubiki desktop-install-host`
- `./scripts/kyuubiki desktop-packaged-smoke macos|linux|windows`
- `./scripts/kyuubiki qualify-desktop-bundle-update-operational-host`
- `./scripts/kyuubiki check-desktop-bundle-update-operational-qualification`
- `./scripts/kyuubiki desktop-release macos|linux|windows|all`
- `./scripts/kyuubiki desktop-verify macos|linux|windows|all`
- `./scripts/kyuubiki desktop-linux-remote`
- `./scripts/kyuubiki desktop-linux-remote install-deps`
- `./scripts/kyuubiki desktop-linux-remote preflight`

`make package-runtime` is the cleanest entry point when you want a portable
runtime layout that keeps component outputs organized in one generated tree.

The managed desktop bundle format is
`kyuubiki.desktop-bundle-set/v1`. Its manifest binds the platform, package
version, exact three-component inventory, entrypoints, executable bits, file
sizes, per-file SHA-256 values, per-component digests, and aggregate payload
digest. Installer stores verified versions immutably and changes the active set
only by appending a `kyuubiki.desktop-bundle-activation/v1` record. Rollback is
therefore another atomic activation of a previously verified complete set, not
a best-effort overwrite of three unrelated applications.

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
- `dist/<platform>/services`
- `dist/<platform>/scripts`
- `dist/<platform>/exports`

## Embedded runtime posture

Self-hosted installs do not require users to install Node. Node is pinned only
for frontend compilation and development; the installed Workbench is a static
export served by `kyuubiki-runtime`. The installer-managed release scaffold writes:

- `dist/<platform>/manifests/embedded-runtimes.json`
- `dist/<platform>/runtimes`

The manifest is generated from `config/toolchains.json` and declares the
runtime payloads expected for self-host operation:

- `elixir-otp` for the control plane, workflow mesh checks, and live headless
  tests
- no JavaScript runtime payload: static asset serving, Orchestra proxying, and
  direct-mesh TCP bridging are native Rust runtime responsibilities

The runtime contract makes versions, target paths, development fallback, and
installer-managed fail-closed policy visible. Missing runtime payloads are
deployment blockers for self-host releases, not hidden user prerequisites.

The service launch manifest points `frontend` back to
`bin/kyuubiki-runtime serve-frontend`. The package therefore contains neither
`services/frontend/server.js` nor `runtimes/<platform>/node`.
Restaging is also a migration boundary: known legacy Node runtime directories,
server entrypoints, and frontend `node_modules` are removed before a new
payload is sealed, so old installs do not retain an unused JavaScript runtime.
The old `runtime-payload.json` seal is removed at the same boundary and is only
recreated after the complete native payload has been assembled.
Runtime payload assembly builds every Rust, Orchestra, and frontend input
before mutating the existing staging directory. A failed prerequisite build
therefore cannot replace a usable staged payload with a partial one.
Before Orchestra assembly, only its generated release tree is reset; compiled
Mix dependency caches are retained. This prevents old application and
dependency versions from leaking into a new payload without forcing a full
dependency rebuild.

Development launch commands, integration tests, remote mesh regression, and
all three desktop shells resolve runtime commands through the shared native
`kyuubiki-desktop-runtime` crate:

1. installer-managed runtime paths declared by `embedded-runtimes.json`
2. host-installed tools as an explicit development-source fallback only
3. hard failure in installer-managed mode when a declared component is
   missing

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

Every local delivery follows a fail-closed integrity path. Catalog artifact
paths must remain relative to the configured source root, while download and
apply paths must remain inside the workspace-managed update directory without
crossing symlinks. The Installer computes a deterministic SHA-256 digest after
copying every file or directory tree, records it in the download manifest, and
revalidates every downloaded artifact before apply. A changed digest, escaped
path, unsupported file type, or mismatched pointer/manifest identity stops the
operation before an applied record is written.

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
2. if the change touches workflow-heavy frontend surfaces, start the native
   local stack with `./scripts/kyuubiki restart-local` and run:
   `make workflow-preflight`
3. stage or refresh rollout scaffolds:
   `make desktop-stage PLATFORM=all`
4. build host-native desktop bundles:
   `make desktop-build-host`
5. prove that every packaged host shell reaches its interactive startup point:
   `make desktop-packaged-smoke PLATFORM=macos|linux|windows`
6. run the integrated release pass for the current host:
   `make desktop-release`
7. re-check descriptors and icon coverage:
   `make desktop-verify PLATFORM=all`

These checks prove different layers. `desktop-verify` validates staged
descriptors and icon inputs. `desktop-packaged-smoke` executes the packaged
shells and verifies the native-to-WebView startup path. `desktop-release`
additionally enforces distribution signing and notarization policy; a local
ad-hoc build passing the smoke test is not a public release artifact.

After installing the bundles, run the same probe against the installed copy:

```sh
./scripts/kyuubiki desktop-packaged-smoke macos \
  --bundle-root /Applications \
  --out tmp/packaged-desktop-installed-smoke.json
```

This prevents a source bundle pass from hiding a stale application under the
system application directory.

For an installed Linux package set, run the native executables under the
desktop smoke runner. The runner creates the required D-Bus and Xvfb session;
calling an installed WebKitGTK binary under Xvfb alone is not equivalent.

```sh
./scripts/kyuubiki desktop-packaged-smoke linux \
  --bundle-root /usr/bin \
  --out tmp/linux-installed-desktop-smoke.json
```

On Windows, the native runner can qualify the complete NSIS lifecycle without
embedding PowerShell deployment logic in CI. It discovers the three packages,
installs them silently for the current user, launches each installed WebView2
shell, verifies its receipt, writes the portable report, and uninstalls all
three packages before returning:

```text
cargo run --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-script-runner -- desktop-build-host --bundles nsis

cargo run --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-script-runner -- desktop-packaged-smoke windows \
  --install-nsis --out tmp/windows-installed-desktop-smoke.json
```

The canonical automation is
`.github/workflows/desktop-windows-qualification.yml`. It uploads both the raw
qualification candidate and the native validator's canonical retained-report
layout. The artifact still has to be reviewed and merged into release evidence
before closing
`packaged_desktop_round_trip/windows-installed` in the usability release gate.

After downloading the candidate into `tmp/`, retain it through the native
validator rather than copying it by hand:

```text
./scripts/kyuubiki desktop-packaged-smoke \
  --retain-report tmp/windows-installed-desktop-smoke.json
```

The command rejects failed, stale-version, incomplete, or path-bearing reports
before atomically writing
`releases/usability-evidence/<version>/windows-installed-desktop-smoke.json`.
It is idempotent for identical evidence and refuses to overwrite a different
report for the same release without explicit review.

Retained reports must not contain host absolute paths. The native smoke runner
encodes external locations as `@external` and paths below the selected bundle
root as `@bundle-root`. Verify retained evidence without launching an app with:

```sh
./scripts/kyuubiki desktop-packaged-smoke \
  --verify-report releases/usability-evidence/2.18.3/macos-installed-desktop-smoke.json

./scripts/kyuubiki desktop-packaged-smoke \
  --verify-report releases/usability-evidence/2.18.3/linux-installed-desktop-smoke.json
```

This verifier is host-independent. It checks the report schema, packaged
version, all three desktop surfaces, successful startup receipts, and portable
paths. Evidence remains platform-specific: macOS evidence does not prove Linux
or Windows, and Linux evidence does not prove macOS or Windows.

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
