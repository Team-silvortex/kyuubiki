# Installer GUI

This app is the desktop installer and operator shell for `kyuubiki`.

It wraps the Rust installer/runtime commands in a Tauri GUI and is the most
deployment- and lifecycle-focused surface in the repository.

## Responsibilities

- setup and environment authoring
- runtime and agent install / uninstall flows
- local/cloud/distributed deployment configuration
- lifecycle, repair, cleanup, and integrity actions
- remote node bootstrap, certificate alignment, and mesh-oriented runtime control
- release staging and desktop installer packaging
- security/runtime settings input

## Main paths

- UI shell:
  [ui/](ui)
- Tauri backend:
  [src-tauri/](src-tauri)
- Packaged icons:
  [src-tauri/icons](src-tauri/icons)
- UI brand assets:
  [ui/assets](ui/assets)

## Commands

- `npm run sync:shared`
- `make installer-gui-dev`
- `make installer-gui-build`
- `make test-installer-gui`
- `make desktop-status PLATFORM=all`
- `make package-desktop PLATFORM=all`
- `make desktop-build-host`
- `make desktop-verify PLATFORM=macos|linux|windows`
- `./scripts/kyuubiki build-installer-gui macos|linux|windows`
- `./scripts/kyuubiki package-desktop macos|linux|windows`

## Validation

- shared UI sync:
  `cd apps/installer-gui && npm run sync:shared`
- smoke test:
  `cd apps/installer-gui && npm run test:smoke`
- Tauri shell check:
  `cargo check --offline --manifest-path src-tauri/Cargo.toml`

## Output

Tauri build output lands under:

- `apps/installer-gui/src-tauri/target`

Platform-scoped staged desktop manifests land under:

- `dist/<platform>/desktop/installer-gui`

Do not treat that directory as source-owned. The source of truth is:

- the Rust installer/runtime crates
- the Tauri shell source in this app
- repo-relative runtime defaults such as `./deploy/agents.local.example.json` and
  `./tmp/data/kyuubiki_dev.sqlite3`
- the shared packaging docs in
  [docs/packaging-and-deployment.md](../../docs/packaging-and-deployment.md)

## Remote deployment guardrails

Remote bootstrap and remote agent startup are intentionally bounded:

- target host must be a plain host token, not a URL
- remote workspace must be an absolute normalized path
- orchestrator URL must be `http://` or `https://` without query, fragment, or
  embedded credentials
- optional desktop-side allowlists can further constrain remote operations:
  - `KYUUBIKI_INSTALLER_REMOTE_ALLOWED_HOSTS`
  - `KYUUBIKI_INSTALLER_REMOTE_ALLOWED_WORKSPACE_ROOTS`
- installer-managed policy is stored at:
  - `config/installer-remote-policy.json`

## Remote control surface

The remote node panel is now a first-class Installer subsystem rather than one
large convenience script.

Main UI controllers:

- `ui/remote-node-panel.js`
- `ui/remote-node-renderer.js`
- `ui/remote-node-executor.js`
- `ui/remote-node-actions.js`
- `ui/remote-node-bulk-actions.js`
- `ui/remote-node-certificates.js`
- `ui/remote-node-mesh.js`
- `ui/remote-node-timeline.js`

That split keeps remote deployment logic, certificate state, mesh rollout, and
workflow snapshot rendering understandable as the `1.10.x` line hardens the
operator-facing remote control model.

Deeper source note:

- [../../docs/installer-remote-control.md](../../docs/installer-remote-control.md)
