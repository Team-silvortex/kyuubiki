# Installer Icons

This directory contains the packaged desktop icons for the Tauri installer.

- `kyuubiki-app.png`
  Main application icon used for the installer window/app bundle.
- `kyuubiki-app.icns`
  macOS application icon bundle.
- `kyuubiki-app.ico`
  Windows application icon bundle.
- `kyuubiki-dock.png`
  Dock-oriented variant kept for future macOS dock-specific packaging polish.
- `kyuubiki-dock.icns`
  macOS dock-oriented icon bundle kept alongside the app icon set.

Source artwork lives under:

- [assets/icons/app](/Users/Shared/chroot/dev/kyuubiki/assets/icons/app)
- [assets/icons/dock](/Users/Shared/chroot/dev/kyuubiki/assets/icons/dock)

This shell now uses the Installer-specific badge variant generated from:

- [/Users/Shared/chroot/dev/kyuubiki/assets/icons/app/kyuubiki-installer.png](/Users/Shared/chroot/dev/kyuubiki/assets/icons/app/kyuubiki-installer.png)
- [/Users/Shared/chroot/dev/kyuubiki/scripts/generate_desktop_icon_variants](/Users/Shared/chroot/dev/kyuubiki/scripts/generate_desktop_icon_variants)
