# Desktop Release Checklist

Use this checklist when preparing desktop-facing `kyuubiki` deliverables.

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

Examples:

- `kyuubiki-installer-v0.9.0-macos-dmg`
- `kyuubiki-hub-v0.9.0-linux-appimage`
- `kyuubiki-workbench-v0.9.0-linux-appimage`
- `kyuubiki-installer-v0.9.0-windows-msi`

Keep these names aligned with:

- the `dist/<platform>/desktop/.../manifest.json` descriptors
- release notes
- uploaded artifacts in CI or manual releases

## Shared preflight

- Review current readiness:
  - `zsh ./scripts/kyuubiki desktop-status all`
- Confirm brand assets exist under:
  - [assets/icons/app](/Users/Shared/chroot/dev/kyuubiki/assets/icons/app)
  - [assets/icons/dock](/Users/Shared/chroot/dev/kyuubiki/assets/icons/dock)
- Confirm desktop icon copies exist under:
  - [apps/hub-gui/src-tauri/icons](/Users/Shared/chroot/dev/kyuubiki/apps/hub-gui/src-tauri/icons)
  - [apps/installer-gui/src-tauri/icons](/Users/Shared/chroot/dev/kyuubiki/apps/installer-gui/src-tauri/icons)
  - [apps/workbench-gui/src-tauri/icons](/Users/Shared/chroot/dev/kyuubiki/apps/workbench-gui/src-tauri/icons)
- Confirm runtime scaffold exists:
  - `zsh ./scripts/kyuubiki package-runtime`
- Confirm desktop manifests exist:
  - `zsh ./scripts/kyuubiki package-desktop all`

## macOS

Expected desktop bundle kinds:

- `.app`
- `.dmg`

Icon inputs:

- `.png`
- `.icns`

Typical commands:

- `zsh ./scripts/kyuubiki build-installer-gui macos`
- `zsh ./scripts/kyuubiki build-hub-gui macos`
- `zsh ./scripts/kyuubiki build-workbench-gui macos`
- `zsh ./scripts/kyuubiki package-desktop macos`
- `zsh ./scripts/kyuubiki desktop-verify macos`

Staged descriptor paths:

- [dist/macos/desktop/hub-gui](/Users/Shared/chroot/dev/kyuubiki/dist/macos/desktop/hub-gui)
- [dist/macos/desktop/installer-gui](/Users/Shared/chroot/dev/kyuubiki/dist/macos/desktop/installer-gui)
- [dist/macos/desktop/workbench-gui](/Users/Shared/chroot/dev/kyuubiki/dist/macos/desktop/workbench-gui)

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

- `zsh ./scripts/kyuubiki build-installer-gui linux`
- `zsh ./scripts/kyuubiki build-hub-gui linux`
- `zsh ./scripts/kyuubiki build-workbench-gui linux`
- `zsh ./scripts/kyuubiki package-desktop linux`
- `zsh ./scripts/kyuubiki desktop-verify linux`

Staged descriptor paths:

- [dist/linux/desktop/hub-gui](/Users/Shared/chroot/dev/kyuubiki/dist/linux/desktop/hub-gui)
- [dist/linux/desktop/installer-gui](/Users/Shared/chroot/dev/kyuubiki/dist/linux/desktop/installer-gui)
- [dist/linux/desktop/workbench-gui](/Users/Shared/chroot/dev/kyuubiki/dist/linux/desktop/workbench-gui)

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

- `zsh ./scripts/kyuubiki build-installer-gui windows`
- `zsh ./scripts/kyuubiki build-hub-gui windows`
- `zsh ./scripts/kyuubiki build-workbench-gui windows`
- `zsh ./scripts/kyuubiki package-desktop windows`
- `zsh ./scripts/kyuubiki desktop-verify windows`

Staged descriptor paths:

- [dist/windows/desktop/hub-gui](/Users/Shared/chroot/dev/kyuubiki/dist/windows/desktop/hub-gui)
- [dist/windows/desktop/installer-gui](/Users/Shared/chroot/dev/kyuubiki/dist/windows/desktop/installer-gui)
- [dist/windows/desktop/workbench-gui](/Users/Shared/chroot/dev/kyuubiki/dist/windows/desktop/workbench-gui)

Suggested verification:

- confirm `.ico` is present in each Tauri icon directory
- confirm staged manifest declares `msi` and `nsis`
- confirm runtime scaffold exists under `dist/windows`

## Suggested release order

When preparing a release, keep the order stable:

1. Update version notes and changelog.
2. Inspect readiness and missing pieces:
   `zsh ./scripts/kyuubiki desktop-status all`
3. Refresh runtime scaffold:
   `zsh ./scripts/kyuubiki desktop-stage all`
4. Refresh desktop manifests:
   `zsh ./scripts/kyuubiki desktop-verify all`
5. Build the current host-platform desktop bundles:
   - `zsh ./scripts/kyuubiki desktop-build-host`
6. Run the host release wrapper:
   - `zsh ./scripts/kyuubiki desktop-release <host-platform>`
7. Verify icon inputs and manifest bundle targets for all three supported platforms.
8. Publish artifacts using the naming convention above.

## Notes

- On a host that does not match the requested platform, `kyuubiki` stages the
  desktop packaging descriptors and directory layout without pretending to
  perform a full cross-platform bundle build.
- The source of truth for bundle metadata is:
  - [docs/packaging-and-deployment.md](/Users/Shared/chroot/dev/kyuubiki/docs/packaging-and-deployment.md)
  - [dist/README.md](/Users/Shared/chroot/dev/kyuubiki/dist/README.md)
