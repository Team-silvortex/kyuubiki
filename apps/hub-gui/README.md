# Hub GUI

`hub-gui/` is the future unified desktop entrypoint for Kyuubiki.

It is intentionally positioned above:

- [installer-gui](/Users/Shared/chroot/dev/kyuubiki/apps/installer-gui)
- [workbench-gui](/Users/Shared/chroot/dev/kyuubiki/apps/workbench-gui)

Its job is not to replace the workbench modeling surface. Its job is to act as
the desktop launcher, runtime controller, and system overview shell.

## Responsibilities

- project launcher
- runtime lifecycle overview
- local/cloud/distributed mode selection
- environment diagnostics
- logs and health summary
- quick launch into `Workbench`, `Installer`, and future admin tools

## Current state

This app currently contains:

- a static shell prototype under [ui/](/Users/Shared/chroot/dev/kyuubiki/apps/hub-gui/ui)
- a Tauri native shell under
  [src-tauri/](/Users/Shared/chroot/dev/kyuubiki/apps/hub-gui/src-tauri)
- repository-level structure and IA guidance in
  [docs/hub-architecture.md](/Users/Shared/chroot/dev/kyuubiki/docs/hub-architecture.md)

It does not yet replace the existing Tauri apps, but it is now scaffolded as a
real Tauri application so it can grow into that role without another platform
split later.

## Run

- `./scripts/kyuubiki hub-gui-dev`
- `./scripts/kyuubiki hub-gui-build`

The native shell reuses the shared desktop runtime control crate and now
follows the same `src-tauri/icons/` layout as the other desktop applications.

## Direction

Short term:

- establish the shell information architecture
- align branding and product split
- give the repository one obvious GUI home for orchestration
- wire the shell to local runtime status and launch actions

Long term:

- absorb frequent runtime/operator tasks from `installer-gui`
- launch and supervise `workbench-gui`
- become the everyday desktop entrypoint for Kyuubiki
