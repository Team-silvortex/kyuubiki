# Brand Assets

This directory contains curated, versioned product icon assets that are still
useful to the repository.

- `brand/`
  Shared brand metadata and product naming copy.
- `icons/app/`
  Main application icon assets (`.png`, `.icns`, `.ico`), including the shared
  brand mark and per-shell variants for Hub, Installer, and Workbench.
- `icons/dock/`
  Dock-oriented icon assets (`.png`, `.icns`) for future macOS-specific polish.

Design-source files such as `.xcf` and intermediate iconset build trees are not
kept here anymore. This directory is for repository-ready deliverables only.

Desktop shell icon variants are regenerated with:

- [/Users/Shared/chroot/dev/kyuubiki/scripts/generate_desktop_icon_variants](/Users/Shared/chroot/dev/kyuubiki/scripts/generate_desktop_icon_variants)
