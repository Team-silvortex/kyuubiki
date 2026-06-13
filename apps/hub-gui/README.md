# Hub GUI

This app is the unified desktop entrypoint and workload shell for `kyuubiki`.

In the `tamamono 1.x` product shape, Hub is the everyday desktop entrypoint
for launch, runtime target overview, bundle intake, and operator guidance.

It sits above:

- [installer-gui](../installer-gui)
- [workbench-gui](../workbench-gui)

Its job is not to replace the modeling workbench or the deployment installer.
Its job is to sit above those runtime-facing surfaces and give one short runway
into the right next shell.

## Responsibilities

- desktop entry shell for `Workbench`, `Installer`, and runtime-facing tools
- local and remote workload intake through bundle tools and workload catalogs
- runtime target overview, health visibility, and operator-facing diagnostics
- guided assistant entrypoint with local hints and optional model-backed plans
- desktop packaging readiness and short preflight checks before handing off to
  `Installer` for heavier deployment work

Quick launch behavior now prefers an already-built host desktop bundle when one
exists, and falls back to the repo-local `tauri:dev` shell during development.

## Main paths

- UI shell:
  [ui/](ui)
- Desktop docs shelf:
  [ui/docs](ui/docs)
- Tauri backend:
  [src-tauri/](src-tauri)
- Packaged icons:
  [src-tauri/icons](src-tauri/icons)
- Product split and IA notes:
  [docs/hub-architecture.md](../../docs/hub-architecture.md)

## Commands

- `npm run sync:shared`
- `make hub-gui-dev`
- `make hub-gui-build`
- `make hot-local`
- `make hot-cloud`
- `make hot-distributed`
- `make test-hub-gui`
- `make desktop-status PLATFORM=all`
- `make desktop-build-host`
- `make desktop-verify PLATFORM=macos|linux|windows`
- `./scripts/kyuubiki build-hub-gui macos|linux|windows`
- `./scripts/kyuubiki package-desktop macos|linux|windows`

## Validation

- shared UI sync:
  `cd apps/hub-gui && npm run sync:shared`
- smoke test:
  `cd apps/hub-gui && npm run test:smoke`
- Tauri shell check:
  `cargo check --offline --manifest-path src-tauri/Cargo.toml`

## Output

Tauri build output lands under:

- `apps/hub-gui/src-tauri/target`

Platform-scoped staged desktop manifests land under:

- `dist/<platform>/desktop/hub-gui`

Do not treat that directory as source-owned. The source of truth is:

- the Hub Tauri shell source in this app
- the shared desktop runtime crate
- the repository-level desktop packaging flow
- the repository-level documentation under `docs/`, with `ui/docs/` as the
  desktop-facing mirror/shelf for shorter operator reading
- [docs/hub-architecture.md](../../docs/hub-architecture.md)
- [deploy/workload-catalog.example.json](../../deploy/workload-catalog.example.json)
- [docs/packaging-and-deployment.md](../../docs/packaging-and-deployment.md)
