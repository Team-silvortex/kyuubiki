# Desktop Shared

`desktop-shared/` holds source-of-truth frontend assets and helpers shared by
the desktop Tauri app family:

- [hub-gui](../hub-gui)
- [installer-gui](../installer-gui)
- [workbench-gui](../workbench-gui)

This directory is intentionally not used directly as a Tauri `frontendDist`.
Instead, shared files are synchronized into each app's local `ui/` tree so
packaging remains predictable and app-local.

Current shared source files:

- `ui/tauri-bridge.js`
  Desktop-safe Tauri invoke/listen helpers plus light brand loading helpers.
- `ui/desktop-shell.css`
  Shared desktop shell tokens plus runtime status, mesh topology, and shell
  surface styles used across the desktop app family.
- `ui/desktop-shell-runtime-mesh.css`
  Runtime mesh, filter, and topology presentation layer imported by the shared
  desktop shell stylesheet.
- `ui/runtime-status-summary.js`
  Shared runtime status formatting and status-plane rendering helpers.
- `ui/platform.js`
  Shared desktop platform normalization, labels, and release-root helpers used
  by installer, hub, and workbench shells.
- `scripts/sync-desktop-shared.mjs`
  Refreshes lightweight app-local wrappers plus the canonical brand manifest
  into each desktop app.

Canonical brand data still lives under:

- [assets/brand/brand.json](../../assets/brand/brand.json)
