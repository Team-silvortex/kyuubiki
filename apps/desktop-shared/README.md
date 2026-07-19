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

- `src/platform.ts`
  TypeScript source for shared desktop platform normalization, labels, and
  release-root helpers. It compiles to `ui/platform.js` before synchronization.
- `src/tauri-bridge.ts`
  TypeScript source for desktop-safe Tauri invoke/listen helpers, language
  preference sync, brand loading, and shared desktop state styling.
- `src/runtime-status-model.ts`
  TypeScript source for the shared runtime/mesh status model, filters, and
  detail selection contract used by the desktop status plane.
- `src/runtime-status-summary.ts`
  TypeScript source for formatting and rendering the shared runtime status
  plane from the typed model contract.
- `ui/desktop-shell.css`
  Shared desktop shell tokens plus runtime status, mesh topology, and shell
  surface styles used across the desktop app family.
- `ui/desktop-shell-runtime-mesh.css`
  Runtime mesh, filter, and topology presentation layer imported by the shared
  desktop shell stylesheet.
- `ui/platform.js`
  Generated JavaScript consumed by installer, hub, and workbench shells.
- `ui/tauri-bridge.js`
  Generated JavaScript bridge consumed by installer, hub, and workbench shells.
- `ui/runtime-status-model.js`
  Generated JavaScript runtime status model consumed by the shared renderer.
- `ui/runtime-status-summary.js`
  Generated JavaScript runtime status renderer consumed by desktop shells.
- `scripts/sync-desktop-shared.mjs`
  Compiles shared TypeScript, then refreshes lightweight app-local wrappers plus
  the canonical brand manifest into each desktop app. Run it with `--check` to
  compile in a temporary directory and verify generated files, app-local
  mirrors, language packs, installer styling, and stale mirror entries without
  writing product assets.

Canonical brand data still lives under:

- [assets/brand/brand.json](../../assets/brand/brand.json)
