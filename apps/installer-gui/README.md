# Installer GUI

This app is the desktop installer and operator shell for `kyuubiki`.

It wraps the Rust installer/runtime commands in a Tauri GUI and is the most
operator-facing surface in the repository.

## Responsibilities

- setup and environment authoring
- local/cloud/distributed deployment mode configuration
- service lifecycle actions
- release staging
- desktop installer packaging
- security/runtime settings input

## Main paths

- UI shell:
  [ui/](/Users/Shared/chroot/dev/kyuubiki/apps/installer-gui/ui)
- Tauri backend:
  [src-tauri/](/Users/Shared/chroot/dev/kyuubiki/apps/installer-gui/src-tauri)
- Packaged icons:
  [src-tauri/icons](/Users/Shared/chroot/dev/kyuubiki/apps/installer-gui/src-tauri/icons)
- UI brand assets:
  [ui/assets](/Users/Shared/chroot/dev/kyuubiki/apps/installer-gui/ui/assets)

## Commands

- `make installer-gui-dev`
- `make installer-gui-build`
- `make package-desktop`
- `zsh ./scripts/kyuubiki build-installer-gui macos|linux|windows`
- `zsh ./scripts/kyuubiki package-desktop macos|linux|windows`

## Output

Tauri build output lands under:

- `apps/installer-gui/src-tauri/target`

Platform-scoped staged desktop manifests land under:

- `dist/<platform>/desktop/installer-gui`

Do not treat that directory as source-owned. The source of truth is:

- the Rust installer/runtime crates
- the Tauri shell source in this app
- repo-relative runtime defaults such as `./deploy/agents.local.json` and
  `./tmp/data/kyuubiki_dev.sqlite3`
- the shared packaging docs in
  [docs/packaging-and-deployment.md](/Users/Shared/chroot/dev/kyuubiki/docs/packaging-and-deployment.md)
