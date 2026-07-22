# Workbench GUI

This app is the native desktop shell for the browser-first Kyuubiki workbench.

It is intentionally thin:

- embeds the local workbench at `http://127.0.0.1:3000`
- exposes native `start / restart / stop / status` controls for the local stack
- exposes quick log viewing for `frontend`, `orchestrator`, and bundled agents
- keeps the workbench product logic separate from the desktop wrapper

## Responsibilities

- native window shell for the local workbench
- local runtime lifecycle control
- local runtime status and log visibility
- lightweight desktop wrapper around the browser workbench

## Main paths

- UI shell:
  [ui/](ui)
- Tauri backend:
  [src-tauri/](src-tauri)
- Packaged icons:
  [src-tauri/icons](src-tauri/icons)
- UI assets:
  [ui/assets](ui/assets)

## Commands

- `npm run sync:shared`
- `make workbench-gui-dev`
- `make workbench-gui-build`
- `make test-workbench-gui`
- `make desktop-status PLATFORM=all`
- `make package-desktop PLATFORM=all`
- `make desktop-build-host`
- `make desktop-verify PLATFORM=macos|linux|windows`
- `./scripts/kyuubiki build-workbench-gui macos|linux|windows`
- `./scripts/kyuubiki package-desktop macos|linux|windows`

The native dev/build/package commands synchronize shared desktop assets
automatically. Use `npm run sync:shared` only for an explicit local refresh or check.

## Validation

- shared UI sync:
  `cd apps/workbench-gui && npm run sync:shared`
- smoke test:
  `cd apps/workbench-gui && npm run test:smoke`
- Tauri shell check:
  `cargo check --offline --manifest-path src-tauri/Cargo.toml`

## Output

Staged platform descriptors land under:

- `dist/<platform>/desktop/workbench-gui`
