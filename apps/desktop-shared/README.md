# Desktop Shared

`desktop-shared/` holds source-of-truth frontend assets and helpers shared by
the desktop Tauri app family:

- [hub-gui](/Users/Shared/chroot/dev/kyuubiki/apps/hub-gui)
- [installer-gui](/Users/Shared/chroot/dev/kyuubiki/apps/installer-gui)
- [workbench-gui](/Users/Shared/chroot/dev/kyuubiki/apps/workbench-gui)

This directory is intentionally not used directly as a Tauri `frontendDist`.
Instead, shared files are synchronized into each app's local `ui/` tree so
packaging remains predictable and app-local.

Current shared source files:

- `ui/tauri-bridge.js`
  Desktop-safe Tauri invoke/listen helpers plus light brand loading helpers.
- `ui/desktop-shell.css`
  Shared desktop shell tokens for typography, radius, shadow, and base button
  behavior, plus semantic shell classes such as eyebrow, card-title, stat-label,
  note, and chip.
- `scripts/sync-desktop-shared.mjs`
  Copies the shared helper and the canonical brand manifest into each desktop
  app.

Canonical brand data still lives under:

- [assets/brand/brand.json](/Users/Shared/chroot/dev/kyuubiki/assets/brand/brand.json)
