# Desktop Release Checklist

Use this checklist when preparing desktop-facing `kyuubiki` deliverables.

Use this page as the execution checklist:

- preflight review
- platform-by-platform verification
- release ordering
- final artifact naming and publication checks

Do not use this page as the main source for:

- runtime mode selection
- packaging architecture explanation
- generated output ownership rules

Those belong to:

- [operations.md](operations.md)
- [packaging-and-deployment.md](packaging-and-deployment.md)

It covers:

- `hub-gui`
- `installer-gui`
- `workbench-gui`
- `macos`
- `linux`
- `windows`

## Release naming convention

Use a predictable versioned prefix for all desktop-facing outputs:

- `kyuubiki-installer-v<version>-<platform>-<bundle>`
- `kyuubiki-hub-v<version>-<platform>-<bundle>`
- `kyuubiki-workbench-v<version>-<platform>-<bundle>`

Examples for the current `1.17.8` workspace-prep line:

- `kyuubiki-installer-v1.17.8-macos-dmg`
- `kyuubiki-hub-v1.17.8-linux-appimage`
- `kyuubiki-workbench-v1.17.8-linux-appimage`
- `kyuubiki-installer-v1.17.8-windows-msi`

Keep these names aligned with:

- the `dist/<platform>/desktop/.../manifest.json` descriptors
- release notes
- uploaded artifacts in CI or manual releases

## Shared preflight

- Review current readiness:
  - `./scripts/kyuubiki desktop-status all`
- If the release includes workflow builder, operator search, package-import,
  dataset editor, or workflow integrity UI changes:
  - start `npm run dev` under `apps/frontend` in a separate shell
  - run `./scripts/kyuubiki workflow-preflight`
- Confirm brand assets exist under:
  - [assets/icons/app](../assets/icons/app)
- Confirm desktop icon copies exist under:
  - [apps/hub-gui/src-tauri/icons](../apps/hub-gui/src-tauri/icons)
  - [apps/installer-gui/src-tauri/icons](../apps/installer-gui/src-tauri/icons)
  - [apps/workbench-gui/src-tauri/icons](../apps/workbench-gui/src-tauri/icons)
- Confirm runtime scaffold exists:
  - `./scripts/kyuubiki package-runtime`
- Confirm desktop manifests exist:
  - `./scripts/kyuubiki package-desktop all`

## macOS

Expected desktop bundle kinds:

- `.app`
- `.dmg`

Icon inputs:

- `.png`
- `.icns`

Typical commands:

- `./scripts/kyuubiki build-installer-gui macos`
- `./scripts/kyuubiki build-hub-gui macos`
- `./scripts/kyuubiki build-workbench-gui macos`
- `./scripts/kyuubiki package-desktop macos`
- `./scripts/kyuubiki desktop-verify macos`

Staged descriptor paths:

- [dist/macos/desktop/hub-gui](../dist/macos/desktop/hub-gui)
- [dist/macos/desktop/installer-gui](../dist/macos/desktop/installer-gui)
- [dist/macos/desktop/workbench-gui](../dist/macos/desktop/workbench-gui)

Suggested verification:

- confirm `.icns` is present in each Tauri icon directory
- confirm staged manifest declares `app` and `dmg`
- confirm runtime scaffold exists under `dist/macos`

## Linux

Expected desktop bundle kinds:

- `AppImage`
- `deb`
- `rpm`

Icon inputs:

- `.png`

Typical commands:

- `./scripts/kyuubiki build-installer-gui linux`
- `./scripts/kyuubiki build-hub-gui linux`
- `./scripts/kyuubiki build-workbench-gui linux`
- `./scripts/kyuubiki package-desktop linux`
- `./scripts/kyuubiki desktop-verify linux`

Staged descriptor paths:

- [dist/linux/desktop/hub-gui](../dist/linux/desktop/hub-gui)
- [dist/linux/desktop/installer-gui](../dist/linux/desktop/installer-gui)
- [dist/linux/desktop/workbench-gui](../dist/linux/desktop/workbench-gui)

Suggested verification:

- confirm `.png` is present in each Tauri icon directory
- confirm staged manifest declares `appimage`, `deb`, and `rpm`
- confirm runtime scaffold exists under `dist/linux`

## Windows

Expected desktop bundle kinds:

- `msi`
- `nsis`

Icon inputs:

- `.png`
- `.ico`

Typical commands:

- `./scripts/kyuubiki build-installer-gui windows`
- `./scripts/kyuubiki build-hub-gui windows`
- `./scripts/kyuubiki build-workbench-gui windows`
- `./scripts/kyuubiki package-desktop windows`
- `./scripts/kyuubiki desktop-verify windows`

Staged descriptor paths:

- [dist/windows/desktop/hub-gui](../dist/windows/desktop/hub-gui)
- [dist/windows/desktop/installer-gui](../dist/windows/desktop/installer-gui)
- [dist/windows/desktop/workbench-gui](../dist/windows/desktop/workbench-gui)

Suggested verification:

- confirm `.ico` is present in each Tauri icon directory
- confirm staged manifest declares `msi` and `nsis`
- confirm runtime scaffold exists under `dist/windows`

## Suggested release order

When preparing a release, keep the order stable:

1. Update version notes and changelog.
2. Inspect readiness and missing pieces:
   `./scripts/kyuubiki desktop-status all`
3. If the release touches workflow-heavy frontend surfaces, run:
   `./scripts/kyuubiki workflow-preflight`
4. Refresh runtime scaffold:
   `./scripts/kyuubiki desktop-stage all`
5. Refresh desktop manifests:
   `./scripts/kyuubiki desktop-verify all`
6. Build the current host-platform desktop bundles:
   - `./scripts/kyuubiki desktop-build-host`
7. Run the host release wrapper:
   - `./scripts/kyuubiki desktop-release <host-platform>`
8. Review staged host artifacts:
   - `dist/<host-platform>/desktop/<app>/artifacts`
   - `dist/<host-platform>/desktop/<app>/artifacts.json`
   - `dist/<host-platform>/desktop/artifacts-summary.json`
   - `dist/<host-platform>/desktop/build-summary.json`
   - interpret `build-summary.json` as:
     - `built`: expected bundle kinds are all present
     - `partial`: some host bundle kinds staged, but not the full expected set
     - `failed`: no host bundle staged
   - on macOS, distinguish:
     - `automated session result`: good for validating `.app` bundling and artifact indexing
     - `full desktop terminal result`: the authoritative confirmation that `.dmg` creation also works
9. Verify icon inputs and manifest bundle targets for all three supported platforms.
10. Publish artifacts using the naming convention above.

## Notes

- On a host that does not match the requested platform, `kyuubiki` stages the
  desktop packaging descriptors and directory layout without pretending to
  perform a full cross-platform bundle build.
- The source of truth for bundle metadata is:
  - [docs/packaging-and-deployment.md](packaging-and-deployment.md)
  - [releases/README.md](../releases/README.md)
